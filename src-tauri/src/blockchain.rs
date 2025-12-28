/// Core blockchain types for the Chert wallet
///
/// This module defines the fundamental blockchain data structures,
/// leveraging shared types from silica-models where appropriate.
use crate::errors::{WalletError, WalletResult};
use serde::{Deserialize, Serialize};
use silica_models::crypto::{ChertCrypto, ChertHash, HashAlgorithm, StandardCrypto};
use std::fmt;
use std::str::FromStr;

const CHERT_BECH32_HRP: &str = "chert";

/// A Chert blockchain address
///
/// Addresses in Chert follow the format: 0x{40_hex_chars}
/// This provides compatibility with Ethereum-style tooling and infrastructure.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address {
    /// The raw address bytes (20 bytes for account addresses)
    raw_bytes: Vec<u8>,
    /// The human-readable string representation (0x prefixed hex)
    hex_address: String,
    /// Address type for different purposes
    address_type: AddressType,
}

/// Types of addresses supported in the Chert ecosystem
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AddressType {
    /// Standard user account address
    Account,
    /// Smart contract address
    Contract,
    /// Multi-signature address
    MultiSig,
    /// Validator address for staking
    Validator,
}

impl Address {
    /// Create a new address from raw bytes
    pub fn from_bytes(bytes: Vec<u8>, address_type: AddressType) -> WalletResult<Self> {
        if bytes.is_empty() {
            return Err(WalletError::InvalidAddress(
                "Address bytes cannot be empty".to_string(),
            ));
        }

        if bytes.len() != 20 {
            return Err(WalletError::InvalidAddress(format!(
                "Invalid address length: expected 20 bytes, got {}",
                bytes.len()
            )));
        }

        // Generate 0x-prefixed hex address
        let hex_address = format!("0x{}", hex::encode(&bytes));

        Ok(Address {
            raw_bytes: bytes,
            hex_address,
            address_type,
        })
    }

    /// Parse an address from a string, supporting both 0x and Bech32 formats
    pub fn from_string(address: &str) -> WalletResult<Self> {
        if address.starts_with("chert") {
            // Bech32 format
            Self::from_bech32(address)
        } else if address.starts_with("0x") {
            // Hex format
            Self::from_hex(address)
        } else {
            Err(WalletError::InvalidAddress(
                "Address must start with '0x' or 'chert'".to_string(),
            ))
        }
    }

    /// Parse a hex address (0x format)
    fn from_hex(hex_address: &str) -> WalletResult<Self> {
        if !hex_address.starts_with("0x") {
            return Err(WalletError::InvalidAddress(
                "Hex address must start with '0x'".to_string(),
            ));
        }

        if hex_address.len() != 42 {
            // "0x" (2) + 40 hex chars = 42 total
            return Err(WalletError::InvalidAddress(format!(
                "Invalid hex address length: expected 42 characters, got {}",
                hex_address.len()
            )));
        }

        let hex_part = &hex_address[2..]; // Skip "0x"
        let bytes = hex::decode(hex_part)
            .map_err(|_| WalletError::InvalidAddress("Invalid hex in address".to_string()))?;

        // For now, assume all parsed addresses are account type
        // In production, this would be determined by address format or prefix
        Self::from_bytes(bytes, AddressType::Account)
    }

    /// Parse a Bech32 address
    fn from_bech32(bech32_address: &str) -> WalletResult<Self> {
        use bech32::Hrp;

        let (hrp, data) = bech32::decode(bech32_address)
            .map_err(|e| WalletError::InvalidAddress(format!("Invalid Bech32: {}", e)))?;

        let expected_hrp = Hrp::parse(CHERT_BECH32_HRP)
            .map_err(|e| WalletError::InvalidAddress(format!("Invalid HRP: {}", e)))?;
        if hrp != expected_hrp {
            return Err(WalletError::InvalidAddress(
                "Invalid Bech32 HRP (must be 'chert')".to_string(),
            ));
        }

        // bech32 0.11+ returns bytes directly from decode
        let bytes = data;

        // For now, assume all parsed addresses are account type
        // In production, this would be determined by address format or prefix
        Self::from_bytes(bytes, AddressType::Account)
    }

