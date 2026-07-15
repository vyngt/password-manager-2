//! Structured content for the Emergency Kit PDF.
//!
//! This type is produced by
//! [`export_emergency_kit`](crate::application::vault::use_cases::export_emergency_kit)
//! and consumed by whatever renderer the shell uses (today: `printpdf` in
//! `vedge-tauri`). It contains **no** cryptographic material in binary
//! form — the Secret Key arrives as a pre-formatted display string so the
//! caller can choose whatever rendering (PDF, on-screen, QR) without
//! handling raw bytes.
//!
//! Zeroization contract: the display string is explicitly out of scope for
//! zeroization — the whole point of the kit is that the user reads it and
//! writes it down. Callers (renderer code) must take care not to linger on
//! the string after the user has stored it.

use crate::domain::shared::Timestamp;

#[derive(Debug, Clone)]
pub struct EmergencyKitContent {
    /// Display name the user gave this vault (from `vault_registry`). Empty
    /// if the vault isn't in the recent list — shell should substitute a
    /// default like "Vedge Vault" before rendering.
    pub vault_name: String,

    /// Canonical `.vdb` path, so users with several vaults can tell two
    /// kits apart by pathname.
    pub vault_path: String,

    /// Timestamp at export time. The renderer chooses the display format.
    pub generated_at: Timestamp,

    /// `A3-XXXXX-XXXXX-…-XXXXX` — safe to print. Produced via
    /// [`format_secret_key`](crate::domain::vault::recovery::format_secret_key).
    pub secret_key_display: String,

    /// Argon2id params in effect when the vault was created, stringified.
    /// Included so a future `vedge-core` with bumped defaults can still
    /// confirm the kit matches the vault.
    pub kdf_params_summary: String,
}
