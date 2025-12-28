//! Transaction fetching module for fast transaction history
//! 
//! This module provides optimized transaction fetching with caching,
//! pagination, and filtering capabilities.

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use js_sys::{Promise, Object, Reflect, Array};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::{WasmError, WasmResult, rust_to_js, js_to_rust};

/// Transaction structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub hash: String,
    pub from: String,
    pub to: String,
    pub amount: String,
    pub fee: String,
    pub nonce: u64,
    pub block_height: u64,
    pub timestamp: u64,
    pub status: TransactionStatus,
    pub memo: Option<String>,
    pub gas_used: u64,
    pub gas_limit: u64,
}

/// Transaction status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
    Replaced,
}

impl TransactionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransactionStatus::Pending => "pending",
            TransactionStatus::Confirmed => "confirmed",
            TransactionStatus::Failed => "failed",
            TransactionStatus::Replaced => "replaced",
        }
    }
}

/// Transaction filter options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionFilter {
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
    pub status: Option<TransactionStatus>,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub min_amount: Option<String>,
    pub max_amount: Option<String>,
    pub memo_contains: Option<String>,
}

/// Transaction fetcher with caching
#[wasm_bindgen]
pub struct TransactionFetcher {
    cache: Arc<Mutex<HashMap<String, (Vec<Transaction>, u64)>>>, // (transactions, timestamp)
    ttl_ms: u64,
    api_endpoint: String,
}

#[wasm_bindgen]
impl TransactionFetcher {
    /// Create a new transaction fetcher
    #[wasm_bindgen(constructor)]
    pub fn new(api_endpoint: String, ttl_ms: u64) -> TransactionFetcher {
        TransactionFetcher {
            cache: Arc::new(Mutex::new(HashMap::new())),
            ttl_ms,
            api_endpoint,
        }
    }

    /// Fetch transactions for an address with pagination
    #[wasm_bindgen]
    pub fn fetch_transactions(
        &self,
        address: String,
        page: u32,
        limit: u32,
        filter_js: Option<JsValue>,
    ) -> Result<Promise, JsValue> {
        if address.is_empty() {
            return Err(WasmError::new("INVALID_ADDRESS", "Address cannot be empty").into());
        }

        if limit == 0 || limit > 1000 {
            return Err(WasmError::new("INVALID_LIMIT", "Limit must be between 1 and 1000").into());
        }

        let filter: Option<TransactionFilter> = match filter_js {
            Some(js) => Some(js_to_rust(&js)?),
            None => None,
        };

        let cache = self.cache.clone();
        let ttl_ms = self.ttl_ms;
        let api_endpoint = self.api_endpoint.clone();

        let promise = future_to_promise(async move {
            let cache_key = format!("{}:{}:{}:{}", address, page, limit, 
                serde_json::to_string(&filter).unwrap_or_default());

            // Check cache first
            {
                let cache_guard = cache.lock().unwrap();
                if let Some((transactions, timestamp)) = cache_guard.get(&cache_key) {
                    let current_time = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;
                    
                    if current_time - timestamp < ttl_ms {
                        let result = rust_to_js(transactions)?;
                        return Ok(result.into());
                    }
                }
            } // Release lock

            // Fetch from API
            let transactions = simulate_transaction_fetch(&address, page, limit, filter).await?;
            
            // Update cache
            {
                let mut cache_guard = cache.lock().unwrap();
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                cache_guard.insert(cache_key, (transactions.clone(), current_time));
            }

            let result = rust_to_js(&transactions)?;
            Ok(result.into())
        });

        Ok(promise)
    }

    /// Fetch a single transaction by hash
    #[wasm_bindgen]
    pub fn fetch_transaction(&self, hash: String) -> Result<Promise, JsValue> {
        if hash.is_empty() {
            return Err(WasmError::new("INVALID_HASH", "Transaction hash cannot be empty").into());
        }

        let api_endpoint = self.api_endpoint.clone();

        let promise = future_to_promise(async move {
            let transaction = simulate_single_transaction_fetch(&hash).await?;
            let result = rust_to_js(&transaction)?;
            Ok(result.into())
        });

        Ok(promise)
    }