    /// Get the raw address bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.raw_bytes
    }

    /// Get the string representation (hex format)
    pub fn as_string(&self) -> &str {
        &self.hex_address
    }

    /// Get the Bech32 string representation
    pub fn as_bech32_string(&self) -> WalletResult<String> {
        use bech32::Hrp;

        let hrp = Hrp::parse(CHERT_BECH32_HRP)
            .map_err(|e| WalletError::InvalidAddress(format!("Invalid HRP: {}", e)))?;
        // bech32 0.11+ encode takes bytes directly
        bech32::encode::<bech32::Bech32>(hrp, &self.raw_bytes)
            .map_err(|e| WalletError::InvalidAddress(format!("Bech32 encoding failed: {}", e)))
    }

    /// Get the address type
    pub fn address_type(&self) -> AddressType {
        self.address_type
    }

    /// Check if this is a valid address format
    pub fn is_valid(&self) -> bool {
        self.raw_bytes.len() == 20
            && self.hex_address.starts_with("0x")
            && self.hex_address.len() == 42
    }

    /// Generate a checksum for this address
    pub fn checksum(&self) -> WalletResult<ChertHash> {
        let hash = StandardCrypto::hash_with_domain(
            HashAlgorithm::Sha256,
            Some(b"CHERT_ADDRESS_V1"),
            &self.raw_bytes,
        );
        Ok(hash)
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // For now, display as hex. In the future, this could be configurable.
        write!(f, "{}", self.hex_address)
    }
}

impl FromStr for Address {
    type Err = WalletError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Address::from_string(s)
    }
}

/// Represents an amount of Chert tokens
///
/// Uses fixed-point arithmetic to avoid floating-point precision issues.
/// The base unit is the smallest divisible unit (like satoshis in Bitcoin).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Amount {
    /// The amount in base units (1 CHERT = 1_000_000_000_000_000_000 base units)
    base_units: u128,
}

impl Amount {
    /// Number of decimal places for CHERT (18 decimals, like ETH)
    pub const DECIMALS: u8 = 18;
    /// Base units per CHERT (10^18)
    pub const UNITS_PER_CHERT: u128 = 1_000_000_000_000_000_000;
    /// Maximum amount (u128 allows for very large supplies)
    pub const MAX_CHERT: u128 = 1_000_000_000_000; // 1 trillion CHERT max supply

    /// Create amount from base units
    pub fn from_base_units(base_units: u128) -> WalletResult<Self> {
        // Use checked multiplication to prevent overflow
        let max_base_units = Self::MAX_CHERT
            .checked_mul(Self::UNITS_PER_CHERT)
            .ok_or_else(|| {
                WalletError::InvalidAmount("Maximum supply calculation overflow".to_string())
            })?;

        if base_units > max_base_units {
            return Err(WalletError::InvalidAmount("Amount too large".to_string()));
        }

        Ok(Amount { base_units })
    }

    /// Create amount from CHERT (whole tokens)
    pub fn from_chert(chert: u128) -> WalletResult<Self> {
        if chert > Self::MAX_CHERT {
            return Err(WalletError::InvalidAmount("Amount too large".to_string()));
        }

        let base_units = chert
            .checked_mul(Self::UNITS_PER_CHERT)
            .ok_or_else(|| WalletError::InvalidAmount("Amount calculation overflow".to_string()))?;
        Self::from_base_units(base_units)
    }

    /// Create amount from string (supports decimal notation)
    pub fn from_string(amount_str: &str) -> WalletResult<Self> {
        if amount_str.is_empty() {
            return Err(WalletError::InvalidAmount(
                "Amount cannot be empty".to_string(),
            ));
        }

        // Handle decimal point
        let parts: Vec<&str> = amount_str.split('.').collect();
        if parts.len() > 2 {
            return Err(WalletError::InvalidAmount(
                "Invalid decimal format".to_string(),
            ));
        }

        let whole_part: u128 = parts[0]
            .parse()
            .map_err(|_| WalletError::InvalidAmount("Invalid number format".to_string()))?;

        let fractional_units = if parts.len() == 2 {
            let fractional_str = parts[1];
            if fractional_str.len() > Self::DECIMALS as usize {
                return Err(WalletError::InvalidAmount(
                    "Too many decimal places".to_string(),
                ));
            }

            // Pad with zeros to get full precision
            let padded = format!("{:0<18}", fractional_str);
            padded
                .parse::<u128>()
                .map_err(|_| WalletError::InvalidAmount("Invalid fractional part".to_string()))?
        } else {
            0
        };

        let total_base_units = whole_part
            .checked_mul(Self::UNITS_PER_CHERT)
            .and_then(|w| w.checked_add(fractional_units))
            .ok_or_else(|| WalletError::InvalidAmount("Amount overflow".to_string()))?;

        Self::from_base_units(total_base_units)
    }

    /// Get base units
    pub fn base_units(&self) -> u128 {
        self.base_units
    }

