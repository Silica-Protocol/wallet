import { Injectable } from '@angular/core';
import { BehaviorSubject, Observable } from 'rxjs';

export interface WasmModuleStatus {
  initialized: boolean;
  version: string;
  buildInfo: any;
  error?: string;
}

export interface WasmKeyPair {
  get_public_key(): string;
  get_private_key(): string;
  get_address(): string;
  get_algorithm(): string;
  sign(data: Uint8Array): Uint8Array;
  verify(data: Uint8Array, signature: Uint8Array): boolean;
  export_encrypted(password: string): Promise<string>;
}

export interface BalanceUpdate {
  address: string;
  balance: string;
  pending: string;
  nonce: number;
  last_update: number;
  block_height: number;
}

export interface Transaction {
  hash: string;
  from: string;
  to: string;
  amount: string;
  fee: string;
  nonce: number;
  block_height: number;
  timestamp: number;
  status: 'pending' | 'confirmed' | 'failed' | 'replaced';
  memo?: string;
  gas_used: number;
  gas_limit: number;
}

@Injectable({
  providedIn: 'root'
})
export class WasmService {
  private wasmModule: any = null;
  private statusSubject = new BehaviorSubject<WasmModuleStatus>({
    initialized: false,
    version: '',
    buildInfo: null
  });

  public status$: Observable<WasmModuleStatus> = this.statusSubject.asObservable();

  constructor() {
    this.initializeWasm();
  }

  private async initializeWasm(): Promise<void> {
    try {
      console.log('Initializing WASM module...');

      // Dynamic import of the WASM module JS wrapper
      const wasmModule = await import('../../../assets/wasm/chert_wallet_wasm');

      // Get the WASM binary URL - Angular serves assets from /assets/
      const wasmUrl = '/assets/wasm/chert_wallet_wasm_bg.wasm';

      // Fetch the WASM binary with correct headers
      const wasmResponse = await fetch(wasmUrl, {
        headers: {
          'Accept': 'application/wasm'
        }
      });

      if (!wasmResponse.ok) {
        throw new Error(`Failed to fetch WASM binary: ${wasmResponse.status} ${wasmResponse.statusText}`);
      }

      // Initialize the WASM module with the fetched binary
      await wasmModule.default(wasmResponse);

      // Now call init_wasm to get the initialized context
      const initResult = wasmModule.init_wasm();
      if (!initResult || !(initResult as any).initialized) {
        throw new Error('WASM module initialization failed');
      }

      this.wasmModule = wasmModule;

      // Get build info
      const buildInfo = wasmModule.get_build_info();
      const version = (buildInfo as any).version || 'unknown';

      this.statusSubject.next({
        initialized: true,
        version,
        buildInfo
      });

      console.log('WASM module initialized successfully:', version);
    } catch (error) {
      console.error('Failed to initialize WASM module:', error);
      this.statusSubject.next({
        initialized: false,
        version: '',
        buildInfo: null,
        error: error instanceof Error ? error.message : 'Unknown error'
      });
    }
  }

  // Check if WASM is ready
  isReady(): boolean {
    return this.wasmModule !== null;
  }

  // Wait for WASM to be ready
  async waitForReady(): Promise<void> {
    if (this.isReady()) return;

    return new Promise((resolve, reject) => {
      const subscription = this.status$.subscribe(status => {
        if (status.initialized) {
          subscription.unsubscribe();
          resolve();
        } else if (status.error) {
          subscription.unsubscribe();
          reject(new Error(status.error));
        }
      });
    });
  }

  // Cryptographic operations
  async generateKeyPair(algorithm: string = 'ed25519'): Promise<WasmKeyPair> {
    await this.waitForReady();
    return this.wasmModule.generate(algorithm);
  }

  async generateMnemonic(wordCount: number = 12): Promise<string> {
    await this.waitForReady();
    return this.wasmModule.generate_mnemonic(wordCount);
  }

  async fromMnemonic(mnemonic: string, passphrase?: string, algorithm: string = 'ed25519'): Promise<WasmKeyPair> {
    await this.waitForReady();
    return this.wasmModule.from_mnemonic(mnemonic, passphrase || null, algorithm);
  }

  async validateMnemonic(mnemonic: string): Promise<boolean> {
    await this.waitForReady();
    return this.wasmModule.validate_mnemonic_js(mnemonic);
  }

  async validateAddress(address: string): Promise<boolean> {
    await this.waitForReady();
    return this.wasmModule.validate_address(address);
  }

  async checkPasswordStrength(password: string): Promise<any> {
    await this.waitForReady();
    return this.wasmModule.check_password_strength(password);
  }

  // Balance operations
  async getBalance(address: string): Promise<BalanceUpdate> {
    await this.waitForReady();
    return this.wasmModule.get_balance(address);
  }

  async getBatchBalances(addresses: string[]): Promise<BalanceUpdate[]> {
    await this.waitForReady();
    return this.wasmModule.get_batch_balances(addresses);
  }

  // Transaction operations
  async fetchTransactions(
    address: string,
    page: number = 1,
    limit: number = 20,
    filter?: any
  ): Promise<Transaction[]> {
    await this.waitForReady();
    return this.wasmModule.fetch_transactions(address, page, limit, filter);
  }

  async fetchTransaction(hash: string): Promise<Transaction | null> {
    await this.waitForReady();
    return this.wasmModule.fetch_transaction(hash);
  }

  async getTransactionCount(address: string, filter?: any): Promise<number> {
    await this.waitForReady();
    return this.wasmModule.get_transaction_count(address, filter);
  }

  // Performance monitoring
  startPerformanceTimer(operation: string): void {
    if (this.wasmModule) {
      this.wasmModule.start_performance_timer(operation);
    }
  }

  endPerformanceTimer(operation: string): number {
    if (this.wasmModule) {
      return this.wasmModule.end_performance_timer(operation);
    }
    return 0;
  }

  getCachedPerformanceMetric(operation: string): number | null {
    if (this.wasmModule) {
      return this.wasmModule.get_cached_performance_metric(operation);
    }
    return null;
  }

  // Configuration
  setConfig(config: any): void {
    if (this.wasmModule) {
      this.wasmModule.set_config(config);
    }
  }

  // Version info
  getVersion(): string {
    if (this.wasmModule) {
      return this.wasmModule.get_version();
    }
    return 'unknown';
  }

  getBuildInfo(): any {
    if (this.wasmModule) {
      return this.wasmModule.get_build_info();
    }
    return null;
  }

  // Test function
  testWasm(): string {
    if (this.wasmModule) {
      return this.wasmModule.test_wasm();
    }
    return 'WASM not loaded';
  }
}