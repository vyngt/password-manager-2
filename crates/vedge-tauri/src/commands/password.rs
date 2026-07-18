//! `change_password` — the KDF re-runs (`spawn_blocking`) inside
//! `vedge-core`, so the shell just forwards the zeroizing inputs.
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
    ChangePasswordInput, change_password as change_password_core, format_secret_key,
    reauthenticate_master_password,
};

use crate::dto::misc::{
    ChangePasswordInputDto, SecretKeyRotationOutputDto, decode_change_password_secret_key,
};
use crate::error::CommandError;
use crate::state::AppState;

fn vault_id_from_string(s: &str) -> VaultId {
    VaultId::new(PathBuf::from(s))
}

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn change_password(
    vault_path: String,
    input: ChangePasswordInputDto,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;

    // Refresh the hard TTL up front so the ~1-2 s KDF re-run below can't be reaped
    // mid-change and lock the user out from under a successful change (Decision 6
    // must hold unconditionally). The user is already authenticated in this
    // unlocked session, so extending it here is sound. Re-stamped again after
    // success to reset to a fresh full deadline.
    let ttl = crate::setup::services::session_ttl(&state).await;
    state.touch_deadline(&vault_id, ttl);

    let mut guard = handle.lock().await;

    // Prove knowledge of the *current* password against the live session before the
    // O(n) rewrap — an unlocked vault left open is exactly what this re-auth defends
    // (slice 5.6 ④). A wrong password returns `WrongCredentials` and mutates nothing.
    let current_password = Zeroizing::new(input.current_password.clone());
    reauthenticate_master_password(
        &guard,
        Arc::clone(&state.kdf),
        Arc::clone(&state.keychain),
        current_password,
    )
    .await?;

    let new_secret_key = decode_change_password_secret_key(&input)?;
    let new_password = Zeroizing::new(input.new_password.clone());

    change_password_core(
        &mut guard,
        Arc::clone(&state.kdf),
        Arc::clone(&state.keychain),
        Arc::clone(&state.biometric),
        ChangePasswordInput {
            new_password,
            new_secret_key,
        },
    )
    .await?;
    drop(guard);

    state.touch_deadline(&vault_id, ttl);
    Ok(())
}

/// Rotate the vault's Secret Key, keeping the same master password.
///
/// Generates a fresh key **server-side** (the CSPRNG lives in core), re-derives
/// every KEK/DEK under it via `change_password`, and returns the new key's
/// display string so the UI can show it once and re-issue the Emergency Kit.
/// Only the display string crosses to the frontend — the same show-once contract
/// as `create_vault`; the raw key never leaves the shell.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn rotate_secret_key(
    vault_path: String,
    current_password: String,
    state: tauri::State<'_, AppState>,
) -> Result<SecretKeyRotationOutputDto, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;

    // Same TTL guard as change_password — the rewrap is O(n) and slow.
    let ttl = crate::setup::services::session_ttl(&state).await;
    state.touch_deadline(&vault_id, ttl);

    let mut guard = handle.lock().await;

    // Prove knowledge of the current password before rotating (slice 5.6 ④).
    let current = Zeroizing::new(current_password);
    reauthenticate_master_password(
        &guard,
        Arc::clone(&state.kdf),
        Arc::clone(&state.keychain),
        current.clone(),
    )
    .await?;

    // Generate the fresh Secret Key server-side and rotate to it, keeping the
    // same master password. `format_secret_key` borrows before the key is moved
    // into the rewrap; only the display string is returned.
    let new_secret_key = state.crypto.generate_secret_key();
    let secret_key_display = format_secret_key(&new_secret_key);

    change_password_core(
        &mut guard,
        Arc::clone(&state.kdf),
        Arc::clone(&state.keychain),
        Arc::clone(&state.biometric),
        ChangePasswordInput {
            new_password: current,
            new_secret_key: Some(new_secret_key),
        },
    )
    .await?;
    drop(guard);

    state.touch_deadline(&vault_id, ttl);
    Ok(SecretKeyRotationOutputDto { secret_key_display })
}
