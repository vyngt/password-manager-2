//! Recovery API — wrapper over `unlock_with_recovery_key`.
//!
//! ⚠️ Deleted once by 2.10.1's dead-code sweep while the UI was pending
//! (Iteration Standards §7 — a sweep can't tell "unused" from "not yet
//! wired"). Wired to the unlock screen in slice 5.1 — do not re-sweep.

use serde::Serialize;

use vedge_ipc::{RecoveryOutcomeDto, UnlockWithRecoveryKeyInputDto};

use crate::api::call::call;
use crate::api::error::ApiError;

/// Unlock a vault whose OS-keychain entry is missing, using the master password
/// plus the `A3-…` Secret Key from the printed Emergency Kit. On success the
/// shell inserts the session itself (same path as `unlock_vault`) and the
/// outcome reports whether the keychain entry was restored on this device.
pub async fn unlock_with_recovery_key(
    input: &UnlockWithRecoveryKeyInputDto,
) -> Result<RecoveryOutcomeDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        input: &'a UnlockWithRecoveryKeyInputDto,
    }
    call("unlock_with_recovery_key", &Args { input }).await
}
