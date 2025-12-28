//! Name resolver module for custom name registration system
//! 
//! This module provides name registration, resolution, and management
//! for the Chert blockchain naming service.

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use js_sys::{Promise, Object, Reflect, Array};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::{WasmError, WasmResult, rust_to_js, js_to_rust};

/// Name registration record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NameRecord {
    pub name: String,
    pub address: String,
    pub owner: String,
    pub registration_date: u64,
    pub expiry_date: u64,
    pub status: NameStatus,
    pub data: Option<String>, // Additional metadata
}

/// Name status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NameStatus {
    Registered,
    Expired,
    Pending,
    Revoked,
}

impl NameStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            NameStatus::Registered => "registered",
            NameStatus::Expired => "expired",
            NameStatus::Pending => "pending",
            NameStatus::Revoked => "revoked",
        }
    }
}

/// Name registration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NameRegistrationRequest {
    pub name: String,
    pub address: String,
    pub owner: String,
    pub duration_years: u32,
    pub data: Option<String>,
}

/// Name search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NameSearchResult {
    pub name: String,
    pub address: String,
    pub owner: String,
    pub registration_date: u64,
    pub expiry_date: u64,
    pub status: NameStatus,
    pub is_available: bool,
}

/// Name resolver for Chert naming service
#[wasm_bindgen]
pub struct NameResolver {
    cache: HashMap<String, NameRecord>,
    registry_contract: String,
    min_name_length: usize,
    max_name_length: usize,
    registration_fee: String,
}

#[wasm_bindgen]
impl NameResolver {
    /// Create a new name resolver
    #[wasm_bindgen(constructor)]
    pub fn new(registry_contract: String) -> NameResolver {
        NameResolver {
            cache: HashMap::new(),
            registry_contract,
            min_name_length: 3,
            max_name_length: 32,
            registration_fee: "1000000000000000000".to_string(), // 1 CHERT
        }
    }

    /// Register a new name
    #[wasm_bindgen]
    pub fn register_name(&self, request_js: &JsValue) -> Result<Promise, JsValue> {
        let request: NameRegistrationRequest = js_to_rust(request_js)?;

        // Validate request
        if let Err(error) = self.validate_registration_request(&request) {
            return Err(error.into());
        }

        let registry_contract = self.registry_contract.clone();

        let promise = future_to_promise(async move {
            let result = simulate_name_registration(&request, &registry_contract).await?;
            let result_js = rust_to_js(&result)?;
            Ok(result_js.into())
        });

        Ok(promise)
    }

    /// Resolve a name to an address
    #[wasm_bindgen]
    pub fn resolve_name(&self, name: String) -> Result<Promise, JsValue> {
        if name.is_empty() {
            return Err(WasmError::new("INVALID_NAME", "Name cannot be empty").into());
        }

        // Check cache first
        if let Some(record) = self.cache.get(&name) {
            let result = JsValue::from_str(&record.address);
            return Ok(Promise::resolve(&result));
        }

        let registry_contract = self.registry_contract.clone();

        let promise = future_to_promise(async move {
            let address = simulate_name_resolution(&name, &registry_contract).await?;
            Ok(JsValue::from_str(&address))
        });

        Ok(promise)
    }

    /// Reverse lookup - get name from address
    #[wasm_bindgen]
    pub fn reverse_lookup(&self, address: String) -> Result<Promise, JsValue> {
        if address.is_empty() {
            return Err(WasmError::new("INVALID_ADDRESS", "Address cannot be empty").into());
        }

        let registry_contract = self.registry_contract.clone();

        let promise = future_to_promise(async move {
            let name = simulate_reverse_lookup(&address, &registry_contract).await?;
            Ok(JsValue::from_str(&name))
        });

        Ok(promise)
    }

    /// Check if a name is available
    #[wasm_bindgen]
    pub fn check_availability(&self, name: String) -> Result<Promise, JsValue> {
        if name.is_empty() {
            return Err(WasmError::new("INVALID_NAME", "Name cannot be empty").into());
        }

        if let Err(error) = self.validate_name_format(&name) {
            return Ok(Promise::resolve(&JsValue::from_bool(false)));
        }

        let registry_contract = self.registry_contract.clone();

        let promise = future_to_promise(async move {
            let is_available = simulate_availability_check(&name, &registry_contract).await?;
            Ok(JsValue::from_bool(is_available))
        });

        Ok(promise)
    }

