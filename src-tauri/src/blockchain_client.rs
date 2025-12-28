/// Blockchain RPC client for communicating with Silica nodes
///
/// This module provides HTTP-based JSON-RPC communication with Chert blockchain nodes,
/// implementing the methods needed for wallet functionality.
use crate::api::types::{
    BalanceResponse, CastVoteRequest, CastVoteResponse, ClaimStakingRewardsResponse,
    CreateLockboxStakeResponse, DelegateRequest, DelegateResponse, DelegateTokensResponse,
    GetAutoStakeStatusResponse, GetDelegationsResponse, GetLockboxRecordsResponse,
    GetProposalResponse, GetProposalVotesResponse, GetProposalsResponse, GetStakingRewardsResponse,
    GetUserDelegationsResponse, GetValidatorsResponse, GetVotingPowerResponse,
    ToggleAutoStakingResponse, TransactionHistoryResponse, TransactionInfo,
    UndelegateTokensResponse,
};
use crate::errors::{WalletError, WalletResult};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// HTTP client for blockchain RPC communication
pub struct BlockchainClient {
    client: Client,
    base_url: String,
}

/// JSON-RPC request structure
#[derive(Debug, Serialize)]
struct JsonRpcRequest<T: Serialize> {
    jsonrpc: String,
    method: String,
    params: T,
    id: u64,
}

/// JSON-RPC response structure
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // fields are populated via serde; not all are read by all call sites
struct JsonRpcResponse<T> {
    jsonrpc: String,
    result: Option<T>,
    error: Option<JsonRpcError>,
    id: u64,
}

/// JSON-RPC error structure
#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

