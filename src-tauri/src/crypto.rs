/// Simplified wallet cryptography module using shared silica-models
///
/// This module focuses on wallet-specific functionality while delegating
/// core cryptographic operations to the shared models for consistency.
use crate::errors::{WalletError, WalletResult};
use curve25519_dalek::scalar::Scalar;
use parking_lot::Mutex;
use pqcrypto_internals::{RandomBytesOverrideGuard, DEFAULT_RANDOMBYTES};
use pqcrypto_traits::sign::{PublicKey as PQPublicKey, SecretKey as PQSecretKey};
use serde::{Deserialize, Serialize};
use sha3::digest::{Digest as ShaDigest, ExtendableOutput, Update as ShakeUpdate, XofReader};
use sha3::{Sha3_256, Shake256};
use silica_models::crypto::{utils, ChertKeyPair, ChertSignature, SignatureAlgorithm};
use std::slice;
use std::sync::OnceLock;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

const MAX_DILITHIUM_SEED_BYTES: usize = 1024;
const MAX_DILITHIUM_RANDOM_REQUEST: usize = 65_536;
const DILITHIUM_DOMAIN_SEPARATOR: &[u8] = b"chert-dilithium-deterministic-v1";
const STEALTH_VIEW_DERIVATION_DOMAIN: &str = "chert.wallet.stealth.view.v1";
const STEALTH_SPEND_DERIVATION_DOMAIN: &str = "chert.wallet.stealth.spend.v1";
const STEALTH_KEY_MATERIAL_VERSION: u8 = 1;
const STEALTH_KEY_MATERIAL_V1_LEN: usize = 1 + 32 + 32;
const STEALTH_SEED_MAX_BYTES: usize = 4 * 1024;

static DILITHIUM_RNG_STATE: OnceLock<Mutex<Option<DeterministicRandomState>>> = OnceLock::new();

#[derive(Zeroize)]
struct DeterministicRandomState {
    seed: [u8; 32],
    counter: u64,
}

impl DeterministicRandomState {
    fn new(seed: [u8; 32]) -> Self {
        assert_eq!(seed.len(), 32, "Deterministic seed must be 32 bytes");
        assert!(
            seed.iter().any(|&byte| byte != 0),
            "Deterministic seed must contain entropy"
        );
        Self { seed, counter: 0 }
    }

    fn fill(&mut self, out: &mut [u8]) {
        assert!(
            out.len() <= MAX_DILITHIUM_RANDOM_REQUEST,
            "Random request exceeds static limit"
        );
        assert!(
            self.counter < u64::MAX,
            "Deterministic RNG counter overflow"
        );

        if out.is_empty() {
            return;
        }

        let mut hasher = Shake256::default();
        ShakeUpdate::update(&mut hasher, DILITHIUM_DOMAIN_SEPARATOR);
        ShakeUpdate::update(&mut hasher, &self.seed);
        let counter_bytes = self.counter.to_be_bytes();
        ShakeUpdate::update(&mut hasher, &counter_bytes);
        let mut reader = hasher.finalize_xof();
        reader.read(out);
        self.counter = self
            .counter
            .checked_add(1)
            .expect("Deterministic RNG counter overflowed");
    }
}

impl Drop for DeterministicRandomState {
    fn drop(&mut self) {
        self.zeroize();
    }
}

struct DeterministicOverrideGuard {
    _provider_guard: RandomBytesOverrideGuard,
}

impl Drop for DeterministicOverrideGuard {
    fn drop(&mut self) {
        let state_mutex = DILITHIUM_RNG_STATE.get_or_init(|| Mutex::new(None));
        let mut state_guard = state_mutex.lock();
        if let Some(mut state) = state_guard.take() {
            state.zeroize();
        }
    }
}

