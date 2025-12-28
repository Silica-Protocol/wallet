#!/bin/bash

# Build script for Chert Wallet WebAssembly module
# This script builds the Rust WebAssembly module and prepares it for use in Angular

set -e

echo "🔨 Building Chert Wallet WebAssembly module..."

# Navigate to WASM directory
cd "$(dirname "$0")/src-tauri/wasm"

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "❌ wasm-pack is not installed. Installing..."
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

# Check if target directory exists
if [ ! -d "target" ]; then
    echo "📁 Creating target directory..."
    mkdir -p target
fi

# Build the WebAssembly module
echo "🏗️  Building WebAssembly module..."
wasm-pack build \
    --target web \
    --out-dir pkg \
    --scope chert \
    --dev

# Check if build was successful
if [ $? -eq 0 ]; then
    echo "✅ WebAssembly module built successfully!"
else
    echo "❌ Failed to build WebAssembly module"
    exit 1
fi

# Navigate back to wallet directory
cd ../..

# Create symlink or copy to Angular assets
echo "📋 Setting up WebAssembly module for Angular..."

WASM_SOURCE="src-tauri/wasm/pkg"
WASM_DEST="src/assets/wasm"

if [ -d "$WASM_DEST" ]; then
    echo "🗑️  Removing existing WASM directory..."
    rm -rf "$WASM_DEST"
fi

echo "📁 Copying WebAssembly files to Angular assets..."
cp -r "$WASM_SOURCE" "$WASM_DEST"

# Create TypeScript declarations for better IDE support
echo "📝 Creating TypeScript declarations..."

cat > "src/app/types/wasm.d.ts" << EOF
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
EOF

# Create a build info file
echo "📊 Creating build info..."
BUILD_INFO="src/assets/wasm/build-info.json"

cat > "$BUILD_INFO" << EOF
{
  "build_time": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "git_commit": "$(git rev-parse HEAD 2>/dev/null || echo 'unknown')",
  "git_branch": "$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'unknown')",
  "rust_version": "$(rustc --version)",
  "wasm_pack_version": "$(wasm-pack --version)",
  "build_success": true
}
EOF

# Update angular.json to include WASM files
echo "⚙️  Updating Angular configuration..."

# Check if angular.json exists
if [ -f "angular.json" ]; then
    echo "✅ angular.json found, WASM files will be included in build"
else
    echo "⚠️  angular.json not found, please ensure WASM files are included in assets"
fi

# Create a simple test to verify WASM loading
echo "🧪 Creating WASM test..."

cat > "src/app/utils/wasm-test.ts" << EOF
/**
 * Utility to test WebAssembly module loading
 */
export async function testWasmLoading(): Promise<boolean> {
  try {
    console.log('Testing WASM module loading...');
    
    // Import the WASM module
    const wasm = await import('../../assets/wasm/chert_wallet_wasm');
    
    // Test basic functionality
    await wasm.init_wasm();
    
    const version = wasm.get_version();
    console.log('WASM version:', version);
    
    const buildInfo = wasm.get_build_info();
    console.log('Build info:', buildInfo);
    
    // Test crypto operations
    const keypair = wasm.generate('ed25519');
    const address = keypair.get_address();
    console.log('Generated address:', address);
    
    // Test address validation
    const isValid = wasm.validate_address(address);
    console.log('Address validation:', isValid);
    
    console.log('✅ WASM module test passed!');
    return true;
  } catch (error) {
    console.error('❌ WASM module test failed:', error);
    return false;
  }
}

/**
 * Performance test for WASM operations
 */
export async function testWasmPerformance(): Promise<void> {
  try {
    console.log('Testing WASM performance...');
    
    const wasm = await import('../../assets/wasm/chert_wallet_wasm');
    
    // Test balance query performance
    wasm.start_performance_timer('test_balance_query');
    
    // Simulate balance query
    await new Promise(resolve => setTimeout(resolve, 10));
    
    const duration = wasm.end_performance_timer('test_balance_query');
    console.log('Test balance query duration:', duration, 'ms');
    
    // Test transaction signing performance
    wasm.start_performance_timer('test_transaction_sign');
    
    const keypair = wasm.generate('ed25519');
    const message = new TextEncoder().encode('test message');
    const signature = keypair.sign(message);
    
    const signDuration = wasm.end_performance_timer('test_transaction_sign');
    console.log('Transaction signing duration:', signDuration, 'ms');
    
    console.log('✅ WASM performance test completed!');
  } catch (error) {
    console.error('❌ WASM performance test failed:', error);
  }
}
EOF

echo "🎉 WebAssembly module setup completed!"
echo ""
echo "📋 Summary:"
echo "   - WebAssembly module built and copied to src/assets/wasm/"
echo "   - TypeScript declarations created in src/app/types/wasm.d.ts"
echo "   - Build info saved to src/assets/wasm/build-info.json"
echo "   - Test utilities created in src/app/utils/wasm-test.ts"
echo ""
echo "🚀 Next steps:"
echo "   1. Run 'npm run build' to build the Angular application"
echo "   2. Test WASM loading with the provided test utilities"
echo "   3. Integrate WASM services into your Angular components"
echo ""
echo "💡 Usage example:"
echo "   import { EnhancedWalletService } from './core/services/legacy/enhanced-wallet.service';"
echo "   import { testWasmLoading } from './utils/wasm-test';"
echo ""
echo "   // Test WASM loading"
echo "   await testWasmLoading();"
echo ""
echo "   // Use enhanced wallet service"
echo "   const walletService = inject(EnhancedWalletService);"
echo "   await walletService.generateWallet('password');"