    /// Get name record details
    #[wasm_bindgen]
    pub fn get_name_record(&self, name: String) -> Result<Promise, JsValue> {
        if name.is_empty() {
            return Err(WasmError::new("INVALID_NAME", "Name cannot be empty").into());
        }

        // Check cache first
        if let Some(record) = self.cache.get(&name) {
            let result = rust_to_js(record)?;
            return Ok(Promise::resolve(&result.into()));
        }

        let registry_contract = self.registry_contract.clone();

        let promise = future_to_promise(async move {
            let record = simulate_get_name_record(&name, &registry_contract).await?;
            let result = rust_to_js(&record)?;
            Ok(result.into())
        });

        Ok(promise)
    }

    /// Search for names
    #[wasm_bindgen]
    pub fn search_names(&self, query: String, limit: Option<u32>) -> Result<Promise, JsValue> {
        if query.is_empty() {
            return Err(WasmError::new("EMPTY_QUERY", "Search query cannot be empty").into());
        }

        let limit = limit.unwrap_or(50);
        let registry_contract = self.registry_contract.clone();

        let promise = future_to_promise(async move {
            let results = simulate_name_search(&query, limit, &registry_contract).await?;
            let result = rust_to_js(&results)?;
            Ok(result.into())
        });

        Ok(promise)
    }

    /// Get names owned by an address
    #[wasm_bindgen]
    pub fn get_names_by_owner(&self, owner: String) -> Result<Promise, JsValue> {
        if owner.is_empty() {
            return Err(WasmError::new("INVALID_OWNER", "Owner address cannot be empty").into());
        }

        let registry_contract = self.registry_contract.clone();

        let promise = future_to_promise(async move {
            let names = simulate_get_names_by_owner(&owner, &registry_contract).await?;
            let result = rust_to_js(&names)?;
            Ok(result.into())
        });

        Ok(promise)
    }

    /// Renew a name registration
    #[wasm_bindgen]
    pub fn renew_name(&self, name: String, years: u32) -> Result<Promise, JsValue> {
        if name.is_empty() {
            return Err(WasmError::new("INVALID_NAME", "Name cannot be empty").into());
        }

        if years == 0 || years > 10 {
            return Err(WasmError::new("INVALID_DURATION", "Renewal duration must be between 1 and 10 years").into());
        }

        let registry_contract = self.registry_contract.clone();

        let promise = future_to_promise(async move {
            let result = simulate_name_renewal(&name, years, &registry_contract).await?;
            let result = rust_to_js(&result)?;
            Ok(result.into())
        });

        Ok(promise)
    }

    /// Transfer name ownership
    #[wasm_bindgen]
    pub fn transfer_name(&self, name: String, new_owner: String) -> Result<Promise, JsValue> {
        if name.is_empty() {
            return Err(WasmError::new("INVALID_NAME", "Name cannot be empty").into());
        }

        if new_owner.is_empty() {
            return Err(WasmError::new("INVALID_NEW_OWNER", "New owner address cannot be empty").into());
        }

        let registry_contract = self.registry_contract.clone();

        let promise = future_to_promise(async move {
            let result = simulate_name_transfer(&name, &new_owner, &registry_contract).await?;
            let result = rust_to_js(&result)?;
            Ok(result.into())
        });

        Ok(promise)
    }

    /// Update name data
    #[wasm_bindgen]
    pub fn update_name_data(&self, name: String, data: String) -> Result<Promise, JsValue> {
        if name.is_empty() {
            return Err(WasmError::new("INVALID_NAME", "Name cannot be empty").into());
        }

        let registry_contract = self.registry_contract.clone();

        let promise = future_to_promise(async move {
            let result = simulate_name_data_update(&name, &data, &registry_contract).await?;
            let result = rust_to_js(&result)?;
            Ok(result.into())
        });

        Ok(promise)
    }

    /// Get registration fee
    #[wasm_bindgen]
    pub fn get_registration_fee(&self) -> String {
        self.registration_fee.clone()
    }

    /// Validate name format
    #[wasm_bindgen]
    pub fn validate_name_format_js(&self, name: String) -> Result<bool, JsValue> {
        match self.validate_name_format(&name) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Clear cache
    #[wasm_bindgen]
    pub fn clear_cache(&mut self) -> Result<(), JsValue> {
        self.cache.clear();
        Ok(())
    }

    /// Get cache statistics
    #[wasm_bindgen]
    pub fn get_cache_stats(&self) -> Result<Object, JsValue> {
        let stats = Object::new();
        let _ = Reflect::set(&stats, &"cache_size".into(), &self.cache.len().into());
        let _ = Reflect::set(&stats, &"registry_contract".into(), &self.registry_contract.into());
        let _ = Reflect::set(&stats, &"min_name_length".into(), &self.min_name_length.into());
        let _ = Reflect::set(&stats, &"max_name_length".into(), &self.max_name_length.into());

        Ok(stats)
    }
}

// Internal validation methods
impl NameResolver {
    fn validate_registration_request(&self, request: &NameRegistrationRequest) -> WasmResult<()> {
        // Validate name format
        self.validate_name_format(&request.name)?;
        
        // Validate addresses
        if request.address.len() != 42 || !request.address.starts_with("0x") {
            return Err(WasmError::new("INVALID_ADDRESS", "Invalid address format"));
        }
        
        if request.owner.len() != 42 || !request.owner.starts_with("0x") {
            return Err(WasmError::new("INVALID_OWNER", "Invalid owner address format"));
        }
        
        // Validate duration
        if request.duration_years == 0 || request.duration_years > 10 {
            return Err(WasmError::new("INVALID_DURATION", "Duration must be between 1 and 10 years"));
        }
        
        Ok(())
    }