fn install_deterministic_rng(seed: [u8; 32]) -> WalletResult<DeterministicOverrideGuard> {
    assert_eq!(seed.len(), 32, "Deterministic seed must be 32 bytes");
    assert!(
        seed.iter().any(|&byte| byte != 0),
        "Deterministic seed must contain entropy"
    );

    let state_mutex = DILITHIUM_RNG_STATE.get_or_init(|| Mutex::new(None));
    let mut state_guard = state_mutex.lock();
    assert!(state_guard.is_none(), "Deterministic RNG already active");
    *state_guard = Some(DeterministicRandomState::new(seed));
    drop(state_guard);

    let provider_guard = unsafe { RandomBytesOverrideGuard::install(deterministic_randombytes) };
    Ok(DeterministicOverrideGuard {
        _provider_guard: provider_guard,
    })
}

unsafe extern "C" fn deterministic_randombytes(buf: *mut u8, len: libc::size_t) -> libc::c_int {
    assert!(
        !buf.is_null(),
        "Random byte buffer pointer must not be null"
    );
    assert!(
        len <= MAX_DILITHIUM_RANDOM_REQUEST,
        "Random request exceeds static limit"
    );

    let state_mutex = DILITHIUM_RNG_STATE.get_or_init(|| Mutex::new(None));
    let mut state_guard = state_mutex.lock();
    if let Some(state) = state_guard.as_mut() {
        let target_len = len;
        if target_len == 0 {
            return 0;
        }
        let out = slice::from_raw_parts_mut(buf, target_len);
        state.fill(out);
        0
    } else {
        drop(state_guard);
        DEFAULT_RANDOMBYTES(buf, len)
    }
}

fn derive_dilithium_key_material(seed: &[u8]) -> WalletResult<(Vec<u8>, Vec<u8>)> {
    use pqcrypto_dilithium::dilithium2;

    assert!(!seed.is_empty(), "Seed must not be empty");
    assert!(
        seed.len() <= MAX_DILITHIUM_SEED_BYTES,
        "Seed length exceeds static limit"
    );

    let mut hasher = Sha3_256::new();
    ShaDigest::update(&mut hasher, DILITHIUM_DOMAIN_SEPARATOR);
    ShaDigest::update(&mut hasher, seed);
    let derived_seed = ShaDigest::finalize(hasher);

    let mut deterministic_seed = [0u8; 32];
    deterministic_seed.copy_from_slice(&derived_seed[..32]);
    let guard = install_deterministic_rng(deterministic_seed)?;

    let (public_key, secret_key) = dilithium2::keypair();
    drop(guard);

    let private_bytes = PQSecretKey::as_bytes(&secret_key).to_vec();
    let public_bytes = PQPublicKey::as_bytes(&public_key).to_vec();

    assert_eq!(
        private_bytes.len(),
        dilithium2::secret_key_bytes(),
        "Dilithium secret length mismatch"
    );
    assert_eq!(
        public_bytes.len(),
        dilithium2::public_key_bytes(),
        "Dilithium public length mismatch"
    );

    Ok((private_bytes, public_bytes))
}

#[derive(Debug, Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct StealthKeyMaterial {
    version: u8,
    view_secret: [u8; 32],
    spend_secret: [u8; 32],
}

impl StealthKeyMaterial {
    pub fn derive_from_seed(seed: &[u8]) -> WalletResult<Self> {
        const _: () = assert!(
            STEALTH_SEED_MAX_BYTES >= 64,
            "Stealth seed bound must accommodate expected key sizes"
        );
        assert!(
            STEALTH_VIEW_DERIVATION_DOMAIN != STEALTH_SPEND_DERIVATION_DOMAIN,
            "Stealth derivation domains must remain distinct"
        );

        if seed.is_empty() {
            return Err(WalletError::ValidationError(
                "Seed material cannot be empty".to_string(),
            ));
        }

        if seed.len() > STEALTH_SEED_MAX_BYTES {
            return Err(WalletError::ValidationError(format!(
                "Seed material exceeds {STEALTH_SEED_MAX_BYTES} byte bound"
            )));
        }

        let view_bytes = Zeroizing::new(blake3::derive_key(STEALTH_VIEW_DERIVATION_DOMAIN, seed));
        let spend_bytes = Zeroizing::new(blake3::derive_key(STEALTH_SPEND_DERIVATION_DOMAIN, seed));

        let view_scalar = Scalar::from_bytes_mod_order(*view_bytes);
        let spend_scalar = Scalar::from_bytes_mod_order(*spend_bytes);

        assert!(
            view_scalar != Scalar::ZERO,
            "Derived view scalar collapsed to zero"
        );
        assert!(
            spend_scalar != Scalar::ZERO,
            "Derived spend scalar collapsed to zero"
        );

        if view_scalar == Scalar::ZERO || spend_scalar == Scalar::ZERO {
            return Err(WalletError::CryptoError(
                "Derived stealth scalar cannot be zero".to_string(),
            ));
        }

        Ok(Self {
            version: STEALTH_KEY_MATERIAL_VERSION,
            view_secret: view_scalar.to_bytes(),
            spend_secret: spend_scalar.to_bytes(),
        })
    }

