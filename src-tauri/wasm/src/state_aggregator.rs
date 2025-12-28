//! State aggregation module for chain state monitoring
//! 
//! This module provides real-time chain state aggregation
//! for the glance dashboard and network monitoring.

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use js_sys::{Promise, Object, Reflect, Array};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::{WasmError, WasmResult, rust_to_js, js_to_rust};

/// Network state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkState {
    pub block_height: u64,
    pub tps: f64,
    pub active_validators: u32,
    pub total_supply: String,
    pub market_cap: String,
    pub network_hashrate: f64,
    pub difficulty: f64,
    pub avg_block_time: f64,
    pub last_updated: u64,
}

/// Account state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountState {
    pub address: String,
    pub balance: String,
    pub pending: String,
    pub nonce: u64,
    pub staked_amount: String,
    pub rewards: String,
    pub voting_power: f64,
    pub last_activity: u64,
}

/// Validator state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorState {
    pub address: String,
    pub name: String,
    pub commission: f64,
    pub voting_power: f64,
    pub status: ValidatorStatus,
    pub uptime: f64,
    pub total_stake: String,
    pub rewards_rate: f64,
    pub last_block: u64,
}

/// Validator status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidatorStatus {
    Active,
    Inactive,
    Jailed,
    Slashed,
}

impl ValidatorStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ValidatorStatus::Active => "active",
            ValidatorStatus::Inactive => "inactive",
            ValidatorStatus::Jailed => "jailed",
            ValidatorStatus::Slashed => "slashed",
        }
    }
}

/// State aggregator for real-time monitoring
#[wasm_bindgen]
pub struct StateAggregator {
    cache: HashMap<String, (u64, u64)>, // (data, timestamp)
    update_interval: u32,
    last_update: u64,
}

#[wasm_bindgen]
impl StateAggregator {
    /// Create a new state aggregator
    #[wasm_bindgen(constructor)]
    pub fn new(update_interval_ms: u32) -> StateAggregator {
        StateAggregator {
            cache: HashMap::new(),
            update_interval: update_interval_ms,
            last_update: 0,
        }
    }

    /// Get current network state
    #[wasm_bindgen]
    pub fn get_network_state(&self) -> Result<Promise, JsValue> {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Check if we need to update
        if current_time - self.last_update < self.update_interval as u64 {
            if let Some((cached_data, _)) = self.cache.get("network_state") {
                // Return cached data
                let result = JsValue::from_f64(*cached_data as f64);
                return Ok(Promise::resolve(&result));
            }
        }

        let promise = future_to_promise(async move {
            let network_state = simulate_network_state().await?;
            let result = rust_to_js(&network_state)?;
            Ok(result.into())
        });

        Ok(promise)
    }

    /// Get account state
    #[wasm_bindgen]
    pub fn get_account_state(&self, address: String) -> Result<Promise, JsValue> {
        if address.is_empty() {
            return Err(WasmError::new("INVALID_ADDRESS", "Address cannot be empty").into());
        }

        let promise = future_to_promise(async move {
            let account_state = simulate_account_state(&address).await?;
            let result = rust_to_js(&account_state)?;
            Ok(result.into())
        });

        Ok(promise)
    }

    /// Get multiple account states in batch
    #[wasm_bindgen]
    pub fn get_batch_account_states(&self, addresses: Vec<String>) -> Result<Promise, JsValue> {
        if addresses.is_empty() {
            return Err(WasmError::new("EMPTY_ADDRESSES", "Address list cannot be empty").into());
        }

        let promise = future_to_promise(async move {
            let mut states = Vec::new();
            
            for address in addresses {
                let state = simulate_account_state(&address).await?;
                states.push(state);
            }
            
            let result = rust_to_js(&states)?;
            Ok(result.into())
        });

        Ok(promise)
    }

    /// Get validator states
    #[wasm_bindgen]
    pub fn get_validator_states(&self, limit: Option<u32>) -> Result<Promise, JsValue> {
        let limit = limit.unwrap_or(100);

        let promise = future_to_promise(async move {
            let validator_states = simulate_validator_states(limit).await?;
            let result = rust_to_js(&validator_states)?;
            Ok(result.into())
        });

        Ok(promise)
    }

