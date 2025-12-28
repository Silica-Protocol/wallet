/**
 * Input validation utilities for the Angular frontend
 * This mirrors the Rust validation logic for consistency
 */

export interface ValidationResult {
  isValid: boolean;
  error?: string;
}

export class InputValidator {
  private readonly addressPattern = /^chert1[a-zA-Z0-9]{39,59}$/;
  private readonly amountPattern = /^\d+(\.\d{1,18})?$/;
  private readonly passwordPattern = /^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)(?=.*[@$!%*?&])[A-Za-z\d@$!%*?&]{12,}$/;
  
  private readonly maliciousPatterns = [
    /<script/i,
    /javascript:/i,
    /data:text\/html/i,
    /vbscript:/i,
    /onload=/i,
    /onerror=/i,
  ];

  private readonly commonPasswords = new Set([
    'password123', '123456789', 'qwertyuiop', 'administrator',
    'password123!', 'welcome123', 'password1234', '123456789a',
    'qwerty123456', 'password@123'
  ]);

  /**
   * Validate a blockchain address
   */
  validateAddress(address: string): ValidationResult {
    const securityCheck = this.checkBasicSecurity(address);
    if (!securityCheck.isValid) {
      return securityCheck;
    }

    if (!address || address.length === 0) {
      return { isValid: false, error: 'Address cannot be empty' };
    }

    if (address.length > 100) {
      return { isValid: false, error: 'Address too long' };
    }

    if (!this.addressPattern.test(address)) {
      return { isValid: false, error: 'Address format is invalid' };
    }

    return { isValid: true };
  }

  /**
   * Validate an amount string
   */
  validateAmount(amount: string): ValidationResult {
    const securityCheck = this.checkBasicSecurity(amount);
    if (!securityCheck.isValid) {
      return securityCheck;
    }

    if (!amount || amount.length === 0) {
      return { isValid: false, error: 'Amount cannot be empty' };
    }

    if (!this.amountPattern.test(amount)) {
      return { isValid: false, error: 'Amount format is invalid' };
    }

    const parsed = parseFloat(amount);
    if (isNaN(parsed) || parsed <= 0) {
      return { isValid: false, error: 'Amount must be positive' };
    }

    if (parsed > 1_000_000_000) {
      return { isValid: false, error: 'Amount too large' };
    }

    return { isValid: true };
  }

  /**
   * Validate password strength
   */
  validatePassword(password: string): ValidationResult {
    if (!password || password.length < 12) {
      return { isValid: false, error: 'Password must be at least 12 characters' };
    }

    if (password.length > 256) {
      return { isValid: false, error: 'Password too long' };
    }

    if (!this.passwordPattern.test(password)) {
      return { 
        isValid: false, 
        error: 'Password must contain uppercase, lowercase, number, and special character' 
      };
    }

    if (this.isCommonPassword(password)) {
      return { 
        isValid: false, 
        error: 'Password is too common, please choose a stronger password' 
      };
    }

    return { isValid: true };
  }

  /**
   * Validate wallet name/label
   */
  validateWalletName(name: string): ValidationResult {
    const securityCheck = this.checkBasicSecurity(name);
    if (!securityCheck.isValid) {
      return securityCheck;
    }

    if (!name || name.length === 0) {
      return { isValid: false, error: 'Wallet name cannot be empty' };
    }

    if (name.length > 50) {
      return { isValid: false, error: 'Wallet name too long' };
    }

    const allowedChars = /^[a-zA-Z0-9\s\-_]+$/;
    if (!allowedChars.test(name)) {
      return { isValid: false, error: 'Wallet name contains invalid characters' };
    }

    return { isValid: true };
  }

  /**
   * Calculate password strength score (0-100)
   */
  calculatePasswordStrength(password: string): number {
    if (!password) return 0;

    let score = 0;

    // Length bonus
    score += Math.min(password.length * 2, 25);

    // Character variety bonus
    if (/[a-z]/.test(password)) score += 15;
    if (/[A-Z]/.test(password)) score += 15;
    if (/\d/.test(password)) score += 15;
    if (/[@$!%*?&]/.test(password)) score += 20;

    // Length penalties for very short passwords
    if (password.length < 8) score -= 20;
    if (password.length < 12) score -= 10;

    // Common password penalty
    if (this.isCommonPassword(password)) score -= 30;

    // Repetition penalty
    if (/(.)\1{2,}/.test(password)) score -= 10;

    return Math.max(0, Math.min(100, score));
  }

  /**
   * Sanitize input string by removing/escaping dangerous characters
   */
  sanitizeInput(input: string): string {
    return input
      .replace(/[<>'"]/g, '') // Remove dangerous HTML characters
      .replace(/javascript:/gi, '')
      .replace(/data:/gi, '')
      .slice(0, 1000); // Limit length
  }

  /**
   * Check for basic security issues in any input
   */
  private checkBasicSecurity(input: string): ValidationResult {
    if (!input) {
      return { isValid: true }; // Empty strings handled by specific validators
    }

    if (input.length > 1000) {
      return { isValid: false, error: 'Input too long' };
    }

    // Check for malicious patterns
    for (const pattern of this.maliciousPatterns) {
      if (pattern.test(input.toLowerCase())) {
        return { isValid: false, error: 'Input contains potentially malicious content' };
      }
    }

    return { isValid: true };
  }

  /**
   * Check if password is in common passwords list
   */
  private isCommonPassword(password: string): boolean {
    return this.commonPasswords.has(password.toLowerCase());
  }
}

// Export singleton instance
export const inputValidator = new InputValidator();