    /// Get transaction count for an address
    #[wasm_bindgen]
    pub fn get_transaction_count(&self, address: String, filter_js: Option<JsValue>) -> Result<Promise, JsValue> {
        if address.is_empty() {
            return Err(WasmError::new("INVALID_ADDRESS", "Address cannot be empty").into());
        }

        let filter: Option<TransactionFilter> = match filter_js {
            Some(js) => Some(js_to_rust(&js)?),
            None => None,
        };

        let promise = future_to_promise(async move {
            let count = simulate_transaction_count(&address, filter).await?;
            Ok(JsValue::from_f64(count as f64))
        });

        Ok(promise)
    }

    /// Search transactions by various criteria
    #[wasm_bindgen]
    pub fn search_transactions(&self, query_js: JsValue) -> Result<Promise, JsValue> {
        let query: TransactionSearchQuery = js_to_rust(&query_js)?;

        let promise = future_to_promise(async move {
            let results = simulate_transaction_search(&query).await?;
            let result = rust_to_js(&results)?;
            Ok(result.into())
        });

        Ok(promise)
    }

    /// Clear the cache
    #[wasm_bindgen]
    pub fn clear_cache(&self) -> Result<(), JsValue> {
        let mut cache_guard = self.cache.lock().unwrap();
        cache_guard.clear();
        Ok(())
    }

