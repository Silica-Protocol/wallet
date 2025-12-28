use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;

use secrecy::SecretString;

use crate::config_store::{ConfigStore, WalletConfig};
use crate::errors::{WalletError, WalletResult};
use crate::session::SessionManager;
use crate::storage::{VaultCreateParams, VaultManager, VaultMetadata, VaultSecrets, WalletPaths};

#[derive(Debug)]
pub struct WalletContext {
    paths: WalletPaths,
    vault: VaultManager,
    config_store: ConfigStore,
    session: SessionManager,
    environment: String,
}

impl WalletContext {
    pub fn initialize(root_dir: PathBuf) -> WalletResult<Self> {
        let environment =
            std::env::var("CHERT_WALLET_ENV").unwrap_or_else(|_| "development".to_string());
        let paths = WalletPaths::new(&root_dir)?;
        paths.ensure_directories()?;

        let vault = VaultManager::from_paths(&paths);
        let config_store = ConfigStore::from_paths(&paths);
        let initial_config = config_store.load_or_default(environment.clone())?;
        let session_timeout = duration_from_minutes(initial_config.session.auto_lock_minutes);
        let session = SessionManager::new(
            session_timeout,
            initial_config.session.max_failed_attempts.max(1),
        );

        Ok(Self {
            paths,
            vault,
            config_store,
            session,
            environment,
        })
    }

    pub fn vault(&self) -> &VaultManager {
        &self.vault
    }

    pub fn session(&self) -> &SessionManager {
        &self.session
    }

    pub fn config_store(&self) -> &ConfigStore {
        &self.config_store
    }

    pub fn environment(&self) -> &str {
        &self.environment
    }

    pub fn load_config(&self) -> WalletResult<WalletConfig> {
        self.config_store.load_or_default(self.environment.clone())
    }

    pub fn update_config<F>(&mut self, updater: F) -> WalletResult<WalletConfig>
    where
        F: FnOnce(&mut WalletConfig) -> WalletResult<()>,
    {
        let updated = self
            .config_store
            .update(self.environment.clone(), updater)?;
        let session_timeout = duration_from_minutes(updated.session.auto_lock_minutes);
        self.session =
            SessionManager::new(session_timeout, updated.session.max_failed_attempts.max(1));
        Ok(updated)
    }

    pub fn paths(&self) -> &WalletPaths {
        &self.paths
    }

    pub fn create_vault(
        &self,
        password: &SecretString,
        metadata: VaultMetadata,
        secrets: VaultSecrets,
    ) -> WalletResult<()> {
        if self.vault.exists() {
            return Err(WalletError::AlreadyExists(
                self.vault.vault_path().display().to_string(),
            ));
        }

        let params = VaultCreateParams {
            password,
            metadata,
            secrets,
        };
        self.vault.create(params)
    }

    pub fn unlock(&self, password: &SecretString) -> WalletResult<()> {
        let unlocked = self.vault.unlock(password)?;
        self.session.unlock(unlocked)
    }

    pub fn lock(&self) {
        self.session.lock();
    }
}

/// Shared wallet context exposed to Tauri commands.
#[derive(Clone)]
pub struct SharedWalletContext(pub Arc<RwLock<WalletContext>>);

impl SharedWalletContext {
    pub fn new(inner: WalletContext) -> Self {
        Self(Arc::new(RwLock::new(inner)))
    }

    pub fn read<F, T>(&self, op: F) -> WalletResult<T>
    where
        F: FnOnce(&WalletContext) -> WalletResult<T>,
    {
        let guard = self
            .0
            .read()
            .map_err(|_| WalletError::Unknown("Poisoned wallet context".into()))?;
        op(&guard)
    }

    pub fn write<F, T>(&self, op: F) -> WalletResult<T>
    where
        F: FnOnce(&mut WalletContext) -> WalletResult<T>,
    {
        let mut guard = self
            .0
            .write()
            .map_err(|_| WalletError::Unknown("Poisoned wallet context".into()))?;
        op(&mut guard)
    }
}

fn duration_from_minutes(minutes: u32) -> Duration {
    let clamped = minutes.max(1) as u64;
    Duration::from_secs(clamped.saturating_mul(60))
}
