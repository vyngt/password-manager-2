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
//! 6. Swap `session.kek` for the new KEK.
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
    input: ChangePasswordInput,
) -> Result<(), VaultError> {
    // ---- 1. Resolve Secret Key ----------------------------------------------
    let secret_key = match input.new_secret_key.clone() {
        Some(sk) => sk,
        None => keychain.read_secret_key(session.vault_id())?,
    };

    // ---- 2. Derive a new KEK + verify_hash ----------------------------------
    let vault_salt = session.config.vault_salt;
    let kdf_params = session.config.kdf_params.clone();
    let password = input.new_password;
    let kdf_job = Arc::clone(&kdf);

    let (new_kek_z, new_verify_hash) = tokio::task::spawn_blocking(
        move || -> Result<(Zeroizing<[u8; KEK_LEN]>, [u8; 32]), VaultError> {
            let input_bytes = kdf_job.preprocess_2skd(password.as_bytes(), &secret_key)?;
            let master_key =
                kdf_job.derive_master_key(&input_bytes, &vault_salt, &kdf_params)?;
            let verify = kdf_job.derive_verify_hash(&master_key)?;
            let kek = kdf_job.derive_kek(&master_key)?;
            Ok((kek, verify))
        },
    )
    .await
    .map_err(|e| VaultError::KeyDerivationFailed(format!("spawn_blocking join: {e}")))??;

    // ---- 3. Re-wrap every entry's DEK under the new KEK --------------------
    let old_kek_bytes: [u8; KEK_LEN] = *session.kek.expose();
    let rows = session.repo.all_entries().await?;
    let mut updates: Vec<(crate::domain::shared::EntryId, [u8; 40])> =
        Vec::with_capacity(rows.len());
    for row in rows {
        let dek = session
            .crypto
            .unwrap_dek(&row.dek_wrapped, &old_kek_bytes)?;
        let new_wrapped = session.crypto.wrap_dek(&dek, &new_kek_z)?;
        // `dek` drops here, zeroizing.
        updates.push((row.id, new_wrapped));
    }
    // Best-effort zero of our stack copy of the old KEK.
    let mut old_kek_bytes = old_kek_bytes;
    zeroize::Zeroize::zeroize(&mut old_kek_bytes);

    // ---- 4. Atomic commit: all entries + config in one transaction ----------
    let when = now();
    let mut new_config = session.config.clone();
    new_config.verify_hash = new_verify_hash;
    new_config.last_unlocked_at = Some(when);
    session.repo.rewrap_all_deks(&updates, &new_config).await?;

    // ---- 5. Keychain update --------------------------------------------------
    if let Some(sk) = input.new_secret_key {
        keychain.store_secret_key(session.vault_id(), &sk)?;
    }

    // ---- 6. Swap the session's KEK ------------------------------------------
    session.kek = SecretMem::new(*new_kek_z)?;
    drop(new_kek_z);
    session.config = new_config;

    // ---- 7. Audit ------------------------------------------------------------
    super::create_entry::append_audit(session, AuditAction::PasswordChanged, None).await?;
    Ok(())
}
