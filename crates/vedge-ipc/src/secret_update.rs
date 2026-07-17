//! Sealed-secret update intents (slice 5.4).
//!
//! After the WASM-Secret Sentinel seals the outbound door, `get_entry` no longer
//! ships a secret field's plaintext — it ships a presence flag / count. The
//! bidirectional payload DTOs therefore carry an **intent** on the inbound
//! (create/update) direction instead of a value, mirroring `TotpUpdateDto`.
//!
//! `Unchanged` is the serde default on every one of these — a form that never
//! touches a field preserves the stored secret. That is the whole safety
//! property: **a dropped field preserves the secret; it never wipes it.**
//!
//! These are structurally identical to [`TotpUpdateDto`](crate::totp::TotpUpdateDto)
//! but kept DISTINCT: a TOTP `Set` carries a Base32 seed with its own validation,
//! a `SecretUpdateDto::Set` carries arbitrary secret bytes. One named type per
//! concept — a future TOTP-only variant must not leak into `password`/`cvv`.

use serde::{Deserialize, Serialize};

/// Scalar sealed-secret intent.
///
/// Backs password, card number/cvv/pin, ssh key/passphrase, api key/secret, and
/// identity `national_id`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum SecretUpdateDto {
    /// Keep the stored value (the serde default — the safe failure mode).
    #[default]
    Unchanged,
    /// Replace with this value.
    Set(String),
    /// Remove the value (optional fields only; required fields reject this
    /// in the use case).
    Clear,
}

/// Ordered-list sealed-secret intent (`Login.recovery_codes`).
///
/// `Set` replaces the whole list; `Unchanged` carries the stored list forward;
/// `Clear` empties it. The recovery-codes field has no form editor, so the form
/// only ever emits `Unchanged` — carry-forward is what keeps an edit from wiping
/// the codes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum SecretListUpdateDto {
    #[default]
    Unchanged,
    Set(Vec<String>),
    Clear,
}

/// One keyed row's intent (`EnvVars`).
///
/// The key is non-secret and crosses plainly; the value is an intent. Removal is
/// expressed by **omitting** the row under the complete-replacement contract, so
/// a present row carries only `Set`/`Unchanged` (a `Clear` on a row is rejected
/// in the use case).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvVarUpdateDto {
    pub key: String,
    #[serde(default)]
    pub value: SecretUpdateDto,
}
