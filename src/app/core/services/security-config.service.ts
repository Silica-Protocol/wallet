import { Injectable } from '@angular/core';

export enum Environment {
  Development = 'development',
  Production = 'production',
  Test = 'test'
}

export interface SecurityConfigOptions {
  logLevel: 'DEBUG' | 'INFO' | 'WARN' | 'ERROR';
  sessionTimeoutMinutes: number;
  autoLockMinutes: number;
  maxFailedAttempts: number;
  enableAnalytics: boolean;
  networkEndpoint: string;
  enableDevMode: boolean;
}

@Injectable({
  providedIn: 'root'
})
export class SecurityConfigService {
  private environment: Environment = Environment.Development;
  private config: SecurityConfigOptions;

  constructor() {
    this.detectEnvironment();
    this.config = this.loadDefaultConfig();
    this.loadFromEnvironment();
  }

  /**
   * Get current environment
   */
  getEnvironment(): Environment {
    return this.environment;
  }

  /**
   * Check if we're in production mode
   */
  isProduction(): boolean {
    return this.environment === Environment.Production;
  }

  /**
   * Check if we're in development mode
   */
  isDevelopment(): boolean {
    return this.environment === Environment.Development;
  }

  /**
   * Get configuration value
   */
  getConfig(): SecurityConfigOptions {
    return { ...this.config }; // Return copy to prevent mutation
  }

  /**
   * Get log level
   */
  getLogLevel(): 'DEBUG' | 'INFO' | 'WARN' | 'ERROR' {
    return this.config.logLevel;
  }

  /**
   * Get session timeout in minutes
   */
  getSessionTimeout(): number {
    return this.config.sessionTimeoutMinutes;
  }

  /**
   * Get auto-lock timeout in minutes
   */
  getAutoLockTimeout(): number {
    return this.config.autoLockMinutes;
  }

  /**
   * Get maximum failed attempts
   */
  getMaxFailedAttempts(): number {
    return this.config.maxFailedAttempts;
  }

  /**
   * Get network endpoint
   */
  getNetworkEndpoint(): string {
    return this.config.networkEndpoint;
  }

  /**
   * Check if analytics are enabled
   */
  isAnalyticsEnabled(): boolean {
    return this.config.enableAnalytics;
  }

  /**
   * Check if development mode features are enabled
   */
  isDevModeEnabled(): boolean {
    return this.config.enableDevMode;
  }

  /**
   * Validate configuration
   */
  validateConfig(): { isValid: boolean; errors: string[] } {
    const errors: string[] = [];

    if (this.config.sessionTimeoutMinutes < 1) {
      errors.push('Session timeout must be at least 1 minute');
    }

    if (this.config.autoLockMinutes < 1) {
      errors.push('Auto-lock timeout must be at least 1 minute');
    }

    if (this.config.maxFailedAttempts < 1) {
      errors.push('Max failed attempts must be at least 1');
    }

    if (!this.config.networkEndpoint) {
      errors.push('Network endpoint is required');
    }

    // Validate network endpoint URL
    try {
      new URL(this.config.networkEndpoint);
    } catch {
      errors.push('Network endpoint must be a valid URL');
    }

    // Production-specific validations
    if (this.isProduction()) {
      if (!this.config.networkEndpoint.startsWith('https://')) {
        errors.push('Production network endpoint must use HTTPS');
      }

      if (this.config.enableDevMode) {
        errors.push('Development mode cannot be enabled in production');
      }
    }

    return {
      isValid: errors.length === 0,
      errors
    };
  }

  /**
   * Get Content Security Policy string
   */
  getCSP(): string {
    if (this.isProduction()) {
      return "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self' https:; object-src 'none'; base-uri 'self'; form-action 'self'";
    } else {
      return "default-src 'self'; script-src 'self' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: http://localhost:*; font-src 'self'; connect-src 'self' https: http://localhost:* ws://localhost:*; object-src 'none'; base-uri 'self'; form-action 'self'";
    }
  }

  /**
   * Detect environment from various sources
   */
  private detectEnvironment(): void {
    // Check for production indicators
    if (typeof window !== 'undefined') {
      // Browser environment
      if (window.location.protocol === 'https:' && 
          !window.location.hostname.includes('localhost') &&
          !window.location.hostname.includes('127.0.0.1')) {
        this.environment = Environment.Production;
        return;
      }

      // Check for test environment
      if (window.location.hostname.includes('test') || 
          window.location.search.includes('test=true')) {
        this.environment = Environment.Test;
        return;
      }
    }

    // Default to development
    this.environment = Environment.Development;
  }

  /**
   * Load default configuration based on environment
   */
  private loadDefaultConfig(): SecurityConfigOptions {
    switch (this.environment) {
      case Environment.Production:
        return {
          logLevel: 'INFO',
          sessionTimeoutMinutes: 30,
          autoLockMinutes: 15,
          maxFailedAttempts: 5,
          enableAnalytics: false,
          networkEndpoint: 'https://mainnet.chert.network',
          enableDevMode: false
        };

      case Environment.Test:
        return {
          logLevel: 'WARN',
          sessionTimeoutMinutes: 5,
          autoLockMinutes: 2,
          maxFailedAttempts: 3,
          enableAnalytics: false,
          networkEndpoint: 'https://testnet.chert.network',
          enableDevMode: false
        };

      default: // Development
        return {
          logLevel: 'DEBUG',
          sessionTimeoutMinutes: 60,
          autoLockMinutes: 30,
          maxFailedAttempts: 10,
          enableAnalytics: false,
          networkEndpoint: 'http://localhost:4242',
          enableDevMode: true
        };
    }
  }

  /**
    * Load configuration from environment (if available)
    */
   private loadFromEnvironment(): void {
     // In a Tauri app, environment variables would be accessed through the backend
     // For now, we'll use default configuration
     // TODO: Implement Tauri command to get environment configuration when needed
   }
}