    fn validate_name_format(&self, name: &str) -> WasmResult<()> {
        if name.len() < self.min_name_length {
            return Err(WasmError::new("NAME_TOO_SHORT", &format!("Name must be at least {} characters", self.min_name_length)));
        }
        
        if name.len() > self.max_name_length {
            return Err(WasmError::new("NAME_TOO_LONG", &format!("Name must be at most {} characters", self.max_name_length)));
        }
        
        // Check for valid characters (alphanumeric and hyphens only)
        if !name.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err(WasmError::new("INVALID_CHARACTERS", "Name can only contain alphanumeric characters and hyphens"));
        }
        
        // Check for invalid patterns
        if name.starts_with('-') || name.ends_with('-') {
            return Err(WasmError::new("INVALID_PATTERN", "Name cannot start or end with a hyphen"));
        }
        
        if name.contains("--") {
            return Err(WasmError::new("INVALID_PATTERN", "Name cannot contain consecutive hyphens"));
        }
        
        Ok(())
    }
}

// Internal simulation functions
async fn simulate_name_registration(
    request: &NameRegistrationRequest,
    _registry_contract: &str,
) -> WasmResult<NameRecord> {
    wasm_bindgen_futures::JsFuture::from(
        js_sys::Promise::resolve(&JsValue::from(100))
    ).await.unwrap();

    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Ok(NameRecord {
        name: request.name.clone(),
        address: request.address.clone(),
        owner: request.owner.clone(),
        registration_date: current_time,
        expiry_date: current_time + (request.duration_years as u64 * 365 * 24 * 60 * 60),
        status: NameStatus::Registered,
        data: request.data.clone(),
    })
}

async fn simulate_name_resolution(name: &str, _registry_contract: &str) -> WasmResult<String> {
    wasm_bindgen_futures::JsFuture::from(
        js_sys::Promise::resolve(&JsValue::from(30))
    ).await.unwrap();

    // Generate deterministic address based on name
    let name_hash = name.chars().map(|c| c as u32).sum::<u32>();
    Ok(format!("0x{:040x}", name_hash))
}

async fn simulate_reverse_lookup(address: &str, _registry_contract: &str) -> WasmResult<String> {
    wasm_bindgen_futures::JsFuture::from(
        js_sys::Promise::resolve(&JsValue::from(25))
    ).await.unwrap();

    // Generate deterministic name based on address
    let address_hash = address.chars().map(|c| c as u32).sum::<u32>();
    Ok(format!("user{}", address_hash % 10000))
}

async fn simulate_availability_check(name: &str, _registry_contract: &str) -> WasmResult<bool> {
    wasm_bindgen_futures::JsFuture::from(
        js_sys::Promise::resolve(&JsValue::from(20))
    ).await.unwrap();

    // Simulate some names being taken
    let name_hash = name.chars().map(|c| c as u32).sum::<u32>();
    Ok(name_hash % 3 != 0) // 2/3 of names are available
}

async fn simulate_get_name_record(name: &str, _registry_contract: &str) -> WasmResult<NameRecord> {
    wasm_bindgen_futures::JsFuture::from(
        js_sys::Promise::resolve(&JsValue::from(25))
    ).await.unwrap();

    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let name_hash = name.chars().map(|c| c as u32).sum::<u32>();
    
    Ok(NameRecord {
        name: name.to_string(),
        address: format!("0x{:040x}", name_hash),
        owner: format!("0x{:040x}", name_hash + 1000),
        registration_date: current_time - 86400 * 30, // Registered 30 days ago
        expiry_date: current_time + 86400 * 365, // Expires in 1 year
        status: if name_hash % 10 == 0 { NameStatus::Expired } else { NameStatus::Registered },
        data: Some(format!("Metadata for {}", name)),
    })
}

