import { Injectable, inject } from '@angular/core';
import { WALLET_BACKEND, WalletBackend } from './wallet-backend.interface';

@Injectable({
  providedIn: 'root'
})
export class BiometricService {
  private walletBackend = inject<WalletBackend>(WALLET_BACKEND);

  async getBiometricStatus() {
    return this.walletBackend.getBiometricStatus();
  }

  async authenticateBiometric(reason: string = 'Access your wallet') {
    return this.walletBackend.authenticateBiometric(reason);
  }

  async createPasskey(userId: string, userName: string, relyingPartyId: string = window.location.hostname) {
    return this.walletBackend.createPasskey({
      challenge: this.generateChallenge(),
      userId,
      userName,
      relyingPartyId
    });
  }

  async authenticatePasskey(credentialIds: string[]) {
    return this.walletBackend.authenticatePasskey({
      challenge: this.generateChallenge(),
      credentialIds
    });
  }

  private generateChallenge(): string {
    const array = new Uint8Array(32);
    crypto.getRandomValues(array);
    return btoa(String.fromCharCode(...array));
  }
}