//! Recovery-kit unlock path. Mirrors
//! `vedge-tauri/src/commands/recovery.rs`.

use serde::Serialize;

use vedge_ipc::{RecoveryOutcomeDto, UnlockWithRecoveryKeyInputDto};

use crate::api::call::call;
use crate::api::error::ApiError;

pub async fn unlock_with_recovery_key(
    input: &UnlockWithRecoveryKeyInputDto,
) -> Result<RecoveryOutcomeDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        input: &'a UnlockWithRecoveryKeyInputDto,
    }
    call("unlock_with_recovery_key", &Args { input }).await
}
