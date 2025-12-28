// lib.rs - Core library structure for the wallet

pub mod api;
pub mod app_state;
pub mod blockchain;
pub mod blockchain_client;
pub mod config_store;
pub mod crypto;
pub mod errors;
pub mod runtime;
pub mod security;
pub mod session;
pub mod storage;
pub mod validation;

// Crypto module is exposed directly via `crate::crypto`

pub mod wallet {
    //! Wallet functionality placeholder

    use crate::errors::WalletResult;
    use crate::security::{init_security_config, Environment};

    /// Initialize wallet subsystem
    pub fn init() -> WalletResult<()> {
        log::info!("Initializing wallet subsystem");

        // Initialize security configuration
        let _ = init_security_config(Environment::Development)?;
        log::info!("Security configuration initialized");

        Ok(())
    }
}

// Re-export common types
pub use api::types::*;
pub use api::types::{BalanceResponse, TransactionHistoryResponse, TransactionInfo};
pub use app_state::{SharedWalletContext, WalletContext};
pub use blockchain::{Address, AddressType, Amount};
pub use blockchain_client::BlockchainClient;
pub use config_store::{ConfigStore, NetworkConfig, SessionConfig, TelemetryConfig, WalletConfig};
pub use errors::{WalletError, WalletResult};
pub use runtime::RuntimeSecurityState;
pub use security::{Environment, SecurityConfig};
pub use session::SessionManager;
pub use storage::{VaultCreateParams, VaultManager, VaultMetadata, VaultSecrets, VaultUnlocked};
pub use validation::InputValidator;