impl BlockchainClient {
    /// Create a new blockchain client
    pub fn new(base_url: String) -> WalletResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| {
                WalletError::NetworkError(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(BlockchainClient {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }

    /// Get account balance
    pub async fn get_balance(&self, address: &str) -> WalletResult<BalanceResponse> {
        let params = serde_json::json!({ "address": address });
        let response = self.rpc_call("get_balance", params).await?;
        Ok(response)
    }

    /// Get transaction by ID
    pub async fn get_transaction(&self, tx_id: &str) -> WalletResult<TransactionInfo> {
        let params = serde_json::json!({ "tx_id": tx_id });
        let response = self.rpc_call("get_transaction", params).await?;
        Ok(response)
    }

    /// Get transaction history for an address
    pub async fn get_transaction_history(
        &self,
        address: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> WalletResult<TransactionHistoryResponse> {
        let params = serde_json::json!({
            "address": address,
            "limit": limit,
            "offset": offset
        });

        let response = self.rpc_call("get_transaction_history", params).await?;
        Ok(response)
    }

    /// Send a transaction
    pub async fn send_transaction(&self, tx_data: serde_json::Value) -> WalletResult<String> {
        let params = serde_json::json!({ "transaction": tx_data });
        let response: serde_json::Value = self.rpc_call("send_transaction", params).await?;
        let tx_id = response
            .get("tx_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| WalletError::NetworkError("Invalid transaction response".to_string()))?;

        Ok(tx_id.to_string())
    }

    /// Get current gas price
    pub async fn get_gas_price(&self) -> WalletResult<serde_json::Value> {
        let response = self
            .rpc_call("eth_gasPrice", serde_json::Value::Null)
            .await?;
        Ok(response)
    }

    /// Get current block number
    pub async fn get_block_number(&self) -> WalletResult<u64> {
        let response: serde_json::Value = self
            .rpc_call("eth_blockNumber", serde_json::Value::Null)
            .await?;
        let block_num_str = response
            .get("block_number")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                WalletError::NetworkError("Invalid block number response".to_string())
            })?;

        let block_num = if let Some(stripped) = block_num_str.strip_prefix("0x") {
            u64::from_str_radix(stripped, 16)
        } else {
            block_num_str.parse()
        }
        .map_err(|_| WalletError::NetworkError("Invalid block number format".to_string()))?;

        Ok(block_num)
    }

    /// Get list of validators
    pub async fn get_validators(&self) -> WalletResult<GetValidatorsResponse> {
        let response = self
            .rpc_call("staking_get_validators", serde_json::Value::Null)
            .await?;
        Ok(response)
    }

    /// Get user delegations
    pub async fn get_user_delegations(
        &self,
        address: &str,
    ) -> WalletResult<GetUserDelegationsResponse> {
        let params = serde_json::json!({ "address": address });
        let response = self
            .rpc_call("staking_get_user_delegations", params)
            .await?;
        Ok(response)
    }

    /// Get staking rewards
    pub async fn get_staking_rewards(
        &self,
        address: &str,
    ) -> WalletResult<GetStakingRewardsResponse> {
        let params = serde_json::json!({ "address": address });
        let response = self.rpc_call("staking_get_rewards", params).await?;
        Ok(response)
    }

    /// Get lockbox records
    pub async fn get_lockbox_records(
        &self,
        address: &str,
    ) -> WalletResult<GetLockboxRecordsResponse> {
        let params = serde_json::json!({ "account": address });
        let response = self.rpc_call("staking_get_lockbox_records", params).await?;
        Ok(response)
    }

    /// Get auto-stake status
    pub async fn get_auto_stake_status(
        &self,
        address: &str,
    ) -> WalletResult<GetAutoStakeStatusResponse> {
        let params = serde_json::json!({ "account": address });
        let response = self
            .rpc_call("staking_get_auto_stake_status", params)
            .await?;
        Ok(response)
    }

    /// Delegate tokens to validator
    pub async fn delegate_tokens(
        &self,
        delegator: &str,
        validator: &str,
        amount: u64,
    ) -> WalletResult<DelegateTokensResponse> {
        let params = serde_json::json!({
            "delegator": delegator,
            "validator": validator,
            "amount": amount
        });
        let response = self.rpc_call("staking_delegate_tokens", params).await?;
        Ok(response)
    }

    /// Undelegate tokens from validator
    pub async fn undelegate_tokens(
        &self,
        delegator: &str,
        validator: &str,
        amount: u64,
    ) -> WalletResult<UndelegateTokensResponse> {
        let params = serde_json::json!({
            "delegator": delegator,
            "validator": validator,
            "amount": amount
        });
        let response = self.rpc_call("staking_undelegate_tokens", params).await?;
        Ok(response)
    }

    /// Create lockbox stake
    pub async fn create_lockbox_stake(
        &self,
        account: &str,
        amount: u64,
        term_months: u32,
    ) -> WalletResult<CreateLockboxStakeResponse> {
        let params = serde_json::json!({
            "account": account,
            "amount": amount,
            "termMonths": term_months
        });
        let response = self
            .rpc_call("staking_create_lockbox_stake", params)
            .await?;
        Ok(response)
    }

    /// Toggle auto-staking
    pub async fn toggle_auto_staking(
        &self,
        account: &str,
        enable: bool,
    ) -> WalletResult<ToggleAutoStakingResponse> {
        let params = serde_json::json!({
            "account": account,
            "enable": enable
        });
        let response = self.rpc_call("staking_toggle_auto_staking", params).await?;
        Ok(response)
    }

    /// Claim staking rewards
    pub async fn claim_staking_rewards(
        &self,
        account: &str,
    ) -> WalletResult<ClaimStakingRewardsResponse> {
        let params = serde_json::json!({ "account": account });
        let response = self.rpc_call("staking_claim_rewards", params).await?;
        Ok(response)
    }

    /// Get governance proposals
    pub async fn get_proposals(
        &self,
        request: Option<serde_json::Value>,
    ) -> WalletResult<GetProposalsResponse> {
        let params = request.unwrap_or(serde_json::Value::Null);
        let response: GetProposalsResponse =
            self.rpc_call("governance_list_proposals", params).await?;
        Ok(response)
    }

    /// Get specific proposal details
    pub async fn get_proposal(&self, proposal_id: i64) -> WalletResult<GetProposalResponse> {
        let params = serde_json::json!({
            "proposal_id": proposal_id
        });
        let response: GetProposalResponse =
            self.rpc_call("governance_get_proposal", params).await?;
        Ok(response)
    }

    /// Get proposal votes
    pub async fn get_proposal_votes(
        &self,
        proposal_id: i64,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> WalletResult<GetProposalVotesResponse> {
        let params = serde_json::json!({
            "proposal_id": proposal_id,
            "limit": limit,
            "offset": offset
        });
        let response: GetProposalVotesResponse = self
            .rpc_call("governance_get_proposal_votes", params)
            .await?;
        Ok(response)
    }

    /// Get voting power for address
    pub async fn get_voting_power(&self, address: &str) -> WalletResult<GetVotingPowerResponse> {
        let params = serde_json::json!({
            "address": address
        });
        let response: GetVotingPowerResponse =
            self.rpc_call("governance_get_voting_power", params).await?;
        Ok(response)
    }

    /// Get delegations for address
    pub async fn get_delegations(&self, address: &str) -> WalletResult<GetDelegationsResponse> {
        let params = serde_json::json!({
            "address": address
        });
        let response: GetDelegationsResponse =
            self.rpc_call("governance_get_delegations", params).await?;
        Ok(response)
    }

    /// Cast vote on proposal
    pub async fn cast_vote(&self, request: CastVoteRequest) -> WalletResult<CastVoteResponse> {
        let params = serde_json::json!(request);
        let response: CastVoteResponse = self.rpc_call("governance_cast_vote", params).await?;
        Ok(response)
    }

    /// Delegate voting power
    pub async fn delegate(&self, request: DelegateRequest) -> WalletResult<DelegateResponse> {
        let params = serde_json::json!(request);
        let response = self
            .rpc_call("governance_delegate_voting_power", params)
            .await?;
        Ok(response)
    }

    /// Make a JSON-RPC call to the node
    async fn rpc_call<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> WalletResult<T> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: 1,
        };

        let url = format!("{}/jsonrpc", self.base_url);
        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| WalletError::NetworkError(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(WalletError::NetworkError(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        let rpc_response: JsonRpcResponse<T> = response
            .json()
            .await
            .map_err(|e| WalletError::NetworkError(format!("Failed to parse response: {}", e)))?;

        if let Some(error) = rpc_response.error {
            return Err(WalletError::NetworkError(format!(
                "RPC error {}: {}",
                error.code, error.message
            )));
        }

        rpc_response
            .result
            .ok_or_else(|| WalletError::NetworkError("No result in RPC response".to_string()))
    }
}

impl Default for BlockchainClient {
    fn default() -> Self {
        Self::new("http://localhost:8545".to_string()).unwrap()
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires running RPC server at localhost:8545"]
    async fn test_real_balance_call() {
        let client = BlockchainClient::new("http://localhost:8545".to_string()).unwrap();
        let result = client.get_balance("test_address").await;
        assert!(result.is_ok(), "Balance call should succeed");
    }

    #[tokio::test]
    #[ignore = "requires running RPC server at localhost:8545"]
    async fn test_real_validators_call() {
        let client = BlockchainClient::new("http://localhost:8545".to_string()).unwrap();
        let result = client.get_validators().await;
        assert!(result.is_ok(), "Validators call should succeed");
    }

    #[tokio::test]
    #[ignore = "requires running RPC server at localhost:8545"]
    async fn test_real_staking_calls() {
        let client = BlockchainClient::new("http://localhost:8545".to_string()).unwrap();

        let delegations_result = client.get_user_delegations("test_address").await;
        assert!(
            delegations_result.is_ok(),
            "Delegations call should succeed"
        );

        let rewards_result = client.get_staking_rewards("test_address").await;
        assert!(
            rewards_result.is_ok(),
            "Staking rewards call should succeed"
        );

        let auto_stake_result = client.get_auto_stake_status("test_address").await;
        assert!(
            auto_stake_result.is_ok(),
            "Auto-stake status call should succeed"
        );
    }
}