    pub fn encode(&self) -> Vec<u8> {
        assert_eq!(
            self.version, STEALTH_KEY_MATERIAL_VERSION,
            "Unsupported stealth key material version"
        );

        let mut encoded = Vec::with_capacity(STEALTH_KEY_MATERIAL_V1_LEN);
        encoded.push(self.version);
        encoded.extend_from_slice(&self.view_secret);
        encoded.extend_from_slice(&self.spend_secret);

        assert_eq!(
            encoded.len(),
            STEALTH_KEY_MATERIAL_V1_LEN,
            "Encoded stealth key material length mismatch"
        );
        encoded
    }

    pub fn decode(bytes: &[u8]) -> WalletResult<Self> {
        const _: () = assert!(
            STEALTH_KEY_MATERIAL_V1_LEN == 65,
            "Stealth key material layout changed unexpectedly"
        );
        const _: () = assert!(
            STEALTH_KEY_MATERIAL_VERSION > 0,
            "Stealth key material version must be non-zero"
        );

        if bytes.is_empty() {
            return Err(WalletError::ValidationError(
                "Stealth key material payload is empty".to_string(),
            ));
        }

        let version = bytes[0];
        if version != STEALTH_KEY_MATERIAL_VERSION {
            return Err(WalletError::ValidationError(format!(
                "Unsupported stealth key material version: {version}"
            )));
        }

        if bytes.len() != STEALTH_KEY_MATERIAL_V1_LEN {
            return Err(WalletError::ValidationError(format!(
                "Invalid stealth key material length: expected {STEALTH_KEY_MATERIAL_V1_LEN}, got {}",
                bytes.len()
            )));
        }

        let mut view_secret = [0u8; 32];
        view_secret.copy_from_slice(&bytes[1..33]);

        let mut spend_secret = [0u8; 32];
        spend_secret.copy_from_slice(&bytes[33..]);

        Ok(Self {
            version,
            view_secret,
            spend_secret,
        })
    }

    pub fn view_secret(&self) -> &[u8; 32] {
        &self.view_secret
    }

    pub fn spend_secret(&self) -> &[u8; 32] {
        &self.spend_secret
    }
}

/// Supported key derivation methods for wallet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyDerivation {
    /// BIP39 mnemonic with derivation path
    Bip39 {
        mnemonic_words: u32, // 12, 15, 18, 21, or 24
        derivation_path: String,
        // Note: passphrase is handled separately for security
    },
    /// Direct key import
    DirectImport,
    /// Hardware wallet integration (future)
    Hardware { device_type: String },
}

/// Enhanced wallet key pair that wraps ChertKeyPair with wallet-specific features
#[derive(Clone, Serialize, Deserialize)]
pub struct WalletKeyPair {
    /// Core cryptographic operations delegated to shared models
    pub core_keypair: ChertKeyPair,

    /// Wallet-specific derivation information
    pub derivation: KeyDerivation,

    /// Optional display name for the key
    pub name: Option<String>,

    /// Creation timestamp
    pub created_at: u64,

    /// Whether this keypair supports post-quantum operations
    pub supports_pq: bool,
}

