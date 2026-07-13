//! Biometric unlock (Windows Hello / Touch ID). Mirrors
//! `vedge-tauri/src/commands/biometric.rs`.
//!
//! `available`/`is_enrolled` return plain flags; `enroll`/`unlock`/`disable` return `()`.
//! `unlock` reuses the normal unlock result — a biometric unlock lands the session in
//! shell state exactly like a password unlock, so callers navigate the same way.

use serde::Serialize;

use vedge_ipc::UnlockResultDto;

use crate::api::call::{call, call_noargs, call_void};
use crate::api::error::ApiError;

/// Whether biometric hardware is present and usable on this device.
pub async fn available() -> Result<bool, ApiError> {
    call_noargs("biometric_available").await
}

/// Whether this vault currently has a KEK stored behind the biometric gate.
pub async fn is_enrolled(vault_path: &str) -> Result<bool, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
    }
    call("biometric_is_enrolled", &Args { vault_path }).await
}

/// Enroll the unlocked vault for biometric unlock. Requires the current master password
/// (re-prompt) to authorize; a wrong password yields [`ApiError::WrongCredentials`].
pub async fn enroll(vault_path: &str, master_password: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        master_password: &'a str,
    }
    call_void(
        "biometric_enroll",
        &Args {
            vault_path,
            master_password,
        },
    )
    .await
}

/// Unlock the vault via the biometric gate (shows the OS prompt). No master password.
pub async fn unlock(vault_path: &str) -> Result<UnlockResultDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
    }
    call("biometric_unlock", &Args { vault_path }).await
}

/// Remove the stored KEK for this vault. Idempotent.
pub async fn disable(vault_path: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
    }
    call_void("biometric_disable", &Args { vault_path }).await
}
