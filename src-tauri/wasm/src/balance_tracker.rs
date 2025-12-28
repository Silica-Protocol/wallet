//! Balance tracking module for real-time balance updates
//! 
//! This module provides WebSocket connections to account chains
//! for instant balance updates and caching.

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use js_sys::{Promise, Object, Reflect, Function};
use web_sys::{WebSocket, MessageEvent, ErrorEvent, CloseEvent, BinaryType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::cell::RefCell;
use crate::{WasmError, WasmResult, get_config, rust_to_js, js_to_rust, ACTIVE_SUBSCRIPTIONS};

/// Balance update structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceUpdate {
    pub address: String,
    pub balance: String,
    pub pending: String,
    pub nonce: u64,
    pub last_update: u64,
    pub block_height: u64,
}

/// Balance subscription for real-time updates
#[wasm_bindgen]
pub struct BalanceSubscription {
    address: String,
    websocket: Option<WebSocket>,
    callback: Option<Function>,
    active: bool,
}

#[wasm_bindgen]
impl BalanceSubscription {
    /// Create a new balance subscription for an address
    #[wasm_bindgen(constructor)]
    pub fn new(address: String) -> Result<BalanceSubscription, JsValue> {
        if address.is_empty() {
            return Err(WasmError::new("INVALID_ADDRESS", "Address cannot be empty").into());
        }

        Ok(BalanceSubscription {
            address,
            websocket: None,
            callback: None,
            active: false,
        })
    }

    /// Subscribe to balance updates with callback
    #[wasm_bindgen]
    pub fn subscribe(&mut self, callback: &Function) -> Result<Promise, JsValue> {
        if self.active {
            return Err(WasmError::new("ALREADY_SUBSCRIBED", "Subscription already active").into());
        }

        self.callback = Some(callback.clone());
        
        let promise = future_to_promise(async move {
            // This would connect to the actual WebSocket endpoint
            // For now, we'll simulate the connection
            Ok(JsValue::from_str("Balance subscription created"))
        });

        // Store subscription in global registry
        ACTIVE_SUBSCRIPTIONS.with(|subscriptions| {
            // Note: In a real implementation, we'd store the actual WebSocket
            subscriptions.borrow_mut().insert(self.address.clone(), WebSocket::new("").unwrap());
        });

        self.active = true;
        Ok(promise)
    }

    /// Unsubscribe from balance updates
    #[wasm_bindgen]
    pub fn unsubscribe(&mut self) -> Result<(), JsValue> {
        if !self.active {
            return Ok(());
        }

        // Close WebSocket if it exists
        if let Some(ws) = &self.websocket {
            let _ = ws.close();
        }

        // Remove from global registry
        ACTIVE_SUBSCRIPTIONS.with(|subscriptions| {
            subscriptions.borrow_mut().remove(&self.address);
        });

        self.active = false;
        self.websocket = None;
        self.callback = None;

        Ok(())
    }

    /// Get current subscription status
    #[wasm_bindgen]
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Get the subscribed address
    #[wasm_bindgen]
    pub fn get_address(&self) -> String {
        self.address.clone()
    }
}

/// Get current balance for an address (single query)
#[wasm_bindgen]
pub fn get_balance(address: String) -> Result<Promise, JsValue> {
    if address.is_empty() {
        return Err(WasmError::new("INVALID_ADDRESS", "Address cannot be empty").into());
    }

    let promise = future_to_promise(async move {
        // Simulate API call to get balance
        // In a real implementation, this would query the Silica node
        let balance_update = simulate_balance_query(&address).await?;
        
        let result = rust_to_js(&balance_update)?;
        Ok(result.into())
    });

    Ok(promise)
}

/// Get multiple balances in batch
#[wasm_bindgen]
pub fn get_batch_balances(addresses: Vec<JsValue>) -> Result<Promise, JsValue> {
    if addresses.is_empty() {
        return Err(WasmError::new("EMPTY_ADDRESSES", "Address list cannot be empty").into());
    }

    let promise = future_to_promise(async move {
        let mut results = Vec::new();
        
        for address_js in addresses {
            let address: String = js_to_rust(&address_js)?;
            let balance_update = simulate_balance_query(&address).await?;
            results.push(balance_update);
        }
        
        let result = rust_to_js(&results)?;
        Ok(result.into())
    });

    Ok(promise)
}

/// Get balance history for an address
#[wasm_bindgen]
pub fn get_balance_history(address: String, from_block: u64, to_block: u64) -> Result<Promise, JsValue> {
    if address.is_empty() {
        return Err(WasmError::new("INVALID_ADDRESS", "Address cannot be empty").into());
    }

    if from_block > to_block {
        return Err(WasmError::new("INVALID_RANGE", "From block cannot be greater than to block").into());
    }

    let promise = future_to_promise(async move {
        let history = simulate_balance_history(&address, from_block, to_block).await?;
        let result = rust_to_js(&history)?;
        Ok(result.into())
    });

    Ok(promise)
}

