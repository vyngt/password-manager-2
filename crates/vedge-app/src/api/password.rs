//! Change password + rotate Secret Key. Mirrors `vedge-tauri/src/commands/password.rs`.

use serde::Serialize;

use vedge_ipc::{ChangePasswordInputDto, SecretKeyRotationOutputDto};

use crate::api::call::{call, call_void};
use crate::api::error::ApiError;

pub async fn change_password(
    vault_path: &str,
    input: &ChangePasswordInputDto,
) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        input: &'a ChangePasswordInputDto,
    }
    call_void("change_password", &Args { vault_path, input }).await
}

/// Rotate the vault's Secret Key (keeping the master password). Generation is
/// server-side; the returned display string is shown once so the user can
/// re-issue their Emergency Kit.
pub async fn rotate_secret_key(
    vault_path: &str,
    current_password: &str,
) -> Result<SecretKeyRotationOutputDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        current_password: &'a str,
    }
    call(
        "rotate_secret_key",
        &Args {
            vault_path,
            current_password,
        },
    )
    .await
}
