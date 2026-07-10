//! Hardened OS-clipboard write for renderer-generated secrets.
//!
//! The password generator (slice 3.1) produces secrets in the WASM renderer,
//! where the browser Clipboard API can't set the platform exclusion hints. This
//! command routes such a secret through the same `ArboardClipboardProvider` the
//! backend `copy_field` path uses (`AppState.clipboard`) — so it inherits the
//! no-history / no-cloud hardening (slice 3.2) — and schedules the same
//! background auto-clear. It never touches the vault, so no session is required.
//!
//! The secret crosses IPC exactly once into the OS clipboard and is zeroized on
//! this (native) side immediately after the write — the same discipline as
//! `copy_field`'s decrypted field. Copying to the OS clipboard fundamentally
//! needs native access, so there is no hardened WASM-only path.
//!
//! See `commands/vault.rs` for the `#![allow]` rationale (the
//! `#[tauri::command(rename_all = "snake_case")]` expansion trips these).

#![allow(
    clippy::unreachable,
    clippy::let_underscore_must_use,
    clippy::significant_drop_tightening
)]

use tracing::instrument;
use vedge_core::place_text_on_clipboard;
use zeroize::Zeroizing;

use crate::error::CommandError;
use crate::state::AppState;

/// Place a secret on the hardened clipboard and schedule its background clear.
///
/// `clear_after_secs = None` uses the 30 s default. `text` is a secret — the
/// span records only its byte length, never its contents.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(bytes = text.len()))]
pub async fn copy_text(
    text: String,
    clear_after_secs: Option<u32>,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    place_text_on_clipboard(
        &state.clipboard,
        Zeroizing::new(text),
        clear_after_secs.unwrap_or(30),
    )?;
    Ok(())
}