    /// Get amount as CHERT (may lose precision)
    pub fn as_chert(&self) -> f64 {
        self.base_units as f64 / Self::UNITS_PER_CHERT as f64
    }

    /// Get amount as string with full precision
    pub fn as_string(&self) -> String {
        let whole = self.base_units / Self::UNITS_PER_CHERT;
        let fractional = self.base_units % Self::UNITS_PER_CHERT;

        if fractional == 0 {
            whole.to_string()
        } else {
            let frac_str = format!("{:018}", fractional)
                .trim_end_matches('0')
                .to_string();
            format!("{}.{}", whole, frac_str)
        }
    }

    /// Check if amount is zero
    pub fn is_zero(&self) -> bool {
        self.base_units == 0
    }

    /// Get amount as string with specified decimal precision
    pub fn to_display_string(self, decimals: u8) -> String {
        let whole = self.base_units / Self::UNITS_PER_CHERT;
        let fractional = self.base_units % Self::UNITS_PER_CHERT;

        if fractional == 0 || decimals == 0 {
            whole.to_string()
        } else {
            let scale = 10_u128.pow((Self::DECIMALS - decimals) as u32);
            let scaled_fractional = (fractional + scale / 2) / scale; // Round to nearest

            if scaled_fractional == 0 {
                whole.to_string()
            } else {
                let frac_str = format!("{:0width$}", scaled_fractional, width = decimals as usize)
                    .trim_end_matches('0')
                    .to_string();
                if frac_str.is_empty() {
                    whole.to_string()
                } else {
                    format!("{}.{}", whole, frac_str)
                }
            }
        }
    }

    /// Add two amounts
    pub fn checked_add(&self, other: &Amount) -> WalletResult<Amount> {
        self.base_units
            .checked_add(other.base_units)
            .and_then(|sum| Amount::from_base_units(sum).ok())
            .ok_or_else(|| WalletError::InvalidAmount("Amount overflow in addition".to_string()))
    }

    /// Subtract two amounts
    pub fn checked_sub(&self, other: &Amount) -> WalletResult<Amount> {
        if self.base_units < other.base_units {
            return Err(WalletError::InvalidAmount(
                "Insufficient amount for subtraction".to_string(),
            ));
        }

        Amount::from_base_units(self.base_units - other.base_units)
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} CHERT", self.as_string())
    }
}

impl FromStr for Amount {
    type Err = WalletError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Amount::from_string(s)
    }
}

/// Represents a cryptographic public key used for address generation and signature verification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicKey {
    /// The raw public key bytes (Ed25519)
    bytes: Vec<u8>,
    /// Hex representation for easy display
    hex: String,
}

impl PublicKey {
    /// Ed25519 public key size in bytes
    pub const SIZE: usize = 32;

    /// Create a public key from raw bytes
    pub fn from_bytes(bytes: Vec<u8>) -> WalletResult<Self> {
        if bytes.len() != Self::SIZE {
            return Err(WalletError::CryptoError(format!(
                "Invalid public key size: expected {} bytes, got {}",
                Self::SIZE,
                bytes.len()
            )));
        }

        let hex = hex::encode(&bytes);
        Ok(PublicKey { bytes, hex })
    }

    /// Create a public key from hex string
    pub fn from_hex(hex_str: &str) -> WalletResult<Self> {
        let bytes = hex::decode(hex_str)
            .map_err(|_| WalletError::CryptoError("Invalid hex in public key".to_string()))?;
        Self::from_bytes(bytes)
    }

    /// Get the raw bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Get hex representation
    pub fn as_hex(&self) -> &str {
        &self.hex
    }

    /// Generate address from this public key using quantum-resistant SHA-3
    pub fn to_address(&self, address_type: AddressType) -> WalletResult<Address> {
        use sha3::{Digest, Sha3_256};

        let address_type_str = match address_type {
            AddressType::Account => "ACCOUNT",
            AddressType::Contract => "CONTRACT",
            AddressType::MultiSig => "MULTISIG",
            AddressType::Validator => "VALIDATOR",
        };

        // Generate address bytes using SHA-3 (same as utils but returning bytes)
        let mut hasher = Sha3_256::new();
        hasher.update(format!("CHERT_ADDRESS_{}_V2", address_type_str).as_bytes());
        hasher.update(&self.bytes);
        let hash = hasher.finalize();
        let address_bytes = hash[..20].to_vec();

        Address::from_bytes(address_bytes, address_type)
    }

    /// Verify a signature against data
    pub fn verify_signature(&self, data: &[u8], signature: &[u8]) -> WalletResult<bool> {
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};

