use chert_wallet_lib::crypto::{StealthKeyMaterial, WalletKeyPair};
use chert_wallet_lib::{VaultMetadata, VaultSecrets, WalletContext, WalletError, WalletResult};
use secrecy::SecretString;
use tempfile::TempDir;

#[test]
fn wallet_create_lock_unlock_export_flow() -> WalletResult<()> {
    std::env::set_var("CHERT_WALLET_ENV", "test");
    let temp_dir = TempDir::new().expect("create temp dir");

    let context = WalletContext::initialize(temp_dir.path().to_path_buf())?;
    let (keypair, mnemonic) = WalletKeyPair::generate_with_mnemonic(12, None, None, false)?;
    let stealth_keys = StealthKeyMaterial::derive_from_seed(&keypair.core_keypair.private_key)?;
    let secrets = VaultSecrets {
        mnemonic_phrase: Some(mnemonic.clone()),
        seed_bytes: keypair.core_keypair.private_key.clone(),
        stealth_material: stealth_keys.encode(),
        pq_material: Vec::new(),
    };

    let password = SecretString::from("Password123!".to_string());
    let metadata = VaultMetadata::new("Integration Wallet");
    context.create_vault(&password, metadata, secrets)?;
    assert!(context.vault().exists());

    context.unlock(&password)?;
    let unlocked_wallet = context.session().peek_unlocked(|metadata, secrets| {
        assert_eq!(metadata.wallet_name, "Integration Wallet");
        Ok(secrets.mnemonic_phrase.clone())
    })?;
    assert_eq!(unlocked_wallet, Some(mnemonic.clone()));

    context.lock();
    assert!(context.session().is_locked());

    // Attempting to unlock with wrong password should register a failure but not panic
    let wrong_password = SecretString::from("WrongPassword123!".to_string());
    let err = context
        .unlock(&wrong_password)
        .expect_err("expected unlock failure");
    assert!(matches!(
        err,
        WalletError::CryptoError(_) | WalletError::ValidationError(_)
    ));

    context.unlock(&password)?;
    let exported = context.session().with_unlocked(|metadata, secrets| {
        assert_eq!(metadata.wallet_name, "Integration Wallet");
        Ok((
            metadata.wallet_name.clone(),
            secrets.mnemonic_phrase.clone(),
        ))
    })?;
    assert_eq!(exported.0, "Integration Wallet");
    assert_eq!(exported.1, Some(mnemonic));

    std::env::remove_var("CHERT_WALLET_ENV");
    Ok(())
}
