import { Component, computed, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink } from '@angular/router';
import { WalletService } from '../../core/services/wallet.service';
import { BalanceResponse } from '../../core/types/wallet.types';

@Component({
  selector: 'app-wallet',
  standalone: true,
  imports: [CommonModule, RouterLink],
  templateUrl: './wallet.html',
  styleUrl: './wallet.scss'
})
export class WalletComponent {
  private walletService = inject(WalletService);

  readonly walletInfo = this.walletService.info;
  readonly metadata = this.walletService.metadata;
  readonly config = this.walletService.config;
  readonly isLocked = this.walletService.isLocked;
  readonly exists = this.walletService.exists;
  readonly remainingAttempts = this.walletService.remainingAttempts;
  readonly createdAddress = this.walletService.createdAddress;
  readonly balance = this.walletService.balance;
  readonly busy = this.walletService.busy;
  readonly error = this.walletService.error;

  statusText = computed(() => {
    if (!this.exists()) {
      return 'No wallet configured';
    }
    return this.isLocked() ? 'Locked' : 'Unlocked';
  });

  hasSummary = computed(() => this.metadata() !== null);

  async refresh() {
    await this.walletService.refresh();
  }

  copyAddress() {
    const address = this.createdAddress();
    if (!address) {
      return;
    }
    navigator.clipboard.writeText(address).catch(error => {
      console.error('Failed to copy address:', error);
    });
  }

  formatBalance(balance: string): string {
    if (!balance) return '0';
    // Simple formatting - convert from base units (assuming 9 decimals like CHERT)
    const num = parseFloat(balance) / 1e9;
    return num.toLocaleString('en-US', { maximumFractionDigits: 4 });
  }
}
