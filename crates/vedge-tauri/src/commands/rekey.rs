//! Re-key command (slice 5.8) — the true re-key: a fresh DEK per entry, every ciphertext surface
//! re-encrypted, old snapshots retired, then the vault LOCKS.
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
    RekeyOutcome, RekeyVaultInput, format_secret_key, reauthenticate_master_password,
    rekey_vault as core_rekey,
};

use crate::dto::rekey::{RekeyInputDto, RekeyProgressDto, RekeyResultDto};
use crate::error::CommandError;
use crate::state::AppState;

fn vault_id_from_string(s: &str) -> VaultId {
    VaultId::new(PathBuf::from(s))
}

/// Re-key the unlocked vault.
///
/// Re-authenticates the current password, optionally rotates the Secret Key (generated
/// server-side, show-once), re-encrypts every surface crash-safely, retires the snapshot store,
/// and — on success — LOCKS the vault so the user re-unlocks against the re-keyed state (the 5.7
/// revert-locks precedent).
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn rekey_vault(
    vault_path: String,
    input: RekeyInputDto,
    on_progress: tauri::ipc::Channel<RekeyProgressDto>,
    state: tauri::State<'_, AppState>,
) -> Result<RekeyResultDto, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;

    // Re-key is O(total plaintext) and may take minutes; refresh the hard TTL up front so it is not
    // reaped mid-operation. (On success it LOCKS, so no post-op re-stamp is needed.)
    let ttl = crate::setup::services::session_ttl(&state).await;
    state.touch_deadline(&vault_id, ttl);

    let guard = handle.lock().await;

    // Prove knowledge of the CURRENT password before this destructive op (re-key retires
    // snapshots) — an unlocked vault left open is exactly what this defends. Wrong → mutates nothing.
    reauthenticate_master_password(
        &guard,
        Arc::clone(&state.kdf),
        Arc::clone(&state.keychain),
        Zeroizing::new(input.current_password.clone()),
    )
    .await?;

    // If rotating the Secret Key, mint it server-side (show-once); the raw key never leaves the shell.
    let (new_secret_key, secret_key_display) = if input.rotate_secret_key {
        let sk = state.crypto.generate_secret_key();
        let display = format_secret_key(&sk);
        (Some(sk), Some(display))
    } else {
        (None, None)
    };

    // A cancel flag reachable by `cancel_rekey` WITHOUT the session guard (it touches only the
    // outer lock), so the user can abort a long re-key before its commit point.
    let cancel = state.register_rekey_cancel(&vault_id);
    // Determinate progress STREAMED over a Tauri Channel: the callback pushes each `(done, total)`
    // to the frontend's `onmessage` as the re-key runs — no polling, no second command. A send
    // error (the frontend went away) is ignored; it never affects the re-key.
    let progress = move |done, total| {
        let _ = on_progress.send(RekeyProgressDto { done, total });
    };

    let outcome = core_rekey(
        &guard,
        Arc::clone(&state.kdf),
        Arc::clone(&state.keychain),
        Arc::clone(&state.biometric),
        RekeyVaultInput {
            new_password: Zeroizing::new(input.new_password.clone()),
            new_secret_key,
        },
        &progress,
        &cancel,
    )
    .await;
    state.clear_rekey_cancel(&vault_id);

    match outcome {
        Ok(RekeyOutcome::Rekeyed { .. }) => {
            // LOCK: the core closed the session DB; drop the guard + remove the slot so the user
            // re-unlocks against the re-keyed state.
            drop(guard);
            let _ = state.remove_session(&vault_id);
            Ok(RekeyResultDto {
                cancelled: false,
                secret_key_display,
            })
        }
        Ok(RekeyOutcome::Cancelled) => {
            // Untouched + still unlocked — keep the session alive on a fresh deadline.
            drop(guard);
            state.touch_deadline(&vault_id, ttl);
            Ok(RekeyResultDto {
                cancelled: true,
                secret_key_display: None,
            })
        }
        Ok(RekeyOutcome::CommitFailed { error }) => {
            // The journal rolled back (vault intact on the OLD password) but the session DB is
            // already closed → lock and surface the error.
            drop(guard);
            let _ = state.remove_session(&vault_id);
            Err(error.into())
        }
        Err(error) => {
            // Pre-commit failure — the session is still alive and the vault untouched.
            drop(guard);
            state.touch_deadline(&vault_id, ttl);
            Err(error.into())
        }
    }
}

/// Signal an in-flight re-key to cancel (before its commit point). Touches only the outer session
/// map lock, so it does NOT block on the session guard the running re-key holds.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn cancel_rekey(
    vault_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    state.signal_rekey_cancel(&vault_id);
    Ok(())
}
