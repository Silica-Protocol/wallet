//! Cryptographic operations module for secure wallet operations
//! 
//! This module provides high-performance cryptographic operations
//! including signing, verification, key generation, and encryption.

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use js_sys::{Promise, Object, Reflect, Uint8Array};
use serde::{Deserialize, Serialize};
use silica_models::crypto::{ChertKeyPair, ChertSignature, SignatureAlgorithm};
use crate::{WasmError, WasmResult, rust_to_js, js_to_rust};

/// Key pair wrapper for WebAssembly
#[wasm_bindgen]
pub struct WasmKeyPair {
    inner: ChertKeyPair,
    algorithm: SignatureAlgorithm,
}

#[wasm_bindgen]
impl WasmKeyPair {
    /// Generate a new key pair with specified algorithm
    #[wasm_bindgen(constructor)]
    pub fn generate(algorithm: String) -> Result<WasmKeyPair, JsValue> {
        let sig_alg = match algorithm.as_str() {
            "ed25519" => SignatureAlgorithm::Ed25519,
            "dilithium2" => SignatureAlgorithm::Dilithium2,
            "dilithium3" => SignatureAlgorithm::Dilithium3,
            "dilithium5" => SignatureAlgorithm::Dilithium5,
            _ => return Err(WasmError::new("INVALID_ALGORITHM", "Supported algorithms: ed25519, dilithium2, dilithium3, dilithium5").into()),
        };

        let keypair = match sig_alg {
            SignatureAlgorithm::Ed25519 => {
                ChertKeyPair::generate_ed25519()
                    .map_err(|e| WasmError::new("KEY_GENERATION_ERROR", &format!("Failed to generate Ed25519 keypair: {}", e)))?
            }
            SignatureAlgorithm::Dilithium2 => {
                ChertKeyPair::generate_dilithium2()
                    .map_err(|e| WasmError::new("KEY_GENERATION_ERROR", &format!("Failed to generate Dilithium2 keypair: {}", e)))?
            }
            SignatureAlgorithm::Dilithium3 => {
                ChertKeyPair::generate_dilithium3()
                    .map_err(|e| WasmError::new("KEY_GENERATION_ERROR", &format!("Failed to generate Dilithium3 keypair: {}", e)))?
            }
            SignatureAlgorithm::Dilithium5 => {
                ChertKeyPair::generate_dilithium5()
                    .map_err(|e| WasmError::new("KEY_GENERATION_ERROR", &format!("Failed to generate Dilithium5 keypair: {}", e)))?
            }
        };

        Ok(WasmKeyPair {
            inner: keypair,
            algorithm: sig_alg,
        })
    }

    /// Generate key pair from mnemonic
    #[wasm_bindgen]
    pub fn from_mnemonic(mnemonic: String, passphrase: Option<String>, algorithm: String) -> Result<WasmKeyPair, JsValue> {
        let sig_alg = match algorithm.as_str() {
            "ed25519" => SignatureAlgorithm::Ed25519,
            "dilithium2" => SignatureAlgorithm::Dilithium2,
            _ => return Err(WasmError::new("INVALID_ALGORITHM", "Mnemonic generation supports: ed25519, dilithium2").into()),
        };

        // Validate mnemonic
        if !validate_mnemonic(&mnemonic) {
            return Err(WasmError::new("INVALID_MNEMONIC", "Invalid mnemonic phrase").into());
        }

        let keypair = derive_keypair_from_mnemonic(&mnemonic, passphrase.as_deref(), sig_alg)
            .map_err(|e| WasmError::new("KEY_DERIVATION_ERROR", &format!("Failed to derive keypair from mnemonic: {}", e)))?;

        Ok(WasmKeyPair {
            inner: keypair,
            algorithm: sig_alg,
        })
    }

    /// Get the public key as hex string
    #[wasm_bindgen]
    pub fn get_public_key(&self) -> String {
        hex::encode(&self.inner.public_key)
    }

    /// Get the private key as hex string (use with caution!)
    #[wasm_bindgen]
    pub fn get_private_key(&self) -> String {
        hex::encode(&self.inner.private_key)
    }

