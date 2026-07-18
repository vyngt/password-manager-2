//! Recovery Key commands (slice 5.7): enroll / recover / forced-reset / revoke / status.
//!
//! See `commands/vault.rs` for the `#![allow]` rationale.

#![allow(
    clippy::unreachable,
    clippy::let_underscore_must_use,
    clippy::significant_drop_tightening
)]

use std::path::PathBuf;
use std::sync::Arc;

use tracing::instrument;
use zeroize::Zeroizing;

use vedge_core::domain::shared::VaultId;
use vedge_core::{
    change_password_after_recovery as core_change_after_recovery,
    enroll_recovery_key as core_enroll, parse_recovery_key, parse_secret_key,
    reauthenticate_master_password, revoke_recovery_key as core_revoke,
};

use crate::dto::recovery::{RecoveryEnrollOutputDto, UnlockWithRecoveryKeyInputDto};
use crate::error::CommandError;
use crate::state::AppState;

fn vault_id_from_string(s: &str) -> VaultId {
    VaultId::new(PathBuf::from(s))
}

/// Enroll a Recovery Key on the unlocked vault, returning the `RK1-` display show-once.
///
/// Re-authenticates the current password first (slice 5.7 — a Recovery Key is a durable
/// credential; minting one on an unattended unlocked vault would be a stealthy backdoor). The
/// fresh key is generated server-side; only its display string crosses to the frontend.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn enroll_recovery_key(
    vault_path: String,
    current_password: String,
    state: tauri::State<'_, AppState>,
) -> Result<RecoveryEnrollOutputDto, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;

    // Refresh the TTL up front so the two Argon2 passes (reauth + KEK_rec derive) can't be
    // reaped mid-enroll (as in change_password).
    let ttl = crate::setup::services::session_ttl(&state).await;
    state.touch_deadline(&vault_id, ttl);

    let mut guard = handle.lock().await;

    reauthenticate_master_password(
        &guard,
        Arc::clone(&state.kdf),
        Arc::clone(&state.keychain),
        Zeroizing::new(current_password),
    )
    .await?;

    let outcome = core_enroll(
        &mut guard,
        Arc::clone(&state.kdf),
        Arc::clone(&state.keychain),
    )
    .await?;
    drop(guard);

    state.touch_deadline(&vault_id, ttl);
    Ok(RecoveryEnrollOutputDto {
        recovery_key_display: outcome.recovery_key_display,
    })
}

/// Recover a vault whose master password is forgotten, using the Recovery Kit + Emergency Kit.
///
/// The vault is LOCKED at entry (recovery lives on the launch screen), so this inserts a fresh
/// session — exactly like `unlock_with_secret_key`. On success the caller must immediately
/// drive `change_password_after_recovery` (the session carries the one-shot reset capability).
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %input.vault_path))]
pub async fn unlock_with_recovery_key(
    input: UnlockWithRecoveryKeyInputDto,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_path = PathBuf::from(&input.vault_path);
    let vault_id = VaultId::new(vault_path.clone());

    if state.is_unlocked(&vault_id) {
        return Err(CommandError::Invalid(format!(
            "vault already unlocked: {}",
            vault_path.display()
        )));
    }

    // Parse + checksum-verify both documents before the KDF (cheap typo rejection). A wrong
    // prefix (an Emergency Kit typed where the Recovery Kit belongs) is rejected here.
    let recovery_key = parse_recovery_key(&input.recovery_key_display)?;
    let secret_key = parse_secret_key(&input.secret_key_display)?;

    let session = state
        .unlock_vault
        .unlock_with_recovery_key(vault_path, recovery_key, secret_key)
        .await?;

    state.insert_session(
        vault_id,
        session,
        crate::setup::services::session_ttl(&state).await,
    )?;
    Ok(())
}

/// Set the new master password immediately after a recovery unlock — no current-password
/// reauth (the user forgot it; recovery already proved possession of both documents).
///
/// The core gate (`pending_recovery_reset`) makes this reachable ONLY on a session freshly
/// opened by `unlock_with_recovery_key`; any other session gets `NoRecoveryResetPending`.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn change_password_after_recovery(
    vault_path: String,
    new_password: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;

    let ttl = crate::setup::services::session_ttl(&state).await;
    state.touch_deadline(&vault_id, ttl);

    let mut guard = handle.lock().await;
    core_change_after_recovery(
        &mut guard,
        Arc::clone(&state.kdf),
        Arc::clone(&state.keychain),
        Arc::clone(&state.biometric),
        Zeroizing::new(new_password),
    )
    .await?;
    drop(guard);

    state.touch_deadline(&vault_id, ttl);
    Ok(())
}

/// Revoke the Recovery Key — deletes the slot from this vault (turns recovery off for future
/// copies). Honest scope: a `.vdb` already copied keeps its slot (see the UI copy → 5.8).
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn revoke_recovery_key(
    vault_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;
    core_revoke(&mut guard).await?;
    Ok(())
}

/// Whether a Recovery Key is enrolled — the derivative bool the Settings row reads (never the
/// slot). Requires an unlocked session.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn recovery_key_enrolled(
    vault_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<bool, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let guard = handle.lock().await;
    Ok(guard.has_recovery_key())
}
