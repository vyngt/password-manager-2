//! Error envelope that Tauri commands serialize on failure.
//!
//! `vedge-tauri`'s `CommandError` is tagged `#[serde(tag = "kind",
//! content = "message")]`, producing JSON like
//! `{"kind": "WrongCredentials", "message": "..."}` when a command
//! returns `Err`. This type is the wire contract — both sides deserialize
//! against it.

use serde::{Deserialize, Serialize};

/// Canonical kinds, kept in sync with `vedge_tauri::error::CommandError`
/// variants. Unknown kinds surface as `ApiError::Transport` on the
/// frontend.
pub mod kind {
    pub const WRONG_CREDENTIALS: &str = "WrongCredentials";
    pub const DECRYPTION: &str = "Decryption";
    pub const NOT_FOUND: &str = "NotFound";
    pub const VAULT_NOT_OPEN: &str = "VaultNotOpen";
    pub const KEYCHAIN: &str = "Keychain";
    pub const BIOMETRIC: &str = "Biometric";
    pub const STORAGE: &str = "Storage";
    pub const INVALID: &str = "Invalid";
    pub const ALREADY_EXISTS: &str = "AlreadyExists";
    pub const INTERNAL: &str = "Internal";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEnvelope {
    pub kind: String,
    #[serde(default)]
    pub message: Option<String>,
}
