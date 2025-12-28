use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalletError {
    // Cryptographic errors
    CryptoError(String),
    InvalidKey(String),
    SignatureError(String),

    // Network errors
    NetworkError(String),
    ConnectionTimeout,
    InvalidResponse(String),

    // Storage errors
    StorageError(String),
    FileNotFound(String),
    PermissionDenied(String),

    // Validation errors
    ValidationError(String),
    InvalidAddress(String),
    InvalidAmount(String),

    // Application errors
    NotInitialized,
    AlreadyExists(String),
    NotFound(String),

    // Generic errors
    Unknown(String),
}

impl fmt::Display for WalletError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WalletError::CryptoError(msg) => write!(f, "Cryptographic error: {}", msg),
            WalletError::InvalidKey(msg) => write!(f, "Invalid key: {}", msg),
            WalletError::SignatureError(msg) => write!(f, "Signature error: {}", msg),

            WalletError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            WalletError::ConnectionTimeout => write!(f, "Connection timeout"),
            WalletError::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),

            WalletError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            WalletError::FileNotFound(msg) => write!(f, "File not found: {}", msg),
            WalletError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),

            WalletError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            WalletError::InvalidAddress(msg) => write!(f, "Invalid address: {}", msg),
            WalletError::InvalidAmount(msg) => write!(f, "Invalid amount: {}", msg),

            WalletError::NotInitialized => write!(f, "Wallet not initialized"),
            WalletError::AlreadyExists(msg) => write!(f, "Already exists: {}", msg),
            WalletError::NotFound(msg) => write!(f, "Not found: {}", msg),

            WalletError::Unknown(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

impl std::error::Error for WalletError {}

pub type WalletResult<T> = Result<T, WalletError>;

// Helper macro for easy error creation
#[macro_export]
macro_rules! wallet_error {
    ($variant:ident, $msg:expr) => {
        WalletError::$variant($msg.to_string())
    };
    ($variant:ident) => {
        WalletError::$variant
    };
}

// Conversion helpers
impl From<std::io::Error> for WalletError {
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::NotFound => WalletError::FileNotFound(error.to_string()),
            std::io::ErrorKind::PermissionDenied => {
                WalletError::PermissionDenied(error.to_string())
            }
            _ => WalletError::StorageError(error.to_string()),
        }
    }
}

impl From<serde_json::Error> for WalletError {
    fn from(error: serde_json::Error) -> Self {
        WalletError::ValidationError(format!("JSON error: {}", error))
    }
}
