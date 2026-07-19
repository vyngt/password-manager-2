//! Re-key (slice 5.8). Mirrors `vedge-tauri/src/commands/rekey.rs`.

use serde::Serialize;

use vedge_ipc::{RekeyInputDto, RekeyResultDto};

use crate::api::call::{call, call_void};
use crate::api::error::ApiError;

/// Re-key the vault: a fresh DEK per entry, every surface re-encrypted, old snapshots retired.
/// On success the vault LOCKS (`RekeyResultDto::cancelled == false`) and the caller navigates to
/// the launch screen; `secret_key_display` is `Some` only when the Secret Key was rotated.
pub async fn rekey_vault(
    vault_path: &str,
    input: &RekeyInputDto,
) -> Result<RekeyResultDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        input: &'a RekeyInputDto,
    }
    call("rekey_vault", &Args { vault_path, input }).await
}

/// Signal an in-flight re-key to cancel (before its commit point).
pub async fn cancel_rekey(vault_path: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
    }
    call_void("cancel_rekey", &Args { vault_path }).await
}
