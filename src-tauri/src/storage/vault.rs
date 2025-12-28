use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use argon2::{Algorithm, Argon2, Params, Version};
use blake3::Hasher as Blake3;
use chrono::{DateTime, Utc};
use rand::rngs::OsRng;
use rand::RngCore;
use ring::aead::{self, Aad, LessSafeKey, Nonce, UnboundKey};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

use super::WalletPaths;
use crate::errors::{WalletError, WalletResult};

const VAULT_MAGIC: &[u8; 8] = b"CHERTWLT";
const VAULT_VERSION: u16 = 1;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

/// Metadata stored alongside encrypted wallet secrets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultMetadata {
    /// Human-readable wallet name.
    pub wallet_name: String,
    /// Timestamp when the vault was created.
    pub created_at: DateTime<Utc>,
    /// Timestamp when the vault was last modified.
    pub updated_at: DateTime<Utc>,
    /// Version identifier for future migrations.
    pub schema_version: u16,
    /// Primary wallet address associated with the vault (if known).
    #[serde(default)]
    pub primary_address: Option<String>,
    /// Public key (hex) associated with the wallet identity, if available.
    #[serde(default)]
    pub public_key_hex: Option<String>,
    /// Signature algorithm in use (e.g. Ed25519, Dilithium2).
    #[serde(default)]
    pub signature_algorithm: Option<String>,
    /// Whether the wallet supports post-quantum cryptography.
    #[serde(default)]
    pub supports_post_quantum: Option<bool>,
}