impl WalletKeyPair {
    /// Generate a new wallet key pair with BIP39 mnemonic
    pub fn generate_with_mnemonic(
        word_count: u32,
        passphrase: Option<&str>,
        derivation_path: Option<String>,
        use_pq: bool,
    ) -> WalletResult<(Self, String)> {
        // Validate word count
        if ![12, 15, 18, 21, 24].contains(&word_count) {
            return Err(WalletError::ValidationError(
                "Invalid word count: must be 12, 15, 18, 21, or 24".to_string(),
            ));
        }

        // Generate BIP39 mnemonic
        let mnemonic_phrase = generate_bip39_mnemonic(word_count)?;

        // Derive keypair from mnemonic
        let core_keypair = derive_keypair_from_mnemonic(&mnemonic_phrase, passphrase, use_pq)?;

        if use_pq {
            assert_eq!(
                core_keypair.algorithm,
                SignatureAlgorithm::Dilithium2,
                "Deterministic PQ derivation must yield Dilithium2 key"
            );
        } else {
            assert_eq!(
                core_keypair.algorithm,
                SignatureAlgorithm::Ed25519,
                "Mnemonic derivation without PQ must yield Ed25519 key"
            );
        }

        let derivation = KeyDerivation::Bip39 {
            mnemonic_words: word_count,
            derivation_path: derivation_path.unwrap_or("m/44'/0'/0'/0/0".to_string()),
        };

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| WalletError::ValidationError(format!("System time error: {}", e)))?
            .as_secs();

        let wallet_keypair = WalletKeyPair {
            core_keypair,
            derivation,
            name: None,
            created_at: timestamp,
            supports_pq: use_pq,
        };

        Ok((wallet_keypair, mnemonic_phrase))
    }

    /// Restore wallet key pair from BIP39 mnemonic
    pub fn from_mnemonic(
        mnemonic_phrase: &str,
        passphrase: Option<&str>,
        derivation_path: Option<String>,
        use_pq: bool,
    ) -> WalletResult<Self> {
        // Validate mnemonic
        validate_bip39_mnemonic(mnemonic_phrase)?;
        let word_count = mnemonic_phrase.split_whitespace().count() as u32;

        // Derive keypair from mnemonic
        let core_keypair = derive_keypair_from_mnemonic(mnemonic_phrase, passphrase, use_pq)?;

        let derivation = KeyDerivation::Bip39 {
            mnemonic_words: word_count,
            derivation_path: derivation_path.unwrap_or("m/44'/0'/0'/0/0".to_string()),
        };

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| WalletError::ValidationError(format!("System time error: {}", e)))?
            .as_secs();

        if use_pq {
            assert_eq!(
                core_keypair.algorithm,
                SignatureAlgorithm::Dilithium2,
                "Mnemonic restoration with PQ must yield Dilithium2 key"
            );
        } else {
            assert_eq!(
                core_keypair.algorithm,
                SignatureAlgorithm::Ed25519,
                "Mnemonic restoration without PQ must yield Ed25519 key"
            );
        }

        Ok(WalletKeyPair {
            core_keypair,
            derivation,
            name: None,
            created_at: timestamp,
            supports_pq: use_pq,
        })
    }

    /// Generate a simple keypair without mnemonic
    pub fn generate_direct(use_pq: bool) -> WalletResult<Self> {
        let core_keypair = if use_pq {
            ChertKeyPair::generate_dilithium2()
        } else {
            ChertKeyPair::generate_ed25519()
        }
        .map_err(|e| WalletError::CryptoError(e.to_string()))?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| WalletError::ValidationError(format!("System time error: {}", e)))?
            .as_secs();

        Ok(WalletKeyPair {
            core_keypair,
            derivation: KeyDerivation::DirectImport,
            name: None,
            created_at: timestamp,
            supports_pq: use_pq,
        })
    }

    /// Sign data using the core keypair
    pub fn sign(&self, data: &[u8]) -> WalletResult<ChertSignature> {
        self.core_keypair
            .sign(data)
            .map_err(|e| WalletError::CryptoError(e.to_string()))
    }

    /// Verify a signature using the core keypair
    pub fn verify(&self, data: &[u8], signature: &ChertSignature) -> WalletResult<bool> {
        self.core_keypair
            .verify(data, signature)
            .map_err(|e| WalletError::CryptoError(e.to_string()))
    }

    /// Get the wallet address for this keypair
    pub fn address(&self) -> String {
        self.core_keypair.address("WALLET")
    }

    /// Get the public key as hex string
    pub fn public_key_hex(&self) -> String {
        hex::encode(&self.core_keypair.public_key)
    }

    /// Set a display name for this keypair
    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }
}

