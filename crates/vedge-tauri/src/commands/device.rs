//! `known_devices` + `extension_sessions` app.db commands. Both tables
//! hold IPC-related public keys / session keys; exposure lets an attacker
//! *impersonate* peers but does not unlock vaults.
//!
//! See `commands/vault.rs` for the `#![allow]` rationale (no session
//! mutex here, but the macro-generated patterns still trip the first
//! two lints).

#![allow(clippy::unreachable, clippy::let_underscore_must_use)]

use tracing::instrument;

use vedge_core::domain::shared::{now, DeviceId, SessionId};

use crate::dto::settings::{ExtensionSessionDto, KnownDeviceDto};
use crate::error::CommandError;
use crate::state::AppState;

// ---- known_devices -----------------------------------------------------------

#[tauri::command]
#[instrument(skip_all)]
pub async fn list_known_devices(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<KnownDeviceDto>, CommandError> {
    let rows = state.known_devices.list().await?;
    Ok(rows.iter().map(KnownDeviceDto::from).collect())
}

#[tauri::command]
#[instrument(skip_all, fields(device_id = %device.device_id))]
pub async fn upsert_known_device(
    device: KnownDeviceDto,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let domain = device.into_domain()?;
    state.known_devices.upsert(&domain).await?;
    Ok(())
}

#[tauri::command]
#[instrument(skip_all, fields(device_id = %device_id))]
pub async fn delete_known_device(
    device_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let did = DeviceId::from_raw(device_id);
    state.known_devices.delete(&did).await?;
    Ok(())
}

#[tauri::command]
#[instrument(skip_all, fields(device_id = %device_id))]
pub async fn touch_known_device_last_seen(
    device_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let did = DeviceId::from_raw(device_id);
    state.known_devices.touch_last_seen(&did, now()).await?;
    Ok(())
}

// ---- extension_sessions ------------------------------------------------------

#[tauri::command]
#[instrument(skip_all)]
pub async fn list_extension_sessions(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ExtensionSessionDto>, CommandError> {
    let rows = state.extension_sessions.list().await?;
    Ok(rows.iter().map(ExtensionSessionDto::from).collect())
}

#[tauri::command]
#[instrument(skip_all, fields(session_id = %session.session_id))]
pub async fn upsert_extension_session(
    session: ExtensionSessionDto,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let domain = session.into_domain()?;
    state.extension_sessions.upsert(&domain).await?;
    Ok(())
}

#[tauri::command]
#[instrument(skip_all, fields(session_id = %session_id))]
pub async fn delete_extension_session(
    session_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let sid = SessionId::from_raw(session_id);
    state.extension_sessions.delete(&sid).await?;
    Ok(())
}

#[tauri::command]
#[instrument(skip_all, fields(session_id = %session_id))]
pub async fn touch_extension_session_last_active(
    session_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let sid = SessionId::from_raw(session_id);
    state
        .extension_sessions
        .touch_last_active(&sid, now())
        .await?;
    Ok(())
}