        let verifying_key = VerifyingKey::from_bytes(
            &self
                .bytes
                .clone()
                .try_into()
                .map_err(|_| WalletError::CryptoError("Invalid public key format".to_string()))?,
        )
        .map_err(|e| WalletError::CryptoError(format!("Failed to create verifying key: {}", e)))?;

        let sig = Signature::from_bytes(
            signature
                .try_into()
                .map_err(|_| WalletError::CryptoError("Invalid signature format".to_string()))?,
        );

        match verifying_key.verify(data, &sig) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

/// Represents a cryptographic private key for signing transactions
#[derive(Clone, Serialize, Deserialize)]
pub struct PrivateKey {
    /// The raw private key bytes (Ed25519)
    bytes: Vec<u8>,
    /// Cached public key
    public_key: PublicKey,
}

impl PrivateKey {
    /// Ed25519 private key size in bytes
    pub const SIZE: usize = 32;

    /// Generate a new random private key
    pub fn generate() -> WalletResult<Self> {
        use ed25519_dalek::SigningKey;
        use rand::rngs::OsRng;
        use rand::RngCore;

        let mut secret_bytes = [0u8; 32];
        let mut rng = OsRng;
        rng.fill_bytes(&mut secret_bytes);

        let signing_key = SigningKey::from_bytes(&secret_bytes);
        let private_bytes = secret_bytes.to_vec();
        let public_bytes = signing_key.verifying_key().to_bytes().to_vec();

        let public_key = PublicKey::from_bytes(public_bytes)?;

        Ok(PrivateKey {
            bytes: private_bytes,
            public_key,
        })
    }

    /// Create a private key from raw bytes
    pub fn from_bytes(bytes: Vec<u8>) -> WalletResult<Self> {
        if bytes.len() != Self::SIZE {
            return Err(WalletError::CryptoError(format!(
                "Invalid private key size: expected {} bytes, got {}",
                Self::SIZE,
                bytes.len()
            )));
        }

        use ed25519_dalek::SigningKey;

        let secret_key_bytes: [u8; 32] = bytes
            .clone()
            .try_into()
            .map_err(|_| WalletError::CryptoError("Invalid private key format".to_string()))?;

        let signing_key = SigningKey::from_bytes(&secret_key_bytes);
        let public_bytes = signing_key.verifying_key().to_bytes().to_vec();
        let public_key = PublicKey::from_bytes(public_bytes)?;

        Ok(PrivateKey { bytes, public_key })
    }

    /// Create a private key from hex string
    pub fn from_hex(hex_str: &str) -> WalletResult<Self> {
        let bytes = hex::decode(hex_str)
            .map_err(|_| WalletError::CryptoError("Invalid hex in private key".to_string()))?;
        Self::from_bytes(bytes)
    }

    /// Get the associated public key
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }

    /// Sign data with this private key
    pub fn sign(&self, data: &[u8]) -> WalletResult<Vec<u8>> {
        use ed25519_dalek::{Signer, SigningKey};

        let secret_key_bytes: [u8; 32] = self
            .bytes
            .clone()
            .try_into()
            .map_err(|_| WalletError::CryptoError("Invalid private key format".to_string()))?;

        let signing_key = SigningKey::from_bytes(&secret_key_bytes);
        let signature = signing_key.sign(data);
        Ok(signature.to_bytes().to_vec())
    }

    /// Get hex representation (be careful with this - sensitive data!)
    pub fn as_hex(&self) -> String {
        hex::encode(&self.bytes)
    }

    /// Securely clear the private key from memory
    pub fn zeroize(&mut self) {
        self.bytes.fill(0);
    }
}

impl std::fmt::Debug for PrivateKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PrivateKey")
            .field("public_key", &self.public_key)
            .field("bytes", &"<redacted>")
            .finish()
    }
}

impl Drop for PrivateKey {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// Transaction input referencing a previous output
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionInput {
    /// Hash of the transaction containing the output being spent
    pub previous_tx_hash: String,
    /// Index of the output in the previous transaction
    pub output_index: u32,
    /// Signature proving ownership of the referenced output
    pub signature: Vec<u8>,
    /// Public key corresponding to the private key that signed
    pub public_key: PublicKey,
}

/// Transaction output creating new spendable tokens
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionOutput {
    /// Amount being sent to the recipient
    pub amount: Amount,
    /// Address of the recipient
    pub recipient: Address,
    /// Optional data payload
    pub data: Option<Vec<u8>>,
}

/// Simple transaction format compatible with the Chert blockchain core
/// This matches the format expected by silica nodes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockchainTransaction {
    /// Unique transaction ID
    pub tx_id: String,
    /// Sender address (0x format)
    pub sender: String,
    /// Recipient address (0x format)
    pub recipient: String,
    /// Amount in base units (u64)
    pub amount: u64,
    /// Transaction fee in base units (u64)
    pub fee: u64,
    /// Nonce for replay protection
    pub nonce: u64,
    /// Timestamp (seconds since epoch)
    pub timestamp: u64,
    /// Signature (hex string)
    pub signature: String,
    /// Optional data payload for smart contracts
    pub data: Option<Vec<u8>>,
}

