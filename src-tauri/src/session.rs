use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{Duration, Instant};

use zeroize::Zeroizing;

use crate::crypto::StealthKeyMaterial;
use crate::errors::{WalletError, WalletResult};
use crate::storage::{VaultMetadata, VaultSecrets, VaultUnlocked};

/// Default duration before an unlocked wallet automatically locks.
pub const DEFAULT_SESSION_TIMEOUT: Duration = Duration::from_secs(15 * 60);

#[derive(Debug)]
struct UnlockedSession {
    metadata: VaultMetadata,
    secrets: Zeroizing<VaultSecrets>,
    expires_at: Instant,
    stealth_keys: Option<StealthKeyMaterial>,
}

impl UnlockedSession {
    fn new(unlocked: VaultUnlocked, timeout: Duration) -> WalletResult<Self> {
        let stealth_keys = if unlocked.secrets.stealth_material.is_empty() {
            None
        } else {
            Some(StealthKeyMaterial::decode(
                &unlocked.secrets.stealth_material,
            )?)
        };

        Ok(Self {
            metadata: unlocked.metadata,
            secrets: Zeroizing::new(unlocked.secrets),
            expires_at: Instant::now() + timeout,
            stealth_keys,
        })
    }

    fn touch(&mut self, timeout: Duration) {
        self.expires_at = Instant::now() + timeout;
    }

    fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

#[derive(Debug, Default)]
struct SessionState {
    unlocked: Option<UnlockedSession>,
    failed_attempts: u32,
    next_allowed_attempt: Option<Instant>,
    backoff_exponent: u32,
}

/// Manages wallet unlock state and in-memory secret access with automatic locking.
#[derive(Debug, Clone)]
pub struct SessionManager {
    state: Arc<RwLock<SessionState>>,
    timeout: Duration,
    max_failed_attempts: u32,
    backoff_base: Duration,
    backoff_cap: Duration,
    max_backoff_exponent: u32,
}

impl SessionManager {
    pub fn new(timeout: Duration, max_failed_attempts: u32) -> Self {
        Self::with_backoff(
            timeout,
            max_failed_attempts,
            Duration::from_secs(1),
            Duration::from_secs(32),
        )
    }

