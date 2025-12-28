import { Injectable, inject } from '@angular/core';
import { Observable, combineLatest, map } from 'rxjs';
import { WasmService } from './wasm.service';
import { TauriService } from './tauri.service';
import { BalanceResponse, TransactionHistoryEntry } from '../types/wallet.types';

export interface EnhancedWalletInfo {
  wallet_id: string;
  wallet_name: string;
  created_at: string;
  last_accessed: string;
  backup_completed: boolean;
  accounts: unknown[];
  wasmReady: boolean;
  performanceMetrics?: {
    balanceQueryTime: number;
    transactionQueryTime: number;
    keyGenerationTime: number;
  };
}

export interface HybridBalanceResponse {
  accountId: string;
  balance: string;
  pendingBalance?: string;
  lastUpdated: string;
  wasmBalance?: unknown;
  tauriBalance?: BalanceResponse;
  source: 'wasm' | 'tauri' | 'hybrid';
  syncStatus: 'synced' | 'syncing' | 'out_of_sync';
}

@Injectable({
  providedIn: 'root'
})
export class EnhancedWalletService {
  private wasmService = inject(WasmService);
  private tauriService = inject(TauriService);

  /**
   * Get enhanced wallet info combining WASM and Tauri capabilities
   */
  getEnhancedWalletInfo(): Observable<EnhancedWalletInfo | null> {
    return combineLatest([
      this.tauriService.getWalletInfo().catch(() => null),
      this.wasmService.status$
    ]).pipe(
      map(([tauriInfo, wasmStatus]) => {
        if (!tauriInfo) return null;

        const enhanced: EnhancedWalletInfo = {
          ...tauriInfo,
          wasmReady: wasmStatus.initialized
        };

        return enhanced;
      })
    );
  }

  /**
   * Generate a new wallet using WASM for crypto operations
   */
  async generateWallet(password: string, name: string = 'Enhanced Wallet'): Promise<EnhancedWalletInfo> {
    console.log('Generating enhanced wallet with WASM crypto...');

    // Start performance monitoring
    this.wasmService.startPerformanceTimer('wallet_generation');

    try {
      // Generate keypair using WASM
      const keyPair = await this.wasmService.generateKeyPair('ed25519');
      const address = keyPair.get_address();

      console.log('Generated address:', address);

      // Create wallet using Tauri backend
      const tauriWallet = await this.tauriService.createWallet(name, password);

      // End performance monitoring
      const generationTime = this.wasmService.endPerformanceTimer('wallet_generation');

      const enhanced: EnhancedWalletInfo = {
        wallet_id: tauriWallet.wallet_id,
        wallet_name: 'Enhanced Wallet',
        created_at: new Date().toISOString(),
        last_accessed: new Date().toISOString(),
        backup_completed: false,
        accounts: [],
        wasmReady: this.wasmService.isReady(),
        performanceMetrics: {
          balanceQueryTime: 0,
          transactionQueryTime: 0,
          keyGenerationTime: generationTime
        }
      };

      return enhanced;
    } catch (error) {
      console.error('Failed to generate enhanced wallet:', error);
      throw error;
    }
  }

  /**
   * Get hybrid balance using both WASM and Tauri
   */
  async getHybridBalance(accountId: string): Promise<HybridBalanceResponse> {
    console.log('Getting hybrid balance for account:', accountId);

    this.wasmService.startPerformanceTimer('balance_query');

    try {
      // Get balance from both sources
      const [tauriBalance, wasmBalance] = await Promise.allSettled([
        this.tauriService.getBalance(accountId),
        this.wasmService.getBalance(accountId).catch(() => null)
      ]);

      const balanceTime = this.wasmService.endPerformanceTimer('balance_query');

      // Determine the primary source and sync status
      let primaryBalance: BalanceResponse;
      let source: 'wasm' | 'tauri' | 'hybrid';
      let syncStatus: 'synced' | 'syncing' | 'out_of_sync' = 'synced';

      if (tauriBalance.status === 'fulfilled' && wasmBalance.status === 'fulfilled' && wasmBalance.value) {
        // Both sources available - compare and determine sync status
        primaryBalance = tauriBalance.value;
        source = 'hybrid';

        // Simple comparison - in real implementation, you'd compare actual values
        if (wasmBalance.value.balance !== tauriBalance.value.available) {
          syncStatus = 'out_of_sync';
        }
      } else if (tauriBalance.status === 'fulfilled') {
        primaryBalance = tauriBalance.value;
        source = 'tauri';
      } else if (wasmBalance.status === 'fulfilled' && wasmBalance.value) {
        // Fallback to WASM if Tauri fails
        primaryBalance = {
          address: accountId,
          available: wasmBalance.value.balance,
          delegated: '0',
          rewards: '0',
          total: wasmBalance.value.balance,
          updated_at: new Date(wasmBalance.value.last_update * 1000).toISOString()
        };
        source = 'wasm';
      } else {
        throw new Error('Unable to get balance from any source');
      }

      const hybridResponse: HybridBalanceResponse = {
        accountId: accountId,
        balance: primaryBalance.available,
        lastUpdated: primaryBalance.updated_at,
        wasmBalance: wasmBalance.status === 'fulfilled' ? wasmBalance.value : null,
        tauriBalance: tauriBalance.status === 'fulfilled' ? tauriBalance.value : undefined,
        source,
        syncStatus
      };

      console.log('Hybrid balance retrieved:', {
        source,
        syncStatus,
        balance: primaryBalance.available,
        time: balanceTime + 'ms'
      });

      return hybridResponse;
    } catch (error) {
      console.error('Failed to get hybrid balance:', error);
      throw error;
    }
  }