impl BlockchainTransaction {
    /// Create a new blockchain-compatible transaction
    pub fn new(
        sender: String,
        recipient: String,
        amount: u64,
        fee: u64,
        nonce: u64,
        data: Option<Vec<u8>>,
    ) -> WalletResult<Self> {
        use uuid::Uuid;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| WalletError::ValidationError(format!("System time error: {}", e)))?
            .as_secs();

        Ok(BlockchainTransaction {
            tx_id: Uuid::new_v4().to_string(),
            sender,
            recipient,
            amount,
            fee,
            nonce,
            timestamp,
            signature: String::new(), // Set after signing
            data,
        })
    }

    /// Calculate transaction hash for signing (compatible with silica)
    pub fn calculate_hash(&self) -> WalletResult<String> {
        use sha3::{Digest, Sha3_256};
        let mut hasher = Sha3_256::new();

        hasher.update(self.sender.as_bytes());
        hasher.update(self.recipient.as_bytes());
        hasher.update(self.amount.to_le_bytes());
        hasher.update(self.fee.to_le_bytes());
        hasher.update(self.nonce.to_le_bytes());
        hasher.update(self.timestamp.to_le_bytes());

        if let Some(ref data) = self.data {
            hasher.update(data);
        }

        Ok(hex::encode(hasher.finalize()))
    }

    /// Sign the transaction with a private key
    pub fn sign(&mut self, private_key: &PrivateKey) -> WalletResult<()> {
        let hash = self.calculate_hash()?;
        let signature_bytes = private_key.sign(hash.as_bytes())?;
        self.signature = hex::encode(signature_bytes);
        Ok(())
    }

    /// Verify the transaction signature
    pub fn verify_signature(&self, public_key: &PublicKey) -> WalletResult<bool> {
        if self.signature.is_empty() {
            return Ok(false);
        }

        let hash = self.calculate_hash()?;
        let signature_bytes = hex::decode(&self.signature)
            .map_err(|_| WalletError::CryptoError("Invalid signature hex".to_string()))?;

        public_key.verify_signature(hash.as_bytes(), &signature_bytes)
    }
}

/// Legacy UTXO-style transaction (deprecated, kept for compatibility)
/// This is the old wallet transaction format
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyTransaction {
    /// Unique transaction ID (hash of transaction content)
    pub id: String,
    /// Transaction version for future compatibility
    pub version: u32,
    /// List of inputs (funds being spent)
    pub inputs: Vec<TransactionInput>,
    /// List of outputs (funds being created/sent)
    pub outputs: Vec<TransactionOutput>,
    /// Transaction fee paid to miners
    pub fee: Amount,
    /// Timestamp when transaction was created
    pub timestamp: u64,
    /// Optional memo/description
    pub memo: Option<String>,
    /// Nonce for replay protection
    pub nonce: u64,
}

impl LegacyTransaction {
    /// Current transaction version
    pub const VERSION: u32 = 1;

