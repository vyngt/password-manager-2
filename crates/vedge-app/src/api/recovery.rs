//! Recovery Key api wrappers (slice 5.7). Mirrors `vedge-tauri/src/commands/recovery.rs`.

use serde::Serialize;

use vedge_ipc::{RecoveryEnrollOutputDto, UnlockWithRecoveryKeyInputDto};

use crate::api::call::{call, call_void};
use crate::api::error::ApiError;

/// Enroll a Recovery Key (re-auths the current password server-side). Returns the `RK1-`
/// display shown once so the user can print the Recovery Kit.
pub async fn enroll_recovery_key(
    vault_path: &str,
    current_password: &str,
) -> Result<RecoveryEnrollOutputDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        current_password: &'a str,
    }
    call(
        "enroll_recovery_key",
        &Args {
            vault_path,
            current_password,
        },
    )
    .await
}

/// Recover a vault whose master password is forgotten. On `Ok` the session is unlocked and
/// pending a forced password change — drive [`change_password_after_recovery`] next.
pub async fn unlock_with_recovery_key(
    input: &UnlockWithRecoveryKeyInputDto,
) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        input: &'a UnlockWithRecoveryKeyInputDto,
    }
    call_void("unlock_with_recovery_key", &Args { input }).await
}

/// Set the new master password after a recovery unlock (no old-password reauth — gated to a
/// just-recovered session by the core).
pub async fn change_password_after_recovery(
    vault_path: &str,
    new_password: &str,
) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        new_password: &'a str,
    }
    call_void(
        "change_password_after_recovery",
        &Args {
            vault_path,
            new_password,
        },
    )
    .await
}

/// Revoke the Recovery Key (deletes the slot from this vault).
pub async fn revoke_recovery_key(vault_path: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
    }
    call_void("revoke_recovery_key", &Args { vault_path }).await
}

/// Whether a Recovery Key is enrolled — the derivative bool the Settings row reads.
pub async fn recovery_key_enrolled(vault_path: &str) -> Result<bool, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
    }
    call("recovery_key_enrolled", &Args { vault_path }).await
}
