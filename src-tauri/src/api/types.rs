use crate::config_store::WalletConfig;
use crate::storage::VaultMetadata;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletSummary {
    pub wallet_name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub schema_version: u16,
    #[serde(default)]
    pub primary_address: Option<String>,
    #[serde(default)]
    pub public_key_hex: Option<String>,
    #[serde(default)]
    pub signature_algorithm: Option<String>,
    #[serde(default)]
    pub supports_post_quantum: Option<bool>,
}

impl From<VaultMetadata> for WalletSummary {
    fn from(metadata: VaultMetadata) -> Self {
        Self {
            wallet_name: metadata.wallet_name,
            created_at: metadata.created_at,
            updated_at: metadata.updated_at,
            schema_version: metadata.schema_version,
            primary_address: metadata.primary_address,
            public_key_hex: metadata.public_key_hex,
            signature_algorithm: metadata.signature_algorithm,
            supports_post_quantum: metadata.supports_post_quantum,
        }
    }
}

impl From<&VaultMetadata> for WalletSummary {
    fn from(metadata: &VaultMetadata) -> Self {
        WalletSummary::from(metadata.clone())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWalletRequest {
    pub wallet_name: String,
    pub password: String,
    pub mnemonic_word_count: u32,
    pub use_post_quantum: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CastVoteRequest {
    pub proposal_id: i64,
    pub support: i32,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CastVoteResponse {
    pub vote: VoteInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegateRequest {
    pub delegatee: String,
    pub amount: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetValidatorsResponse {
    pub validators: Vec<ValidatorInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetUserDelegationsRequest {
    pub user_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetUserDelegationsResponse {
    pub delegations: Vec<DelegationInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetStakingRewardsRequest {
    pub user_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetStakingRewardsResponse {
    pub rewards: StakingRewards,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetLockboxRecordsRequest {
    pub user_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetLockboxRecordsResponse {
    pub records: Vec<LockBoxRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAutoStakeStatusRequest {
    pub user_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAutoStakeStatusResponse {
    pub status: AutoStakeRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegateTokensRequest {
    pub delegator_address: String,
    pub validator_address: String,
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegateTokensResponse {
    pub transaction_id: String,
    pub delegation: DelegationInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UndelegateTokensRequest {
    pub delegator_address: String,
    pub validator_address: String,
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UndelegateTokensResponse {
    pub transaction_id: String,
    pub updated_delegation: DelegationInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLockboxStakeRequest {
    pub account: String,
    pub amount: u64,
    pub term_months: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLockboxStakeResponse {
    pub transaction_id: String,
    pub lockbox_record: LockBoxRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToggleAutoStakingRequest {
    pub account: String,
    pub enable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToggleAutoStakingResponse {
    pub success: bool,
    pub auto_stake_status: AutoStakeRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimStakingRewardsRequest {
    pub account: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimStakingRewardsResponse {
    pub transaction_id: String,
    pub claimed_amount: u64,
}

// Staking types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatorInfo {
    pub address: String,
    pub public_key: String,
    pub network_key: String,
    pub stake: u64,
    pub stake_amount: u64,
    pub is_active: bool,
    pub commission_rate: u32,  // 0-100 percentage
    pub reputation_score: u32, // 0-100 score
    pub last_activity: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_delegated: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delegator_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StakingRewards {
    pub total_earned: u64,
    pub pending_rewards: u64,
    pub current_apy: f64, // percentage
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockBoxRecord {
    pub account: String,
    pub amount: u64,
    pub term_months: u32,
    pub locked_at: DateTime<Utc>,
    pub unlock_at: DateTime<Utc>,
    pub multiplier: f64,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoStakeRecord {
    pub account: String,
    pub balance: u64,
    pub maturity_timestamp: DateTime<Utc>,
    pub is_active: bool,
}

// Staking API request/response types

// Basic wallet types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWalletResponse {
    pub summary: WalletSummary,
    pub address: String,
    pub public_key: String,
    pub mnemonic: String,
    pub supports_post_quantum: bool,
    pub algorithm: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportWalletRequest {
    pub wallet_name: String,
    pub password: String,
    pub mnemonic: String,
    pub use_post_quantum: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportWalletResponse {
    pub summary: WalletSummary,
    pub address: String,
    pub public_key: String,
    pub supports_post_quantum: bool,
    pub algorithm: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockWalletResponse {
    pub success: bool,
    pub summary: Option<WalletSummary>,
    pub remaining_attempts: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletInfoResponse {
    pub exists: bool,
    pub is_locked: bool,
    pub remaining_attempts: u32,
    pub metadata: Option<WalletSummary>,
    pub config: WalletConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignMessageResponse {
    pub signature: String,
    pub algorithm: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordResponse {
    pub success: bool,
    pub summary: WalletSummary,
}

// Basic wallet types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportWalletResponse {
    pub summary: WalletSummary,
    pub mnemonic: Option<String>,
    pub seed_hex: String,
    #[serde(default)]
    pub stealth_material_hex: Option<String>,
    #[serde(default)]
    pub pq_material_hex: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatAmountRequest {
    pub amount: String,
    pub decimals: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatAmountResponse {
    pub formatted: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetProposalsRequest {
    pub state: Option<String>,
    pub proposer: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockWalletResponse {
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignMessageRequest {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockWalletRequest {
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateAddressRequest {
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateAddressResponse {
    pub is_valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifySignatureRequest {
    pub message: String,
    pub signature: String,
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifySignatureResponse {
    pub valid: bool,
}

// Governance types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposalSummary {
    pub proposal_id: i64,
    pub proposer: String,
    pub description: String,
    pub vote_start: i64,
    pub vote_end: i64,
    pub votes_for: i64,
    pub votes_against: i64,
    pub votes_abstain: i64,
    pub state: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposalDetail {
    pub proposal_id: i64,
    pub proposer: String,
    pub targets: Vec<String>,
    pub values: Vec<String>,
    pub calldatas: Vec<String>,
    pub description: String,
    pub vote_start: i64,
    pub vote_end: i64,
    pub votes_for: i64,
    pub votes_against: i64,
    pub votes_abstain: i64,
    pub state: String,
    pub executed_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub has_voted: Option<bool>,
    pub user_vote: Option<VoteInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoteInfo {
    pub proposal_id: i64,
    pub voter: String,
    pub support: i32,
    pub weight: i64,
    pub reason: Option<String>,
    pub voted_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VotingPowerInfo {
    pub address: String,
    pub voting_power: i64,
    pub delegated_power: i64,
    pub total_power: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegationInfo {
    pub delegator: String,
    pub delegatee: String,
    pub amount: i64,
    pub delegated_at: i64,
}

// Missing basic types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceResponse {
    pub address: String,
    pub balance: String,
    pub nonce: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionInfo {
    pub transaction_id: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: String,
    pub fee: String,
    pub status: String,
    pub timestamp: String,
    pub block_height: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionHistoryResponse {
    pub transactions: Vec<TransactionInfo>,
    pub total_count: u64,
}

// Governance response types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetProposalsResponse {
    pub proposals: Vec<ProposalSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetProposalResponse {
    pub proposal: ProposalDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetProposalVotesResponse {
    pub votes: Vec<VoteInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetVotingPowerResponse {
    pub voting_power: VotingPowerInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDelegationsResponse {
    pub delegations: Vec<DelegationInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegateResponse {
    pub delegation: DelegationInfo,
}

// Biometric authentication types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiometricStatusResponse {
    pub available: bool,
    pub enrolled: bool,
    pub supported_types: Vec<String>, // "fingerprint", "face", "iris", "voice"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiometricUnlockRequest {
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiometricUnlockResponse {
    pub success: bool,
    pub token: Option<String>, // Temporary auth token
}

// Push notification types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushNotificationStatusResponse {
    pub available: bool,
    pub enabled: bool,
    pub permission_granted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushNotificationRegisterRequest {
    pub token: String, // Device token for push notifications
    pub enable_transaction_alerts: bool,
    pub enable_staking_alerts: bool,
    pub enable_governance_alerts: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushNotificationRegisterResponse {
    pub success: bool,
    pub registration_id: Option<String>,
}

// Passkey/WebAuthn types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasskeyCreateRequest {
    pub challenge: String,
    pub user_id: String,
    pub user_name: String,
    pub relying_party_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasskeyCreateResponse {
    pub credential_id: String,
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasskeyAuthenticateRequest {
    pub challenge: String,
    pub credential_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasskeyAuthenticateResponse {
    pub credential_id: String,
    pub authenticator_data: String,
    pub client_data_json: String,
    pub signature: String,
}
