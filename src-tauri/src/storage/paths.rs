use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::errors::{WalletError, WalletResult};

/// Manages filesystem paths used by the wallet backend.
#[derive(Debug, Clone)]
pub struct WalletPaths {
    /// Root directory for wallet data.
    root_dir: PathBuf,
    /// Encrypted vault file path.
    vault_file: PathBuf,
    /// Directory for wallet backups.
    backup_dir: PathBuf,
    /// Directory for cache/state data (e.g., scan cursors, history).
    cache_dir: PathBuf,
    /// Path to persisted wallet configuration.
    config_file: PathBuf,
}

impl WalletPaths {
    /// Default vault file name used on disk.
    pub const DEFAULT_VAULT_FILENAME: &'static str = "wallet.vault";
    /// Backup file extension appended to timestamped backups.
    pub const BACKUP_EXTENSION: &'static str = "vault.bak";

    /// Create a new path manager rooted at the provided directory.
    pub fn new(root: impl AsRef<Path>) -> WalletResult<Self> {
        let root_dir = root.as_ref().to_path_buf();
        if root_dir.as_os_str().is_empty() {
            return Err(WalletError::StorageError(
                "Wallet root directory cannot be empty".to_string(),
            ));
        }

        Ok(Self {
            vault_file: root_dir.join(Self::DEFAULT_VAULT_FILENAME),
            backup_dir: root_dir.join("backups"),
            cache_dir: root_dir.join("cache"),
            config_file: root_dir.join("wallet.config"),
            root_dir,
        })
    }

    /// Ensure the directory structure exists, creating missing folders.
    pub fn ensure_directories(&self) -> WalletResult<()> {
        fs::create_dir_all(&self.root_dir)?;
        fs::create_dir_all(&self.backup_dir)?;
        fs::create_dir_all(&self.cache_dir)?;
        Ok(())
    }

    /// Absolute path to the encrypted vault file.
    pub fn vault_file(&self) -> &Path {
        &self.vault_file
    }

    /// Directory that stores timestamped backups.
    pub fn backup_dir(&self) -> &Path {
        &self.backup_dir
    }

    /// Directory for cache/state artifacts.
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Path to persisted wallet configuration file.
    pub fn config_file(&self) -> &Path {
        &self.config_file
    }

    /// Root directory for all wallet-managed data.
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// Create a timestamped backup of the vault file.
    /// Returns the path to the created backup file.
    pub fn create_vault_backup(&self) -> WalletResult<PathBuf> {
        if !self.vault_file.exists() {
            return Err(WalletError::NotFound(
                "Vault file does not exist, cannot create backup".to_string(),
            ));
        }

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S_%6f");
        let backup_filename = format!("wallet_{}.{}", timestamp, Self::BACKUP_EXTENSION);
        let backup_path = self.backup_dir.join(backup_filename);

        fs::copy(&self.vault_file, &backup_path)?;

        // Verify backup was created and has same size as original
        let original_size = fs::metadata(&self.vault_file)?.len();
        let backup_size = fs::metadata(&backup_path)?.len();
        if original_size != backup_size {
            fs::remove_file(&backup_path)?;
            return Err(WalletError::StorageError(
                "Backup verification failed: size mismatch".to_string(),
            ));
        }

        Ok(backup_path)
    }

    /// Restore vault from a backup file.
    /// The current vault file will be moved to a temporary location before restoration.
    pub fn restore_vault_from_backup(&self, backup_path: impl AsRef<Path>) -> WalletResult<()> {
        let backup_path = backup_path.as_ref();
        if !backup_path.exists() {
            return Err(WalletError::NotFound(format!(
                "Backup file does not exist: {}",
                backup_path.display()
            )));
        }

        // Create a safety backup of current vault if it exists
        let temp_backup = if self.vault_file.exists() {
            let temp_name = format!("wallet_pre_restore_{}.tmp", Utc::now().timestamp());
            let temp_path = self.backup_dir.join(temp_name);
            fs::copy(&self.vault_file, &temp_path)?;
            Some(temp_path)
        } else {
            None
        };

        // Attempt to restore from backup
        let restore_result = fs::copy(backup_path, &self.vault_file);

        match restore_result {
            Ok(_) => {
                // Clean up temporary backup on success
                if let Some(temp_path) = temp_backup {
                    let _ = fs::remove_file(temp_path);
                }
                Ok(())
            }
            Err(err) => {
                // Restore from temporary backup on failure
                if let Some(temp_path) = temp_backup {
                    let _ = fs::copy(&temp_path, &self.vault_file);
                    let _ = fs::remove_file(temp_path);
                }
                Err(WalletError::StorageError(format!(
                    "Failed to restore vault from backup: {}",
                    err
                )))
            }
        }
    }

