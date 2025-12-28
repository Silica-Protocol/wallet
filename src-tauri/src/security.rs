use crate::errors::{WalletError, WalletResult};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const KEY_LOG_LEVEL: &str = "LOG_LEVEL";
const KEY_NETWORK_ENDPOINT: &str = "NETWORK_ENDPOINT";
const KEY_TEST_NETWORK_ENDPOINT: &str = "TEST_NETWORK_ENDPOINT";
const KEY_BACKUP_ENCRYPTION_KEY: &str = "BACKUP_ENCRYPTION_KEY";
const KEY_ENABLE_ANALYTICS: &str = "ENABLE_ANALYTICS";
const KEY_SESSION_TIMEOUT: &str = "SESSION_TIMEOUT_MINUTES";
const KEY_AUTO_LOCK_MINUTES: &str = "AUTO_LOCK_MINUTES";
const KEY_MAX_FAILED_ATTEMPTS: &str = "MAX_FAILED_ATTEMPTS";
const KEY_ENABLE_BIOMETRICS: &str = "ENABLE_BIOMETRICS";
const KEY_BIOMETRIC_ENROLLED: &str = "BIOMETRIC_ENROLLED";
const KEY_BIOMETRIC_SUPPORTED: &str = "BIOMETRIC_SUPPORTED";
const KEY_BIOMETRIC_TOKEN_RETENTION: &str = "BIOMETRIC_TOKEN_RETENTION";
const KEY_ENABLE_PUSH: &str = "ENABLE_PUSH_NOTIFICATIONS";
const KEY_PUSH_PERMISSION_REQUIRED: &str = "PUSH_PERMISSION_REQUIRED";
const KEY_PUSH_MAX_DEVICES: &str = "PUSH_MAX_DEVICES";
const KEY_ENABLE_PASSKEYS: &str = "ENABLE_PASSKEYS";
const KEY_PASSKEY_MAX_CREDENTIALS: &str = "PASSKEY_MAX_CREDENTIALS";

/// Environment types for different security configurations
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Environment {
    Development,
    Production,
    Test,
}

/// Security configuration manager
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    environment: Environment,
    config_map: HashMap<String, String>,
}

impl SecurityConfig {
    /// Create a new security configuration
    pub fn new(environment: Environment) -> Self {
        let mut config = SecurityConfig {
            environment,
            config_map: HashMap::new(),
        };

        // Load default configuration based on environment
        config.load_defaults();
        config
    }