impl std::fmt::Debug for WalletKeyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WalletKeyPair")
            .field("algorithm", &self.core_keypair.algorithm)
            .field("public_key", &self.public_key_hex())
            .field("derivation", &self.derivation)
            .field("name", &self.name)
            .field("created_at", &self.created_at)
            .field("supports_pq", &self.supports_pq)
            .field("private_key", &"<redacted>")
            .finish()
    }
}

/// Generate a BIP39 mnemonic with specified word count
fn generate_bip39_mnemonic(word_count: u32) -> WalletResult<String> {
    use bip39::Mnemonic;
    use rand::{rngs::OsRng, RngCore};

    let entropy_bits = match word_count {
        12 => 128,
        15 => 160,
        18 => 192,
        21 => 224,
        24 => 256,
        _ => {
            return Err(WalletError::ValidationError(
                "Invalid word count".to_string(),
            ))
        }
    };

    let mut entropy = vec![0u8; entropy_bits / 8];
    let mut rng = OsRng;
    rng.try_fill_bytes(&mut entropy)
        .map_err(|e| WalletError::CryptoError(format!("Failed to generate entropy: {}", e)))?;

    let mnemonic = Mnemonic::from_entropy(&entropy)
        .map_err(|e| WalletError::CryptoError(format!("Failed to create mnemonic: {}", e)))?;

    Ok(mnemonic.to_string())
}

/// Validate a BIP39 mnemonic phrase
fn validate_bip39_mnemonic(mnemonic: &str) -> WalletResult<()> {
    use bip39::{Language, Mnemonic};

    Mnemonic::parse_in_normalized(Language::English, mnemonic)
        .map_err(|e| WalletError::ValidationError(format!("Invalid mnemonic: {}", e)))?;

    Ok(())
}

/// Derive a keypair from BIP39 mnemonic
fn derive_keypair_from_mnemonic(
    mnemonic: &str,
    passphrase: Option<&str>,
    use_pq: bool,
) -> WalletResult<ChertKeyPair> {
    use bip39::{Language, Mnemonic};

    // Parse and validate mnemonic
    let mnemonic_obj = Mnemonic::parse_in_normalized(Language::English, mnemonic)
        .map_err(|e| WalletError::CryptoError(format!("Invalid mnemonic: {}", e)))?;

    // Generate seed from mnemonic with optional passphrase
    let seed_bytes = mnemonic_obj.to_seed(passphrase.unwrap_or(""));

    if use_pq {
        let (private_bytes, public_bytes) = derive_dilithium_key_material(seed_bytes.as_slice())?;
        Ok(ChertKeyPair {
            algorithm: SignatureAlgorithm::Dilithium2,
            public_key: public_bytes,
            private_key: private_bytes,
        })
    } else {
        // For Ed25519, derive private key from first 32 bytes of HMAC-SHA512(seed)
        use hmac::{Hmac, Mac};
        use sha2::Sha512;

        let mut hmac = Hmac::<Sha512>::new_from_slice(b"CHERT_ED25519_DERIVE_V1")
            .map_err(|e| WalletError::CryptoError(format!("HMAC error: {}", e)))?;
        hmac::Mac::update(&mut hmac, &seed_bytes);
        let result = hmac.finalize();
        let private_bytes: [u8; 32] = result.into_bytes()[..32]
            .try_into()
            .map_err(|_| WalletError::CryptoError("Key derivation failed".to_string()))?;

        // Create Ed25519 keypair from derived private key
        use ed25519_dalek::SigningKey;
        let signing_key = SigningKey::from_bytes(&private_bytes);
        let verifying_key = signing_key.verifying_key();

        Ok(ChertKeyPair {
            algorithm: SignatureAlgorithm::Ed25519,
            public_key: verifying_key.to_bytes().to_vec(),
            private_key: signing_key.to_bytes().to_vec(),
        })
    }
}

