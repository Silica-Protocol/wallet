// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code)] // wallet exposes a broad API surface to the frontend; not all items are referenced in Rust yet

mod api;
mod app_state;
mod blockchain;
mod blockchain_client;
mod config_store;
mod crypto;
mod errors;
mod runtime;
mod security;
mod session;
mod storage;
mod validation;

use crate::api::types::{
    BalanceResponse, BiometricStatusResponse, BiometricUnlockRequest, BiometricUnlockResponse,
    CastVoteRequest, CastVoteResponse, ChangePasswordRequest, ChangePasswordResponse,
    ClaimStakingRewardsResponse, CreateLockboxStakeResponse, CreateWalletRequest,
    CreateWalletResponse, DelegateRequest, DelegateResponse, DelegateTokensResponse,
    ExportWalletResponse, FormatAmountRequest, FormatAmountResponse, GetAutoStakeStatusResponse,
    GetDelegationsResponse, GetLockboxRecordsResponse, GetProposalResponse,
    GetProposalVotesResponse, GetProposalsRequest, GetProposalsResponse, GetStakingRewardsResponse,
    GetUserDelegationsResponse, GetValidatorsResponse, GetVotingPowerResponse, ImportWalletRequest,
    ImportWalletResponse, LockWalletResponse, PasskeyAuthenticateRequest,
    PasskeyAuthenticateResponse, PasskeyCreateRequest, PasskeyCreateResponse,
    PushNotificationRegisterRequest, PushNotificationRegisterResponse,
    PushNotificationStatusResponse, SignMessageRequest, SignMessageResponse,
    ToggleAutoStakingResponse, TransactionHistoryResponse, UndelegateTokensResponse,
    UnlockWalletRequest, UnlockWalletResponse, ValidateAddressRequest, ValidateAddressResponse,
    VerifySignatureRequest, VerifySignatureResponse, WalletInfoResponse, WalletSummary,
};
use crate::app_state::{SharedWalletContext, WalletContext};
use crate::blockchain::{Address, Amount};
use crate::blockchain_client::BlockchainClient;
use crate::crypto::{StealthKeyMaterial, WalletKeyPair};
use crate::errors::WalletError;
use crate::runtime::RuntimeSecurityState;
use crate::security::init_security_config_from_env;
use crate::storage::{VaultMetadata, VaultSecrets};
use crate::validation::InputValidator;
use ed25519_dalek::Signer;
use secrecy::SecretString;
use silica_models::crypto::{verify_signature_standalone, ChertSignature, SignatureAlgorithm};
use tauri::Manager;
use tauri::State;

fn to_frontend_error(err: WalletError) -> String {
    err.to_string()
}

#[tauri::command]
fn create_wallet(
    state: State<'_, SharedWalletContext>,
    request: CreateWalletRequest,
) -> Result<CreateWalletResponse, String> {
    let CreateWalletRequest {
        wallet_name,
        password,
        mnemonic_word_count,
        use_post_quantum,
    } = request;

    let validator = InputValidator::default();
    validator
        .validate_wallet_name(&wallet_name)
        .map_err(|e| e.to_string())?;
    validator
        .validate_password(&password)
        .map_err(|e| e.to_string())?;

    let password_secret = SecretString::from(password);
    let response = state
        .write(|ctx| {
            if ctx.vault().exists() {
                return Err(WalletError::AlreadyExists(
                    ctx.vault().vault_path().display().to_string(),
                ));
            }

            let (keypair, mnemonic) = WalletKeyPair::generate_with_mnemonic(
                mnemonic_word_count,
                None,
                None,
                use_post_quantum,
            )?;

            let stealth_keys =
                StealthKeyMaterial::derive_from_seed(&keypair.core_keypair.private_key)?;

            let secrets = VaultSecrets {
                mnemonic_phrase: Some(mnemonic.clone()),
                seed_bytes: keypair.core_keypair.private_key.clone(),
                stealth_material: stealth_keys.encode(),
                pq_material: Vec::new(),
            };

            let mut metadata = VaultMetadata::new(&wallet_name);
            metadata.primary_address = Some(keypair.address());
            metadata.public_key_hex = Some(keypair.public_key_hex());
            metadata.signature_algorithm = Some(format!("{:?}", keypair.core_keypair.algorithm));
            metadata.supports_post_quantum = Some(keypair.supports_pq);

            ctx.create_vault(&password_secret, metadata, secrets)?;
            ctx.unlock(&password_secret)?;

            let metadata = ctx
                .session()
                .peek_unlocked(|metadata, _| Ok(metadata.clone()))?;
            Ok(CreateWalletResponse {
                summary: WalletSummary::from(metadata),
                address: keypair.address(),
                public_key: keypair.public_key_hex(),
                mnemonic,
                supports_post_quantum: keypair.supports_pq,
                algorithm: format!("{:?}", keypair.core_keypair.algorithm),
            })
        })
        .map_err(to_frontend_error)?;

    Ok(response)
}