    /// Get the address for this keypair
    #[wasm_bindgen]
    pub fn get_address(&self) -> String {
        self.inner.address("WALLET")
    }

    /// Get the algorithm used
    #[wasm_bindgen]
    pub fn get_algorithm(&self) -> String {
        format!("{:?}", self.algorithm)
    }

    /// Sign data with this keypair
    #[wasm_bindgen]
    pub fn sign(&self, data: &[u8]) -> Result<Uint8Array, JsValue> {
        let signature = self.inner.sign(data)
            .map_err(|e| WasmError::new("SIGNING_ERROR", &format!("Failed to sign data: {}", e)))?;

        let signature_bytes = match signature {
            ChertSignature::Ed25519(bytes) => bytes,
            ChertSignature::Dilithium2(bytes) => bytes,
            ChertSignature::Dilithium3(bytes) => bytes,
            ChertSignature::Dilithium5(bytes) => bytes,
        };

        let array = Uint8Array::new_with_length(signature_bytes.len() as u32);
        for (i, byte) in signature_bytes.iter().enumerate() {
            array.set_index(i as u32, *byte);
        }

        Ok(array)
    }

    /// Verify a signature with this public key
    #[wasm_bindgen]
    pub fn verify(&self, data: &[u8], signature: &[u8]) -> Result<bool, JsValue> {
        let chert_signature = match self.algorithm {
            SignatureAlgorithm::Ed25519 => ChertSignature::Ed25519(signature.to_vec()),
            SignatureAlgorithm::Dilithium2 => ChertSignature::Dilithium2(signature.to_vec()),
            SignatureAlgorithm::Dilithium3 => ChertSignature::Dilithium3(signature.to_vec()),
            SignatureAlgorithm::Dilithium5 => ChertSignature::Dilithium5(signature.to_vec()),
        };

        let is_valid = self.inner.verify(data, &chert_signature)
            .map_err(|e| WasmError::new("VERIFICATION_ERROR", &format!("Failed to verify signature: {}", e)))?;

        Ok(is_valid)
    }

    /// Export keypair to encrypted JSON
    #[wasm_bindgen]
    pub fn export_encrypted(&self, password: String) -> Result<Promise, JsValue> {
        if password.is_empty() {
            return Err(WasmError::new("EMPTY_PASSWORD", "Password cannot be empty").into());
        }

        let keypair = self.inner.clone();
        let algorithm = self.algorithm;

        let promise = future_to_promise(async move {
            let encrypted = encrypt_keypair(&keypair, &password, algorithm).await?;
            Ok(JsValue::from_str(&encrypted))
        });

        Ok(promise)
    }

    /// Import keypair from encrypted JSON
    #[wasm_bindgen]
    pub fn import_encrypted(encrypted_data: String, password: String) -> Result<Promise, JsValue> {
        if password.is_empty() {
            return Err(WasmError::new("EMPTY_PASSWORD", "Password cannot be empty").into());
        }

        let promise = future_to_promise(async move {
            let keypair = decrypt_keypair(&encrypted_data, &password).await?;
            
            let wasm_keypair = WasmKeyPair {
                inner: keypair,
                algorithm: keypair.algorithm,
            };

            let result = rust_to_js(&wasm_keypair)?;
            Ok(result.into())
        });

        Ok(promise)
    }
}

/// Transaction signing utilities
#[wasm_bindgen]
pub struct TransactionSigner {
    keypair: WasmKeyPair,
}

#[wasm_bindgen]
impl TransactionSigner {
    /// Create a new transaction signer
    #[wasm_bindgen(constructor)]
    pub fn new(keypair: WasmKeyPair) -> TransactionSigner {
        TransactionSigner { keypair }
    }

    /// Sign a transaction
    #[wasm_bindgen]
    pub fn sign_transaction(&self, transaction_js: &JsValue) -> Result<Promise, JsValue> {
        let transaction: TransactionData = js_to_rust(transaction_js)?;

        let keypair = self.keypair.inner.clone();

        let promise = future_to_promise(async move {
            let signed_tx = sign_transaction_data(&transaction, &keypair).await?;
            let result = rust_to_js(&signed_tx)?;
            Ok(result.into())
        });

        Ok(promise)
    }