    /// Create a new legacy transaction
    pub fn new(
        inputs: Vec<TransactionInput>,
        outputs: Vec<TransactionOutput>,
        fee: Amount,
        memo: Option<String>,
        nonce: u64,
    ) -> WalletResult<Self> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| WalletError::ValidationError(format!("System time error: {}", e)))?
            .as_secs();

        let mut tx = LegacyTransaction {
            id: String::new(), // Will be calculated
            version: Self::VERSION,
            inputs,
            outputs,
            fee,
            timestamp,
            memo,
            nonce,
        };

        tx.id = tx.calculate_id()?;
        Ok(tx)
    }

    /// Calculate transaction ID (hash of content)
    pub fn calculate_id(&self) -> WalletResult<String> {
        // Create a version without the ID for hashing
        let tx_for_hash = TransactionForHash {
            version: self.version,
            inputs: &self.inputs,
            outputs: &self.outputs,
            fee: &self.fee,
            timestamp: self.timestamp,
            memo: &self.memo,
            nonce: self.nonce,
        };

        let serialized = serde_json::to_string(&tx_for_hash)
            .map_err(|e| WalletError::ValidationError(format!("Serialization error: {}", e)))?;

        let hash = StandardCrypto::hash_with_domain(
            HashAlgorithm::Sha256,
            Some(b"CHERT_TRANSACTION_V1"),
            serialized.as_bytes(),
        );

        Ok(hash.hex)
    }

    /// Get total input amount
    pub fn total_input_amount(&self) -> Amount {
        // In a real implementation, this would look up the referenced outputs
        // For now, we'll return zero as a placeholder
        Amount::from_base_units(0).unwrap()
    }

    /// Get total output amount
    pub fn total_output_amount(&self) -> Amount {
        self.outputs
            .iter()
            .map(|output| &output.amount)
            .fold(Amount::from_base_units(0).unwrap(), |acc, amount| {
                acc.checked_add(amount).unwrap_or(acc)
            })
    }

    /// Validate transaction structure and amounts
    pub fn validate(&self) -> WalletResult<()> {
        // Check inputs exist
        if self.inputs.is_empty() {
            return Err(WalletError::ValidationError(
                "Transaction must have at least one input".to_string(),
            ));
        }

        // Check outputs exist
        if self.outputs.is_empty() {
            return Err(WalletError::ValidationError(
                "Transaction must have at least one output".to_string(),
            ));
        }

        // Validate ID matches content
        let calculated_id = self.calculate_id()?;
        if self.id != calculated_id {
            return Err(WalletError::ValidationError(
                "Transaction ID mismatch".to_string(),
            ));
        }

        // In a real implementation, we would validate:
        // - Input/output balance (inputs >= outputs + fee)
        // - Signature verification for all inputs
        // - Address validity for all outputs
        // - Double-spend prevention

        Ok(())
    }

    /// Sign transaction inputs with the provided private key
    pub fn sign_inputs(&mut self, private_key: &PrivateKey) -> WalletResult<()> {
        let tx_data = self.get_signing_data()?;

        for input in &mut self.inputs {
            let signature = private_key.sign(&tx_data)?;
            input.signature = signature;
            input.public_key = private_key.public_key().clone();
        }

        // Recalculate ID after signing
        self.id = self.calculate_id()?;
        Ok(())
    }

    /// Get data that should be signed for this transaction
    fn get_signing_data(&self) -> WalletResult<Vec<u8>> {
        // Create a version with empty signatures for signing
        let tx_for_signing = TransactionForSigning {
            version: self.version,
            inputs: self
                .inputs
                .iter()
                .map(|input| InputForSigning {
                    previous_tx_hash: &input.previous_tx_hash,
                    output_index: input.output_index,
                })
                .collect(),
            outputs: &self.outputs,
            fee: &self.fee,
            timestamp: self.timestamp,
            memo: &self.memo,
            nonce: self.nonce,
        };

        let serialized = serde_json::to_string(&tx_for_signing)
            .map_err(|e| WalletError::ValidationError(format!("Serialization error: {}", e)))?;

        Ok(serialized.as_bytes().to_vec())
    }
}

/// Type alias for backward compatibility
/// The main Transaction type is now the blockchain-compatible format
pub type Transaction = BlockchainTransaction;

impl BlockchainTransaction {
    /// Convert a legacy UTXO transaction to blockchain format
    /// This is a lossy conversion - UTXO inputs/outputs are simplified to sender/recipient
    pub fn from_legacy(
        legacy_tx: &LegacyTransaction,
        sender: String,
        recipient: String,
    ) -> WalletResult<Self> {
        // Calculate total output amount (excluding fee)
        let total_output = legacy_tx.total_output_amount();
        let amount = total_output.base_units();
        let fee = legacy_tx.fee.base_units();

        // Ensure amounts fit in u64
        if amount > u64::MAX as u128 {
            return Err(WalletError::InvalidAmount(
                "Amount too large for blockchain format".to_string(),
            ));
        }
        if fee > u64::MAX as u128 {
            return Err(WalletError::InvalidAmount(
                "Fee too large for blockchain format".to_string(),
            ));
        }

        BlockchainTransaction::new(
            sender,
            recipient,
            amount as u64,
            fee as u64,
            legacy_tx.nonce,
            legacy_tx.memo.as_ref().map(|m| m.as_bytes().to_vec()),
        )
    }