#[tauri::command]
fn import_wallet(
    state: State<'_, SharedWalletContext>,
    request: ImportWalletRequest,
) -> Result<ImportWalletResponse, String> {
    let ImportWalletRequest {
        wallet_name,
        password,
        mnemonic,
        use_post_quantum,
    } = request;

    let validator = InputValidator::default();
    validator
        .validate_wallet_name(&wallet_name)
        .map_err(|e| e.to_string())?;
    validator
        .validate_password(&password)
        .map_err(|e| e.to_string())?;

    let password_secret = SecretString::from(password);
    state
        .write(|ctx| {
            if ctx.vault().exists() {
                return Err(WalletError::AlreadyExists(
                    ctx.vault().vault_path().display().to_string(),
                ));
            }

            let keypair = WalletKeyPair::from_mnemonic(&mnemonic, None, None, use_post_quantum)?;
            let stealth_keys =
                StealthKeyMaterial::derive_from_seed(&keypair.core_keypair.private_key)?;
            let secrets = VaultSecrets {
                mnemonic_phrase: Some(mnemonic.clone()),
                seed_bytes: keypair.core_keypair.private_key.clone(),
                stealth_material: stealth_keys.encode(),
                pq_material: Vec::new(),
            };

            let mut metadata = VaultMetadata::new(&wallet_name);
            metadata.primary_address = Some(keypair.address());
            metadata.public_key_hex = Some(keypair.public_key_hex());
            metadata.signature_algorithm = Some(format!("{:?}", keypair.core_keypair.algorithm));
            metadata.supports_post_quantum = Some(keypair.supports_pq);

            ctx.create_vault(&password_secret, metadata, secrets)?;
            ctx.unlock(&password_secret)?;

            let metadata = ctx
                .session()
                .peek_unlocked(|metadata, _| Ok(metadata.clone()))?;
            Ok(ImportWalletResponse {
                summary: WalletSummary::from(metadata),
                address: keypair.address(),
                public_key: keypair.public_key_hex(),
                supports_post_quantum: keypair.supports_pq,
                algorithm: format!("{:?}", keypair.core_keypair.algorithm),
            })
        })
        .map_err(to_frontend_error)
}

#[tauri::command]
fn unlock_wallet(
    state: State<'_, SharedWalletContext>,
    request: UnlockWalletRequest,
) -> Result<UnlockWalletResponse, String> {
    let password_secret = SecretString::from(request.password);
    state
        .write(|ctx| {
            if !ctx.vault().exists() {
                return Err(WalletError::NotFound(
                    "Wallet vault has not been initialized".to_string(),
                ));
            }

            match ctx.unlock(&password_secret) {
                Ok(()) => {
                    let metadata = ctx
                        .session()
                        .peek_unlocked(|metadata, _| Ok(metadata.clone()))?;
                    Ok(UnlockWalletResponse {
                        success: true,
                        summary: Some(WalletSummary::from(metadata)),
                        remaining_attempts: Some(ctx.session().remaining_attempts()),
                    })
                }
                Err(err) => {
                    if matches!(
                        err,
                        WalletError::CryptoError(_) | WalletError::ValidationError(_)
                    ) {
                        ctx.session().register_failed_attempt()?;
                    }
                    Err(err)
                }
            }
        })
        .map_err(to_frontend_error)
}

#[tauri::command]
fn lock_wallet(state: State<'_, SharedWalletContext>) -> Result<LockWalletResponse, String> {
    state
        .write(|ctx| {
            ctx.lock();
            Ok(LockWalletResponse { locked: true })
        })
        .map_err(to_frontend_error)
}

