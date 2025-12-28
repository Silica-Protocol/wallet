use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use blake3::Hasher as Blake3;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::errors::{WalletError, WalletResult};
use crate::storage::WalletPaths;

const CONFIG_VERSION: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkConfig {
    pub primary_endpoint: String,
    pub failover_endpoints: Vec<String>,
    pub allow_untrusted_certs: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            primary_endpoint: "https://mainnet.chert.network".to_string(),
            failover_endpoints: vec!["https://rpc-backup.chert.network".to_string()],
            allow_untrusted_certs: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionConfig {
    pub auto_lock_minutes: u32,
    pub max_failed_attempts: u32,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            auto_lock_minutes: 15,
            max_failed_attempts: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct TelemetryConfig {
    pub enable_analytics: bool,
    pub allow_error_reports: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WalletConfig {
    pub network: NetworkConfig,
    pub session: SessionConfig,
    pub telemetry: TelemetryConfig,
    pub environment: String,
    pub last_updated: DateTime<Utc>,
    pub version: u16,
}

impl WalletConfig {
    pub fn new(environment: impl Into<String>) -> Self {
        Self {
            network: NetworkConfig::default(),
            session: SessionConfig::default(),
            telemetry: TelemetryConfig::default(),
            environment: environment.into(),
            last_updated: Utc::now(),
            version: CONFIG_VERSION,
        }
    }

    pub fn touch(&mut self) {
        self.last_updated = Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConfigEnvelope {
    version: u16,
    checksum: [u8; 32],
    payload: WalletConfig,
    modified_at_unix: i64,
}

/// Handles persistence of wallet configuration with integrity checks.
#[derive(Debug, Clone)]
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn from_paths(paths: &WalletPaths) -> Self {
        Self {
            path: paths.config_file().to_path_buf(),
        }
    }

    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn load_or_default(&self, environment: impl Into<String>) -> WalletResult<WalletConfig> {
        if !self.path.exists() {
            let config = WalletConfig::new(environment);
            self.save(&config)?;
            return Ok(config);
        }

        let bytes = fs::read(&self.path)?;
        let envelope: ConfigEnvelope = serde_json::from_slice(&bytes)?;
        if envelope.version != CONFIG_VERSION {
            return Err(WalletError::ValidationError(format!(
                "Unsupported config version {}",
                envelope.version
            )));
        }

        let checksum = checksum(&envelope.payload);
        if checksum != envelope.checksum {
            return Err(WalletError::ValidationError(
                "Config integrity verification failed".to_string(),
            ));
        }

        Ok(envelope.payload)
    }

    pub fn save(&self, config: &WalletConfig) -> WalletResult<()> {
        let mut payload = config.clone();
        payload.touch();

        let envelope = ConfigEnvelope {
            version: CONFIG_VERSION,
            checksum: checksum(&payload),
            modified_at_unix: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_err(|e| WalletError::StorageError(e.to_string()))?
                .as_secs() as i64,
            payload,
        };

        let serialized = serde_json::to_vec_pretty(&envelope)?;
        let tmp_path = self.path.with_extension("new");
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        {
            let mut file = File::create(&tmp_path)?;
            file.write_all(&serialized)?;
            file.sync_all()?;
        }
        fs::rename(tmp_path, &self.path)?;
        Ok(())
    }

    pub fn update<F>(
        &self,
        environment: impl Into<String>,
        updater: F,
    ) -> WalletResult<WalletConfig>
    where
        F: FnOnce(&mut WalletConfig) -> WalletResult<()>,
    {
        let mut config = self.load_or_default(environment)?;
        updater(&mut config)?;
        config.touch();
        self.save(&config)?;
        Ok(config)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn checksum(config: &WalletConfig) -> [u8; 32] {
    let mut hasher = Blake3::new();
    let encoded = serde_json::to_vec(config).expect("config serialization must succeed");
    hasher.update(&encoded);
    let mut output = [0u8; 32];
    output.copy_from_slice(hasher.finalize().as_bytes());
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn save_and_load_config_round_trip() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("wallet.config");
        let store = ConfigStore::new(&path);

        let mut config = WalletConfig::new("development");
        config.network.primary_endpoint = "http://localhost:8545".into();
        store.save(&config).unwrap();

        let loaded = store.load_or_default("development").unwrap();
        assert_eq!(loaded.network.primary_endpoint, "http://localhost:8545");
    }

    #[test]
    fn tampered_config_detected() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("wallet.config");
        let store = ConfigStore::new(&path);
        store.save(&WalletConfig::new("test")).unwrap();

        let mut bytes = fs::read(&path).unwrap();
        if let Some(byte) = bytes.iter_mut().find(|b| **b != 0) {
            *byte ^= 0xAA;
        }
        fs::write(&path, bytes).unwrap();

        let result = store.load_or_default("test");
        assert!(matches!(result, Err(WalletError::ValidationError(_))));
    }
}