    /// Convert to legacy format (for internal wallet operations)
    /// This creates a simple single-input, single-output UTXO transaction
    pub fn to_legacy(&self) -> WalletResult<LegacyTransaction> {
        // Create simple input (placeholder for account-based tx)
        let input = TransactionInput {
            previous_tx_hash: "account_balance".to_string(),
            output_index: 0,
            signature: hex::decode(&self.signature)
                .map_err(|_| WalletError::CryptoError("Invalid signature hex".to_string()))?,
            public_key: PublicKey::from_bytes(vec![0u8; 32])?, // Placeholder
        };

        // Create output
        let recipient_addr = Address::from_string(&self.recipient)?;
        let amount = Amount::from_base_units(self.amount as u128)?;
        let output = TransactionOutput {
            amount,
            recipient: recipient_addr,
            data: self.data.clone(),
        };

        let fee = Amount::from_base_units(self.fee as u128)?;
        let memo = self
            .data
            .as_ref()
            .and_then(|d| String::from_utf8(d.clone()).ok());

        LegacyTransaction::new(vec![input], vec![output], fee, memo, self.nonce)
    }
}

/// Helper struct for transaction hashing (excludes ID)
#[derive(Serialize)]
struct TransactionForHash<'a> {
    version: u32,
    inputs: &'a [TransactionInput],
    outputs: &'a [TransactionOutput],
    fee: &'a Amount,
    timestamp: u64,
    memo: &'a Option<String>,
    nonce: u64,
}

/// Helper struct for transaction signing (excludes signatures)
#[derive(Serialize)]
struct TransactionForSigning<'a> {
    version: u32,
    inputs: Vec<InputForSigning<'a>>,
    outputs: &'a [TransactionOutput],
    fee: &'a Amount,
    timestamp: u64,
    memo: &'a Option<String>,
    nonce: u64,
}

#[derive(Serialize)]
struct InputForSigning<'a> {
    previous_tx_hash: &'a str,
    output_index: u32,
}