/// Balance history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceHistoryEntry {
    pub block_height: u64,
    pub balance: String,
    pub timestamp: u64,
    pub transaction_hash: Option<String>,
}

// Internal simulation functions
async fn simulate_balance_query(address: &str) -> WasmResult<BalanceUpdate> {
    // Simulate network latency
    wasm_bindgen_futures::JsFuture::from(
        js_sys::Promise::resolve(&JsValue::from(100))
    ).await.unwrap();

    // Simulate balance data
    Ok(BalanceUpdate {
        address: address.to_string(),
        balance: "1234567890000000000".to_string(), // 1.23456789 CHERT
        pending: "100000000000000000".to_string(),   // 0.1 CHERT pending
        nonce: 42,
        last_update: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        block_height: 12345678,
    })
}

async fn simulate_balance_history(address: &str, from_block: u64, to_block: u64) -> WasmResult<Vec<BalanceHistoryEntry>> {
    let mut history = Vec::new();
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    for block in from_block..=to_block {
        // Simulate a balance change every 100 blocks
        if block % 100 == 0 {
            history.push(BalanceHistoryEntry {
                block_height: block,
                balance: format!("{}000000000000000000", 1000000 + (block / 100)),
                timestamp: current_time - (to_block - block) * 600, // 10 minutes per block
                transaction_hash: Some(format!("0x{:064x}", block)),
            });
        }
    }

    Ok(history)
}

/// Balance cache for performance optimization
#[wasm_bindgen]
pub struct BalanceCache {
    cache: Arc<Mutex<HashMap<String, (BalanceUpdate, u64)>>>, // (balance, timestamp)
    ttl_ms: u64,
}

#[wasm_bindgen]
impl BalanceCache {
    /// Create a new balance cache with specified TTL
    #[wasm_bindgen(constructor)]
    pub fn new(ttl_ms: u64) -> BalanceCache {
        BalanceCache {
            cache: Arc::new(Mutex::new(HashMap::new())),
            ttl_ms,
        }
    }

    /// Get balance from cache or fetch if not available/expired
    #[wasm_bindgen]
    pub fn get_or_fetch(&self, address: String) -> Result<Promise, JsValue> {
        let cache = self.cache.clone();
        let ttl_ms = self.ttl_ms;

        let promise = future_to_promise(async move {
            // Check cache first
            {
                let cache_guard = cache.lock().unwrap();
                if let Some((balance, timestamp)) = cache_guard.get(&address) {
                    let current_time = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;
                    
                    if current_time - timestamp < ttl_ms {
                        let result = rust_to_js(balance)?;
                        return Ok(result.into());
                    }
                }
            } // Release lock

            // Fetch fresh balance
            let balance_update = simulate_balance_query(&address).await?;
            
            // Update cache
            {
                let mut cache_guard = cache.lock().unwrap();
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                cache_guard.insert(address.clone(), (balance_update.clone(), current_time));
            }

            let result = rust_to_js(&balance_update)?;
            Ok(result.into())
        });

        Ok(promise)
    }

    /// Clear the cache
    #[wasm_bindgen]
    pub fn clear(&self) -> Result<(), JsValue> {
        let mut cache_guard = self.cache.lock().unwrap();
        cache_guard.clear();
        Ok(())
    }

    /// Get cache statistics
    #[wasm_bindgen]
    pub fn get_stats(&self) -> Result<Object, JsValue> {
        let cache_guard = self.cache.lock().unwrap();
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let mut valid_entries = 0;
        let mut expired_entries = 0;

        for (_, (_, timestamp)) in cache_guard.iter() {
            if current_time - timestamp < self.ttl_ms {
                valid_entries += 1;
            } else {
                expired_entries += 1;
            }
        }

        let stats = Object::new();
        let _ = Reflect::set(&stats, &"total_entries".into(), &cache_guard.len().into());
        let _ = Reflect::set(&stats, &"valid_entries".into(), &valid_entries.into());
        let _ = Reflect::set(&stats, &"expired_entries".into(), &expired_entries.into());
        let _ = Reflect::set(&stats, &"ttl_ms".into(), &self.ttl_ms.into());

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_balance_subscription_creation() {
        let subscription = BalanceSubscription::new("0x1234567890123456789012345678901234567890".to_string());
        assert!(subscription.is_ok());
        
        let sub = subscription.unwrap();
        assert_eq!(sub.get_address(), "0x1234567890123456789012345678901234567890");
        assert!(!sub.is_active());
    }

    #[wasm_bindgen_test]
    fn test_balance_cache() {
        let cache = BalanceCache::new(60000); // 1 minute TTL
        let stats = cache.get_stats().unwrap();
        
        let total = Reflect::get(&stats, &"total_entries".into()).unwrap().as_f64().unwrap();
        assert_eq!(total, 0.0);
    }

    #[wasm_bindgen_test]
    fn test_invalid_address() {
        let result = BalanceSubscription::new("".to_string());
        assert!(result.is_err());
    }
}