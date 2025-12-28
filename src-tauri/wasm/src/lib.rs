//! Chert Wallet WebAssembly Library
//!
//! This library provides high-performance blockchain operations
//! for the Chert wallet using WebAssembly.

use js_sys::{Object, Reflect};
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Include the simple module for testing
mod simple;

// Re-export simple functions
pub use simple::*;

mod zkbridge;
pub use zkbridge::*;

// Export a simple function to test WASM is working
#[wasm_bindgen]
pub fn test_wasm() -> String {
    "Chert Wallet WASM is working!".to_string()
}

/// Initialize the WASM module
#[wasm_bindgen]
pub fn init_wasm() -> Result<Object, JsValue> {
    let status = Object::new();
    let _ = Reflect::set(&status, &"initialized".into(), &true.into());
    let _ = Reflect::set(&status, &"modules".into(), &JsValue::from_str("simple"));

    Ok(status)
}

// Get build information - moved to simple.rs to avoid duplication

// Module initialization
#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"Chert Wallet WASM module loaded".into());
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_init_wasm() {
        let status = init_wasm().unwrap();
        let initialized = Reflect::get(&status, &"initialized".into())
            .unwrap()
            .as_bool()
            .unwrap();
        assert!(initialized);
    }
}