async fn simulate_name_search(
    query: &str,
    limit: u32,
    _registry_contract: &str,
) -> WasmResult<Vec<NameSearchResult>> {
    wasm_bindgen_futures::JsFuture::from(
        js_sys::Promise::resolve(&JsValue::from(50))
    ).await.unwrap();

    let mut results = Vec::new();
    let query_hash = query.chars().map(|c| c as u32).sum::<u32>();

    for i in 0..limit.min(20) {
        let name = format!("{}{}", query, i);
        let name_hash = name.chars().map(|c| c as u32).sum::<u32>();
        
        results.push(NameSearchResult {
            name,
            address: format!("0x{:040x}", name_hash),
            owner: format!("0x{:040x}", name_hash + 1000),
            registration_date: 1640995200 + i as u64 * 86400, // Start from 2022
            expiry_date: 1672531200 + i as u64 * 86400, // Start from 2023
            status: if name_hash % 5 == 0 { NameStatus::Expired } else { NameStatus::Registered },
            is_available: name_hash % 3 != 0,
        });
    }

    Ok(results)
}

async fn simulate_get_names_by_owner(owner: &str, _registry_contract: &str) -> WasmResult<Vec<String>> {
    wasm_bindgen_futures::JsFuture::from(
        js_sys::Promise::resolve(&JsValue::from(40))
    ).await.unwrap();

    let owner_hash = owner.chars().map(|c| c as u32).sum::<u32>();
    let mut names = Vec::new();

    for i in 0..(owner_hash % 5 + 1) {
        names.push(format!("name{}{}", owner_hash, i));
    }

    Ok(names)
}

async fn simulate_name_renewal(
    name: &str,
    years: u32,
    _registry_contract: &str,
) -> WasmResult<NameRecord> {
    wasm_bindgen_futures::JsFuture::from(
        js_sys::Promise::resolve(&JsValue::from(60))
    ).await.unwrap();

    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Ok(NameRecord {
        name: name.to_string(),
        address: format!("0x{:040x}", 12345),
        owner: format!("0x{:040x}", 67890),
        registration_date: current_time - 86400 * 30,
        expiry_date: current_time + (years as u64 * 365 * 24 * 60 * 60),
        status: NameStatus::Registered,
        data: None,
    })
}

async fn simulate_name_transfer(
    name: &str,
    new_owner: &str,
    _registry_contract: &str,
) -> WasmResult<NameRecord> {
    wasm_bindgen_futures::JsFuture::from(
        js_sys::Promise::resolve(&JsValue::from(50))
    ).await.unwrap();

    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Ok(NameRecord {
        name: name.to_string(),
        address: format!("0x{:040x}", 12345),
        owner: new_owner.to_string(),
        registration_date: current_time - 86400 * 30,
        expiry_date: current_time + 86400 * 365,
        status: NameStatus::Registered,
        data: None,
    })
}

async fn simulate_name_data_update(
    name: &str,
    data: &str,
    _registry_contract: &str,
) -> WasmResult<NameRecord> {
    wasm_bindgen_futures::JsFuture::from(
        js_sys::Promise::resolve(&JsValue::from(30))
    ).await.unwrap();

    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Ok(NameRecord {
        name: name.to_string(),
        address: format!("0x{:040x}", 12345),
        owner: format!("0x{:040x}", 67890),
        registration_date: current_time - 86400 * 30,
        expiry_date: current_time + 86400 * 365,
        status: NameStatus::Registered,
        data: Some(data.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_name_resolver_creation() {
        let resolver = NameResolver::new("0xCONTRACT".to_string());
        let stats = resolver.get_cache_stats().unwrap();
        
        let contract = Reflect::get(&stats, &"registry_contract".into()).unwrap().as_string().unwrap();
        assert_eq!(contract, "0xCONTRACT");
    }

    #[wasm_bindgen_test]
    fn test_name_validation() {
        let resolver = NameResolver::new("0xCONTRACT".to_string());
        
        // Valid names
        assert!(resolver.validate_name_format_js("valid-name".to_string()).unwrap());
        assert!(resolver.validate_name_format_js("user123".to_string()).unwrap());
        
        // Invalid names
        assert!(!resolver.validate_name_format_js("-invalid".to_string()).unwrap());
        assert!(!resolver.validate_name_format_js("invalid-".to_string()).unwrap());
        assert!(!resolver.validate_name_format_js("invalid--name".to_string()).unwrap());
        assert!(!resolver.validate_name_format_js("ab".to_string()).unwrap()); // Too short
    }

    #[wasm_bindgen_test]
    fn test_name_status() {
        let status = NameStatus::Registered;
        assert_eq!(status.as_str(), "registered");
        
        let expired = NameStatus::Expired;
        assert_eq!(expired.as_str(), "expired");
    }
}