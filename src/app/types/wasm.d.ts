// TypeScript declarations for Chert Wallet WebAssembly module

declare module '../../../assets/wasm/chert_wallet_wasm' {
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

  export interface WasmKeyPair {
    get_public_key(): string;
    get_private_key(): string;
    get_address(): string;
    get_algorithm(): string;
    sign(data: Uint8Array): Uint8Array;
    verify(data: Uint8Array, signature: Uint8Array): boolean;
    export_encrypted(password: string): Promise<string>;
  }

  export interface TransactionSigner {
    sign_transaction(transaction: any): Promise<any>;
    sign_message(message: string): Uint8Array;
    verify_message(message: string, signature: Uint8Array): boolean;
  }

  export interface BalanceSubscription {
    subscribe(callback: Function): Promise<void>;
    unsubscribe(): void;
    is_active(): boolean;
    get_address(): string;
  }

  export interface TransactionFetcher {
    fetch_transactions(address: string, page: number, limit: number, filter?: any): Promise<Transaction[]>;
    fetch_transaction(hash: string): Promise<Transaction | null>;
    get_transaction_count(address: string, filter?: any): Promise<number>;
    search_transactions(query: any): Promise<Transaction[]>;
    clear_cache(): void;
    get_cache_stats(): any;
  }

  export interface WasmConfig {
    api_endpoint: string;
    ws_endpoint: string;
    network_name: string;
    chain_id: number;
    enable_performance_monitoring: boolean;
  }

  export interface WasmError {
    code: string;
    message: string;
    details?: string;
  }

  // Main module functions
  export function init_wasm(): void;
  export function set_config(config: WasmConfig): void;
  export function get_version(): string;
  export function get_build_info(): any;
  
  // Performance monitoring
  export function start_performance_timer(operation: string): void;
  export function end_performance_timer(operation: string): number;
  export function get_cached_performance_metric(operation: string): number | null;
  
  // Balance tracking
  export function get_balance(address: string): Promise<BalanceUpdate>;
  export function get_batch_balances(addresses: string[]): Promise<BalanceUpdate[]>;
  export function get_balance_history(address: string, from_block: number, to_block: number): Promise<any[]>;
  
  // Transaction operations
  export function fetch_transactions(address: string, page: number, limit: number, filter?: any): Promise<Transaction[]>;
  export function fetch_transaction(hash: string): Promise<Transaction | null>;
  export function get_transaction_count(address: string, filter?: any): Promise<number>;
  export function search_transactions(query: any): Promise<Transaction[]>;
  export function export_transactions(transactions: Transaction[], format: string): Promise<string>;
  
  // Cryptographic operations
  export function generate(algorithm: string): WasmKeyPair;
  export function from_mnemonic(mnemonic: string, passphrase: string | null, algorithm: string): WasmKeyPair;
  export function generate_mnemonic(word_count: number): string;
  export function validate_mnemonic_js(mnemonic: string): boolean;
  export function validate_address(address: string): boolean;
  export function generate_address_from_public_key(public_key: string, address_type: string): string;
  export function check_password_strength(password: string): any;
  
  // Classes
  export class BalanceSubscription {
    constructor(address: string);
  }
  
  export class TransactionFetcher {
    constructor(api_endpoint: string, ttl_ms: number);
  }
  
  export class TransactionSigner {
    constructor(keypair: WasmKeyPair);
  }
  
  export class WasmKeyPair {
    constructor(algorithm: string);
  }
}