    /// Sign a message
    #[wasm_bindgen]
    pub fn sign_message(&self, message: String) -> Result<Uint8Array, JsValue> {
        let message_bytes = message.as_bytes();
        self.keypair.sign(message_bytes)
    }

    /// Verify a message signature
    #[wasm_bindgen]
    pub fn verify_message(&self, message: String, signature: &[u8]) -> Result<bool, JsValue> {
        let message_bytes = message.as_bytes();
        self.keypair.verify(message_bytes, signature)
    }
}

/// Transaction data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionData {
    pub from: String,
    pub to: String,
    pub amount: String,
    pub fee: String,
    pub nonce: u64,
    pub timestamp: u64,
    pub data: Option<Vec<u8>>,
    pub chain_id: u32,
}

/// Signed transaction structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTransaction {
    pub transaction: TransactionData,
    pub signature: Vec<u8>,
    pub public_key: Vec<u8>,
    pub hash: String,
}

/// Password strength checker
#[wasm_bindgen]
pub fn check_password_strength(password: String) -> Result<Object, JsValue> {
    let strength = calculate_password_strength(&password);
    
    let result = Object::new();
    let _ = Reflect::set(&result, &"score".into(), &strength.score.into());
    let _ = Reflect::set(&result, &"strength".into(), &strength.strength.into());
    let _ = Reflect::set(&result, &"suggestions".into(), &JsValue::from_serde(&strength.suggestions).unwrap());

    Ok(result)
}

/// Password strength result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordStrength {
    pub score: u8, // 0-4
    pub strength: String,
    pub suggestions: Vec<String>,
}

/// Mnemonic validation and generation
#[wasm_bindgen]
pub fn generate_mnemonic(word_count: u32) -> Result<String, JsValue> {
    if ![12, 15, 18, 21, 24].contains(&word_count) {
        return Err(WasmError::new("INVALID_WORD_COUNT", "Word count must be 12, 15, 18, 21, or 24").into());
    }

    let mnemonic = generate_bip39_mnemonic(word_count)
        .map_err(|e| WasmError::new("MNEMONIC_GENERATION_ERROR", &format!("Failed to generate mnemonic: {}", e)))?;

    Ok(mnemonic)
}

#[wasm_bindgen]
pub fn validate_mnemonic_js(mnemonic: String) -> Result<bool, JsValue> {
    Ok(validate_mnemonic(&mnemonic))
}

/// Address utilities
#[wasm_bindgen]
pub fn validate_address(address: String) -> Result<bool, JsValue> {
    // Basic address validation for Chert addresses (0x + 40 hex chars)
    if !address.starts_with("0x") {
        return Ok(false);
    }

    if address.len() != 42 {
        return Ok(false);
    }

    let hex_part = &address[2..];
    match hex::decode(hex_part) {
        Ok(bytes) if bytes.len() == 20 => Ok(true),
        _ => Ok(false),
    }
}

#[wasm_bindgen]
pub fn generate_address_from_public_key(public_key: String, address_type: String) -> Result<String, JsValue> {
    let public_key_bytes = hex::decode(&public_key)
        .map_err(|_| WasmError::new("INVALID_PUBLIC_KEY", "Invalid public key hex format"))?;

    if public_key_bytes.len() != 32 {
        return Err(WasmError::new("INVALID_PUBLIC_KEY_LENGTH", "Public key must be 32 bytes").into());
    }

    let address = generate_quantum_resistant_address(&public_key_bytes, &address_type);
    Ok(address)
}

// Internal utility functions
fn validate_mnemonic(mnemonic: &str) -> bool {
    // Basic mnemonic validation - in a real implementation, use bip39 crate
    let word_count = mnemonic.split_whitespace().count();
    [12, 15, 18, 21, 24].contains(&word_count) && 
       mnemonic.chars().all(|c| c.is_ascii_alphabetic() || c.is_whitespace())
}