    /// Load configuration from environment variables
    pub fn from_env() -> WalletResult<Self> {
        let env_str =
            std::env::var("CHERT_ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
        let environment = match env_str.to_lowercase().as_str() {
            "production" | "prod" => Environment::Production,
            "test" | "testing" => Environment::Test,
            _ => Environment::Development,
        };

        Self::from_environment(environment)
    }

    /// Construct a configuration for the provided environment and apply overrides.
    pub fn from_environment(environment: Environment) -> WalletResult<Self> {
        assert!(
            matches!(
                environment,
                Environment::Development | Environment::Production | Environment::Test
            ),
            "unsupported environment"
        );
        let mut config = Self::new(environment);
        assert_eq!(config.environment, environment);
        assert!(
            !config.config_map.is_empty(),
            "default security configuration must not be empty"
        );

        // Load environment-specific variables
        config.load_from_env_vars()?;

        Ok(config)
    }

    /// Get a configuration value
    pub fn get(&self, key: &str) -> Option<&String> {
        self.config_map.get(key)
    }

    /// Get a configuration value or return default
    pub fn get_or_default(&self, key: &str, default: &str) -> String {
        self.config_map
            .get(key)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }

    /// Get a required configuration value
    pub fn get_required(&self, key: &str) -> WalletResult<&String> {
        self.config_map.get(key).ok_or_else(|| {
            WalletError::ValidationError(format!("Required config key '{}' not found", key))
        })
    }

    /// Retrieve a boolean flag, returning an error when missing.
    pub fn get_bool(&self, key: &str) -> WalletResult<bool> {
        assert!(!key.is_empty(), "configuration key must not be empty");
        assert_eq!(key, key.trim(), "configuration key must be trimmed");
        let value = self.get_required(key)?;
        parse_bool_flag(value, key)
    }

    /// Retrieve a boolean flag with a default fallback when the key is absent.
    pub fn get_bool_with_default(&self, key: &str, default: bool) -> WalletResult<bool> {
        assert!(!key.is_empty(), "configuration key must not be empty");
        assert_eq!(key, key.trim(), "configuration key must be trimmed");
        match self.config_map.get(key) {
            Some(value) => parse_bool_flag(value, key),
            None => Ok(default),
        }
    }

    /// Retrieve an unsigned 32-bit value with a default fallback.
    pub fn get_u32_with_default(&self, key: &str, default: u32) -> WalletResult<u32> {
        assert!(!key.is_empty(), "configuration key must not be empty");
        assert_eq!(key, key.trim(), "configuration key must be trimmed");
        match self.config_map.get(key) {
            Some(value) => parse_u32_value(value, key),
            None => Ok(default),
        }
    }

    /// Retrieve a comma-separated list of strings, guaranteeing at least one entry.
    pub fn get_string_list(&self, key: &str) -> WalletResult<Vec<String>> {
        assert!(!key.is_empty(), "configuration key must not be empty");
        assert_eq!(key, key.trim(), "configuration key must be trimmed");
        match self.config_map.get(key) {
            Some(value) => {
                let entries: Vec<String> = value
                    .split(',')
                    .map(|item| item.trim())
                    .filter(|item| !item.is_empty())
                    .map(|item| item.to_string())
                    .collect();
                if entries.is_empty() {
                    return Err(WalletError::ValidationError(format!(
                        "Configuration key '{}' cannot be an empty list",
                        key
                    )));
                }
                Ok(entries)
            }
            None => Err(WalletError::ValidationError(format!(
                "Configuration key '{}' not found",
                key
            ))),
        }
    }

    /// Set a configuration value (for testing purposes)
    pub fn set(&mut self, key: String, value: String) {
        self.config_map.insert(key, value);
    }

    /// Check if we're in production mode
    pub fn is_production(&self) -> bool {
        self.environment == Environment::Production
    }

    /// Check if we're in development mode
    pub fn is_development(&self) -> bool {
        self.environment == Environment::Development
    }

    /// Get the current environment
    pub fn environment(&self) -> &Environment {
        &self.environment
    }

    /// Validate that all required secrets are present
    pub fn validate_required_configs(&self) -> WalletResult<()> {
        let mut required_keys = vec![
            KEY_LOG_LEVEL,
            KEY_SESSION_TIMEOUT,
            KEY_AUTO_LOCK_MINUTES,
            KEY_MAX_FAILED_ATTEMPTS,
            KEY_ENABLE_ANALYTICS,
            KEY_ENABLE_BIOMETRICS,
            KEY_BIOMETRIC_ENROLLED,
            KEY_BIOMETRIC_SUPPORTED,
            KEY_BIOMETRIC_TOKEN_RETENTION,
            KEY_ENABLE_PUSH,
            KEY_PUSH_PERMISSION_REQUIRED,
            KEY_PUSH_MAX_DEVICES,
            KEY_ENABLE_PASSKEYS,
            KEY_PASSKEY_MAX_CREDENTIALS,
        ];

        match self.environment {
            Environment::Production | Environment::Development => {
                required_keys.push(KEY_NETWORK_ENDPOINT)
            }
            Environment::Test => required_keys.push(KEY_TEST_NETWORK_ENDPOINT),
        }

        if matches!(self.environment, Environment::Production) {
            required_keys.push(KEY_BACKUP_ENCRYPTION_KEY);
        }

        for key in required_keys {
            if !self.config_map.contains_key(key) {
                return Err(WalletError::ValidationError(format!(
                    "Required configuration key '{}' is missing for {} environment",
                    key,
                    format!("{:?}", self.environment).to_lowercase()
                )));
            }
        }

        Ok(())
    }

    /// Load default configuration values
    fn load_defaults(&mut self) {
        // Defaults shared across environments
        self.config_map
            .insert(KEY_ENABLE_ANALYTICS.to_string(), "false".to_string());

        match self.environment {
            Environment::Production => {
                self.config_map
                    .insert(KEY_LOG_LEVEL.to_string(), "INFO".to_string());
                self.config_map
                    .insert(KEY_SESSION_TIMEOUT.to_string(), "30".to_string());
                self.config_map
                    .insert(KEY_AUTO_LOCK_MINUTES.to_string(), "15".to_string());
                self.config_map
                    .insert(KEY_MAX_FAILED_ATTEMPTS.to_string(), "5".to_string());
                self.config_map.insert(
                    KEY_NETWORK_ENDPOINT.to_string(),
                    "https://mainnet.chert.network".to_string(),
                );
                self.config_map
                    .insert(KEY_ENABLE_BIOMETRICS.to_string(), "false".to_string());
                self.config_map
                    .insert(KEY_BIOMETRIC_ENROLLED.to_string(), "false".to_string());
                self.config_map.insert(
                    KEY_BIOMETRIC_SUPPORTED.to_string(),
                    "fingerprint,face".to_string(),
                );
                self.config_map
                    .insert(KEY_BIOMETRIC_TOKEN_RETENTION.to_string(), "24".to_string());
                self.config_map
                    .insert(KEY_ENABLE_PUSH.to_string(), "false".to_string());
                self.config_map
                    .insert(KEY_PUSH_PERMISSION_REQUIRED.to_string(), "true".to_string());
                self.config_map
                    .insert(KEY_PUSH_MAX_DEVICES.to_string(), "32".to_string());
                self.config_map
                    .insert(KEY_ENABLE_PASSKEYS.to_string(), "false".to_string());
                self.config_map
                    .insert(KEY_PASSKEY_MAX_CREDENTIALS.to_string(), "16".to_string());
            }
            Environment::Development => {
                self.config_map
                    .insert(KEY_LOG_LEVEL.to_string(), "DEBUG".to_string());
                self.config_map
                    .insert(KEY_SESSION_TIMEOUT.to_string(), "60".to_string());
                self.config_map
                    .insert(KEY_AUTO_LOCK_MINUTES.to_string(), "30".to_string());
                self.config_map
                    .insert(KEY_MAX_FAILED_ATTEMPTS.to_string(), "10".to_string());
                self.config_map.insert(
                    KEY_NETWORK_ENDPOINT.to_string(),
                    "http://localhost:8545".to_string(),
                );
                self.config_map
                    .insert(KEY_ENABLE_BIOMETRICS.to_string(), "true".to_string());
                self.config_map
                    .insert(KEY_BIOMETRIC_ENROLLED.to_string(), "true".to_string());
                self.config_map.insert(
                    KEY_BIOMETRIC_SUPPORTED.to_string(),
                    "fingerprint,face".to_string(),
                );
                self.config_map
                    .insert(KEY_BIOMETRIC_TOKEN_RETENTION.to_string(), "32".to_string());
                self.config_map
                    .insert(KEY_ENABLE_PUSH.to_string(), "true".to_string());
                self.config_map.insert(
                    KEY_PUSH_PERMISSION_REQUIRED.to_string(),
                    "false".to_string(),
                );
                self.config_map
                    .insert(KEY_PUSH_MAX_DEVICES.to_string(), "32".to_string());
                self.config_map
                    .insert(KEY_ENABLE_PASSKEYS.to_string(), "true".to_string());
                self.config_map
                    .insert(KEY_PASSKEY_MAX_CREDENTIALS.to_string(), "32".to_string());
            }
            Environment::Test => {
                self.config_map
                    .insert(KEY_LOG_LEVEL.to_string(), "WARN".to_string());
                self.config_map
                    .insert(KEY_SESSION_TIMEOUT.to_string(), "5".to_string());
                self.config_map
                    .insert(KEY_AUTO_LOCK_MINUTES.to_string(), "2".to_string());
                self.config_map
                    .insert(KEY_MAX_FAILED_ATTEMPTS.to_string(), "3".to_string());
                self.config_map.insert(
                    KEY_TEST_NETWORK_ENDPOINT.to_string(),
                    "https://testnet.chert.network".to_string(),
                );
                // Provide a default network endpoint to simplify integration tests
                self.config_map.insert(
                    KEY_NETWORK_ENDPOINT.to_string(),
                    "https://testnet.chert.network".to_string(),
                );
                self.config_map
                    .insert(KEY_ENABLE_BIOMETRICS.to_string(), "true".to_string());
                self.config_map
                    .insert(KEY_BIOMETRIC_ENROLLED.to_string(), "false".to_string());
                self.config_map.insert(
                    KEY_BIOMETRIC_SUPPORTED.to_string(),
                    "fingerprint".to_string(),
                );
                self.config_map
                    .insert(KEY_BIOMETRIC_TOKEN_RETENTION.to_string(), "16".to_string());
                self.config_map
                    .insert(KEY_ENABLE_PUSH.to_string(), "true".to_string());
                self.config_map
                    .insert(KEY_PUSH_PERMISSION_REQUIRED.to_string(), "true".to_string());
                self.config_map
                    .insert(KEY_PUSH_MAX_DEVICES.to_string(), "16".to_string());
                self.config_map
                    .insert(KEY_ENABLE_PASSKEYS.to_string(), "true".to_string());
                self.config_map
                    .insert(KEY_PASSKEY_MAX_CREDENTIALS.to_string(), "16".to_string());
            }
        }
    }

    /// Load configuration from environment variables
    fn load_from_env_vars(&mut self) -> WalletResult<()> {
        // Define environment variable mappings
        let env_mappings = [
            ("CHERT_LOG_LEVEL", KEY_LOG_LEVEL),
            ("CHERT_NETWORK_ENDPOINT", KEY_NETWORK_ENDPOINT),
            ("CHERT_TEST_NETWORK_ENDPOINT", KEY_TEST_NETWORK_ENDPOINT),
            ("CHERT_SESSION_TIMEOUT", KEY_SESSION_TIMEOUT),
            ("CHERT_AUTO_LOCK_TIMEOUT", KEY_AUTO_LOCK_MINUTES),
            ("CHERT_MAX_FAILED_ATTEMPTS", KEY_MAX_FAILED_ATTEMPTS),
            ("CHERT_BACKUP_KEY", KEY_BACKUP_ENCRYPTION_KEY),
            ("CHERT_ENABLE_ANALYTICS", KEY_ENABLE_ANALYTICS),
            ("CHERT_ENABLE_BIOMETRICS", KEY_ENABLE_BIOMETRICS),
            ("CHERT_BIOMETRIC_ENROLLED", KEY_BIOMETRIC_ENROLLED),
            ("CHERT_BIOMETRIC_TYPES", KEY_BIOMETRIC_SUPPORTED),
            (
                "CHERT_BIOMETRIC_TOKEN_RETENTION",
                KEY_BIOMETRIC_TOKEN_RETENTION,
            ),
            ("CHERT_ENABLE_PUSH", KEY_ENABLE_PUSH),
            (
                "CHERT_PUSH_PERMISSION_REQUIRED",
                KEY_PUSH_PERMISSION_REQUIRED,
            ),
            ("CHERT_PUSH_MAX_DEVICES", KEY_PUSH_MAX_DEVICES),
            ("CHERT_ENABLE_PASSKEYS", KEY_ENABLE_PASSKEYS),
            ("CHERT_PASSKEY_MAX_CREDENTIALS", KEY_PASSKEY_MAX_CREDENTIALS),
        ];

        for (env_var, config_key) in &env_mappings {
            if let Ok(value) = std::env::var(env_var) {
                // Validate that the value is not empty and doesn't contain suspicious content
                if value.trim().is_empty() {
                    log::warn!("Environment variable {} is empty", env_var);
                    continue;
                }

                // Basic security check - no newlines or control characters
                if value.chars().any(|c| c.is_control()) {
                    log::warn!(
                        "Environment variable {} contains control characters, ignoring",
                        env_var
                    );
                    continue;
                }

                self.config_map.insert(config_key.to_string(), value);
                log::debug!(
                    "Loaded configuration {} from environment variable {}",
                    config_key,
                    env_var
                );
            }
        }

        Ok(())
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self::new(Environment::Development)
    }
}

fn parse_bool_flag(value: &str, key: &str) -> WalletResult<bool> {
    assert!(!key.is_empty(), "configuration key must not be empty");
    assert!(
        value.len() <= 128,
        "configuration '{}' must not exceed 128 characters",
        key
    );
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(WalletError::ValidationError(format!(
            "Configuration key '{}' cannot be empty",
            key
        )));
    }

