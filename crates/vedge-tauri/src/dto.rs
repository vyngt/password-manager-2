//! DTO layer. Every domain type that crosses the Tauri IPC boundary has a
//! matching Serialize+Deserialize wrapper here.
//!
//! Rules (mirrored from `docs/ImplementShellRewire.md` Phase 4):
//!
//! - **No `SecretString` in DTOs.** Secrets cross as `String`; the
//!   converter wraps them in `SecretString` before handing the input to a
//!   use case. The incoming `serde_json::Value` is transient and its
//!   backing storage drops as soon as the command returns.
//! - **IDs cross as `String`.** `EntryId` / `TagId` / `ThemeId` /
//!   `DeviceId` / `SessionId` round-trip via their `from_raw` / `as_str`
//!   pair. `VaultId` is a path and crosses as a path-string.
//! - **Timestamps cross as RFC-3339 strings.** Matches the on-disk format
//!   and the JS `Date` / `Temporal` constructors.
//! - **Binary blobs cross as base64 strings on output.** Inputs may use
//!   `Vec<u8>` (e.g. document import) where the JS array route is
//!   preferred.

pub mod audit;
pub mod backup;
pub mod common;
pub mod emergency_kit;
pub mod entry;
pub mod health;
pub mod misc;
pub mod settings;
pub mod snapshot;
pub mod tag;
pub mod totp;