#[tauri::command]
fn get_wallet_info(state: State<'_, SharedWalletContext>) -> Result<WalletInfoResponse, String> {
    state
        .read(|ctx| {
            let exists = ctx.vault().exists();
            let is_locked = ctx.session().is_locked();
            let remaining_attempts = ctx.session().remaining_attempts();
            let metadata = if exists {
                if is_locked {
                    ctx.vault().read_metadata()?.map(WalletSummary::from)
                } else {
                    let meta = ctx
                        .session()
                        .peek_unlocked(|metadata, _| Ok(metadata.clone()))?;
                    Some(WalletSummary::from(meta))
                }
            } else {
                None
            };
            let config = ctx.load_config()?;

            Ok(WalletInfoResponse {
                exists,
                is_locked,
                remaining_attempts,
                metadata,
                config,
            })
        })
        .map_err(to_frontend_error)
}

#[tauri::command]
fn export_wallet(state: State<'_, SharedWalletContext>) -> Result<ExportWalletResponse, String> {
    state
        .read(|ctx| {
            ctx.session().with_unlocked(|metadata, secrets| {
                let seed_hex = hex::encode(&secrets.seed_bytes);
                let stealth_hex = if secrets.stealth_material.is_empty() {
                    None
                } else {
                    Some(hex::encode(&secrets.stealth_material))
                };
                let pq_hex = if secrets.pq_material.is_empty() {
                    None
                } else {
                    Some(hex::encode(&secrets.pq_material))
                };

                Ok(ExportWalletResponse {
                    summary: WalletSummary::from(metadata),
                    mnemonic: secrets.mnemonic_phrase.clone(),
                    seed_hex,
                    stealth_material_hex: stealth_hex,
                    pq_material_hex: pq_hex,
                })
            })
        })
        .map_err(to_frontend_error)
}

#[tauri::command]
fn change_password(
    state: State<'_, SharedWalletContext>,
    request: ChangePasswordRequest,
) -> Result<ChangePasswordResponse, String> {
    let validator = InputValidator::default();
    validator
        .validate_password(&request.new_password)
        .map_err(|e| e.to_string())?;

    let current_secret = SecretString::from(request.old_password.clone());
    let new_secret = SecretString::from(request.new_password);

    state
        .write(|ctx| {
            if !ctx.vault().exists() {
                return Err(WalletError::NotFound(
                    "Wallet vault has not been initialized".to_string(),
                ));
            }

            if let Err(err) = ctx.vault().change_password(&current_secret, &new_secret) {
                if matches!(
                    err,
                    WalletError::CryptoError(_) | WalletError::ValidationError(_)
                ) {
                    ctx.session().register_failed_attempt()?;
                }
                return Err(err);
            }

            ctx.unlock(&new_secret)?;
            let metadata = ctx
                .session()
                .peek_unlocked(|metadata, _| Ok(metadata.clone()))?;
            Ok(ChangePasswordResponse {
                success: true,
                summary: WalletSummary::from(metadata),
            })
        })
        .map_err(to_frontend_error)
}

#[tauri::command]
fn sign_message(
    state: State<'_, SharedWalletContext>,
    request: SignMessageRequest,
) -> Result<SignMessageResponse, String> {
    state
        .read(|ctx| {
            ctx.session().with_unlocked(|_, secrets| {
                if secrets.seed_bytes.len() != 32 {
                    return Err(WalletError::CryptoError(
                        "Unsupported key material for signing".to_string(),
                    ));
                }

                let signing_bytes: [u8; 32] =
                    secrets.seed_bytes.clone().try_into().map_err(|_| {
                        WalletError::CryptoError("Failed to load signing key".to_string())
                    })?;

                let signing_key = ed25519_dalek::SigningKey::from_bytes(&signing_bytes);
                let signature = signing_key.sign(request.message.as_bytes());

                Ok(SignMessageResponse {
                    signature: hex::encode(signature.to_bytes()),
                    algorithm: "Ed25519".to_string(),
                })
            })
        })
        .map_err(to_frontend_error)
}

#[tauri::command]
fn verify_message_signature(
    _state: State<'_, SharedWalletContext>,
    request: VerifySignatureRequest,
) -> Result<VerifySignatureResponse, String> {
    let signature_bytes =
        hex::decode(&request.signature).map_err(|e| format!("Invalid signature hex: {}", e))?;
    let public_key_bytes =
        hex::decode(&request.public_key).map_err(|e| format!("Invalid public key hex: {}", e))?;

    let signature = ChertSignature {
        algorithm: SignatureAlgorithm::Ed25519,
        signature: signature_bytes,
        public_key: public_key_bytes,
    };

    let valid = verify_signature_standalone(request.message.as_bytes(), &signature)
        .map_err(|e| e.to_string())?;
    Ok(VerifySignatureResponse { valid })
}

