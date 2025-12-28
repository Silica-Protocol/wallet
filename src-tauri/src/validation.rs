use crate::errors::{WalletError, WalletResult};
use regex::Regex;
use std::collections::HashSet;

/// Input validation utilities for the wallet
pub struct InputValidator {
    // Compiled regex patterns for performance
    address_pattern: Regex,
    amount_pattern: Regex,
    password_pattern: Regex,

    // Blacklisted patterns for security
    malicious_patterns: Vec<Regex>,
}

impl InputValidator {
    pub fn new() -> WalletResult<Self> {
        let address_pattern = Regex::new(r"^0x[a-fA-F0-9]{40}$")
            .map_err(|e| WalletError::ValidationError(format!("Invalid address regex: {}", e)))?;

        let amount_pattern = Regex::new(r"^\d+(\.\d{1,18})?$")
            .map_err(|e| WalletError::ValidationError(format!("Invalid amount regex: {}", e)))?;

        let password_pattern = Regex::new(r"^[A-Za-z\d@$!%*?&]{12,}$")
            .map_err(|e| WalletError::ValidationError(format!("Invalid password regex: {}", e)))?;

        // Common malicious patterns to block
        let malicious_patterns = vec![
            Regex::new(r"<script").unwrap(),
            Regex::new(r"javascript:").unwrap(),
            Regex::new(r"data:text/html").unwrap(),
            Regex::new(r"vbscript:").unwrap(),
            Regex::new(r"onload=").unwrap(),
            Regex::new(r"onerror=").unwrap(),
        ];

        Ok(InputValidator {
            address_pattern,
            amount_pattern,
            password_pattern,
            malicious_patterns,
        })
    }

    /// Validate a blockchain address
    pub fn validate_address(&self, address: &str) -> WalletResult<()> {
        self.check_basic_security(address)?;

        if address.is_empty() {
            return Err(WalletError::ValidationError(
                "Address cannot be empty".to_string(),
            ));
        }

        if address.len() > 100 {
            return Err(WalletError::ValidationError("Address too long".to_string()));
        }

        if !self.address_pattern.is_match(address) {
            return Err(WalletError::InvalidAddress(
                "Address format is invalid".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate an amount string
    pub fn validate_amount(&self, amount: &str) -> WalletResult<()> {
        self.check_basic_security(amount)?;

        if amount.is_empty() {
            return Err(WalletError::ValidationError(
                "Amount cannot be empty".to_string(),
            ));
        }

        if !self.amount_pattern.is_match(amount) {
            return Err(WalletError::InvalidAmount(
                "Amount format is invalid".to_string(),
            ));
        }

        // Check for reasonable limits
        let parsed: f64 = amount
            .parse()
            .map_err(|_| WalletError::InvalidAmount("Invalid number format".to_string()))?;

        if parsed <= 0.0 {
            return Err(WalletError::InvalidAmount(
                "Amount must be positive".to_string(),
            ));
        }

        if parsed > 1_000_000_000.0 {
            return Err(WalletError::InvalidAmount("Amount too large".to_string()));
        }

        Ok(())
    }

    /// Validate password strength
    pub fn validate_password(&self, password: &str) -> WalletResult<()> {
        if password.len() < 12 {
            return Err(WalletError::ValidationError(
                "Password must be at least 12 characters".to_string(),
            ));
        }

        if password.len() > 256 {
            return Err(WalletError::ValidationError(
                "Password too long".to_string(),
            ));
        }

        if !self.password_pattern.is_match(password) {
            return Err(WalletError::ValidationError(
                "Password contains unsupported characters".to_string(),
            ));
        }

        let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
        let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        let special_chars = "@$!%*?&";
        let has_special = password.chars().any(|c| special_chars.contains(c));

        if !(has_upper && has_lower && has_digit && has_special) {
            return Err(WalletError::ValidationError(
                "Password must contain uppercase, lowercase, number, and special character"
                    .to_string(),
            ));
        }

        // Check against common passwords
        if self.is_common_password(password) {
            return Err(WalletError::ValidationError(
                "Password is too common, please choose a stronger password".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate wallet name/label
    pub fn validate_wallet_name(&self, name: &str) -> WalletResult<()> {
        self.check_basic_security(name)?;

        if name.is_empty() {
            return Err(WalletError::ValidationError(
                "Wallet name cannot be empty".to_string(),
            ));
        }

        if name.len() > 50 {
            return Err(WalletError::ValidationError(
                "Wallet name too long".to_string(),
            ));
        }

        // Allow only alphanumeric, spaces, hyphens, underscores
        let allowed_chars = Regex::new(r"^[a-zA-Z0-9\s\-_]+$").unwrap();
        if !allowed_chars.is_match(name) {
            return Err(WalletError::ValidationError(
                "Wallet name contains invalid characters".to_string(),
            ));
        }

        Ok(())
    }

    /// Sanitize input string by removing/escaping dangerous characters
    pub fn sanitize_input(&self, input: &str) -> String {
        input
            .chars()
            .filter(|c| c.is_alphanumeric() || " .-_@".contains(*c))
            .take(1000) // Limit length
            .collect()
    }

    /// Check for basic security issues in any input
    fn check_basic_security(&self, input: &str) -> WalletResult<()> {
        if input.len() > 1000 {
            return Err(WalletError::ValidationError("Input too long".to_string()));
        }

        // Check for malicious patterns
        for pattern in &self.malicious_patterns {
            if pattern.is_match(&input.to_lowercase()) {
                return Err(WalletError::ValidationError(
                    "Input contains potentially malicious content".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Check if password is in common passwords list (simplified version)
    fn is_common_password(&self, password: &str) -> bool {
        let common_passwords: HashSet<&str> = [
            "password123",
            "123456789",
            "qwertyuiop",
            "administrator",
            "Password123!",
            "welcome123",
            "password1234",
            "123456789a",
            "qwerty123456",
            "password@123",
        ]
        .iter()
        .cloned()
        .collect();

        common_passwords.contains(&password.to_lowercase().as_str())
    }
}

impl Default for InputValidator {
    fn default() -> Self {
        Self::new().expect("Failed to create InputValidator")
    }
}
