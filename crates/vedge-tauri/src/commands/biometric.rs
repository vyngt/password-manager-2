//! Biometric unlock commands (Windows Hello / Touch ID).
//!
//! `enroll`/`unlock` route through the [`BiometricAuthenticator`] port + core use cases;
//! `available`/`is_enrolled`/`disable` are thin port pass-throughs. On enroll the KEK is
//! captured from the re-derived password; on unlock the gate releases the KEK and hands
//! it to `unlock_with_kek` — no master password crosses the boundary for a biometric
//! unlock.
//!
//! See `commands/vault.rs` for the `#![allow]` rationale (macro-generated lints).

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
use vedge_core::enroll_biometric as enroll_biometric_core;

use crate::error::CommandError;
use crate::state::AppState;

fn vault_id_from_string(s: &str) -> VaultId {
    VaultId::new(PathBuf::from(s))
}

/// Whether biometric hardware is present and usable on this device. Never prompts.
#[tauri::command(rename_all = "snake_case")]
pub async fn biometric_available(state: tauri::State<'_, AppState>) -> Result<bool, CommandError> {
    Ok(state.biometric.is_available())
}

/// Whether this vault currently has a KEK stored behind the biometric gate. Never
/// prompts.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn biometric_is_enrolled(
    vault_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<bool, CommandError> {
    Ok(state
        .biometric
        .is_enrolled(&vault_id_from_string(&vault_path))?)
}

/// Enroll the (already-unlocked) vault for biometric unlock.
///
/// Verifies the re-prompted master password, then stores the derived KEK behind the gate
/// (shows the OS prompt). A wrong password returns `WrongCredentials` and stores nothing.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn biometric_enroll(
    vault_path: String,
    master_password: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let guard = handle.lock().await;

    enroll_biometric_core(
        &guard,
        Arc::clone(&state.kdf),
        Arc::clone(&state.keychain),
        Arc::clone(&state.biometric),
        Zeroizing::new(master_password),
    )
    .await?;
    Ok(())
}

/// Unlock a vault via the biometric gate: prompt → release the KEK → `unlock_with_kek`.
///
/// No master password required. A wrong/absent/cancelled key surfaces as
/// `WrongCredentials` / `Biometric` so the UI falls back to the password screen.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn biometric_unlock(
    vault_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let path = PathBuf::from(&vault_path);
    let vault_id = VaultId::new(path.clone());

    if state.is_unlocked(&vault_id) {
        return Err(CommandError::Invalid(format!(
            "vault already unlocked: {}",
            path.display()
        )));
    }

    let kek = state.biometric.retrieve(&vault_id)?;
    let session = state.unlock_vault.unlock_with_kek(path, kek).await?;
    state.insert_session(
        vault_id,
        session,
        crate::setup::services::session_ttl(&state).await,
    )?;
    Ok(())
}

/// Remove the stored KEK for this vault. Idempotent — disabling when not enrolled is not
/// an error.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn biometric_disable(
    vault_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    state
        .biometric
        .disable(&vault_id_from_string(&vault_path))?;
    Ok(())
}
