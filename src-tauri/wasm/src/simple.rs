//! Simple WebAssembly module for initial testing
//!
//! This is a minimal WASM module to test the build system
//! before implementing the full complex modules.

use js_sys::{Object, Reflect};
use wasm_bindgen::prelude::*;

// This is like the `main` function, but for WASM.
// Note: main function is defined in lib.rs to avoid duplication

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

#[wasm_bindgen]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[wasm_bindgen]
pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

#[wasm_bindgen]
pub fn get_version() -> String {
    "0.1.0".to_string()
}

#[wasm_bindgen]
pub fn get_build_info() -> Result<Object, JsValue> {
    let info = Object::new();
    let _ = Reflect::set(&info, &"version".into(), &"0.1.0".into());
    let _ = Reflect::set(&info, &"build_date".into(), &"2024-01-01".into());
    let _ = Reflect::set(&info, &"wasm".into(), &true.into());
    Ok(info)
}

#[wasm_bindgen]
pub fn async_greet(name: String) -> String {
    // Simulate async operation with a simple synchronous version
    format!("Async hello, {}!", name)
}

// Export a simple test function
#[wasm_bindgen]
pub fn test_performance() -> f64 {
    let start = web_sys::window().unwrap().performance().unwrap().now();

    // Do some computation
    let mut result: u64 = 0;
    for i in 0..1000000 {
        result = result.wrapping_add(i);
    }

    let end = web_sys::window().unwrap().performance().unwrap().now();

    // Log checksum so the loop cannot be optimized away entirely.
    web_sys::console::log_1(&format!("checksum={result}").into());

    end - start
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
        assert_eq!(add(-1, 1), 0);
    }

    #[wasm_bindgen_test]
    fn test_multiply() {
        assert_eq!(multiply(3, 4), 12);
        assert_eq!(multiply(-2, 5), -10);
    }

    #[wasm_bindgen_test]
    fn test_greet() {
        assert_eq!(greet("World"), "Hello, World!");
        assert_eq!(greet("WASM"), "Hello, WASM!");
    }

    #[wasm_bindgen_test]
    fn test_get_version() {
        assert_eq!(get_version(), "0.1.0");
    }

    #[wasm_bindgen_test]
    fn test_get_build_info() {
        let info = get_build_info().unwrap();
        let version = Reflect::get(&info, &"version".into())
            .unwrap()
            .as_string()
            .unwrap();
        assert_eq!(version, "0.1.0");
    }
}
