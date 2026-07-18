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
    ChangePasswordInput, change_password as change_password_core, reauthenticate_master_password,
};

use crate::dto::misc::{ChangePasswordInputDto, decode_change_password_secret_key};
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