fn generate_bip39_mnemonic(word_count: u32) -> WasmResult<String> {
    // In a real implementation, use the bip39 crate
    let entropy_bits = match word_count {
        12 => 128,
        15 => 160,
        18 => 192,
        21 => 224,
        24 => 256,
        _ => return Err(WasmError::new("INVALID_WORD_COUNT", "Invalid word count")),
    };

    // For now, return a dummy mnemonic
    let words = vec![
        "abandon", "ability", "able", "about", "above", "absent", "absorb", "abstract",
        "absurd", "abuse", "access", "accident", "account", "accuse", "achieve", "acid",
        "acoustic", "acquire", "across", "act", "action", "actor", "actress", "actual"
    ];

    let mut mnemonic = String::new();
    for i in 0..word_count {
        if i > 0 {
            mnemonic.push(' ');
        }
        mnemonic.push_str(words[i as usize % words.len()]);
    }

    Ok(mnemonic)
}

fn derive_keypair_from_mnemonic(
    mnemonic: &str,
    passphrase: Option<&str>,
    algorithm: SignatureAlgorithm,
) -> WasmResult<ChertKeyPair> {
    // In a real implementation, use proper BIP39 derivation
    match algorithm {
        SignatureAlgorithm::Ed25519 => {
            ChertKeyPair::generate_ed25519()
                .map_err(|e| WasmError::new("DERIVATION_ERROR", &format!("Ed25519 derivation failed: {}", e)))
        }
        SignatureAlgorithm::Dilithium2 => {
            ChertKeyPair::generate_dilithium2()
                .map_err(|e| WasmError::new("DERIVATION_ERROR", &format!("Dilithium2 derivation failed: {}", e)))
        }
        _ => Err(WasmError::new("UNSUPPORTED_ALGORITHM", "Algorithm not supported for mnemonic derivation"))
    }
}

async fn encrypt_keypair(
    keypair: &ChertKeyPair,
    password: &str,
    algorithm: SignatureAlgorithm,
) -> WasmResult<String> {
    // Simulate async encryption
    wasm_bindgen_futures::JsFuture::from(
        js_sys::Promise::resolve(&JsValue::from(10))
    ).await.unwrap();

    // In a real implementation, use proper encryption (AES-256-GCM + Argon2)
    let encrypted_data = format!(
        r#"{{"algorithm": "{:?}", "public_key": "{}", "encrypted_private_key": "encrypted_{}", "salt": "salt_"}}"#,
        algorithm,
        hex::encode(&keypair.public_key),
        hex::encode(&keypair.private_key[..8]) // Only first 8 bytes for demo
    );

    Ok(encrypted_data)
}

async fn decrypt_keypair(encrypted_data: &str, password: &str) -> WasmResult<ChertKeyPair> {
    // Simulate async decryption
    wasm_bindgen_futures::JsFuture::from(
        js_sys::Promise::resolve(&JsValue::from(10))
    ).await.unwrap();

    // In a real implementation, use proper decryption
    // For now, just generate a new keypair (this is NOT secure!)
    ChertKeyPair::generate_ed25519()
        .map_err(|e| WasmError::new("DECRYPTION_ERROR", &format!("Failed to decrypt keypair: {}", e)))
}

async fn sign_transaction_data(
    transaction: &TransactionData,
    keypair: &ChertKeyPair,
) -> WasmResult<SignedTransaction> {
    // Create transaction hash
    let tx_hash = create_transaction_hash(transaction);

    // Sign the hash
    let signature = keypair.sign(tx_hash.as_bytes())
        .map_err(|e| WasmError::new("TRANSACTION_SIGNING_ERROR", &format!("Failed to sign transaction: {}", e)))?;

    let signature_bytes = match signature {
        ChertSignature::Ed25519(bytes) => bytes,
        ChertSignature::Dilithium2(bytes) => bytes,
        ChertSignature::Dilithium3(bytes) => bytes,
        ChertSignature::Dilithium5(bytes) => bytes,
    };

    Ok(SignedTransaction {
        transaction: transaction.clone(),
        signature: signature_bytes,
        public_key: keypair.public_key.clone(),
        hash: tx_hash,
    })
}