// Note: bech32 0.11+ handles byte conversion internally in encode/decode functions.
// The legacy bech32_data_to_bytes and bytes_to_bech32_data functions have been removed.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_creation() {
        let bytes = vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ];
        let addr = Address::from_bytes(bytes.clone(), AddressType::Account).unwrap();
        assert_eq!(addr.as_bytes(), &bytes);
        assert!(addr.is_valid());
    }

    #[test]
    fn test_address_parsing() {
        let addr_str = "0x0102030405060708090a0b0c0d0e0f1011121314";
        let addr = Address::from_string(addr_str).unwrap();
        assert_eq!(addr.as_string(), addr_str);
    }

    #[test]
    fn test_amount_creation() {
        let amount = Amount::from_chert(5).unwrap();
        assert_eq!(amount.as_chert(), 5.0);
        assert_eq!(amount.base_units(), 5 * Amount::UNITS_PER_CHERT);
    }

    #[test]
    fn test_amount_parsing() {
        let amount = Amount::from_string("1.5").unwrap();
        assert_eq!(amount.base_units(), 1_500_000_000_000_000_000);
        assert_eq!(amount.as_string(), "1.5");
    }

    #[test]
    fn test_amount_arithmetic() {
        let a1 = Amount::from_chert(3).unwrap();
        let a2 = Amount::from_chert(2).unwrap();

        let sum = a1.checked_add(&a2).unwrap();
        assert_eq!(sum.as_chert(), 5.0);

        let diff = a1.checked_sub(&a2).unwrap();
        assert_eq!(diff.as_chert(), 1.0);
    }

    #[test]
    fn test_key_generation() {
        let private_key = PrivateKey::generate().unwrap();
        let public_key = private_key.public_key();

        // Test key sizes
        assert_eq!(private_key.bytes.len(), PrivateKey::SIZE);
        assert_eq!(public_key.as_bytes().len(), PublicKey::SIZE);

        // Test address generation
        let address = public_key.to_address(AddressType::Account).unwrap();
        assert!(address.is_valid());
    }

    #[test]
    fn test_signing_and_verification() {
        let private_key = PrivateKey::generate().unwrap();
        let public_key = private_key.public_key();

        let data = b"Hello, Chert!";
        let signature = private_key.sign(data).unwrap();

        // Verify with correct key
        assert!(public_key.verify_signature(data, &signature).unwrap());

        // Should fail with different data
        let wrong_data = b"Wrong message";
        assert!(!public_key.verify_signature(wrong_data, &signature).unwrap());
    }

    #[test]
    fn test_key_serialization() {
        let private_key = PrivateKey::generate().unwrap();
        let hex_private = private_key.as_hex();
        let hex_public = private_key.public_key().as_hex();

        // Test round-trip
        let restored_private = PrivateKey::from_hex(&hex_private).unwrap();
        let restored_public = PublicKey::from_hex(hex_public).unwrap();

        assert_eq!(
            private_key.public_key().as_bytes(),
            restored_private.public_key().as_bytes()
        );
        assert_eq!(
            restored_public.as_bytes(),
            private_key.public_key().as_bytes()
        );
    }

    #[test]
    fn test_transaction_creation() {
        let private_key = PrivateKey::generate().unwrap();
        let recipient_key = PrivateKey::generate().unwrap();

        let sender_address = private_key
            .public_key()
            .to_address(AddressType::Account)
            .unwrap();
        let recipient_address = recipient_key
            .public_key()
            .to_address(AddressType::Account)
            .unwrap();

        // Test new blockchain-compatible transaction format
        let mut tx = Transaction::new(
            sender_address.as_string().to_string(),
            recipient_address.as_string().to_string(),
            10_000_000_000_000_000_000, // 10 CHERT in base units (reduced to fit u64)
            1_000_000_000_000_000_000,  // 1 CHERT fee in base units
            1,                          // nonce
            Some("Test transaction".as_bytes().to_vec()), // data
        )
        .unwrap();

        // Test signing
        tx.sign(&private_key).unwrap();

        assert!(!tx.tx_id.is_empty());
        assert_eq!(tx.sender, sender_address.as_string());
        assert_eq!(tx.recipient, recipient_address.as_string());
        assert_eq!(tx.amount, 10_000_000_000_000_000_000);
        assert_eq!(tx.fee, 1_000_000_000_000_000_000);
        assert_eq!(tx.nonce, 1);
        assert!(!tx.signature.is_empty());

        // Test signature verification
        let is_valid = tx.verify_signature(private_key.public_key()).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_transaction_validation() {
        let private_key = PrivateKey::generate().unwrap();
        let recipient_key = PrivateKey::generate().unwrap();

        let sender_address = private_key
            .public_key()
            .to_address(AddressType::Account)
            .unwrap();
        let recipient_address = recipient_key
            .public_key()
            .to_address(AddressType::Account)
            .unwrap();

        // Test creating and validating a blockchain transaction
        let mut tx = Transaction::new(
            sender_address.as_string().to_string(),
            recipient_address.as_string().to_string(),
            5_000_000_000_000_000_000, // 5 CHERT in base units (reduced to fit u64)
            1_000_000_000_000_000_000, // 1 CHERT fee in base units
            1,                         // nonce
            None,                      // no data
        )
        .unwrap();

        // Should have valid structure
        assert!(!tx.tx_id.is_empty());
        assert!(!tx.sender.is_empty());
        assert!(!tx.recipient.is_empty());
        assert!(tx.amount > 0);
        assert!(tx.fee > 0);

        // Test signing and verification
        tx.sign(&private_key).unwrap();
        let is_valid = tx.verify_signature(private_key.public_key()).unwrap();
        assert!(is_valid);

        // Test invalid signature detection
        let wrong_key = PrivateKey::generate().unwrap();
        let is_wrong = tx.verify_signature(wrong_key.public_key()).unwrap();
        assert!(!is_wrong);
    }

    #[test]
    fn test_transaction_signing() {
        let private_key = PrivateKey::generate().unwrap();
        let recipient_key = PrivateKey::generate().unwrap();

        let sender_address = private_key
            .public_key()
            .to_address(AddressType::Account)
            .unwrap();
        let recipient_address = recipient_key
            .public_key()
            .to_address(AddressType::Account)
            .unwrap();

        // Create unsigned transaction
        let mut tx = Transaction::new(
            sender_address.as_string().to_string(),
            recipient_address.as_string().to_string(),
            7_500_000_000_000_000_000, // 7.5 CHERT in base units (reduced to fit u64)
            2_000_000_000_000_000_000, // 2 CHERT fee in base units
            5,                         // nonce
            Some(b"Hello world".to_vec()), // data
        )
        .unwrap();

        let _original_id = tx.tx_id.clone();

        // Should not have signature initially
        assert!(tx.signature.is_empty());

        // Sign the transaction
        tx.sign(&private_key).unwrap();

        // Should now have signature
        assert!(!tx.signature.is_empty());

        // Verify signature is valid
        let is_valid = tx.verify_signature(private_key.public_key()).unwrap();
        assert!(is_valid);

        // Test conversion to/from legacy format
        let legacy_tx = tx.to_legacy().unwrap();
        assert_eq!(legacy_tx.nonce, tx.nonce);

        let converted_back =
            Transaction::from_legacy(&legacy_tx, tx.sender.clone(), tx.recipient.clone()).unwrap();
        assert_eq!(converted_back.amount, tx.amount);
        assert_eq!(converted_back.fee, tx.fee);
        assert_eq!(converted_back.nonce, tx.nonce);
    }
}
