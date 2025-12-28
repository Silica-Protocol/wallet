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
