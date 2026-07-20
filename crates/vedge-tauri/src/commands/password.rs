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
use vedge_core::{ChangePasswordInput, change_password as change_password_core};

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
    let mut guard = handle.lock().await;

    let new_secret_key = decode_change_password_secret_key(&input)?;
    let new_password = Zeroizing::new(input.new_password.clone());

    change_password_core(
        &mut guard,
        Arc::clone(&state.kdf),
        Arc::clone(&state.keychain),
        ChangePasswordInput {
            new_password,
            new_secret_key,
        },
    )
    .await?;
    Ok(())
}
