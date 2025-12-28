use crate::api::types::{
    BiometricStatusResponse, BiometricUnlockRequest, BiometricUnlockResponse,
    PasskeyAuthenticateRequest, PasskeyAuthenticateResponse, PasskeyCreateRequest,
    PasskeyCreateResponse, PushNotificationRegisterRequest, PushNotificationRegisterResponse,
    PushNotificationStatusResponse,
};
use crate::errors::{WalletError, WalletResult};
use crate::security::{Environment, SecurityConfig};
use arrayvec::{ArrayString, ArrayVec};
use blake3::Hasher;
use ed25519_dalek::{Signer, SigningKey};
use parking_lot::Mutex;
use rand::rngs::OsRng;
use rand::RngCore;
use serde_json::json;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use zeroize::Zeroizing;

const MAX_BIOMETRIC_TYPES: usize = 4;
const MAX_BIOMETRIC_TOKENS: usize = 32;
const MAX_PUSH_REGISTRATIONS: usize = 64;
const MAX_PASSKEY_RECORDS: usize = 64;
const MAX_BOUNDED_STR_LENGTH: usize = 120;
const MAX_REASON_LENGTH: usize = 256;
const MAX_CHALLENGE_LENGTH: usize = 512;
const TOKEN_BYTES: usize = 32;
const REGISTRATION_ID_BYTES: usize = 16;

#[repr(align(64))]
#[derive(Debug)]
pub struct RuntimeSecurityState {
    config: &'static SecurityConfig,
    policy: SecurityPolicy,
    biometric: BiometricRegistry,
    push: PushRegistry,
    passkeys: PasskeyRegistry,
}

impl RuntimeSecurityState {
    pub fn new(config: &'static SecurityConfig) -> WalletResult<Self> {
        const _: () = assert!(
            MAX_BIOMETRIC_TYPES > 0,
            "biometric capacity must be non-zero"
        );
        const _: () = assert!(MAX_PUSH_REGISTRATIONS > 0, "push capacity must be non-zero");
        let policy = SecurityPolicy::from_config(config)?;
        Ok(Self {
            config,
            biometric: BiometricRegistry::new(&policy),
            push: PushRegistry::new(&policy),
            passkeys: PasskeyRegistry::new(&policy),
            policy,
        })
    }

    pub fn config(&self) -> &'static SecurityConfig {
        self.config
    }

    pub fn biometric_status(&self) -> BiometricStatusResponse {
        self.biometric.status(self.policy.biometrics_enabled)
    }

    pub fn authenticate_biometric(
        &self,
        request: &BiometricUnlockRequest,
    ) -> WalletResult<BiometricUnlockResponse> {
        if !self.policy.biometrics_enabled {
            return Err(WalletError::PermissionDenied(
                "Biometric authentication disabled by security policy".to_string(),
            ));
        }
        self.biometric
            .authenticate(request, self.policy.biometric_token_retention)
    }

    pub fn push_status(&self) -> PushNotificationStatusResponse {
        self.push.status(
            self.policy.push_enabled,
            self.policy.push_permission_required,
        )
    }

    pub fn register_push(
        &self,
        request: &PushNotificationRegisterRequest,
    ) -> WalletResult<PushNotificationRegisterResponse> {
        if !self.policy.push_enabled {
            return Err(WalletError::PermissionDenied(
                "Push notifications disabled by security policy".to_string(),
            ));
        }
        self.push.register(request)
    }

    pub fn create_passkey(
        &self,
        request: &PasskeyCreateRequest,
    ) -> WalletResult<PasskeyCreateResponse> {
        if !self.policy.passkeys_enabled {
            return Err(WalletError::PermissionDenied(
                "Passkey registration disabled by security policy".to_string(),
            ));
        }
        self.passkeys.create(request)
    }

    pub fn authenticate_passkey(
        &self,
        request: &PasskeyAuthenticateRequest,
    ) -> WalletResult<PasskeyAuthenticateResponse> {
        if !self.policy.passkeys_enabled {
            return Err(WalletError::PermissionDenied(
                "Passkey authentication disabled by security policy".to_string(),
            ));
        }
        self.passkeys.authenticate(request)
    }
}