/// Secure password hashing for wallet encryption (delegated to Argon2)
pub fn hash_password(password: &str, salt: &[u8]) -> WalletResult<Vec<u8>> {
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};

    let salt_string = SaltString::encode_b64(salt)
        .map_err(|e| WalletError::CryptoError(format!("Salt encoding error: {}", e)))?;

    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt_string)
        .map_err(|e| WalletError::CryptoError(format!("Password hashing failed: {}", e)))?;

    Ok(password_hash.hash.unwrap().as_bytes().to_vec())
}

/// Generate quantum-resistant address (delegated to utils)
pub fn generate_address(public_key: &[u8], address_type: &str) -> String {
    utils::generate_quantum_resistant_address(public_key, address_type)
}

/// Verify address matches public key (delegated to utils)
pub fn verify_address(address: &str, public_key: &[u8], address_type: &str) -> bool {
    utils::verify_address(address, public_key, address_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex as StdMutex, MutexGuard as StdMutexGuard, OnceLock as StdOnceLock};

    fn deterministic_test_guard() -> StdMutexGuard<'static, ()> {
        static DILITHIUM_TEST_MUTEX: StdOnceLock<StdMutex<()>> = StdOnceLock::new();
        let mutex = DILITHIUM_TEST_MUTEX.get_or_init(|| StdMutex::new(()));
        mutex.lock().expect("deterministic test mutex poisoned")
    }

    #[test]
    fn test_mnemonic_generation_and_restoration() {
        let (keypair, mnemonic) =
            WalletKeyPair::generate_with_mnemonic(12, None, None, false).unwrap();

        // Mnemonic should have 12 words
        assert_eq!(mnemonic.split_whitespace().count(), 12);

        // Should be able to restore from mnemonic
        let restored = WalletKeyPair::from_mnemonic(&mnemonic, None, None, false).unwrap();
        assert_eq!(keypair.public_key_hex(), restored.public_key_hex());
        assert_eq!(keypair.address(), restored.address());
    }

    #[test]
    fn test_post_quantum_keypair() {
        let keypair = WalletKeyPair::generate_direct(true).unwrap();
        assert_eq!(
            keypair.core_keypair.algorithm,
            SignatureAlgorithm::Dilithium2
        );
        assert!(keypair.supports_pq);

        // Test signing and verification
        let data = b"test message";
        let signature = keypair.sign(data).unwrap();
        assert!(keypair.verify(data, &signature).unwrap());
    }

    #[test]
    fn test_classical_keypair() {
        let keypair = WalletKeyPair::generate_direct(false).unwrap();
        assert_eq!(keypair.core_keypair.algorithm, SignatureAlgorithm::Ed25519);
        assert!(!keypair.supports_pq);

        // Test signing and verification
        let data = b"test message";
        let signature = keypair.sign(data).unwrap();
        assert!(keypair.verify(data, &signature).unwrap());
    }

    #[test]
    fn test_address_generation() {
        let keypair = WalletKeyPair::generate_direct(false).unwrap();
        let address = keypair.address();

        // Address should start with 0x and be 42 characters
        assert!(address.starts_with("0x"));
        assert_eq!(address.len(), 42);

        // Address should verify against public key
        assert!(verify_address(
            &address,
            &keypair.core_keypair.public_key,
            "WALLET"
        ));
    }

    #[test]
    fn stealth_key_material_is_deterministic() {
        let seed = vec![7u8; 48];
        let first = StealthKeyMaterial::derive_from_seed(&seed).unwrap();
        let second = StealthKeyMaterial::derive_from_seed(&seed).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.encode().len(), STEALTH_KEY_MATERIAL_V1_LEN);
    }

    #[test]
    fn stealth_key_material_round_trip() {
        let seed = (0u8..48).collect::<Vec<u8>>();
        let material = StealthKeyMaterial::derive_from_seed(&seed).unwrap();
        let encoded = material.encode();
        let decoded = StealthKeyMaterial::decode(&encoded).unwrap();

        assert_eq!(material.view_secret(), decoded.view_secret());
        assert_eq!(material.spend_secret(), decoded.spend_secret());
    }

    #[test]
    fn stealth_key_material_rejects_empty_seed() {
        let err = StealthKeyMaterial::derive_from_seed(&[]).unwrap_err();
        match err {
            WalletError::ValidationError(_) => {}
            other => panic!("Expected validation error, got: {other:?}"),
        }
    }

    #[test]
    fn test_password_hashing() {
        let password = "secure_password_123";
        let salt = b"test_salt_16byte";

        let hash1 = hash_password(password, salt).unwrap();
        let hash2 = hash_password(password, salt).unwrap();

        // Same password + salt should produce same hash
        assert_eq!(hash1, hash2);

        // Different salt should produce different hash
        let salt2 = b"different_salt16";
        let hash3 = hash_password(password, salt2).unwrap();
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_dilithium_deterministic_material_stability() {
        let _guard = deterministic_test_guard();
        use pqcrypto_dilithium::dilithium2;

        let seed = [0xABu8; 64];
        assert_eq!(seed.len(), 64);
        assert!(seed.iter().any(|&byte| byte != 0));

        let (priv_one, pub_one) = super::derive_dilithium_key_material(&seed).unwrap();
        let (priv_two, pub_two) = super::derive_dilithium_key_material(&seed).unwrap();

        assert_eq!(priv_one, priv_two);
        assert_eq!(pub_one, pub_two);
        assert_eq!(priv_one.len(), dilithium2::secret_key_bytes());
        assert_eq!(pub_one.len(), dilithium2::public_key_bytes());
    }

    #[test]
    fn test_dilithium_deterministic_material_variation() {
        let _guard = deterministic_test_guard();
        use pqcrypto_dilithium::dilithium2;

        let seed_a = b"seed-a-quantum-resistant";
        let seed_b = b"seed-b-quantum-resistant";
        assert!(seed_a.len() <= MAX_DILITHIUM_SEED_BYTES);
        assert!(seed_b.len() <= MAX_DILITHIUM_SEED_BYTES);

        let (priv_a, pub_a) = super::derive_dilithium_key_material(seed_a).unwrap();
        let (priv_b, pub_b) = super::derive_dilithium_key_material(seed_b).unwrap();

        assert_ne!(priv_a, priv_b);
        assert_ne!(pub_a, pub_b);
        assert_eq!(priv_a.len(), dilithium2::secret_key_bytes());
        assert_eq!(pub_a.len(), dilithium2::public_key_bytes());
    }

    #[test]
    fn test_dilithium_round_trip_serialization_and_restore() {
        let _guard = deterministic_test_guard();
        use pqcrypto_dilithium::dilithium2;

        let passphrase = Some("round-trip passphrase");
        let (keypair, mnemonic) = WalletKeyPair::generate_with_mnemonic(24, passphrase, None, true)
            .expect("failed to generate wallet keypair");

        assert!(keypair.supports_pq);
        assert_eq!(
            keypair.core_keypair.algorithm,
            SignatureAlgorithm::Dilithium2
        );

        let serialized = serde_json::to_vec(&keypair).expect("serialization should succeed");
        let restored: WalletKeyPair =
            serde_json::from_slice(&serialized).expect("deserialization should succeed");

        assert_eq!(restored.supports_pq, keypair.supports_pq);
        assert_eq!(
            restored.core_keypair.public_key,
            keypair.core_keypair.public_key
        );
        assert_eq!(
            restored.core_keypair.private_key,
            keypair.core_keypair.private_key
        );

        let restored_from_mnemonic =
            WalletKeyPair::from_mnemonic(&mnemonic, passphrase, None, true)
                .expect("mnemonic restoration should succeed");

        assert!(restored_from_mnemonic.supports_pq);
        assert_eq!(
            restored_from_mnemonic.core_keypair.public_key,
            keypair.core_keypair.public_key
        );
        assert_eq!(
            restored_from_mnemonic.core_keypair.private_key,
            keypair.core_keypair.private_key
        );
        assert_eq!(
            restored_from_mnemonic.core_keypair.private_key.len(),
            dilithium2::secret_key_bytes()
        );
        assert_eq!(
            restored_from_mnemonic.core_keypair.public_key.len(),
            dilithium2::public_key_bytes()
        );
    }
}