    /// Get network statistics
    #[wasm_bindgen]
    pub fn get_network_statistics(&self) -> Result<Promise, JsValue> {
        let promise = future_to_promise(async move {
            let stats = simulate_network_statistics().await?;
            let result = rust_to_js(&stats)?;
            Ok(result.into())
        });

        Ok(promise)
    }

    /// Get real-time metrics
    #[wasm_bindgen]
    pub fn get_real_time_metrics(&self) -> Result<Object, JsValue> {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let metrics = Object::new();
        
        // Current timestamp
        let _ = Reflect::set(&metrics, &"timestamp".into(), &current_time.into());
        
        // Cache hit rate
        let cache_hit_rate = self.cache.len() as f64 / 100.0; // Simulated
        let _ = Reflect::set(&metrics, &"cache_hit_rate".into(), &cache_hit_rate.into());
        
        // Update frequency
        let _ = Reflect::set(&metrics, &"update_interval_ms".into(), &self.update_interval.into());
        
        // Last update
        let _ = Reflect::set(&metrics, &"last_update".into(), &self.last_update.into());

        Ok(metrics)
    }

    /// Start real-time monitoring
    #[wasm_bindgen]
    pub fn start_monitoring(&mut self, callback: &js_sys::Function) -> Result<(), JsValue> {
        // In a real implementation, this would set up WebSocket connections
        // and start real-time data streaming
        web_sys::console::log_1(&"Starting real-time state monitoring".into());
        Ok(())
    }

    /// Stop real-time monitoring
    #[wasm_bindgen]
    pub fn stop_monitoring(&mut self) -> Result<(), JsValue> {
        web_sys::console::log_1(&"Stopping real-time state monitoring".into());
        Ok(())
    }

    /// Clear cache
    #[wasm_bindgen]
    pub fn clear_cache(&mut self) -> Result<(), JsValue> {
        self.cache.clear();
        self.last_update = 0;
        Ok(())
    }
}

/// Network statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatistics {
    pub total_transactions: u64,
    pub total_addresses: u64,
    pub total_blocks: u64,
    pub avg_gas_price: String,
    pub network_utilization: f64,
    pub mempool_size: u64,
    pub mempool_bytes: u64,
    pub chain_size_mb: f64,
    pub peer_count: u32,
}

// Internal simulation functions
async fn simulate_network_state() -> WasmResult<NetworkState> {
    // Simulate network latency
    wasm_bindgen_futures::JsFuture::from(
        js_sys::Promise::resolve(&JsValue::from(20))
    ).await.unwrap();

    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Ok(NetworkState {
        block_height: 12345678 + (current_time % 1000),
        tps: 15.5 + (current_time % 10) as f64 * 0.5,
        active_validators: 125,
        total_supply: "1000000000000000000000000000".to_string(), // 1B CHERT
        market_cap: "50000000000".to_string(), // $50B
        network_hashrate: 1500000000.0, // 1.5 TH/s
        difficulty: 2500000000000.0,
        avg_block_time: 6.0,
        last_updated: current_time,
    })
}

async fn simulate_account_state(address: &str) -> WasmResult<AccountState> {
    wasm_bindgen_futures::JsFuture::from(
        js_sys::Promise::resolve(&JsValue::from(15))
    ).await.unwrap();

    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Generate deterministic but varied data based on address
    let address_hash = address.chars().map(|c| c as u32).sum::<u32>();
    
    Ok(AccountState {
        address: address.to_string(),
        balance: format!("{}000000000000000000", 1000000 + (address_hash % 1000000)),
        pending: format!("{}000000000000000000", (address_hash % 100000)),
        nonce: (address_hash % 1000) as u64,
        staked_amount: format!("{}000000000000000000", (address_hash % 500000)),
        rewards: format!("{}000000000000000000", (address_hash % 10000)),
        voting_power: (address_hash % 10000) as f64 / 100.0,
        last_activity: current_time - (address_hash % 86400) as u64,
    })
}