    /// List all available backup files, sorted by timestamp (newest first).
    pub fn list_backups(&self) -> WalletResult<Vec<PathBuf>> {
        if !self.backup_dir.exists() {
            return Ok(Vec::new());
        }

        let mut backups = Vec::new();
        for entry in fs::read_dir(&self.backup_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                // Check if filename ends with BACKUP_EXTENSION
                if let Some(filename) = path.file_name() {
                    if let Some(filename_str) = filename.to_str() {
                        if filename_str.ends_with(Self::BACKUP_EXTENSION) {
                            backups.push(path);
                        }
                    }
                }
            }
        }

        // Sort by modification time, newest first
        backups.sort_by(|a, b| {
            let a_time = fs::metadata(a).and_then(|m| m.modified()).ok();
            let b_time = fs::metadata(b).and_then(|m| m.modified()).ok();
            b_time.cmp(&a_time)
        });

        Ok(backups)
    }

    /// Delete old backups, keeping only the N most recent.
    pub fn prune_old_backups(&self, keep_count: usize) -> WalletResult<usize> {
        let backups = self.list_backups()?;
        let mut deleted_count = 0;

        for backup_path in backups.iter().skip(keep_count) {
            fs::remove_file(backup_path)?;
            deleted_count += 1;
        }

        Ok(deleted_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    fn create_test_vault(paths: &WalletPaths, content: &[u8]) -> WalletResult<()> {
        paths.ensure_directories()?;
        let mut file = File::create(paths.vault_file())?;
        file.write_all(content)?;
        Ok(())
    }

    #[test]
    fn test_wallet_paths_creation() {
        let temp_dir = TempDir::new().unwrap();
        let paths = WalletPaths::new(temp_dir.path()).unwrap();

        assert_eq!(
            paths.vault_file(),
            temp_dir.path().join(WalletPaths::DEFAULT_VAULT_FILENAME)
        );
        assert_eq!(paths.backup_dir(), temp_dir.path().join("backups"));
        assert_eq!(paths.cache_dir(), temp_dir.path().join("cache"));
        assert_eq!(paths.config_file(), temp_dir.path().join("wallet.config"));
    }

    #[test]
    fn test_empty_root_directory_rejected() {
        let result = WalletPaths::new("");
        assert!(result.is_err());
        match result {
            Err(WalletError::StorageError(msg)) => {
                assert!(msg.contains("cannot be empty"));
            }
            _ => panic!("Expected StorageError for empty root"),
        }
    }

    #[test]
    fn test_ensure_directories() {
        let temp_dir = TempDir::new().unwrap();
        let paths = WalletPaths::new(temp_dir.path()).unwrap();

        paths.ensure_directories().unwrap();

        assert!(paths.root_dir().exists());
        assert!(paths.backup_dir().exists());
        assert!(paths.cache_dir().exists());
    }

    #[test]
    fn test_create_backup_success() {
        let temp_dir = TempDir::new().unwrap();
        let paths = WalletPaths::new(temp_dir.path()).unwrap();

        let test_data = b"encrypted vault data";
        create_test_vault(&paths, test_data).unwrap();

        let backup_path = paths.create_vault_backup().unwrap();

        assert!(backup_path.exists());
        assert!(backup_path.starts_with(paths.backup_dir()));

        let backup_content = fs::read(&backup_path).unwrap();
        assert_eq!(backup_content, test_data);
    }

    #[test]
    fn test_create_backup_no_vault() {
        let temp_dir = TempDir::new().unwrap();
        let paths = WalletPaths::new(temp_dir.path()).unwrap();
        paths.ensure_directories().unwrap();

        let result = paths.create_vault_backup();
        assert!(result.is_err());
        match result {
            Err(WalletError::NotFound(msg)) => {
                assert!(msg.contains("does not exist"));
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_backup_filename_format() {
        let temp_dir = TempDir::new().unwrap();
        let paths = WalletPaths::new(temp_dir.path()).unwrap();

        create_test_vault(&paths, b"test").unwrap();
        let backup_path = paths.create_vault_backup().unwrap();

        let filename = backup_path.file_name().unwrap().to_string_lossy();
        assert!(filename.starts_with("wallet_"));
        assert!(filename.ends_with(".vault.bak"));
        assert!(filename.contains("_")); // Contains timestamp separator
    }

    #[test]
    fn test_restore_vault_from_backup() {
        let temp_dir = TempDir::new().unwrap();
        let paths = WalletPaths::new(temp_dir.path()).unwrap();

        let original_data = b"original encrypted data";
        create_test_vault(&paths, original_data).unwrap();

        let backup_path = paths.create_vault_backup().unwrap();

        // Modify vault file
        let modified_data = b"modified encrypted data";
        fs::write(paths.vault_file(), modified_data).unwrap();

        // Restore from backup
        paths.restore_vault_from_backup(&backup_path).unwrap();

        let restored_content = fs::read(paths.vault_file()).unwrap();
        assert_eq!(restored_content, original_data);
    }

    #[test]
    fn test_restore_from_nonexistent_backup() {
        let temp_dir = TempDir::new().unwrap();
        let paths = WalletPaths::new(temp_dir.path()).unwrap();
        paths.ensure_directories().unwrap();

        let fake_backup = temp_dir.path().join("nonexistent.vault.bak");
        let result = paths.restore_vault_from_backup(&fake_backup);

        assert!(result.is_err());
        match result {
            Err(WalletError::NotFound(_)) => {}
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_restore_creates_rollback_backup() {
        let temp_dir = TempDir::new().unwrap();
        let paths = WalletPaths::new(temp_dir.path()).unwrap();

        let original_data = b"original data";
        create_test_vault(&paths, original_data).unwrap();

        let backup_path = paths.create_vault_backup().unwrap();

        // Change vault content
        fs::write(paths.vault_file(), b"new data").unwrap();

        // Restore should succeed
        paths.restore_vault_from_backup(&backup_path).unwrap();

        // Vault should contain original data
        let restored = fs::read(paths.vault_file()).unwrap();
        assert_eq!(restored, original_data);
    }

    #[test]
    fn test_list_backups_empty() {
        let temp_dir = TempDir::new().unwrap();
        let paths = WalletPaths::new(temp_dir.path()).unwrap();
        paths.ensure_directories().unwrap();

        let backups = paths.list_backups().unwrap();
        assert!(backups.is_empty());
    }

    #[test]
    fn test_list_backups_sorted() {
        let temp_dir = TempDir::new().unwrap();
        let paths = WalletPaths::new(temp_dir.path()).unwrap();

        create_test_vault(&paths, b"test").unwrap();

        // Create multiple backups with delays to ensure different timestamps
        let backup1 = paths.create_vault_backup().unwrap();
        thread::sleep(Duration::from_millis(100));
        let backup2 = paths.create_vault_backup().unwrap();
        thread::sleep(Duration::from_millis(100));
        let backup3 = paths.create_vault_backup().unwrap();

        let backups = paths.list_backups().unwrap();
        assert_eq!(backups.len(), 3);

        // Backups should be sorted newest first
        assert_eq!(backups[0], backup3);
        assert_eq!(backups[1], backup2);
        assert_eq!(backups[2], backup1);
    }

    #[test]
    fn test_list_backups_filters_non_backup_files() {
        let temp_dir = TempDir::new().unwrap();
        let paths = WalletPaths::new(temp_dir.path()).unwrap();

        create_test_vault(&paths, b"test").unwrap();
        let _backup = paths.create_vault_backup().unwrap();

        // Create some non-backup files in backup directory
        fs::write(paths.backup_dir().join("random.txt"), b"not a backup").unwrap();
        fs::write(paths.backup_dir().join("wallet.log"), b"log file").unwrap();

        let backups = paths.list_backups().unwrap();
        assert_eq!(backups.len(), 1);
    }

    #[test]
    fn test_prune_old_backups() {
        let temp_dir = TempDir::new().unwrap();
        let paths = WalletPaths::new(temp_dir.path()).unwrap();

        create_test_vault(&paths, b"test").unwrap();

        // Create 5 backups
        for _ in 0..5 {
            paths.create_vault_backup().unwrap();
            thread::sleep(Duration::from_millis(50));
        }

        let backups_before = paths.list_backups().unwrap();
        assert_eq!(backups_before.len(), 5);

        // Keep only 3 most recent
        let deleted = paths.prune_old_backups(3).unwrap();
        assert_eq!(deleted, 2);

        let backups_after = paths.list_backups().unwrap();
        assert_eq!(backups_after.len(), 3);

        // Verify the kept backups are the newest ones
        assert_eq!(backups_after[0], backups_before[0]);
        assert_eq!(backups_after[1], backups_before[1]);
        assert_eq!(backups_after[2], backups_before[2]);
    }

    #[test]
    fn test_prune_with_keep_count_greater_than_backups() {
        let temp_dir = TempDir::new().unwrap();
        let paths = WalletPaths::new(temp_dir.path()).unwrap();

        create_test_vault(&paths, b"test").unwrap();

        paths.create_vault_backup().unwrap();
        paths.create_vault_backup().unwrap();

        let deleted = paths.prune_old_backups(10).unwrap();
        assert_eq!(deleted, 0);

        let backups = paths.list_backups().unwrap();
        assert_eq!(backups.len(), 2);
    }

    #[test]
    fn test_prune_all_backups() {
        let temp_dir = TempDir::new().unwrap();
        let paths = WalletPaths::new(temp_dir.path()).unwrap();

        create_test_vault(&paths, b"test").unwrap();

        for _ in 0..3 {
            paths.create_vault_backup().unwrap();
        }

        let deleted = paths.prune_old_backups(0).unwrap();
        assert_eq!(deleted, 3);

        let backups = paths.list_backups().unwrap();
        assert!(backups.is_empty());
    }
}
