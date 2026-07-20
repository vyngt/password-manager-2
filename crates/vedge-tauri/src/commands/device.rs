//! `known_devices` + `extension_sessions` app.db commands. Every command
//! routes through a `vedge-core` use case.
//!
//! See `commands/vault.rs` for the `#![allow]` rationale.

#![allow(clippy::unreachable, clippy::let_underscore_must_use)]

use tracing::instrument;

use vedge_core::domain::shared::{DeviceId, SessionId};
use vedge_core::{
    delete_extension_session as delete_extension_session_core,
    delete_known_device as delete_known_device_core,
    list_extension_sessions as list_extension_sessions_core,
    list_known_devices as list_known_devices_core,
    touch_extension_session_last_active as touch_extension_session_last_active_core,
    touch_known_device_last_seen as touch_known_device_last_seen_core,
    upsert_extension_session as upsert_extension_session_core,
    upsert_known_device as upsert_known_device_core,
};

use crate::dto::settings::{
    ExtensionSessionDto, KnownDeviceDto, extension_session_from_dto, extension_session_to_dto,
    known_device_from_dto, known_device_to_dto,
};
use crate::error::CommandError;
use crate::state::AppState;

// ---- known_devices -----------------------------------------------------------

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all)]
pub async fn list_known_devices(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<KnownDeviceDto>, CommandError> {
    let rows = list_known_devices_core(&*state.known_devices).await?;
    Ok(rows.iter().map(known_device_to_dto).collect())
}

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(device_id = %device.device_id))]
pub async fn upsert_known_device(
    device: KnownDeviceDto,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let domain = known_device_from_dto(device)?;
    upsert_known_device_core(&*state.known_devices, &domain).await?;
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(device_id = %device_id))]
pub async fn delete_known_device(
    device_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let did = DeviceId::from_raw(device_id);
    delete_known_device_core(&*state.known_devices, &did).await?;
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(device_id = %device_id))]
pub async fn touch_known_device_last_seen(
    device_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let did = DeviceId::from_raw(device_id);
    touch_known_device_last_seen_core(&*state.known_devices, &did).await?;
    Ok(())
}

// ---- extension_sessions ------------------------------------------------------

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all)]
pub async fn list_extension_sessions(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ExtensionSessionDto>, CommandError> {
    let rows = list_extension_sessions_core(&*state.extension_sessions).await?;
    Ok(rows.iter().map(extension_session_to_dto).collect())
}

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(session_id = %session.session_id))]
pub async fn upsert_extension_session(
    session: ExtensionSessionDto,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let domain = extension_session_from_dto(session)?;
    upsert_extension_session_core(&*state.extension_sessions, &domain).await?;
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(session_id = %session_id))]
pub async fn delete_extension_session(
    session_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let sid = SessionId::from_raw(session_id);
    delete_extension_session_core(&*state.extension_sessions, &sid).await?;
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(session_id = %session_id))]
pub async fn touch_extension_session_last_active(
    session_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let sid = SessionId::from_raw(session_id);
    touch_extension_session_last_active_core(&*state.extension_sessions, &sid).await?;
    Ok(())
}