async fn simulate_validator_states(limit: u32) -> WasmResult<Vec<ValidatorState>> {
    wasm_bindgen_futures::JsFuture::from(
        js_sys::Promise::resolve(&JsValue::from(30))
    ).await.unwrap();

    let mut validators = Vec::new();
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    for i in 0..limit {
        let status = match i % 10 {
            0 => ValidatorStatus::Jailed,
            1 => ValidatorStatus::Inactive,
            2 => ValidatorStatus::Slashed,
            _ => ValidatorStatus::Active,
        };

        validators.push(ValidatorState {
            address: format!("0x{:064x}", i + 1000),
            name: format!("Validator #{}", i + 1),
            commission: 5.0 + (i % 10) as f64 * 0.5,
            voting_power: (1000000.0 - (i as f64 * 1000.0)).max(1000.0),
            status,
            uptime: 95.0 + (i % 5) as f64 * 1.0,
            total_stake: format!("{}000000000000000000", 1000000 + i as u64 * 10000),
            rewards_rate: 8.5 + (i % 3) as f64 * 0.5,
            last_block: current_time - (i as u64 * 60),
        });
    }

    Ok(validators)
}

async fn simulate_network_statistics() -> WasmResult<NetworkStatistics> {
    wasm_bindgen_futures::JsFuture::from(
        js_sys::Promise::resolve(&JsValue::from(25))
    ).await.unwrap();

    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Ok(NetworkStatistics {
        total_transactions: 500000000 + (current_time % 1000000),
        total_addresses: 2500000 + (current_time % 10000),
        total_blocks: 12345678,
        avg_gas_price: "1000000000".to_string(), // 1 Gwei
        network_utilization: 65.5 + (current_time % 20) as f64 * 0.5,
        mempool_size: 5000 + (current_time % 1000),
        mempool_bytes: 50000000 + (current_time % 10000000),
        chain_size_mb: 10240.5 + (current_time % 100) as f64 * 0.1,
        peer_count: 50 + (current_time % 20) as u32,
    })
}

/// Performance metrics for state aggregation
#[wasm_bindgen]
pub struct StateAggregatorMetrics {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub total_requests: u64,
    pub avg_response_time: f64,
    pub error_rate: f64,
}

#[wasm_bindgen]
impl StateAggregatorMetrics {
    /// Create new metrics instance
    #[wasm_bindgen(constructor)]
    pub fn new() -> StateAggregatorMetrics {
        StateAggregatorMetrics {
            cache_hits: 0,
            cache_misses: 0,
            total_requests: 0,
            avg_response_time: 0.0,
            error_rate: 0.0,
        }
    }

    /// Get current metrics as JavaScript object
    #[wasm_bindgen]
    pub fn to_js_object(&self) -> Result<Object, JsValue> {
        let metrics = Object::new();
        
        let _ = Reflect::set(&metrics, &"cache_hits".into(), &self.cache_hits.into());
        let _ = Reflect::set(&metrics, &"cache_misses".into(), &self.cache_misses.into());
        let _ = Reflect::set(&metrics, &"total_requests".into(), &self.total_requests.into());
        let _ = Reflect::set(&metrics, &"avg_response_time".into(), &self.avg_response_time.into());
        let _ = Reflect::set(&metrics, &"error_rate".into(), &self.error_rate.into());

        Ok(metrics)
    }

    /// Calculate cache hit rate
    #[wasm_bindgen]
    pub fn cache_hit_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.cache_hits as f64 / self.total_requests as f64 * 100.0
        }
    }
}

// Console logging utility
fn console_log(message: &str) {
    web_sys::console::log_1(&JsValue::from_str(message));
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_state_aggregator_creation() {
        let aggregator = StateAggregator::new(5000);
        let metrics = aggregator.get_real_time_metrics().unwrap();
        
        let update_interval = Reflect::get(&metrics, &"update_interval_ms".into()).unwrap().as_f64().unwrap();
        assert_eq!(update_interval, 5000.0);
    }

    #[wasm_bindgen_test]
    fn test_validator_status() {
        let status = ValidatorStatus::Active;
        assert_eq!(status.as_str(), "active");
        
        let jailed = ValidatorStatus::Jailed;
        assert_eq!(jailed.as_str(), "jailed");
    }

    #[wasm_bindgen_test]
    fn test_metrics_creation() {
        let metrics = StateAggregatorMetrics::new();
        assert_eq!(metrics.cache_hits, 0);
        assert_eq!(metrics.cache_misses, 0);
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.cache_hit_rate(), 0.0);
    }
}