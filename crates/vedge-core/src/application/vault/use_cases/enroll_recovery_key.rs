//! `enroll_recovery_key` — turn on the opt-in Recovery Key (slice 5.7).
//!
//! Generates a 32-byte Recovery Key `R` server-side, derives
//! `KEK_rec = HKDF-recovery(Argon2id(2SKD(R, Secret Key)))`, and stores
//! `recovery_slot = AES-KW(wrap_key = KEK_rec, payload = the live KEK)`. Returns the `RK1-`
//! display **once** (the create-vault show-once contract — `R` never touches disk). The
//! current-password re-auth is the caller's (the command), mirroring `change_password`.

use std::sync::Arc;

use tracing::instrument;
use zeroize::Zeroizing;

use crate::application::vault::ports::kdf::KeyDerivationProvider;
use crate::application::vault::ports::keychain::KeychainProvider;
use crate::application::vault::session::VaultSession;
use crate::domain::vault::crypto_constants::KEK_LEN;
use crate::domain::vault::entities::AuditAction;
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::secret_key::format_recovery_key;

/// The result of enrolling — the show-once `RK1-` display. Nothing is persisted here beyond
/// the slot; the caller renders the Recovery Kit PDF and drops the string.
pub struct EnrollRecoveryOutcome {
    pub recovery_key_display: String,
}

#[instrument(skip_all, fields(vault_id = %session.vault_id()))]
pub async fn enroll_recovery_key(
    session: &mut VaultSession,
    kdf: Arc<dyn KeyDerivationProvider>,
    keychain: Arc<dyn KeychainProvider>,
) -> Result<EnrollRecoveryOutcome, VaultError> {
    // KEK_rec is bound to 2SKD(Recovery Key, Secret Key), so we need the CURRENT Secret Key
    // on-device — the same precondition as biometric enroll / reauth. A biometric-only vault
    // whose keychain lacks it surfaces the pointed "restore via your kit" message.
    let uuid = session
        .vault_uuid()
        .ok_or(VaultError::KeychainEntryNotFound)?;
    let secret_key = keychain.read_secret_key(uuid)?;

    // Generate R server-side (the sentinel rule — the raw key never crosses to WASM), format
    // it once for display, then derive KEK_rec on the blocking pool (Argon2id is CPU-bound).
    let recovery_key = session.crypto.generate_recovery_key();
    let recovery_key_display = format_recovery_key(&recovery_key);

    let vault_salt = session.config.vault_salt;
    let kdf_params = session.config.kdf_params.clone();
    let kek_rec =
        tokio::task::spawn_blocking(move || -> Result<Zeroizing<[u8; KEK_LEN]>, VaultError> {
            let input = kdf.preprocess_2skd(recovery_key.as_slice(), &secret_key)?;
            let master_key = kdf.derive_master_key(&input, &vault_salt, &kdf_params)?;
            kdf.derive_recovery_kek(&master_key)
        })
        .await
        .map_err(|e| VaultError::KeyDerivationFailed(format!("spawn_blocking join: {e}")))??;

    // slot = AES-KW(wrap_key = KEK_rec, payload = the live KEK). Both are 32-byte keys.
    let slot = {
        let kek = session.kek.expose();
        session.crypto.wrap_dek(kek, &kek_rec)?
    };

    session.repo.set_recovery_slot(&slot).await?;
    session.config.recovery_slot = Some(slot);
    super::create_entry::append_audit(session, AuditAction::RecoveryKeyEnabled, None).await?;

    Ok(EnrollRecoveryOutcome {
        recovery_key_display,
    })
}