#[tauri::command]
fn validate_address(
    _state: State<'_, SharedWalletContext>,
    request: ValidateAddressRequest,
) -> Result<ValidateAddressResponse, String> {
    let validator = InputValidator::default();
    if validator.validate_address(&request.address).is_err() {
        return Ok(ValidateAddressResponse { is_valid: false });
    }

    let valid = Address::from_string(&request.address).is_ok();
    Ok(ValidateAddressResponse { is_valid: valid })
}

#[tauri::command]
fn format_amount(
    _state: State<'_, SharedWalletContext>,
    request: FormatAmountRequest,
) -> Result<FormatAmountResponse, String> {
    let validator = InputValidator::default();
    validator
        .validate_amount(&request.amount)
        .map_err(|e| e.to_string())?;

    let amount = Amount::from_string(&request.amount).map_err(|e| e.to_string())?;

    Ok(FormatAmountResponse {
        formatted: amount.to_display_string(request.decimals.unwrap_or(8) as u8),
    })
}

fn resolve_rpc_endpoints(
    state: &State<'_, SharedWalletContext>,
    override_url: Option<String>,
) -> Result<Vec<String>, String> {
    if let Some(url) = override_url {
        let trimmed = url.trim().to_string();
        return if trimmed.is_empty() {
            Err("Override RPC endpoint cannot be empty".to_string())
        } else {
            Ok(vec![trimmed])
        };
    }

    let mut endpoints = state
        .read(|ctx| {
            let config = ctx.load_config()?;
            let mut resolved = Vec::with_capacity(1 + config.network.failover_endpoints.len());
            resolved.push(config.network.primary_endpoint);
            resolved.extend(config.network.failover_endpoints);
            Ok::<Vec<String>, WalletError>(resolved)
        })
        .map_err(to_frontend_error)?;

    endpoints.retain(|url| !url.trim().is_empty());
    if endpoints.is_empty() {
        return Err("No RPC endpoints configured".to_string());
    }

    for endpoint in &mut endpoints {
        *endpoint = endpoint.trim().to_string();
    }

    Ok(endpoints)
}

