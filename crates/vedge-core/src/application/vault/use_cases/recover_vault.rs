//! `recover_vault` — unlock a vault using a typed Emergency Kit string
//! instead of the keychain-stored Secret Key, then re-register the key in
//! the OS keychain so subsequent unlocks work normally.
//!
//! ## Why this wraps `UnlockVault`
//!
//! `UnlockVault` already accepts a caller-supplied Secret Key via
//! `UnlockVaultInput.secret_key`. Recovery adds two things on top:
//!
//! 1. Parse the user's typed display string (with checksum verification)
//!    *before* handing bytes to `UnlockVault` — cheap typo rejection.
//! 2. After a successful unlock, silently write the key back into the OS
//!    keychain so the next unlock takes the fast path. This is the whole
//!    UX point of recovery: type the kit once, never again.
//!
//! ## Return shape
//!
//! Returns [`RecoveryOutcome`] instead of plain `VaultSession` so the
//! shell can distinguish "keychain restored cleanly" from "unlock
//! succeeded but keychain write failed" without forcing the user to
//! re-enter the kit (Argon2id is ~500 ms — user-hostile to retry).

use std::path::PathBuf;
use std::sync::Arc;

use tracing::instrument;
use zeroize::Zeroizing;

use crate::application::vault::ports::keychain::KeychainProvider;
use crate::application::vault::session::VaultSession;
use crate::application::vault::use_cases::unlock_vault::{UnlockVault, UnlockVaultInput};
use crate::domain::shared::now;
use crate::domain::vault::entities::{AuditAction, AuditEvent};
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::recovery::parse_secret_key;

#[derive(Debug)]
pub struct RecoverVaultInput {
    pub vault_path: PathBuf,
    pub master_password: Zeroizing<String>,
    /// `A3-XXXXX-XXXXX-…` as typed by the user. Stripped and validated
    /// internally by [`parse_secret_key`].
    pub recovery_key_display: String,
}

/// Successful recovery. The session is always valid; `keychain_restored`
/// indicates whether the Secret Key was also successfully re-written to
/// the OS keychain.
pub struct RecoveryOutcome {
    pub session: VaultSession,
    /// `true` if `keychain.store_secret_key` succeeded; `false` if the
    /// unlock worked but persisting the key did not. In the `false` case,
    /// [`keychain_error`](Self::keychain_error) carries the underlying
    /// error's display string.
    pub keychain_restored: bool,
    pub keychain_error: Option<String>,
}

impl std::fmt::Debug for RecoveryOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecoveryOutcome")
            .field("keychain_restored", &self.keychain_restored)
            .field("keychain_error", &self.keychain_error)
            .finish_non_exhaustive()
    }
}

#[instrument(skip_all, fields(vault_path = %input.vault_path.display()))]
pub async fn recover_vault(
    unlock: &UnlockVault,
    keychain: Arc<dyn KeychainProvider>,
    input: RecoverVaultInput,
) -> Result<RecoveryOutcome, VaultError> {
    // 1. Parse + checksum-verify before touching the KDF — saves ~500 ms on typos.
    let secret_key = parse_secret_key(&input.recovery_key_display)?;

    // Keep a copy for the post-unlock keychain write. The unlock path takes
    // ownership of the original (moved into `spawn_blocking`).
    let sk_bytes: [u8; crate::domain::vault::crypto_constants::SECRET_KEY_LEN] = *secret_key;

    // 2. Delegate to UnlockVault with the parsed key as an explicit override.
    let session = unlock
        .execute(UnlockVaultInput {
            vault_path: input.vault_path,
            master_password: input.master_password,
            secret_key: Some(secret_key),
        })
        .await?;

    // 3. Append a distinct audit trail event. UnlockVault already appended
    //    `Unlocked`; `RecoveryUsed` tags this unlock as coming from the kit.
    let event = AuditEvent {
        id: ulid::Ulid::new().to_string(),
        entry_id: None,
        action: AuditAction::RecoveryUsed,
        occurred_at: now(),
        device_id: None,
    };
    session.repo.append_audit(&event).await?;

    // 4. Attempt to re-populate the keychain. Failure here doesn't void the
    //    session — the user can still operate the vault this session, and
    //    they'll re-enter the kit next time until the keychain is fixed.
    let (keychain_restored, keychain_error) = match keychain
        .store_secret_key(session.vault_id(), &sk_bytes)
    {
        Ok(()) => (true, None),
        Err(e) => (false, Some(e.to_string())),
    };

    // `sk_bytes` is a plain array; manually zeroize.
    let mut sk_bytes = sk_bytes;
    <[u8; crate::domain::vault::crypto_constants::SECRET_KEY_LEN] as zeroize::Zeroize>::zeroize(
        &mut sk_bytes,
    );

    Ok(RecoveryOutcome {
        session,
        keychain_restored,
        keychain_error,
    })
}