    match normalized.as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(WalletError::ValidationError(format!(
            "Invalid boolean value '{}' for key '{}'",
            value, key
        ))),
    }
}

fn parse_u32_value(value: &str, key: &str) -> WalletResult<u32> {
    assert!(!key.is_empty(), "configuration key must not be empty");
    assert!(value.len() <= 32, "configuration '{}' value too long", key);
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(WalletError::ValidationError(format!(
            "Configuration key '{}' cannot be empty",
            key
        )));
    }

    trimmed.parse::<u32>().map_err(|_| {
        WalletError::ValidationError(format!(
            "Invalid numeric value '{}' for key '{}'",
            value, key
        ))
    })
}

/// Global security configuration instance
static SECURITY_CONFIG: OnceCell<SecurityConfig> = OnceCell::new();

fn init_security_config_internal(
    environment: Option<Environment>,
) -> WalletResult<&'static SecurityConfig> {
    SECURITY_CONFIG.get_or_try_init(|| {
        let config = match environment {
            Some(explicit) => SecurityConfig::from_environment(explicit)?,
            None => SecurityConfig::from_env()?,
        };

        // Ensure the configuration maintains core invariants before it becomes global state.
        assert!(
            !config.config_map.is_empty(),
            "security configuration must pre-populate defaults"
        );
        assert!(
            config.config_map.contains_key(KEY_LOG_LEVEL),
            "security configuration must provide a log level"
        );

        config.validate_required_configs()?;
        log::info!(
            "Security configuration initialized for {:?} environment",
            config.environment
        );
        Ok(config)
    })
}

/// Initialize security configuration for a specific environment.
pub fn init_security_config(environment: Environment) -> WalletResult<&'static SecurityConfig> {
    init_security_config_internal(Some(environment))
}

/// Initialize security configuration using the environment selection logic.
pub fn init_security_config_from_env() -> WalletResult<&'static SecurityConfig> {
    init_security_config_internal(None)
}

/// Get global security configuration
pub fn get_security_config() -> WalletResult<&'static SecurityConfig> {
    SECURITY_CONFIG.get().ok_or(WalletError::NotInitialized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_config_creation() {
        let config = SecurityConfig::new(Environment::Development);
        assert_eq!(config.environment(), &Environment::Development);
        assert!(config.get("LOG_LEVEL").is_some());
    }

    #[test]
    fn test_required_config_validation() {
        let config = SecurityConfig::new(Environment::Development);
        assert!(config.validate_required_configs().is_ok());
    }

    #[test]
    fn test_environment_detection() {
        let config = SecurityConfig::new(Environment::Production);
        assert!(config.is_production());
        assert!(!config.is_development());
    }
}