#[tauri::command]
async fn get_balance(
    state: State<'_, SharedWalletContext>,
    address: String,
    node_url: Option<String>,
) -> Result<BalanceResponse, String> {
    let validator = InputValidator::default();
    validator
        .validate_address(&address)
        .map_err(|e| e.to_string())?;

    let endpoints = resolve_rpc_endpoints(&state, node_url)?;
    assert!(!endpoints.is_empty(), "RPC endpoint list must not be empty");

    let mut last_error: Option<String> = None;
    for endpoint in endpoints {
        let client = match BlockchainClient::new(endpoint) {
            Ok(client) => client,
            Err(err) => {
                last_error = Some(to_frontend_error(err));
                continue;
            }
        };

        match client.get_balance(&address).await {
            Ok(result) => return Ok(result),
            Err(err) => {
                last_error = Some(to_frontend_error(err));
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "Failed to fetch balance from all RPC endpoints".to_string()))
}

#[tauri::command]
async fn get_transaction_history(
    state: State<'_, SharedWalletContext>,
    address: String,
    limit: Option<u32>,
    offset: Option<u32>,
    node_url: Option<String>,
) -> Result<TransactionHistoryResponse, String> {
    let validator = InputValidator::default();
    validator
        .validate_address(&address)
        .map_err(|e| e.to_string())?;

    let endpoints = resolve_rpc_endpoints(&state, node_url)?;
    assert!(!endpoints.is_empty(), "RPC endpoint list must not be empty");

    let mut last_error: Option<String> = None;
    for endpoint in endpoints {
        let client = match BlockchainClient::new(endpoint) {
            Ok(client) => client,
            Err(err) => {
                last_error = Some(to_frontend_error(err));
                continue;
            }
        };

        match client
            .get_transaction_history(&address, limit, offset)
            .await
        {
            Ok(result) => return Ok(result),
            Err(err) => {
                last_error = Some(to_frontend_error(err));
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        "Failed to fetch transaction history from all RPC endpoints".to_string()
    }))
}

// Staking commands
#[tauri::command]
async fn get_validators(
    _state: State<'_, SharedWalletContext>,
) -> Result<GetValidatorsResponse, String> {
    let client = BlockchainClient::default();
    client.get_validators().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_user_delegations(
    _state: State<'_, SharedWalletContext>,
    request: serde_json::Value,
) -> Result<GetUserDelegationsResponse, String> {
    let user_address = request
        .get("userAddress")
        .and_then(|v| v.as_str())
        .ok_or("Missing userAddress parameter")?;

    let client = BlockchainClient::default();
    client
        .get_user_delegations(user_address)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_staking_rewards(
    _state: State<'_, SharedWalletContext>,
    request: serde_json::Value,
) -> Result<GetStakingRewardsResponse, String> {
    let user_address = request
        .get("userAddress")
        .and_then(|v| v.as_str())
        .ok_or("Missing userAddress parameter")?;

    let client = BlockchainClient::default();
    client
        .get_staking_rewards(user_address)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_lockbox_records(
    _state: State<'_, SharedWalletContext>,
    request: serde_json::Value,
) -> Result<GetLockboxRecordsResponse, String> {
    let user_address = request
        .get("userAddress")
        .and_then(|v| v.as_str())
        .ok_or("Missing userAddress parameter")?;

    let client = BlockchainClient::default();
    client
        .get_lockbox_records(user_address)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_auto_stake_status(
    _state: State<'_, SharedWalletContext>,
    request: serde_json::Value,
) -> Result<GetAutoStakeStatusResponse, String> {
    let user_address = request
        .get("userAddress")
        .and_then(|v| v.as_str())
        .ok_or("Missing userAddress parameter")?;

    let client = BlockchainClient::default();
    client
        .get_auto_stake_status(user_address)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delegate_tokens(
    _state: State<'_, SharedWalletContext>,
    request: serde_json::Value,
) -> Result<DelegateTokensResponse, String> {
    let delegator_address = request
        .get("delegatorAddress")
        .and_then(|v| v.as_str())
        .ok_or("Missing delegatorAddress parameter")?;
    let validator_address = request
        .get("validatorAddress")
        .and_then(|v| v.as_str())
        .ok_or("Missing validatorAddress parameter")?;
    let amount = request
        .get("amount")
        .and_then(|v| v.as_u64())
        .ok_or("Missing or invalid amount parameter")?;

    let client = BlockchainClient::default();
    client
        .delegate_tokens(delegator_address, validator_address, amount)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn undelegate_tokens(
    _state: State<'_, SharedWalletContext>,
    request: serde_json::Value,
) -> Result<UndelegateTokensResponse, String> {
    let delegator_address = request
        .get("delegatorAddress")
        .and_then(|v| v.as_str())
        .ok_or("Missing delegatorAddress parameter")?;
    let validator_address = request
        .get("validatorAddress")
        .and_then(|v| v.as_str())
        .ok_or("Missing validatorAddress parameter")?;
    let amount = request
        .get("amount")
        .and_then(|v| v.as_u64())
        .ok_or("Missing or invalid amount parameter")?;

    let client = BlockchainClient::default();
    client
        .undelegate_tokens(delegator_address, validator_address, amount)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_lockbox_stake(
    _state: State<'_, SharedWalletContext>,
    request: serde_json::Value,
) -> Result<CreateLockboxStakeResponse, String> {
    let account = request
        .get("account")
        .and_then(|v| v.as_str())
        .ok_or("Missing account parameter")?;
    let amount = request
        .get("amount")
        .and_then(|v| v.as_u64())
        .ok_or("Missing or invalid amount parameter")?;
    let term_months = request
        .get("termMonths")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .ok_or("Missing or invalid termMonths parameter")?;

    let client = BlockchainClient::default();
    client
        .create_lockbox_stake(account, amount, term_months)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn toggle_auto_staking(
    _state: State<'_, SharedWalletContext>,
    request: serde_json::Value,
) -> Result<ToggleAutoStakingResponse, String> {
    let account = request
        .get("account")
        .and_then(|v| v.as_str())
        .ok_or("Missing account parameter")?;
    let enable = request
        .get("enable")
        .and_then(|v| v.as_bool())
        .ok_or("Missing or invalid enable parameter")?;

    let client = BlockchainClient::default();
    client
        .toggle_auto_staking(account, enable)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn claim_staking_rewards(
    _state: State<'_, SharedWalletContext>,
    request: serde_json::Value,
) -> Result<ClaimStakingRewardsResponse, String> {
    let account = request
        .get("account")
        .and_then(|v| v.as_str())
        .ok_or("Missing account parameter")?;

    let client = BlockchainClient::default();
    client
        .claim_staking_rewards(account)
        .await
        .map_err(|e| e.to_string())
}

// Biometric and push notification commands
#[tauri::command]
async fn get_biometric_status(
    security_state: State<'_, RuntimeSecurityState>,
) -> Result<BiometricStatusResponse, String> {
    Ok(security_state.biometric_status())
}

#[tauri::command]
async fn authenticate_biometric(
    security_state: State<'_, RuntimeSecurityState>,
    request: BiometricUnlockRequest,
) -> Result<BiometricUnlockResponse, String> {
    security_state
        .authenticate_biometric(&request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_push_notification_status(
    security_state: State<'_, RuntimeSecurityState>,
) -> Result<PushNotificationStatusResponse, String> {
    Ok(security_state.push_status())
}

#[tauri::command]
async fn register_push_notifications(
    security_state: State<'_, RuntimeSecurityState>,
    request: PushNotificationRegisterRequest,
) -> Result<PushNotificationRegisterResponse, String> {
    security_state
        .register_push(&request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_passkey(
    security_state: State<'_, RuntimeSecurityState>,
    request: PasskeyCreateRequest,
) -> Result<PasskeyCreateResponse, String> {
    security_state
        .create_passkey(&request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn authenticate_passkey(
    security_state: State<'_, RuntimeSecurityState>,
    request: PasskeyAuthenticateRequest,
) -> Result<PasskeyAuthenticateResponse, String> {
    security_state
        .authenticate_passkey(&request)
        .map_err(|e| e.to_string())
}

// Governance commands
#[tauri::command]
async fn get_proposals(
    _state: State<'_, SharedWalletContext>,
    request: Option<GetProposalsRequest>,
) -> Result<GetProposalsResponse, String> {
    let client = BlockchainClient::default();
    let request_value = request
        .map(|r| serde_json::to_value(r).unwrap())
        .unwrap_or(serde_json::Value::Null);
    client
        .get_proposals(Some(request_value))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_proposal(
    _state: State<'_, SharedWalletContext>,
    proposal_id: i64,
) -> Result<GetProposalResponse, String> {
    let client = BlockchainClient::default();
    client
        .get_proposal(proposal_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_proposal_votes(
    _state: State<'_, SharedWalletContext>,
    proposal_id: i64,
    limit: Option<u64>,
    offset: Option<u64>,
) -> Result<GetProposalVotesResponse, String> {
    let client = BlockchainClient::default();
    client
        .get_proposal_votes(proposal_id, limit, offset)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_voting_power(
    _state: State<'_, SharedWalletContext>,
    address: String,
) -> Result<GetVotingPowerResponse, String> {
    let client = BlockchainClient::default();
    client
        .get_voting_power(&address)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_delegations(
    _state: State<'_, SharedWalletContext>,
    address: String,
) -> Result<GetDelegationsResponse, String> {
    let client = BlockchainClient::default();
    client
        .get_delegations(&address)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn cast_vote(
    _state: State<'_, SharedWalletContext>,
    request: CastVoteRequest,
) -> Result<CastVoteResponse, String> {
    let client = BlockchainClient::default();
    client.cast_vote(request).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn delegate(
    _state: State<'_, SharedWalletContext>,
    request: DelegateRequest,
) -> Result<DelegateResponse, String> {
    let client = BlockchainClient::default();
    client.delegate(request).await.map_err(|e| e.to_string())
}

fn main() {
    let security_config =
        init_security_config_from_env().expect("Failed to initialize security configuration");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(move |app| {
            let security_state = RuntimeSecurityState::new(security_config)?;
            let config_dir = app
                .path()
                .app_config_dir()
                .map_err(|e| WalletError::Unknown(e.to_string()))?;
            let context = WalletContext::initialize(config_dir)?;
            app.manage(SharedWalletContext::new(context));
            app.manage(security_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_wallet,
            import_wallet,
            unlock_wallet,
            lock_wallet,
            get_wallet_info,
            export_wallet,
            change_password,
            sign_message,
            verify_message_signature,
            validate_address,
            format_amount,
            get_balance,
            get_transaction_history,
            get_validators,
            get_user_delegations,
            get_staking_rewards,
            get_lockbox_records,
            get_auto_stake_status,
            delegate_tokens,
            undelegate_tokens,
            create_lockbox_stake,
            toggle_auto_staking,
            claim_staking_rewards,
            get_biometric_status,
            authenticate_biometric,
            get_push_notification_status,
            register_push_notifications,
            create_passkey,
            authenticate_passkey,
            get_proposals,
            get_proposal,
            get_proposal_votes,
            get_voting_power,
            get_delegations,
            cast_vote,
            delegate
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