impl VaultMetadata {
    pub fn new(wallet_name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            wallet_name: wallet_name.into(),
            created_at: now,
            updated_at: now,
            schema_version: VAULT_VERSION,
            primary_address: None,
            public_key_hex: None,
            signature_algorithm: None,
            supports_post_quantum: None,
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

/// Secrets encrypted within the vault.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Zeroize)]
#[zeroize(drop)]
pub struct VaultSecrets {
    /// Mnemonic phrase (optional) for wallet recovery.
    pub mnemonic_phrase: Option<String>,
    /// Seed bytes used to derive keys (BIP39/BIP32 or custom derivation).
    pub seed_bytes: Vec<u8>,
    /// Serialized stealth key material (view/spend keys, encrypted memos, etc.).
    pub stealth_material: Vec<u8>,
    /// Serialized PQ key material for future use.
    pub pq_material: Vec<u8>,
}

impl VaultSecrets {
    pub fn new(seed_bytes: Vec<u8>) -> Self {
        Self {
            mnemonic_phrase: None,
            seed_bytes,
            stealth_material: Vec::new(),
            pq_material: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VaultFile {
    magic: [u8; 8],
    version: u16,
    nonce: [u8; NONCE_LEN],
    kdf: KdfParameters,
    checksum: [u8; 32],
    ciphertext: Vec<u8>,
    metadata: VaultMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KdfParameters {
    m_cost_kib: u32,
    t_cost: u32,
    p_cost: u32,
    salt: [u8; SALT_LEN],
}

impl Default for KdfParameters {
    fn default() -> Self {
        Self {
            m_cost_kib: 256 * 1024, // 256 MiB
            t_cost: 4,
            p_cost: 1,
            salt: [0u8; SALT_LEN],
        }
    }
}

/// Parameters required to create a new vault on disk.
pub struct VaultCreateParams<'a> {
    pub password: &'a SecretString,
    pub metadata: VaultMetadata,
    pub secrets: VaultSecrets,
}

/// Result returned after successfully unlocking a vault.
#[derive(Debug, Clone)]
pub struct VaultUnlocked {
    pub metadata: VaultMetadata,
    pub secrets: VaultSecrets,
}

/// Handles persistence and encryption of the wallet vault file.
#[derive(Debug, Clone)]
pub struct VaultManager {
    vault_path: PathBuf,
    wallet_paths: Option<WalletPaths>,
}

impl VaultManager {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            vault_path: path.as_ref().to_path_buf(),
            wallet_paths: None,
        }
    }

    pub fn from_paths(paths: &WalletPaths) -> Self {
        Self {
            vault_path: paths.vault_file().to_path_buf(),
            wallet_paths: Some(paths.clone()),
        }
    }

    pub fn vault_path(&self) -> &Path {
        &self.vault_path
    }

    /// Create a new encrypted vault on disk. Fails if a vault already exists.
    pub fn create(&self, params: VaultCreateParams<'_>) -> WalletResult<()> {
        if self.vault_path.exists() {
            return Err(WalletError::AlreadyExists(
                self.vault_path.display().to_string(),
            ));
        }

        let mut file = create_atomic_file(&self.vault_path)?;
        let encrypted = self.encrypt_payload(params)?;
        let serialized = serde_json::to_vec(&encrypted)?;
        file.write_all(&serialized)?;
        file.sync_all()?;
        finalize_atomic_file(file, &self.vault_path)?;
        Ok(())
    }

    /// Overwrite an existing vault file with new secrets and metadata.
    pub fn update(&self, params: VaultCreateParams<'_>) -> WalletResult<()> {
        self.snapshot_existing_vault()?;
        let mut file = create_atomic_file(&self.vault_path)?;
        let encrypted = self.encrypt_payload(params)?;
        let serialized = serde_json::to_vec(&encrypted)?;
        file.write_all(&serialized)?;
        file.sync_all()?;
        finalize_atomic_file(file, &self.vault_path)?;
        Ok(())
    }

    /// Unlock the vault and return decrypted secrets.
    pub fn unlock(&self, password: &SecretString) -> WalletResult<VaultUnlocked> {
        let vault_file = self.read_vault_file()?;
        let plaintext = self.decrypt_payload(password, &vault_file)?;
        let computed_checksum = blake3_checksum(&plaintext);
        if computed_checksum != vault_file.checksum {
            return Err(WalletError::ValidationError(
                "Vault integrity verification failed".to_string(),
            ));
        }

        let secrets: VaultSecrets = serde_json::from_slice(&plaintext)?;
        Ok(VaultUnlocked {
            metadata: vault_file.metadata,
            secrets,
        })
    }

    /// Read vault metadata without decrypting secrets.
    pub fn read_metadata(&self) -> WalletResult<Option<VaultMetadata>> {
        if !self.exists() {
            return Ok(None);
        }

        let vault_file = self.read_vault_file()?;
        Ok(Some(vault_file.metadata))
    }

    /// Change the password by re-encrypting the existing vault with a new password.
    pub fn change_password(
        &self,
        current_password: &SecretString,
        new_password: &SecretString,
    ) -> WalletResult<()> {
        let unlocked = self.unlock(current_password)?;
        let mut metadata = unlocked.metadata.clone();
        metadata.touch();
        let params = VaultCreateParams {
            password: new_password,
            metadata,
            secrets: unlocked.secrets,
        };
        self.update(params)
    }

    /// Check if the vault file exists on disk.
    pub fn exists(&self) -> bool {
        self.vault_path.exists()
    }

    /// List available vault backups ordered by newest first.
    pub fn available_backups(&self) -> WalletResult<Vec<PathBuf>> {
        if let Some(paths) = &self.wallet_paths {
            return paths.list_backups();
        }

        Ok(Vec::new())
    }

    /// Restore the vault state from a specific backup file.
    pub fn restore_from_backup(&self, backup_path: &Path) -> WalletResult<()> {
        let paths = self.wallet_paths.as_ref().ok_or_else(|| {
            WalletError::StorageError("Vault manager configured without wallet paths".to_string())
        })?;

        paths.restore_vault_from_backup(backup_path)?;
        Ok(())
    }

    fn read_vault_file(&self) -> WalletResult<VaultFile> {
        let bytes = fs::read(&self.vault_path)?;
        let vault_file: VaultFile = serde_json::from_slice(&bytes)?;

        if &vault_file.magic != VAULT_MAGIC {
            return Err(WalletError::ValidationError(
                "Invalid vault magic marker".to_string(),
            ));
        }

        if vault_file.version != VAULT_VERSION {
            return Err(WalletError::ValidationError(format!(
                "Unsupported vault version: {}",
                vault_file.version
            )));
        }

        Ok(vault_file)
    }

    fn encrypt_payload(&self, params: VaultCreateParams<'_>) -> WalletResult<VaultFile> {
        let mut rng = OsRng;
        let mut salt = [0u8; SALT_LEN];
        rng.fill_bytes(&mut salt);

        let mut nonce_bytes = [0u8; NONCE_LEN];
        rng.fill_bytes(&mut nonce_bytes);

        let kdf_params = KdfParameters {
            salt,
            ..Default::default()
        };

        let key = derive_key(params.password, &kdf_params)?;
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);

        let json_secrets = serde_json::to_vec(&params.secrets)?;
        let checksum = blake3_checksum(&json_secrets);

        let mut buffer = Zeroizing::new(json_secrets);
        let ciphertext = encrypt_aes_gcm(&key, nonce, &mut buffer)?;

        Ok(VaultFile {
            magic: *VAULT_MAGIC,
            version: VAULT_VERSION,
            nonce: nonce_bytes,
            kdf: kdf_params,
            checksum,
            ciphertext,
            metadata: params.metadata,
        })
    }

    fn decrypt_payload(
        &self,
        password: &SecretString,
        vault_file: &VaultFile,
    ) -> WalletResult<Zeroizing<Vec<u8>>> {
        let key = derive_key(password, &vault_file.kdf)?;
        let nonce = Nonce::assume_unique_for_key(vault_file.nonce);
        decrypt_aes_gcm(&key, nonce, &vault_file.ciphertext)
    }

    fn snapshot_existing_vault(&self) -> WalletResult<()> {
        if let Some(paths) = &self.wallet_paths {
            if self.exists() {
                let backup_path = paths.create_vault_backup()?;
                debug_assert!(
                    backup_path.exists(),
                    "backup path should exist after creation"
                );
            }
        }
        Ok(())
    }
}

fn derive_key(
    password: &SecretString,
    params: &KdfParameters,
) -> WalletResult<Zeroizing<[u8; KEY_LEN]>> {
    let argon_params = Params::new(
        params.m_cost_kib,
        params.t_cost,
        params.p_cost,
        Some(KEY_LEN),
    )
    .map_err(|e| WalletError::CryptoError(format!("Invalid Argon2 params: {e}")))?;

    let argon2 = Argon2::new_with_secret(&[], Algorithm::Argon2id, Version::V0x13, argon_params)
        .map_err(|e| WalletError::CryptoError(format!("Failed to init Argon2: {e}")))?;

    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    argon2
        .hash_password_into(
            password.expose_secret().as_bytes(),
            &params.salt,
            key.as_mut(),
        )
        .map_err(|e| WalletError::CryptoError(format!("KDF failed: {e}")))?;
    Ok(key)
}

fn encrypt_aes_gcm(
    key: &Zeroizing<[u8; KEY_LEN]>,
    nonce: Nonce,
    buffer: &mut Zeroizing<Vec<u8>>,
) -> WalletResult<Vec<u8>> {
    let unbound_key = UnboundKey::new(&aead::AES_256_GCM, key.as_ref())
        .map_err(|e| WalletError::CryptoError(format!("Invalid encryption key: {e}")))?;
    let key = LessSafeKey::new(unbound_key);

    let mut in_out: Vec<u8> = buffer.iter().copied().collect();
    key.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
        .map_err(|_| WalletError::CryptoError("Encryption failure".to_string()))?;
    Ok(in_out)
}

fn decrypt_aes_gcm(
    key: &Zeroizing<[u8; KEY_LEN]>,
    nonce: Nonce,
    ciphertext: &[u8],
) -> WalletResult<Zeroizing<Vec<u8>>> {
    let unbound_key = UnboundKey::new(&aead::AES_256_GCM, key.as_ref())
        .map_err(|e| WalletError::CryptoError(format!("Invalid encryption key: {e}")))?;
    let key = LessSafeKey::new(unbound_key);

    if ciphertext.len() < aead::AES_256_GCM.tag_len() {
        return Err(WalletError::CryptoError(
            "Ciphertext shorter than authentication tag".to_string(),
        ));
    }

    let mut in_out = Zeroizing::new(ciphertext.to_vec());
    let plaintext = key
        .open_in_place(nonce, Aad::empty(), &mut in_out)
        .map_err(|_| WalletError::CryptoError("Decryption failure".to_string()))?;
    let plaintext_len = plaintext.len();
    in_out.truncate(plaintext_len);
    Ok(in_out)
}

fn blake3_checksum(data: &[u8]) -> [u8; 32] {
    let mut hasher = Blake3::new();
    hasher.update(data);
    let mut output = [0u8; 32];
    output.copy_from_slice(hasher.finalize().as_bytes());
    output
}

fn create_atomic_file(path: &Path) -> WalletResult<File> {
    let dir = path
        .parent()
        .ok_or_else(|| WalletError::StorageError("Invalid vault path".to_string()))?;
    fs::create_dir_all(dir)?;
    let tmp_path = path.with_extension("new");
    Ok(File::create(&tmp_path)?)
}

fn finalize_atomic_file(mut file: File, final_path: &Path) -> WalletResult<()> {
    file.flush()?;
    drop(file);
    let tmp_path = final_path.with_extension("new");
    fs::rename(tmp_path, final_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::WalletPaths;
    use tempfile::TempDir;

    fn secret(password: &str) -> SecretString {
        SecretString::from(password.to_string())
    }

    #[test]
    fn create_and_unlock_vault_round_trip() {
        let dir = TempDir::new().unwrap();
        let vault_path = dir.path().join("wallet.vault");
        let manager = VaultManager::new(&vault_path);

        let metadata = VaultMetadata::new("Test Wallet");
        let secrets = VaultSecrets {
            mnemonic_phrase: Some("abandon abandon abandon".into()),
            seed_bytes: vec![1, 2, 3, 4],
            stealth_material: vec![9, 9, 9],
            pq_material: vec![7, 7],
        };
        let params = VaultCreateParams {
            password: &secret("correct horse battery staple"),
            metadata,
            secrets: secrets.clone(),
        };

        manager.create(params).unwrap();
        assert!(manager.exists());

        let unlocked = manager
            .unlock(&secret("correct horse battery staple"))
            .unwrap();
        assert_eq!(unlocked.secrets, secrets);
        assert_eq!(unlocked.metadata.wallet_name, "Test Wallet");
    }

    #[test]
    fn unlocking_with_wrong_password_fails() {
        let dir = TempDir::new().unwrap();
        let manager = VaultManager::new(dir.path().join("wallet.vault"));

        let params = VaultCreateParams {
            password: &secret("hunter2"),
            metadata: VaultMetadata::new("Guarded"),
            secrets: VaultSecrets::new(vec![0; 32]),
        };

        manager.create(params).unwrap();
        let result = manager.unlock(&secret("incorrect"));
        assert!(matches!(
            result,
            Err(WalletError::CryptoError(_)) | Err(WalletError::ValidationError(_))
        ));
    }

    #[test]
    fn change_password_re_encrypts_vault() {
        let dir = TempDir::new().unwrap();
        let manager = VaultManager::new(dir.path().join("wallet.vault"));

        let params = VaultCreateParams {
            password: &secret("old password"),
            metadata: VaultMetadata::new("Password Change"),
            secrets: VaultSecrets::new(vec![1, 1, 2, 3, 5, 8]),
        };
        manager.create(params).unwrap();

        manager
            .change_password(&secret("old password"), &secret("new password"))
            .unwrap();

        // Old password should fail
        assert!(manager.unlock(&secret("old password")).is_err());
        // New password should succeed
        assert!(manager.unlock(&secret("new password")).is_ok());
    }

    #[test]
    fn tampered_ciphertext_is_detected() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("wallet.vault");
        let manager = VaultManager::new(&path);

        let params = VaultCreateParams {
            password: &secret("tamper test"),
            metadata: VaultMetadata::new("Tamper"),
            secrets: VaultSecrets::new(vec![42; 16]),
        };
        manager.create(params).unwrap();

        let mut bytes = fs::read(&path).unwrap();
        // flip some bits inside ciphertext
        if let Some(byte) = bytes.iter_mut().find(|b| **b != 0) {
            *byte ^= 0xFF;
        }
        fs::write(&path, &bytes).unwrap();

        let result = manager.unlock(&secret("tamper test"));
        assert!(result.is_err());
    }

    #[test]
    fn metadata_can_be_read_without_unlock() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("wallet.vault");
        let manager = VaultManager::new(&path);

        let params = VaultCreateParams {
            password: &secret("metadata"),
            metadata: VaultMetadata::new("Metadata Test"),
            secrets: VaultSecrets::new(vec![5; 8]),
        };
        manager.create(params).unwrap();

        let metadata = manager.read_metadata().unwrap().expect("metadata present");
        assert_eq!(metadata.wallet_name, "Metadata Test");
    }

    #[test]
    fn update_creates_backup_when_wallet_paths_configured() {
        let dir = TempDir::new().unwrap();
        let paths = WalletPaths::new(dir.path()).unwrap();
        paths.ensure_directories().unwrap();

        let manager = VaultManager::from_paths(&paths);

        let initial_params = VaultCreateParams {
            password: &secret("initial"),
            metadata: VaultMetadata::new("Backup Test"),
            secrets: VaultSecrets::new(vec![1, 2, 3, 4]),
        };
        manager.create(initial_params).unwrap();

        let update_params = VaultCreateParams {
            password: &secret("initial"),
            metadata: VaultMetadata::new("Backup Test"),
            secrets: VaultSecrets::new(vec![9, 9, 9, 9]),
        };
        manager.update(update_params).unwrap();

        let backups = manager.available_backups().unwrap();
        assert_eq!(backups.len(), 1, "expected exactly one backup after update");

        let backup_content = fs::read(&backups[0]).unwrap();
        let vault_content = fs::read(paths.vault_file()).unwrap();
        assert_ne!(
            backup_content, vault_content,
            "backup should capture previous vault state"
        );
    }

    #[test]
    fn restore_from_backup_uses_wallet_paths_logic() {
        let dir = TempDir::new().unwrap();
        let paths = WalletPaths::new(dir.path()).unwrap();
        paths.ensure_directories().unwrap();

        let manager = VaultManager::from_paths(&paths);

        let initial_params = VaultCreateParams {
            password: &secret("restore"),
            metadata: VaultMetadata::new("Restore Test"),
            secrets: VaultSecrets::new(b"original".to_vec()),
        };
        manager.create(initial_params).unwrap();

        let mut updated_metadata = VaultMetadata::new("Restore Test");
        updated_metadata.touch();
        let update_params = VaultCreateParams {
            password: &secret("restore"),
            metadata: updated_metadata,
            secrets: VaultSecrets::new(b"updated".to_vec()),
        };
        manager.update(update_params).unwrap();

        let backups = manager.available_backups().unwrap();
        assert!(!backups.is_empty(), "expected at least one backup");
        let latest_backup = &backups[0];

        fs::write(paths.vault_file(), b"corrupted").unwrap();
        manager.restore_from_backup(latest_backup).unwrap();

        let unlocked = manager
            .unlock(&secret("restore"))
            .expect("unlock succeeds after restore");
        assert_eq!(unlocked.secrets.seed_bytes, b"original");
    }
}