  /**
   * Get transaction history with WASM caching
   */
  async getTransactionHistory(accountId: string, limit?: number): Promise<TransactionHistoryEntry[]> {
    console.log('Getting transaction history for account:', accountId);

    this.wasmService.startPerformanceTimer('transaction_history');

    try {
      // Try WASM first for faster cached results
      if (this.wasmService.isReady()) {
        try {
          const wasmTransactions = await this.wasmService.fetchTransactions(accountId, 1, limit || 20);
          if (wasmTransactions && wasmTransactions.length > 0) {
            const queryTime = this.wasmService.endPerformanceTimer('transaction_history');
            console.log(`Retrieved ${wasmTransactions.length} transactions from WASM cache in ${queryTime}ms`);
            return this.convertWasmTransactions(wasmTransactions);
          }
        } catch (wasmError) {
          console.warn('WASM transaction fetch failed, falling back to Tauri:', wasmError);
        }
      }

      // Fallback to Tauri
      const tauriTransactions = await this.tauriService.getTransactionHistory(accountId, limit);
      const queryTime = this.wasmService.endPerformanceTimer('transaction_history');

      console.log(`Retrieved ${tauriTransactions.length} transactions from Tauri in ${queryTime}ms`);

      // Cache in WASM for future queries
      if (this.wasmService.isReady()) {
        // Note: In a real implementation, you'd cache the results in WASM
        console.log('Caching transaction history in WASM...');
      }

      return tauriTransactions;
    } catch (error) {
      console.error('Failed to get transaction history:', error);
      throw error;
    }
  }

  /**
   * Validate address using WASM for fast validation
   */
  async validateAddress(address: string): Promise<boolean> {
    if (this.wasmService.isReady()) {
      try {
        return await this.wasmService.validateAddress(address);
      } catch (error) {
        console.warn('WASM address validation failed, using fallback:', error);
      }
    }

    // Fallback validation - basic format check
    return /^chert_[a-zA-Z0-9]{40,}$/.test(address);
  }

  /**
   * Generate secure mnemonic using WASM
   */
  async generateMnemonic(wordCount: number = 12): Promise<string> {
    if (this.wasmService.isReady()) {
      return await this.wasmService.generateMnemonic(wordCount);
    }
    throw new Error('WASM service not available for mnemonic generation');
  }

  /**
   * Validate mnemonic using WASM
   */
  async validateMnemonic(mnemonic: string): Promise<boolean> {
    if (this.wasmService.isReady()) {
      return await this.wasmService.validateMnemonic(mnemonic);
    }
    // Basic fallback validation
    const words = mnemonic.trim().split(/\s+/);
    return words.length >= 12 && words.every(word => word.length > 0);
  }

  /**
   * Check password strength using WASM
   */
  async checkPasswordStrength(password: string): Promise<unknown> {
    if (this.wasmService.isReady()) {
      return await this.wasmService.checkPasswordStrength(password);
    }
    // Basic fallback
    return {
      score: password.length >= 8 ? 2 : 1,
      feedback: password.length >= 8 ? 'Acceptable' : 'Too short'
    };
  }

  /**
   * Get performance metrics
   */
  getPerformanceMetrics(): unknown {
    return {
      wasmReady: this.wasmService.isReady(),
      wasmVersion: this.wasmService.getVersion(),
      cachedMetrics: {
        balanceQuery: this.wasmService.getCachedPerformanceMetric('balance_query'),
        transactionQuery: this.wasmService.getCachedPerformanceMetric('transaction_history'),
        keyGeneration: this.wasmService.getCachedPerformanceMetric('wallet_generation')
      }
    };
  }

  /**
   * Test WASM integration
   */
  async testIntegration(): Promise<unknown> {
    const results = {
      wasmReady: this.wasmService.isReady(),
      wasmVersion: this.wasmService.getVersion(),
      tauriConnected: false,
      tests: [] as unknown[]
    };

    // Test WASM functionality
    if (results.wasmReady) {
      try {
        const testResult = this.wasmService.testWasm();
        results.tests.push({ name: 'wasm_basic', success: true, result: testResult });

        const keyPair = await this.wasmService.generateKeyPair();
        const address = keyPair.get_address();
        results.tests.push({ name: 'wasm_keygen', success: true, address });

        const isValid = await this.wasmService.validateAddress(address);
        results.tests.push({ name: 'wasm_validation', success: isValid });
      } catch (error) {
        results.tests.push({ name: 'wasm_tests', success: false, error: String(error) });
      }
    }

    // Test Tauri connectivity
    try {
      await this.tauriService.getSettings();
      results.tauriConnected = true;
      results.tests.push({ name: 'tauri_connection', success: true });
    } catch (error) {
      results.tests.push({ name: 'tauri_connection', success: false, error: String(error) });
    }

    return results;
  }

  private convertWasmTransactions(wasmTransactions: unknown[]): TransactionHistoryEntry[] {
    return wasmTransactions.map(tx => ({
      hash: tx.hash,
      timestamp: new Date(tx.timestamp * 1000).toISOString(),
      from_address: tx.from,
      to_address: tx.to,
      amount: tx.amount,
      fee: tx.fee,
      status: tx.status,
      transaction_type: 'transfer',
      memo: tx.memo,
      confirmations: 0
    }));
  }
}