//! `change_password` — rotate the master password and/or Secret Key.
//!
//! ## Algorithm (O(n) AES-KW rewrap, O(1) ciphertext)
//!
//! 1. Resolve the new Secret Key (caller-provided OR existing keychain).
//! 2. Derive a *new* `(master_key, kek, verify_hash)` on `spawn_blocking`.
//! 3. For every entry row: `unwrap_dek(old_kek)` → `wrap_dek(new_kek)`.
//!    DEK bytes live only on the current-iteration stack; both ends are
//!    zeroized via `Zeroizing`.
//! 4. Persist the batch **atomically**: the repo updates every row's
//!    `dek_wrapped` AND writes the new `vault_config` (`verify_hash` /
//!    `kdf_params` / `last_unlocked_at`) in a single transaction. If any step
//!    fails the whole batch rolls back so the vault never lands in a
//!    mixed-KEK state.
//! 5. Write the new Secret Key to the keychain if it changed.
//! 6. Swap `session.kek` for the new KEK; if biometric unlock is enrolled, re-store the
//!    *new* KEK behind the gate. The old stored KEK is now stale (it can't unwrap the
//!    freshly-rewrapped DEKs), so a biometric unlock would fail until re-enrolled.
//! 7. Audit `PasswordChanged`.
//!
//! ## Ciphertext is untouched
//!
//! Entry ciphertexts (and nonces, versions) are unchanged — only the
//! wrapped per-entry DEKs rotate. AAD bindings still hold. This keeps the
//! operation O(n) in wrap/unwrap and avoids re-encrypting payload bodies
//! (which could be megabytes for Documents).

use std::sync::Arc;

use tracing::instrument;
use zeroize::Zeroizing;

use crate::application::vault::ports::biometric::BiometricAuthenticator;
use crate::application::vault::ports::kdf::KeyDerivationProvider;
use crate::application::vault::ports::keychain::KeychainProvider;
use crate::application::vault::session::VaultSession;
use crate::domain::shared::now;
use crate::domain::vault::crypto_constants::{KEK_LEN, SECRET_KEY_LEN};
use crate::domain::vault::entities::AuditAction;
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::crypto::secret_mem::SecretMem;

#[derive(Debug)]
pub struct ChangePasswordInput {
    pub new_password: Zeroizing<String>,
    /// `None` = keep the Secret Key already on file.
    pub new_secret_key: Option<Zeroizing<[u8; SECRET_KEY_LEN]>>,
}

#[instrument(skip_all, fields(vault_id = %session.vault_id()))]
pub async fn change_password(
    session: &mut VaultSession,
    kdf: Arc<dyn KeyDerivationProvider>,
    keychain: Arc<dyn KeychainProvider>,
    biometric: Arc<dyn BiometricAuthenticator>,
    input: ChangePasswordInput,
) -> Result<(), VaultError> {
    // ---- 1. Resolve Secret Key ----------------------------------------------
    let secret_key = if let Some(sk) = input.new_secret_key.clone() {
        sk
    } else {
        let uuid = session
            .vault_uuid()
            .ok_or(VaultError::KeychainEntryNotFound)?;
        keychain.read_secret_key(uuid)?
    };

    // ---- 2. Derive a new KEK + verify_hash ----------------------------------
    let vault_salt = session.config.vault_salt;
    let kdf_params = session.config.kdf_params.clone();
    let password = input.new_password;
    let kdf_job = Arc::clone(&kdf);

    let (new_kek_z, new_verify_hash) = tokio::task::spawn_blocking(
        move || -> Result<(Zeroizing<[u8; KEK_LEN]>, [u8; 32]), VaultError> {
            let input_bytes = kdf_job.preprocess_2skd(password.as_bytes(), &secret_key)?;
            let master_key = kdf_job.derive_master_key(&input_bytes, &vault_salt, &kdf_params)?;
            let verify = kdf_job.derive_verify_hash(&master_key)?;
            let kek = kdf_job.derive_kek(&master_key)?;
            Ok((kek, verify))
        },
    )
    .await
    .map_err(|e| VaultError::KeyDerivationFailed(format!("spawn_blocking join: {e}")))??;

    // ---- 3. Re-wrap every entry's DEK under the new KEK --------------------
    // Kept as `Zeroizing` (not zeroized inline) because the snapshot rewrap (⑬, step 4b)
    // needs BOTH KEKs AFTER the commit; it drops — and zeroizes — at the end of this fn.
    let old_kek_z: Zeroizing<[u8; KEK_LEN]> = Zeroizing::new(*session.kek.expose());
    let rows = session.repo.all_entries().await?;
    let mut updates: Vec<(crate::domain::shared::EntryId, [u8; 40])> =
        Vec::with_capacity(rows.len());
    for row in rows {
        let dek = session.crypto.unwrap_dek(&row.dek_wrapped, &old_kek_z)?;
        let new_wrapped = session.crypto.wrap_dek(&dek, &new_kek_z)?;
        // `dek` drops here, zeroizing.
        updates.push((row.id, new_wrapped));
    }

    // ---- 4. Atomic commit: all entries + config in one transaction ----------
    let when = now();
    let mut new_config = session.config.clone();
    new_config.verify_hash = new_verify_hash;
    new_config.last_unlocked_at = Some(when);
    session.repo.rewrap_all_deks(&updates, &new_config).await?;

    // ---- 4b. Rewrap the local snapshot store (⑬) — AFTER the live vault commits ----------
    // A crash here leaves snapshots on the OLD password (which the user just typed), never a
    // password that does not yet exist. A failed rewrap NEVER fails the change (L1) — the
    // snapshot keeps its old credentials and is badged stale.
    let snapshots_dir = session.vault_id().snapshots_dir();
    let rewrap = super::rewrap_snapshots::rewrap_snapshots(
        session.crypto.as_ref(),
        &snapshots_dir,
        &old_kek_z,
        &new_kek_z,
        &new_verify_hash,
    )
    .await;
    if !rewrap.failed.is_empty() {
        tracing::warn!(
            failed = ?rewrap.failed,
            rewrapped = rewrap.rewrapped,
            "some snapshots kept the previous password — retire or re-take them"
        );
    }
    drop(old_kek_z);

    // ---- 5. Keychain update --------------------------------------------------
    if let Some(sk) = input.new_secret_key {
        let uuid = session
            .vault_uuid()
            .ok_or(VaultError::KeychainEntryNotFound)?;
        keychain.store_secret_key(uuid, &sk)?;
    }

    // ---- 6. Swap the session's KEK ------------------------------------------
    session.kek = SecretMem::new(*new_kek_z)?;
    drop(new_kek_z);
    session.config = new_config;

    // ---- 6b. Refresh the biometric-gated KEK if enrolled --------------------
    // The stored KEK is now stale (the DEKs were re-wrapped under the new KEK). Re-store
    // the new KEK so biometric unlock keeps working. Done after the atomic commit so a
    // rollback never leaves the gate holding a KEK the DB doesn't match.
    if biometric.is_enrolled(session.vault_id())? {
        biometric.enroll(session.vault_id(), session.kek.expose())?;
    }

    // ---- 7. Audit ------------------------------------------------------------
    super::create_entry::append_audit(session, AuditAction::PasswordChanged, None).await?;
    Ok(())
}
