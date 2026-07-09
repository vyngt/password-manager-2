//! Known devices + extension sessions. Mirrors
//! `vedge-tauri/src/commands/device.rs`.

use serde::Serialize;

use vedge_ipc::{ExtensionSessionDto, KnownDeviceDto};

use crate::api::call::{call_noargs, call_void};
use crate::api::error::ApiError;

// ---- known_devices -----------------------------------------------------------

pub async fn list_known_devices() -> Result<Vec<KnownDeviceDto>, ApiError> {
    call_noargs("list_known_devices").await
}

pub async fn upsert_known_device(device: &KnownDeviceDto) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        device: &'a KnownDeviceDto,
    }
    call_void("upsert_known_device", &Args { device }).await
}

pub async fn delete_known_device(device_id: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        device_id: &'a str,
    }
    call_void("delete_known_device", &Args { device_id }).await
}

pub async fn touch_known_device_last_seen(device_id: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        device_id: &'a str,
    }
    call_void("touch_known_device_last_seen", &Args { device_id }).await
}

// ---- extension_sessions ------------------------------------------------------

pub async fn list_extension_sessions() -> Result<Vec<ExtensionSessionDto>, ApiError> {
    call_noargs("list_extension_sessions").await
}

pub async fn upsert_extension_session(session: &ExtensionSessionDto) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        session: &'a ExtensionSessionDto,
    }
    call_void("upsert_extension_session", &Args { session }).await
}

pub async fn delete_extension_session(session_id: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        session_id: &'a str,
    }
    call_void("delete_extension_session", &Args { session_id }).await
}

pub async fn touch_extension_session_last_active(session_id: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        session_id: &'a str,
    }
    call_void("touch_extension_session_last_active", &Args { session_id }).await
}
