import { Component, inject, signal, computed, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink } from '@angular/router';
import { WalletService } from '../../core/services/wallet.service';
import { WALLET_BACKEND, WalletBackend } from '../../core/services/wallet-backend.interface';
import { TransactionInfo, TransactionHistoryResponse } from '../../core/types/wallet.types';

@Component({
  selector: 'app-transactions',
  standalone: true,
  imports: [CommonModule, RouterLink],
  templateUrl: './transactions.html',
  styleUrl: './transactions.scss'
})
export class TransactionsComponent implements OnInit {
  private walletService = inject(WalletService);
  private walletBackend = inject<WalletBackend>(WALLET_BACKEND);

  readonly walletInfo = this.walletService.info;
  readonly isLocked = this.walletService.isLocked;
  readonly exists = this.walletService.exists;
  readonly createdAddress = this.walletService.createdAddress;

  private transactions = signal<TransactionInfo[]>([]);
  private loading = signal(false);
  private error = signal<string | null>(null);
  private hasMore = signal(false);
  private currentOffset = signal(0);
  private readonly limit = 20;

  readonly transactionList = this.transactions.asReadonly();
  readonly isLoading = this.loading.asReadonly();
  readonly loadError = this.error.asReadonly();
  readonly canLoadMore = this.hasMore.asReadonly();

  readonly hasTransactions = computed(() => this.transactions().length > 0);
  readonly isWalletReady = computed(() => this.exists() && !this.isLocked() && this.createdAddress());

  ngOnInit() {
    if (this.isWalletReady()) {
      this.loadTransactions();
    }
  }

  async loadTransactions() {
    if (!this.isWalletReady()) {
      return;
    }

    this.loading.set(true);
    this.error.set(null);

    try {
      const address = this.createdAddress()!;
      const walletInfo = this.walletInfo();
      const nodeUrl = walletInfo?.config?.network.primaryEndpoint?.trim();
      const currentOffset = this.currentOffset();
      const response: TransactionHistoryResponse = await this.walletBackend.getTransactionHistory(
        address,
        this.limit,
        currentOffset,
        nodeUrl || undefined
      );

      const transactions = response.transactions ?? [];
      if (currentOffset === 0) {
        this.transactions.set(transactions);
      } else {
        this.transactions.update(current => [...current, ...transactions]);
      }

      const newOffset = currentOffset + transactions.length;
      this.currentOffset.set(newOffset);

      const totalCount = response.totalCount ?? newOffset;
      this.hasMore.set(newOffset < totalCount);
    } catch (err) {
      console.error('Failed to load transactions:', err);
      this.error.set(err instanceof Error ? err.message : 'Failed to load transactions');
    } finally {
      this.loading.set(false);
    }
  }

  async loadMore() {
    if (!this.isLoading() && this.canLoadMore()) {
      await this.loadTransactions();
    }
  }

  async refresh() {
    this.currentOffset.set(0);
    await this.loadTransactions();
  }

  getTransactionType(tx: TransactionInfo): 'sent' | 'received' {
    const currentAddress = this.createdAddress();
    return tx.fromAddress === currentAddress ? 'sent' : 'received';
  }

  getTransactionIcon(tx: TransactionInfo): string {
    return this.getTransactionType(tx) === 'sent' ? '📤' : '📥';
  }

  getTransactionColor(tx: TransactionInfo): string {
    return this.getTransactionType(tx) === 'sent' ? '#e74c3c' : '#27ae60';
  }

  formatAmount(amount: string): string {
    // Simple formatting - in a real app you'd use the formatAmount service
    const num = parseFloat(amount);
    if (num >= 1e9) return (num / 1e9).toFixed(2) + 'B';
    if (num >= 1e6) return (num / 1e6).toFixed(2) + 'M';
    if (num >= 1e3) return (num / 1e3).toFixed(2) + 'K';
    return num.toFixed(2);
  }

  formatAddress(address: string): string {
    if (address.length <= 20) return address;
    return address.substring(0, 10) + '...' + address.substring(address.length - 8);
  }

  copyTxId(txId: string) {
    navigator.clipboard.writeText(txId).catch(error => {
      console.error('Failed to copy transaction ID:', error);
    });
  }

  getStatusColor(status: string): string {
    switch (status.toLowerCase()) {
      case 'confirmed': return '#27ae60';
      case 'pending': return '#f39c12';
      case 'failed': return '#e74c3c';
      default: return '#95a5a6';
    }
  }
}
