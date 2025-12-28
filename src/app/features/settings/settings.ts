import { Component, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { BiometricService } from '../../core/services/biometric.service';
import { WALLET_BACKEND, WalletBackend } from '../../core/services/wallet-backend.interface';

interface BiometricStatus {
  available: boolean;
  enrolled: boolean;
  supportedTypes: string[];
}

interface PushNotificationStatus {
  available: boolean;
  enabled: boolean;
  permissionGranted: boolean;
}

@Component({
  selector: 'app-settings',
  standalone: true,
  imports: [CommonModule, FormsModule],
  templateUrl: './settings.html',
  styleUrl: './settings.scss'
})
export class SettingsComponent {
  private biometricService = inject(BiometricService);
  private walletBackend = inject<WalletBackend>(WALLET_BACKEND);

  // Biometric settings
  biometricStatus = signal<BiometricStatus | null>(null);
  biometricEnabled = signal(false);
  passkeyEnabled = signal(false);

  // Push notification settings
  pushStatus = signal<PushNotificationStatus | null>(null);
  transactionAlerts = signal(true);
  stakingAlerts = signal(true);
  governanceAlerts = signal(true);

  // Loading states
  loadingBiometric = signal(false);
  loadingPush = signal(false);
  testingBiometric = signal(false);

  async ngOnInit() {
    await this.loadSettings();
  }

  private async loadSettings() {
    try {
      // Load biometric status
      this.loadingBiometric.set(true);
      const biometricStatus = await this.biometricService.getBiometricStatus();
      this.biometricStatus.set(biometricStatus);

      // Load push notification status
      this.loadingPush.set(true);
      const pushStatus = await this.walletBackend.getPushNotificationStatus();
      this.pushStatus.set(pushStatus);

      // Load current preferences from local storage or config
      this.loadSavedPreferences();
    } catch (error) {
      console.error('Failed to load settings:', error);
    } finally {
      this.loadingBiometric.set(false);
      this.loadingPush.set(false);
    }
  }

  private loadSavedPreferences() {
    // Load from localStorage or wallet config
    const saved = localStorage.getItem('wallet-settings');
    if (saved) {
      try {
        const settings = JSON.parse(saved);
        this.biometricEnabled.set(settings.biometricEnabled || false);
        this.passkeyEnabled.set(settings.passkeyEnabled || false);
        this.transactionAlerts.set(settings.transactionAlerts ?? true);
        this.stakingAlerts.set(settings.stakingAlerts ?? true);
        this.governanceAlerts.set(settings.governanceAlerts ?? true);
      } catch (error) {
        console.error('Failed to parse saved settings:', error);
      }
    }
  }

  private savePreferences() {
    const settings = {
      biometricEnabled: this.biometricEnabled(),
      passkeyEnabled: this.passkeyEnabled(),
      transactionAlerts: this.transactionAlerts(),
      stakingAlerts: this.stakingAlerts(),
      governanceAlerts: this.governanceAlerts(),
    };
    localStorage.setItem('wallet-settings', JSON.stringify(settings));
  }

  async testBiometric() {
    if (!this.biometricStatus()?.available) return;

    this.testingBiometric.set(true);
    try {
      const result = await this.biometricService.authenticateBiometric('Test biometric authentication');
      if (result.success) {
        alert('Biometric authentication successful!');
      } else {
        alert('Biometric authentication failed.');
      }
    } catch (error) {
      console.error('Biometric test failed:', error);
      alert('Biometric test failed. Please check your device settings.');
    } finally {
      this.testingBiometric.set(false);
    }
  }

  async toggleBiometric() {
    const enabled = !this.biometricEnabled();
    this.biometricEnabled.set(enabled);
    this.savePreferences();

    if (enabled && this.biometricStatus()?.available) {
      // Test biometric on enable
      await this.testBiometric();
    }
  }

  async togglePasskey() {
    const enabled = !this.passkeyEnabled();
    this.passkeyEnabled.set(enabled);
    this.savePreferences();

    if (enabled) {
      try {
        // Create a passkey for the current user
        const userId = 'wallet-user'; // In real app, use actual user ID
        const userName = 'Wallet User'; // In real app, use actual username
        const result = await this.biometricService.createPasskey(userId, userName);
        alert(`Passkey created successfully! Credential ID: ${result.credentialId}`);
      } catch (error) {
        console.error('Failed to create passkey:', error);
        alert('Failed to create passkey. Please try again.');
        this.passkeyEnabled.set(false);
      }
    }
  }

  async togglePushNotifications() {
    const currentStatus = this.pushStatus();
    if (!currentStatus?.available) return;

    try {
      this.loadingPush.set(true);

      if (!currentStatus.enabled) {
        // Request permission and register
        if ('Notification' in window) {
          const permission = await Notification.requestPermission();
          if (permission === 'granted') {
            // In a real app, you'd get a push token from FCM/APNs
            // For now, use a mock token
            const mockToken = 'mock-push-token-' + Date.now();

            const result = await this.walletBackend.registerPushNotifications({
              token: mockToken,
              enableTransactionAlerts: this.transactionAlerts(),
              enableStakingAlerts: this.stakingAlerts(),
              enableGovernanceAlerts: this.governanceAlerts(),
            });

            if (result.success) {
              this.pushStatus.set({
                ...currentStatus,
                enabled: true,
                permissionGranted: true
              });
              alert('Push notifications enabled successfully!');
            } else {
              alert('Failed to register for push notifications.');
            }
          } else {
            alert('Notification permission denied. Please enable notifications in your browser settings.');
          }
        }
      } else {
        // Disable push notifications
        this.pushStatus.set({
          ...currentStatus,
          enabled: false
        });
        alert('Push notifications disabled.');
      }
    } catch (error) {
      console.error('Failed to toggle push notifications:', error);
      alert('Failed to update push notification settings.');
    } finally {
      this.loadingPush.set(false);
    }
  }

  updatePushPreferences() {
    this.savePreferences();

    // If push notifications are enabled, update the registration
    if (this.pushStatus()?.enabled) {
      this.updatePushRegistration();
    }
  }

  private async updatePushRegistration() {
    try {
      const mockToken = 'mock-push-token-updated-' + Date.now();
      await this.walletBackend.registerPushNotifications({
        token: mockToken,
        enableTransactionAlerts: this.transactionAlerts(),
        enableStakingAlerts: this.stakingAlerts(),
        enableGovernanceAlerts: this.governanceAlerts(),
      });
    } catch (error) {
      console.error('Failed to update push registration:', error);
    }
  }

  getBiometricTypeText(): string {
    const types = this.biometricStatus()?.supportedTypes || [];
    if (types.includes('face')) return 'Face ID';
    if (types.includes('fingerprint')) return 'Fingerprint';
    if (types.includes('touch')) return 'Touch ID';
    return 'Biometric';
  }
}