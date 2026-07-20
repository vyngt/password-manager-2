//! Version + update check (PG.3). Both are no-arg commands: the app version and
//! the GitHub release fetch happen Rust-side (`vedge-tauri/src/commands/update.rs`),
//! so the frontend needs no HTTP capability and PG.2b's `connect-src` stays closed.

use vedge_ipc::UpdateCheckDto;

use crate::api::call::call_noargs;
use crate::api::error::ApiError;

/// The running app version (`app.package_info().version`), e.g. `"1.0.0"`.
pub async fn app_version() -> Result<String, ApiError> {
    call_noargs("app_version").await
}

/// Check the public GitHub releases for a newer version. A failure to reach or
/// parse the feed comes back as [`UpdateStatus::Unknown`](vedge_ipc::UpdateStatus),
/// not an `Err` — the manual caller shows an inline message, the auto caller stays
/// silent.
pub async fn check_for_update() -> Result<UpdateCheckDto, ApiError> {
    call_noargs("check_for_update").await
}
