//! Re-key wire DTOs (slice 5.8).
//!
//! 🔴 Nothing secret crosses OUT: no DEK, no plaintext body, no KEK. The credentials cross IN
//! (current + new password) exactly like `change_password`; the only material crossing back is
//! the show-once new Secret-Key display when the SK was rotated — the accepted create-vault /
//! rotate-SK show-once contract.

use serde::{Deserialize, Serialize};

/// Re-key inputs. The new password is required (re-key is a credential-reset moment); the Secret
/// Key is rotated only on request and is generated server-side (never sent from the frontend).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RekeyInputDto {
    /// Proof of knowledge of the CURRENT password — re-key is destructive (it retires snapshots),
    /// so it re-authenticates like change-password / rotate-SK.
    pub current_password: String,
    pub new_password: String,
    /// Also mint a fresh Secret Key (recommended in a breach). Generated server-side; only its
    /// show-once display crosses back in [`RekeyResultDto`].
    #[serde(default)]
    pub rotate_secret_key: bool,
}

/// Re-key result. On success the vault is LOCKED (the frontend navigates to the launch screen);
/// on a pre-commit cancel it is untouched and still unlocked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RekeyResultDto {
    /// `true` if the user cancelled before the commit point — the vault is untouched and still
    /// unlocked. `false` means the re-key committed and the vault is now LOCKED.
    pub cancelled: bool,
    /// The show-once new Secret-Key display — `Some` only when the SK was rotated (re-issue the
    /// Emergency Kit before navigating to the launch screen).
    #[serde(default)]
    pub secret_key_display: Option<String>,
}