    pub fn with_backoff(
        timeout: Duration,
        max_failed_attempts: u32,
        backoff_base: Duration,
        backoff_cap: Duration,
    ) -> Self {
        Self {
            state: Arc::new(RwLock::new(SessionState::default())),
            timeout,
            max_failed_attempts,
            backoff_base,
            backoff_cap,
            max_backoff_exponent: 8,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(DEFAULT_SESSION_TIMEOUT, 5)
    }

    pub fn is_locked(&self) -> bool {
        let state = self.state.read();
        state.unlocked.is_none()
    }

    /// Unlock the session with decrypted vault contents.
    pub fn unlock(&self, unlocked: VaultUnlocked) -> WalletResult<()> {
        let mut state = self.state.write();
        state.failed_attempts = 0;
        state.unlocked = Some(UnlockedSession::new(unlocked, self.timeout)?);
        state.next_allowed_attempt = None;
        state.backoff_exponent = 0;
        Ok(())
    }

    /// Record a failed unlock attempt and return remaining attempts.
    pub fn register_failed_attempt(&self) -> WalletResult<u32> {
        let mut state = self.state.write();
        let now = Instant::now();

        if let Some(until) = state.next_allowed_attempt {
            if now < until {
                let remaining = until.saturating_duration_since(now);
                let seconds = remaining.as_secs();
                let millis = remaining.subsec_millis();
                return Err(WalletError::PermissionDenied(format!(
                    "Unlock temporarily disabled. Retry in {}.{:03} seconds",
                    seconds, millis
                )));
            }
        }

        state.failed_attempts += 1;
        if state.failed_attempts >= self.max_failed_attempts {
            state.unlocked = None;
            state.next_allowed_attempt = None;
            state.backoff_exponent = 0;
            return Err(WalletError::PermissionDenied(
                "Maximum unlock attempts exceeded".to_string(),
            ));
        }
        state.backoff_exponent = (state.backoff_exponent + 1).min(self.max_backoff_exponent);
        let multiplier = 1_u32 << state.backoff_exponent.saturating_sub(1);
        let mut delay = if multiplier <= 1 {
            self.backoff_base
        } else {
            self.backoff_base
                .checked_mul(multiplier)
                .unwrap_or(self.backoff_cap)
        };
        if delay > self.backoff_cap {
            delay = self.backoff_cap;
        }
        state.next_allowed_attempt = Some(now + delay);

        Ok(self.max_failed_attempts - state.failed_attempts)
    }

    /// Explicitly lock the wallet session, zeroizing secrets.
    pub fn lock(&self) {
        let mut state = self.state.write();
        state.unlocked = None;
        state.next_allowed_attempt = None;
        state.backoff_exponent = 0;
    }

    /// Access unlocked secrets while refreshing the timeout.
    pub fn with_unlocked<F, T>(&self, operation: F) -> WalletResult<T>
    where
        F: FnOnce(&VaultMetadata, &VaultSecrets) -> WalletResult<T>,
    {
        let mut state = self.state.write();
        let session = state
            .unlocked
            .as_mut()
            .ok_or_else(|| WalletError::PermissionDenied("Wallet is locked".to_string()))?;

        if session.is_expired() {
            state.unlocked = None;
            return Err(WalletError::PermissionDenied(
                "Wallet session expired".to_string(),
            ));
        }

        session.touch(self.timeout);
        operation(&session.metadata, &session.secrets)
    }

    /// Access decoded stealth key material while refreshing the timeout.
    pub fn with_stealth_keys<F, T>(&self, operation: F) -> WalletResult<T>
    where
        F: FnOnce(&VaultMetadata, &StealthKeyMaterial) -> WalletResult<T>,
    {
        let mut state = self.state.write();
        let session = state
            .unlocked
            .as_mut()
            .ok_or_else(|| WalletError::PermissionDenied("Wallet is locked".to_string()))?;

        if session.is_expired() {
            state.unlocked = None;
            return Err(WalletError::PermissionDenied(
                "Wallet session expired".to_string(),
            ));
        }

        session.touch(self.timeout);
        let stealth_keys = session.stealth_keys.as_ref().ok_or_else(|| {
            WalletError::NotFound("Stealth key material not available".to_string())
        })?;

        operation(&session.metadata, stealth_keys)
    }

    /// Access unlocked secrets without extending the timeout (use sparingly for observers).
    pub fn peek_unlocked<F, T>(&self, operation: F) -> WalletResult<T>
    where
        F: FnOnce(&VaultMetadata, &VaultSecrets) -> WalletResult<T>,
    {
        let state = self.state.read();
        let session = state
            .unlocked
            .as_ref()
            .ok_or_else(|| WalletError::PermissionDenied("Wallet is locked".to_string()))?;

        if session.is_expired() {
            drop(state);
            self.lock();
            return Err(WalletError::PermissionDenied(
                "Wallet session expired".to_string(),
            ));
        }

        operation(&session.metadata, &session.secrets)
    }

    pub fn remaining_attempts(&self) -> u32 {
        let state = self.state.read();
        self.max_failed_attempts
            .saturating_sub(state.failed_attempts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{VaultMetadata, VaultSecrets};

    fn unlocked() -> VaultUnlocked {
        VaultUnlocked {
            metadata: VaultMetadata::new("Session Test"),
            secrets: VaultSecrets::new(vec![1, 2, 3]),
        }
    }

    #[test]
    fn unlock_and_lock_cycle() {
        let manager = SessionManager::with_defaults();
        assert!(manager.is_locked());

        manager.unlock(unlocked()).unwrap();
        assert!(!manager.is_locked());

        manager.lock();
        assert!(manager.is_locked());
    }

    #[test]
    fn timeout_enforced() {
        let manager = SessionManager::new(Duration::from_millis(10), 5);
        manager.unlock(unlocked()).unwrap();
        std::thread::sleep(Duration::from_millis(30));
        let result = manager.with_unlocked(|_, _| Ok(()));
        assert!(result.is_err());
        assert!(manager.is_locked());
    }

    #[test]
    fn failed_attempts_limit() {
        let manager = SessionManager::with_backoff(
            DEFAULT_SESSION_TIMEOUT,
            2,
            Duration::from_millis(10),
            Duration::from_millis(80),
        );
        assert_eq!(manager.remaining_attempts(), 2);
        assert_eq!(manager.register_failed_attempt().unwrap(), 1);
        std::thread::sleep(Duration::from_millis(15));
        let err = manager.register_failed_attempt().unwrap_err();
        assert!(matches!(err, WalletError::PermissionDenied(_)));
        assert_eq!(manager.remaining_attempts(), 0);
    }

    #[test]
    fn with_unlocked_provides_secrets() {
        let manager = SessionManager::with_defaults();
        manager.unlock(unlocked()).unwrap();
        let result = manager
            .with_unlocked(|metadata, secrets| {
                assert_eq!(metadata.wallet_name, "Session Test");
                assert_eq!(secrets.seed_bytes, vec![1, 2, 3]);
                Ok(42)
            })
            .unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn with_stealth_keys_provides_material() {
        let manager = SessionManager::with_defaults();
        let mut unlocked = unlocked();
        let stealth_seed = vec![9u8; 96];
        let material = StealthKeyMaterial::derive_from_seed(&stealth_seed).unwrap();
        unlocked.secrets.stealth_material = material.encode();

        manager.unlock(unlocked).unwrap();

        manager
            .with_stealth_keys(|_, keys| {
                assert_eq!(keys.view_secret(), material.view_secret());
                assert_eq!(keys.spend_secret(), material.spend_secret());
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn with_stealth_keys_errors_when_missing() {
        let manager = SessionManager::with_defaults();
        manager.unlock(unlocked()).unwrap();
        let err = manager
            .with_stealth_keys(|_, _| Ok(()))
            .expect_err("expected missing stealth material error");
        assert!(matches!(err, WalletError::NotFound(_)));
    }

    #[test]
    fn register_failed_attempt_enforces_backoff() {
        let manager = SessionManager::with_backoff(
            DEFAULT_SESSION_TIMEOUT,
            5,
            Duration::from_millis(10),
            Duration::from_millis(160),
        );
        assert_eq!(manager.register_failed_attempt().unwrap(), 4);
        let err = manager.register_failed_attempt().unwrap_err();
        assert!(matches!(err, WalletError::PermissionDenied(msg) if msg.contains("Retry")));
        std::thread::sleep(Duration::from_millis(15));
        assert_eq!(manager.register_failed_attempt().unwrap(), 3);
    }
}
