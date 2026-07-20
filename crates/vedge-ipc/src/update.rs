//! Update-check DTOs (PG.3).
//!
//! The check runs Rust-side (`vedge-tauri/src/commands/update.rs`, using the
//! `tauri_plugin_http::reqwest` re-export HIBP already uses) so the frontend
//! needs no HTTP capability and the CSP's `connect-src` stays closed. These
//! types are the only thing that crosses the boundary.

use serde::{Deserialize, Serialize};

/// The outcome of an update check against the public GitHub releases API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateCheckDto {
    /// The running app version (`app.package_info().version`), e.g. `"1.0.0"`.
    pub current: String,
    /// The latest published release version, normalised (a leading `v` stripped),
    /// when one was found. `None` when no release is published yet, or the check
    /// could not be completed.
    pub latest: Option<String>,
    pub status: UpdateStatus,
    /// The release's browser URL, for a "Download" link. Only set when an update
    /// is actually available (never a downgrade link).
    pub release_url: Option<String>,
}

/// The verdict of an update check.
///
/// 🔴 Failure is a *status*, not an error: the command always returns a
/// `UpdateCheckDto`, so the manual caller can show an inline message on
/// [`Unknown`](UpdateStatus::Unknown) while the automatic caller stays silent —
/// without either having to distinguish an `Err`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateStatus {
    /// Running the newest release, ahead of it (a dev build off `develop` —
    /// never a downgrade prompt), or no release is published yet.
    UpToDate,
    /// A newer stable release exists.
    UpdateAvailable,
    /// The check could not determine an answer — network failure, a non-success
    /// response, or an unparseable tag. Manual → inline message; auto → silent.
    Unknown,
}