#[derive(Debug, Clone)]
struct SecurityPolicy {
    #[allow(dead_code)]
    environment: Environment,
    biometrics_enabled: bool,
    biometrics_enrolled: bool,
    biometric_types: ArrayVec<BiometricFactor, MAX_BIOMETRIC_TYPES>,
    biometric_token_retention: usize,
    push_enabled: bool,
    push_permission_required: bool,
    push_max_devices: usize,
    passkeys_enabled: bool,
    passkey_max_credentials: usize,
}

impl SecurityPolicy {
    fn from_config(config: &SecurityConfig) -> WalletResult<Self> {
        const _: () = assert!(MAX_PASSKEY_RECORDS > 0, "passkey capacity must be non-zero");
        let biometrics_enabled = config.get_bool_with_default("ENABLE_BIOMETRICS", false)?;
        let biometrics_enrolled = config.get_bool_with_default("BIOMETRIC_ENROLLED", false)?;
        let supported_raw = config.get_string_list("BIOMETRIC_SUPPORTED")?;
        let mut biometric_types = ArrayVec::<BiometricFactor, MAX_BIOMETRIC_TYPES>::new();
        for entry in supported_raw {
            if biometric_types.is_full() {
                break;
            }
            biometric_types.push(BiometricFactor::from_str(&entry)?);
        }
        if biometric_types.is_empty() {
            return Err(WalletError::ValidationError(
                "At least one biometric factor must be configured".to_string(),
            ));
        }
        let biometric_token_retention = config
            .get_u32_with_default("BIOMETRIC_TOKEN_RETENTION", MAX_BIOMETRIC_TOKENS as u32)?
            .clamp(1, MAX_BIOMETRIC_TOKENS as u32) as usize;

        let push_enabled = config.get_bool_with_default("ENABLE_PUSH_NOTIFICATIONS", false)?;
        let push_permission_required =
            config.get_bool_with_default("PUSH_PERMISSION_REQUIRED", true)?;
        let push_max_devices = config
            .get_u32_with_default("PUSH_MAX_DEVICES", MAX_PUSH_REGISTRATIONS as u32)?
            .clamp(1, MAX_PUSH_REGISTRATIONS as u32) as usize;

        let passkeys_enabled = config.get_bool_with_default("ENABLE_PASSKEYS", false)?;
        let passkey_max_credentials = config
            .get_u32_with_default("PASSKEY_MAX_CREDENTIALS", MAX_PASSKEY_RECORDS as u32)?
            .clamp(1, MAX_PASSKEY_RECORDS as u32) as usize;

        Ok(Self {
            environment: *config.environment(),
            biometrics_enabled,
            biometrics_enrolled,
            biometric_types,
            biometric_token_retention,
            push_enabled,
            push_permission_required,
            push_max_devices,
            passkeys_enabled,
            passkey_max_credentials,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BiometricFactor {
    Fingerprint,
    Face,
    Iris,
    Voice,
}

impl BiometricFactor {
    fn from_str(value: &str) -> WalletResult<Self> {
        assert!(
            value.len() <= MAX_BOUNDED_STR_LENGTH,
            "biometric factor exceeds bounded length"
        );
        let normalized = value.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            return Err(WalletError::ValidationError(
                "Biometric factor cannot be empty".to_string(),
            ));
        }
        match normalized.as_str() {
            "fingerprint" => Ok(Self::Fingerprint),
            "face" => Ok(Self::Face),
            "iris" => Ok(Self::Iris),
            "voice" => Ok(Self::Voice),
            _ => Err(WalletError::ValidationError(format!(
                "Unsupported biometric factor '{}'",
                value
            ))),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            BiometricFactor::Fingerprint => "fingerprint",
            BiometricFactor::Face => "face",
            BiometricFactor::Iris => "iris",
            BiometricFactor::Voice => "voice",
        }
    }
}

#[repr(align(64))]
#[derive(Debug)]
struct BiometricRegistry {
    state: Mutex<BiometricState>,
    #[allow(dead_code)]
    retention: usize,
}

#[derive(Clone, Debug)]
struct BiometricState {
    available: bool,
    enrolled: bool,
    supported: ArrayVec<BiometricFactor, MAX_BIOMETRIC_TYPES>,
    tokens: ArrayVec<[u8; TOKEN_BYTES], MAX_BIOMETRIC_TOKENS>,
}

impl BiometricRegistry {
    fn new(policy: &SecurityPolicy) -> Self {
        assert!(policy.biometric_token_retention >= 1);
        assert!(policy.biometric_token_retention <= MAX_BIOMETRIC_TOKENS);
        let mut supported = ArrayVec::new();
        supported.extend(policy.biometric_types.iter().copied());
        Self {
            state: Mutex::new(BiometricState {
                available: policy.biometrics_enabled,
                enrolled: policy.biometrics_enrolled,
                supported,
                tokens: ArrayVec::new(),
            }),
            retention: policy.biometric_token_retention,
        }
    }

    fn status(&self, enabled: bool) -> BiometricStatusResponse {
        let state = self.state.lock();
        assert!(state.supported.len() <= MAX_BIOMETRIC_TYPES);
        assert!(state.tokens.len() <= MAX_BIOMETRIC_TOKENS);
        let available = enabled && state.available;
        let enrolled = available && state.enrolled;
        let supported_types = if available {
            state
                .supported
                .iter()
                .map(|factor| factor.as_str().to_string())
                .collect()
        } else {
            Vec::new()
        };
        BiometricStatusResponse {
            available,
            enrolled,
            supported_types,
        }
    }

    fn authenticate(
        &self,
        request: &BiometricUnlockRequest,
        retention: usize,
    ) -> WalletResult<BiometricUnlockResponse> {
        assert!(retention >= 1, "biometric retention must be positive");
        assert!(
            retention <= MAX_BIOMETRIC_TOKENS,
            "retention exceeds bounds"
        );
        let reason = request.reason.trim();
        if reason.is_empty() {
            return Err(WalletError::ValidationError(
                "Biometric unlock reason cannot be empty".to_string(),
            ));
        }
        if reason.len() > MAX_REASON_LENGTH {
            return Err(WalletError::ValidationError(format!(
                "Biometric unlock reason exceeds {} characters",
                MAX_REASON_LENGTH
            )));
        }

        let mut state = self.state.lock();
        assert!(state.tokens.len() <= MAX_BIOMETRIC_TOKENS);
        if !state.available {
            return Err(WalletError::PermissionDenied(
                "Biometric hardware unavailable".to_string(),
            ));
        }
        if !state.enrolled {
            return Err(WalletError::PermissionDenied(
                "No biometric enrollment detected".to_string(),
            ));
        }

        let mut entropy = [0u8; TOKEN_BYTES];
        OsRng.fill_bytes(&mut entropy);
        assert_eq!(entropy.len(), TOKEN_BYTES);
        let mut hasher = Hasher::new();
        hasher.update(reason.as_bytes());
        hasher.update(&entropy);
        let token_hash = hasher.finalize();
        let mut token = [0u8; TOKEN_BYTES];
        token.copy_from_slice(token_hash.as_bytes());

        while state.tokens.len() >= retention {
            state.tokens.remove(0);
        }
        state.tokens.push(token);

        Ok(BiometricUnlockResponse {
            success: true,
            token: Some(hex::encode(token)),
        })
    }
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
struct AlertFlags(u8);

impl AlertFlags {
    const TX: u8 = 0b001;
    const STAKING: u8 = 0b010;
    const GOVERNANCE: u8 = 0b100;

    fn from_request(request: &PushNotificationRegisterRequest) -> WalletResult<Self> {
        let mut flags = 0u8;
        if request.enable_transaction_alerts {
            flags |= Self::TX;
        }
        if request.enable_staking_alerts {
            flags |= Self::STAKING;
        }
        if request.enable_governance_alerts {
            flags |= Self::GOVERNANCE;
        }
        if flags == 0 {
            return Err(WalletError::ValidationError(
                "At least one push notification category must be enabled".to_string(),
            ));
        }
        Ok(Self(flags))
    }
}

#[derive(Clone, Debug)]
struct PushRegistration {
    token_hash: [u8; TOKEN_BYTES],
    alerts: AlertFlags,
    registration_id: [u8; REGISTRATION_ID_BYTES],
    sequence: u64,
}

#[repr(align(64))]
#[derive(Debug)]
struct PushRegistry {
    entries: Mutex<ArrayVec<PushRegistration, MAX_PUSH_REGISTRATIONS>>,
    limit: usize,
    permission_granted: AtomicBool,
    sequence: AtomicU64,
}

impl PushRegistry {
    fn new(policy: &SecurityPolicy) -> Self {
        assert!(policy.push_max_devices >= 1);
        assert!(policy.push_max_devices <= MAX_PUSH_REGISTRATIONS);
        Self {
            entries: Mutex::new(ArrayVec::new()),
            limit: policy.push_max_devices,
            permission_granted: AtomicBool::new(false),
            sequence: AtomicU64::new(0),
        }
    }

    fn status(&self, enabled: bool, permission_required: bool) -> PushNotificationStatusResponse {
        let entries = self.entries.lock();
        assert!(entries.len() <= MAX_PUSH_REGISTRATIONS);
        let has_registration = !entries.is_empty();
        let available = enabled;
        let effective_permission = if !available {
            false
        } else if permission_required {
            self.permission_granted.load(Ordering::Relaxed) && has_registration
        } else {
            true
        };
        PushNotificationStatusResponse {
            available,
            enabled: available && has_registration,
            permission_granted: effective_permission,
        }
    }

    fn register(
        &self,
        request: &PushNotificationRegisterRequest,
    ) -> WalletResult<PushNotificationRegisterResponse> {
        let token = request.token.trim();
        if token.is_empty() {
            return Err(WalletError::ValidationError(
                "Push notification token cannot be empty".to_string(),
            ));
        }
        if token.len() > MAX_BOUNDED_STR_LENGTH * 4 {
            return Err(WalletError::ValidationError(format!(
                "Push token exceeds {} characters",
                MAX_BOUNDED_STR_LENGTH * 4
            )));
        }

        let flags = AlertFlags::from_request(request)?;
        let mut hasher = Hasher::new();
        hasher.update(token.as_bytes());
        let mut token_hash = [0u8; TOKEN_BYTES];
        token_hash.copy_from_slice(hasher.finalize().as_bytes());

        let mut entries = self.entries.lock();
        assert!(entries.len() <= MAX_PUSH_REGISTRATIONS);
        if let Some(existing) = entries
            .iter_mut()
            .find(|entry| entry.token_hash == token_hash)
        {
            existing.alerts = flags;
            existing.sequence = self.sequence.fetch_add(1, Ordering::SeqCst);
            self.permission_granted.store(true, Ordering::SeqCst);
            return Ok(PushNotificationRegisterResponse {
                success: true,
                registration_id: Some(hex::encode(existing.registration_id)),
            });
        }

        if entries.len() >= self.limit {
            if let Some((index, _)) = entries
                .iter()
                .enumerate()
                .min_by_key(|(_, entry)| entry.sequence)
            {
                entries.remove(index);
            }
        }

        let mut registration_id = [0u8; REGISTRATION_ID_BYTES];
        OsRng.fill_bytes(&mut registration_id);
        assert_eq!(registration_id.len(), REGISTRATION_ID_BYTES);

        entries.push(PushRegistration {
            token_hash,
            alerts: flags,
            registration_id,
            sequence: self.sequence.fetch_add(1, Ordering::SeqCst),
        });
        self.permission_granted.store(true, Ordering::SeqCst);

        Ok(PushNotificationRegisterResponse {
            success: true,
            registration_id: Some(hex::encode(registration_id)),
        })
    }
}

#[derive(Debug)]
struct PasskeyCredential {
    credential_id: [u8; TOKEN_BYTES],
    #[allow(dead_code)]
    public_key: [u8; TOKEN_BYTES],
    private_key: Zeroizing<[u8; TOKEN_BYTES]>,
    relying_party: ArrayString<MAX_BOUNDED_STR_LENGTH>,
    user_id: ArrayString<MAX_BOUNDED_STR_LENGTH>,
    user_name: ArrayString<MAX_BOUNDED_STR_LENGTH>,
    sequence: u64,
    counter: u32,
}

#[repr(align(64))]
#[derive(Debug)]
struct PasskeyRegistry {
    entries: Mutex<ArrayVec<PasskeyCredential, MAX_PASSKEY_RECORDS>>,
    limit: usize,
    sequence: AtomicU64,
}

impl PasskeyRegistry {
    fn new(policy: &SecurityPolicy) -> Self {
        assert!(policy.passkey_max_credentials >= 1);
        assert!(policy.passkey_max_credentials <= MAX_PASSKEY_RECORDS);
        Self {
            entries: Mutex::new(ArrayVec::new()),
            limit: policy.passkey_max_credentials,
            sequence: AtomicU64::new(0),
        }
    }

    fn create(&self, request: &PasskeyCreateRequest) -> WalletResult<PasskeyCreateResponse> {
        assert!(self.limit >= 1);
        let relying_party = to_bounded_string(&request.relying_party_id, "relying party")?;
        let user_id = to_bounded_string(&request.user_id, "user id")?;
        let user_name = to_bounded_string(&request.user_name, "user name")?;

        let mut private_bytes = [0u8; TOKEN_BYTES];
        OsRng.fill_bytes(&mut private_bytes);
        let signing_key = SigningKey::from_bytes(&private_bytes);
        let public_key = signing_key.verifying_key().to_bytes();

        let mut credential_id = [0u8; TOKEN_BYTES];
        OsRng.fill_bytes(&mut credential_id);

        let mut entries = self.entries.lock();
        assert!(entries.len() <= MAX_PASSKEY_RECORDS);
        if entries.len() >= self.limit {
            if let Some((index, _)) = entries
                .iter()
                .enumerate()
                .min_by_key(|(_, entry)| entry.sequence)
            {
                entries.remove(index);
            }
        }

        entries.push(PasskeyCredential {
            credential_id,
            public_key,
            private_key: Zeroizing::new(private_bytes),
            relying_party,
            user_id,
            user_name,
            sequence: self.sequence.fetch_add(1, Ordering::SeqCst),
            counter: 0,
        });

        Ok(PasskeyCreateResponse {
            credential_id: hex::encode(credential_id),
            public_key: hex::encode(public_key),
        })
    }

    fn authenticate(
        &self,
        request: &PasskeyAuthenticateRequest,
    ) -> WalletResult<PasskeyAuthenticateResponse> {
        if request.credential_ids.is_empty() {
            return Err(WalletError::ValidationError(
                "At least one credential identifier must be provided".to_string(),
            ));
        }
        let challenge = request.challenge.trim();
        if challenge.is_empty() {
            return Err(WalletError::ValidationError(
                "Passkey challenge cannot be empty".to_string(),
            ));
        }
        if challenge.len() > MAX_CHALLENGE_LENGTH {
            return Err(WalletError::ValidationError(format!(
                "Passkey challenge exceeds {} characters",
                MAX_CHALLENGE_LENGTH
            )));
        }

        let mut entries = self.entries.lock();
        assert!(entries.len() <= MAX_PASSKEY_RECORDS);
        let mut matched_index = None;
        let mut matched_id = [0u8; TOKEN_BYTES];
        for candidate in &request.credential_ids {
            let trimmed = candidate.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(bytes) = hex::decode(trimmed) {
                if bytes.len() == TOKEN_BYTES {
                    let mut array = [0u8; TOKEN_BYTES];
                    array.copy_from_slice(&bytes);
                    if let Some((index, _)) = entries
                        .iter()
                        .enumerate()
                        .find(|(_, entry)| entry.credential_id == array)
                    {
                        matched_index = Some(index);
                        matched_id = array;
                        break;
                    }
                }
            }
        }

        let index = matched_index.ok_or_else(|| {
            WalletError::NotFound("No matching passkey credential found".to_string())
        })?;
        let entry = &mut entries[index];

        let challenge_hash = blake3::hash(challenge.as_bytes());
        let mut authenticator_hasher = Hasher::new();
        authenticator_hasher.update(entry.relying_party.as_bytes());
        authenticator_hasher.update(entry.user_id.as_bytes());
        let counter_bytes = entry.counter.to_le_bytes();
        authenticator_hasher.update(&counter_bytes[..]);
        authenticator_hasher.update(challenge_hash.as_bytes());
        let authenticator_digest = authenticator_hasher.finalize();

        let mut signature_payload = Hasher::new();
        signature_payload.update(authenticator_digest.as_bytes());
        signature_payload.update(challenge_hash.as_bytes());
        let payload_digest = signature_payload.finalize();

        let signing_key = SigningKey::from_bytes(&entry.private_key);
        let signature = signing_key.sign(payload_digest.as_bytes());

        let client_data = json!({
            "type": "webauthn.get",
            "challenge": hex::encode(challenge_hash.as_bytes()),
            "origin": entry.relying_party.as_str(),
            "userHandle": entry.user_id.as_str(),
            "userName": entry.user_name.as_str(),
            "signCount": entry.counter,
        });
        let client_data_json = serde_json::to_string(&client_data)
            .map_err(|e| WalletError::Unknown(format!("Failed to serialize client data: {}", e)))?;

        entry.counter = entry.counter.saturating_add(1);

        Ok(PasskeyAuthenticateResponse {
            credential_id: hex::encode(matched_id),
            authenticator_data: hex::encode(authenticator_digest.as_bytes()),
            client_data_json,
            signature: hex::encode(signature.to_bytes()),
        })
    }
}

fn to_bounded_string(
    input: &str,
    field: &str,
) -> WalletResult<ArrayString<MAX_BOUNDED_STR_LENGTH>> {
    assert!(!field.is_empty(), "field name must not be empty");
    const _: () = assert!(
        MAX_BOUNDED_STR_LENGTH > 0,
        "bounded string capacity must be positive"
    );
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(WalletError::ValidationError(format!(
            "{} cannot be empty",
            field
        )));
    }
    if trimmed.len() > MAX_BOUNDED_STR_LENGTH {
        return Err(WalletError::ValidationError(format!(
            "{} exceeds {} characters",
            field, MAX_BOUNDED_STR_LENGTH
        )));
    }
    ArrayString::from(trimmed).map_err(|_| {
        WalletError::ValidationError(format!(
            "{} exceeds {} characters",
            field, MAX_BOUNDED_STR_LENGTH
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::{
        BiometricUnlockRequest, PasskeyAuthenticateRequest, PasskeyCreateRequest,
        PushNotificationRegisterRequest,
    };
    use crate::security::{Environment, SecurityConfig};

    fn runtime_state() -> RuntimeSecurityState {
        let config = SecurityConfig::new(Environment::Development);
        let leaked: &'static SecurityConfig = Box::leak(Box::new(config));
        RuntimeSecurityState::new(leaked).expect("runtime state initialization")
    }

    #[test]
    fn biometric_status_and_authentication() {
        let state = runtime_state();
        let status = state.biometric_status();
        assert!(
            status.available,
            "biometrics must be available in dev config"
        );
        assert!(status.enrolled, "biometrics should be enrolled by default");
        assert!(
            !status.supported_types.is_empty(),
            "expected supported biometric types"
        );

        let request = BiometricUnlockRequest {
            reason: "Unlock wallet".to_string(),
        };
        let response = state
            .authenticate_biometric(&request)
            .expect("biometric authentication");
        let token = response.token.expect("authentication token");
        assert_eq!(token.len(), TOKEN_BYTES * 2, "token should be hex encoded");
    }

    #[test]
    fn push_registration_updates_status() {
        let state = runtime_state();
        let initial = state.push_status();
        assert!(
            initial.available,
            "push notifications must be available in dev"
        );
        assert!(!initial.enabled, "no registrations yet");
        assert!(
            initial.permission_granted,
            "dev config does not require permission"
        );

        let request = PushNotificationRegisterRequest {
            token: "device-token-123".to_string(),
            enable_transaction_alerts: true,
            enable_staking_alerts: false,
            enable_governance_alerts: false,
        };
        let response = state.register_push(&request).expect("push registration");
        assert!(response.success);
        let registration_id = response.registration_id.expect("registration identifier");
        assert_eq!(registration_id.len(), REGISTRATION_ID_BYTES * 2);

        let after = state.push_status();
        assert!(after.enabled, "registration should toggle enabled flag");
        assert!(after.permission_granted, "permission remains granted");
    }

    #[test]
    fn passkey_create_and_authenticate_flow() {
        let state = runtime_state();
        let create_request = PasskeyCreateRequest {
            challenge: "setup".to_string(),
            user_id: "user-123".to_string(),
            user_name: "Tester".to_string(),
            relying_party_id: "wallet.test".to_string(),
        };
        let created = state
            .create_passkey(&create_request)
            .expect("passkey creation");
        assert_eq!(created.credential_id.len(), TOKEN_BYTES * 2);
        assert_eq!(created.public_key.len(), TOKEN_BYTES * 2);

        let auth_request = PasskeyAuthenticateRequest {
            challenge: "login-challenge".to_string(),
            credential_ids: vec![created.credential_id.clone()],
        };
        let auth_response = state
            .authenticate_passkey(&auth_request)
            .expect("passkey authentication");
        assert_eq!(auth_response.credential_id, created.credential_id);
        assert!(!auth_response.authenticator_data.is_empty());
        assert!(!auth_response.client_data_json.is_empty());
        // Ed25519 signature is 64 bytes, hex-encoded to 128 characters
        assert_eq!(auth_response.signature.len(), 128);
    }
}
