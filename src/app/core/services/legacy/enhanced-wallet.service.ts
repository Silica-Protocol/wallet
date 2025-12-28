import { Injectable } from '@angular/core';
import { WasmLoaderService, MeasurePerformance } from './wasm-loader.service';

export interface EnhancedWalletInfo {
  address: string;
  balance: string;
  pending: string;
  nonce: number;
  network: string;
  lastUpdate: number;
  blockHeight: number;
}

export interface TransactionRequest {
  to: string;
  amount: string;
  fee?: string;
  memo?: string;
  nonce?: number;
}

export interface EnhancedTransaction {
  hash: string;
  from: string;
  to: string;
  amount: string;
  fee: string;
  nonce: number;
  blockHeight: number;
  timestamp: number;
  status: 'pending' | 'confirmed' | 'failed' | 'replaced';
  memo?: string;
  gasUsed: number;
  gasLimit: number;
  confirmations?: number;
}

export interface BalanceSubscription {
  address: string;
  callback: (update: EnhancedWalletInfo) => void;
  unsubscribe: () => void;
}

export interface WalletConfig {
  apiEndpoint: string;
  wsEndpoint: string;
  networkName: string;
  chainId: number;
  enablePerformanceMonitoring: boolean;
}

@Injectable({
  providedIn: 'root'
})
export class EnhancedWalletService {
  private subscriptions = new Map<string, any>();

  constructor(private wasmLoader: WasmLoaderService) {
    void this.initializeService();
  }

  private async initializeService(): Promise<void> {
    try {
      await this.waitForWasmLoad();
      await this.configureWasm();
      console.log('Enhanced wallet service initialized');
    } catch (error) {
      console.error('Failed to initialize wallet service:', error);
    }
  }

  private async waitForWasmLoad(): Promise<void> {
    return new Promise((resolve, reject) => {
      const subscription = this.wasmLoader.isWasmLoaded().subscribe(loaded => {
        if (loaded) {
          resolve();
        }
      });

      setTimeout(() => {
        subscription.unsubscribe();
        reject(new Error('WASM module loading timeout'));
      }, 10000);
    });
  }

  private async configureWasm(): Promise<void> {
    const config: WalletConfig = {
      apiEndpoint: 'https://api.chert.com',
      wsEndpoint: 'wss://ws.chert.com',
      networkName: 'mainnet',
      chainId: 1,
      enablePerformanceMonitoring: true
    };

    await this.wasmLoader.executeWasmFunction('set_config', config);
  }

  @MeasurePerformance('wallet_generation')
  async generateWallet(password: string, algorithm: 'ed25519' | 'dilithium2' = 'ed25519'): Promise<EnhancedWalletInfo> {
    const keypair = await this.wasmLoader.executeWasmFunction('generate', algorithm);
    const address = keypair.get_address();

    const walletInfo: EnhancedWalletInfo = {
      address,
      balance: '0',
      pending: '0',
      nonce: 0,
      network: 'mainnet',
      lastUpdate: Date.now(),
      blockHeight: 0
    };

    await this.subscribeToBalanceUpdates(address);
    return walletInfo;
  }

  @MeasurePerformance('wallet_import')
  async importWallet(mnemonic: string, password: string, algorithm: 'ed25519' | 'dilithium2' = 'ed25519'): Promise<EnhancedWalletInfo> {
    const isValid = await this.wasmLoader.executeWasmFunction('validate_mnemonic_js', mnemonic);
    if (!isValid) {
      throw new Error('Invalid mnemonic phrase');
    }

    const keypair = await this.wasmLoader.executeWasmFunction('from_mnemonic', mnemonic, null, algorithm);
    const address = keypair.get_address();

    const walletInfo: EnhancedWalletInfo = {
      address,
      balance: '0',
      pending: '0',
      nonce: 0,
      network: 'mainnet',
      lastUpdate: Date.now(),
      blockHeight: 0
    };

    await this.subscribeToBalanceUpdates(address);
    return walletInfo;
  }

  @MeasurePerformance('balance_query')
  async getBalance(address?: string): Promise<string> {
    const update = await this.wasmLoader.executeWasmFunction('get_balance', address);
    return update.balance;
  }

  async subscribeToBalanceUpdates(address: string): Promise<BalanceSubscription> {
    const subscription = await this.wasmLoader.executeWasmFunction('subscribe', address, (update: any) => {
      console.log('Balance update:', update);
    });

    this.subscriptions.set(address, subscription);

    return {
      address,
      callback: update => console.log('Balance update callback:', update),
      unsubscribe: () => {
        subscription.unsubscribe();
        this.subscriptions.delete(address);
      }
    };
  }

  @MeasurePerformance('transaction_send')
  async sendTransaction(request: TransactionRequest, password: string): Promise<string> {
    const keypair = await this.wasmLoader.executeWasmFunction('get_keypair', password);
    const signed = await this.wasmLoader.executeWasmFunction('sign_transaction', request, keypair);
    return this.wasmLoader.executeWasmFunction('broadcast_transaction', signed);
  }

  @MeasurePerformance('transaction_history')
  async getTransactionHistory(address?: string, page = 0, limit = 50): Promise<EnhancedTransaction[]> {
    const transactions = await this.wasmLoader.executeWasmFunction('fetch_transactions', address, page, limit, null);
    return transactions.map((tx: any) => ({
      hash: tx.hash,
      from: tx.from,
      to: tx.to,
      amount: tx.amount,
      fee: tx.fee,
      nonce: tx.nonce,
      blockHeight: tx.block_height,
      timestamp: tx.timestamp,
      status: tx.status,
      memo: tx.memo,
      gasUsed: tx.gas_used,
      gasLimit: tx.gas_limit,
      confirmations: tx.status === 'confirmed' ? Math.max(0, 12345678 - tx.block_height) : 0
    }));
  }
}