    /// Get cache statistics
    #[wasm_bindgen]
    pub fn get_cache_stats(&self) -> Result<Object, JsValue> {
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

/// Transaction search query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionSearchQuery {
    pub query: String,
    pub search_type: TransactionSearchType,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Transaction search type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionSearchType {
    ByHash,
    ByAddress,
    ByMemo,
    ByAmount,
    ByBlock,
}

/// Transaction batch for bulk operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionBatch {
    pub transactions: Vec<Transaction>,
    pub total_count: u64,
    pub page: u32,
    pub limit: u32,
    pub has_more: bool,
}

// Internal simulation functions
async fn simulate_transaction_fetch(
    address: &str,
    page: u32,
    limit: u32,
    filter: Option<TransactionFilter>,
) -> WasmResult<Vec<Transaction>> {
    // Simulate network latency
    wasm_bindgen_futures::JsFuture::from(
        js_sys::Promise::resolve(&JsValue::from(50))
    ).await.unwrap();

    let mut transactions = Vec::new();
    let start_index = page as usize * limit as usize;
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    for i in 0..limit {
        let index = start_index + i as usize;
        let tx_hash = format!("0x{:064x}", index + 1000);
        
        // Apply filters if provided
        if let Some(ref f) = filter {
            if let Some(ref status) = f.status {
                if index % 10 == 0 && *status != TransactionStatus::Failed {
                    continue;
                }
            }
        }

        transactions.push(Transaction {
            hash: tx_hash,
            from: if index % 2 == 0 { address.to_string() } else { "0xOTHERADDRESS".to_string() },
            to: if index % 2 == 0 { "0xOTHERADDRESS".to_string() } else { address.to_string() },
            amount: format!("{}000000000000000000", 1000 + i),
            fee: "1000000000000000".to_string(),
            nonce: (index as u64),
            block_height: 1000000 + index as u64,
            timestamp: current_time - (index as u64 * 600), // 10 minutes apart
            status: if index % 10 == 0 { TransactionStatus::Failed } else { TransactionStatus::Confirmed },
            memo: Some(format!("Transaction #{}", index + 1)),
            gas_used: 21000,
            gas_limit: 21000,
        });
    }

    Ok(transactions)
}

async fn simulate_single_transaction_fetch(hash: &str) -> WasmResult<Transaction> {
    wasm_bindgen_futures::JsFuture::from(
        js_sys::Promise::resolve(&JsValue::from(30))
    ).await.unwrap();

    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Ok(Transaction {
        hash: hash.to_string(),
        from: "0xSENDERADDRESS".to_string(),
        to: "0xRECEIVERADDRESS".to_string(),
        amount: "1000000000000000000".to_string(),
        fee: "1000000000000000".to_string(),
        nonce: 42,
        block_height: 12345678,
        timestamp: current_time,
        status: TransactionStatus::Confirmed,
        memo: Some("Sample transaction".to_string()),
        gas_used: 21000,
        gas_limit: 21000,
    })
}

async fn simulate_transaction_count(address: &str, _filter: Option<TransactionFilter>) -> WasmResult<u64> {
    wasm_bindgen_futures::JsFuture::from(
        js_sys::Promise::resolve(&JsValue::from(20))
    ).await.unwrap();

    // Simulate varying transaction counts based on address
    let count = if address.contains("1") {
        150
    } else if address.contains("2") {
        75
    } else {
        200
    };

    Ok(count)
}

async fn simulate_transaction_search(query: &TransactionSearchQuery) -> WasmResult<Vec<Transaction>> {
    wasm_bindgen_futures::JsFuture::from(
        js_sys::Promise::resolve(&JsValue::from(100))
    ).await.unwrap();

    let mut transactions = Vec::new();
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Simulate search results
    for i in 0..10 {
        transactions.push(Transaction {
            hash: format!("0x{:064x}", i + 5000),
            from: "0xSEARCHRESULT".to_string(),
            to: "0xSEARCHRESULT".to_string(),
            amount: format!("{}000000000000000000", 500 + i),
            fee: "1000000000000000".to_string(),
            nonce: i as u64,
            block_height: 1000000 + i as u64,
            timestamp: current_time - (i as u64 * 600),
            status: TransactionStatus::Confirmed,
            memo: Some(format!("Search result for: {}", query.query)),
            gas_used: 21000,
            gas_limit: 21000,
        });
    }

    Ok(transactions)
}

/// Transaction export functionality
#[wasm_bindgen]
pub fn export_transactions(transactions_js: JsValue, format: String) -> Result<Promise, JsValue> {
    let transactions: Vec<Transaction> = js_to_rust(&transactions_js)?;

    let promise = future_to_promise(async move {
        let result = match format.as_str() {
            "json" => export_to_json(&transactions).await?,
            "csv" => export_to_csv(&transactions).await?,
            _ => return Err(WasmError::new("INVALID_FORMAT", "Supported formats: json, csv").into()),
        };

        Ok(JsValue::from_str(&result))
    });

    Ok(promise)
}

async fn export_to_json(transactions: &[Transaction]) -> WasmResult<String> {
    serde_json::to_string_pretty(transactions)
        .map_err(|e| WasmError::new("EXPORT_ERROR", &format!("JSON export failed: {}", e)))
}

async fn export_to_csv(transactions: &[Transaction]) -> WasmResult<String> {
    let mut csv = "Hash,From,To,Amount,Fee,Status,Timestamp,Memo\n".to_string();
    
    for tx in transactions {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            tx.hash,
            tx.from,
            tx.to,
            tx.amount,
            tx.fee,
            tx.status.as_str(),
            tx.timestamp,
            tx.memo.as_deref().unwrap_or("")
        ));
    }

    Ok(csv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_transaction_fetcher_creation() {
        let fetcher = TransactionFetcher::new("https://api.chert.com".to_string(), 60000);
        // Test that it was created successfully
        let stats = fetcher.get_cache_stats().unwrap();
        let total = Reflect::get(&stats, &"total_entries".into()).unwrap().as_f64().unwrap();
        assert_eq!(total, 0.0);
    }

    #[wasm_bindgen_test]
    fn test_transaction_status() {
        let status = TransactionStatus::Confirmed;
        assert_eq!(status.as_str(), "confirmed");
    }

    #[wasm_bindgen_test]
    fn test_invalid_parameters() {
        let fetcher = TransactionFetcher::new("https://api.chert.com".to_string(), 60000);
        
        // Test empty address
        let result = fetcher.fetch_transactions("".to_string(), 0, 10, None);
        assert!(result.is_err());
        
        // Test invalid limit
        let result = fetcher.fetch_transactions("0x123".to_string(), 0, 0, None);
        assert!(result.is_err());
    }
}