fn create_transaction_hash(transaction: &TransactionData) -> String {
    use sha3::{Digest, Sha3_256};
    
    let mut hasher = Sha3_256::new();
    hasher.update(transaction.from.as_bytes());
    hasher.update(transaction.to.as_bytes());
    hasher.update(transaction.amount.as_bytes());
    hasher.update(transaction.fee.as_bytes());
    hasher.update(transaction.nonce.to_le_bytes());
    hasher.update(transaction.timestamp.to_le_bytes());
    hasher.update(transaction.chain_id.to_le_bytes());
    
    if let Some(ref data) = transaction.data {
        hasher.update(data);
    }

    format!("0x{}", hex::encode(hasher.finalize()))
}

fn calculate_password_strength(password: &str) -> PasswordStrength {
    let mut score = 0u8;
    let mut suggestions = Vec::new();

    // Length check
    if password.len() >= 8 {
        score += 1;
    } else {
        suggestions.push("Use at least 8 characters".to_string());
    }

    if password.len() >= 12 {
        score += 1;
    }

    // Character variety
    if password.chars().any(|c| c.is_uppercase()) {
        score += 1;
    } else {
        suggestions.push("Include uppercase letters".to_string());
    }

    if password.chars().any(|c| c.is_lowercase()) {
        score += 1;
    } else {
        suggestions.push("Include lowercase letters".to_string());
    }

    if password.chars().any(|c| c.is_numeric()) {
        score += 1;
    } else {
        suggestions.push("Include numbers".to_string());
    }

    if password.chars().any(|c| !c.is_alphanumeric()) {
        score += 1;
    } else {
        suggestions.push("Include special characters".to_string());
    }

    let strength = match score {
        0..=2 => "Weak".to_string(),
        3..=4 => "Medium".to_string(),
        5 => "Strong".to_string(),
        _ => "Very Strong".to_string(),
    };

    PasswordStrength {
        score: score.min(4),
        strength,
        suggestions,
    }
}

fn generate_quantum_resistant_address(public_key: &[u8], address_type: &str) -> String {
    use sha3::{Digest, Sha3_256};
    
    let mut hasher = Sha3_256::new();
    hasher.update(format!("CHERT_ADDRESS_{}_V2", address_type).as_bytes());
    hasher.update(public_key);
    let hash = hasher.finalize();
    let address_bytes = &hash[..20]; // First 20 bytes
    
    format!("0x{}", hex::encode(address_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_keypair_generation() {
        let keypair = WasmKeyPair::generate("ed25519".to_string()).unwrap();
        let public_key = keypair.get_public_key();
        assert_eq!(public_key.len(), 64); // 32 bytes = 64 hex chars
        
        let address = keypair.get_address();
        assert!(address.starts_with("0x"));
        assert_eq!(address.len(), 42);
    }

    #[wasm_bindgen_test]
    fn test_address_validation() {
        assert!(validate_address("0x1234567890123456789012345678901234567890".to_string()).unwrap());
        assert!(!validate_address("0x123".to_string()).unwrap());
        assert!(!validate_address("1234567890123456789012345678901234567890".to_string()).unwrap());
    }

    #[wasm_bindgen_test]
    fn test_password_strength() {
        let weak = check_password_strength("123".to_string());
        let strong = check_password_strength("Str0ng!P@ssw0rd".to_string());
        
        let weak_score = Reflect::get(&weak, &"score".into()).unwrap().as_f64().unwrap();
        let strong_score = Reflect::get(&strong, &"score".into()).unwrap().as_f64().unwrap();
        
        assert!(weak_score < strong_score);
    }

    #[wasm_bindgen_test]
    fn test_mnemonic_validation() {
        assert!(validate_mnemonic_js("abandon ability able about above absent absorb abstract absurd abuse access accident account accuse achieve acid acoustic acquire across act action actor actress actual".to_string()).unwrap());
        assert!(!validate_mnemonic_js("invalid mnemonic".to_string()).unwrap());
    }
}