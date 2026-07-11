//! Entry wire types.
//!
//! - `IndexEntryDto` — read-side projection, non-secret only.
//! - Per-variant payload DTOs (`LoginPayloadDto`, …) carry secrets as
//!   `String` / `Vec<String>` on the wire.
//! - `PayloadDto` is the adjacently-tagged write-side union.
//!
//! Secret fields stay plain `String` on the wire — zeroization is the
//! domain layer's responsibility (vedge-tauri wraps them into
//! `SecretString` during conversion).

use serde::{Deserialize, Serialize};

use crate::common::{CommonMetaDto, EntryTypeDto};
use crate::totp::{TotpAlgorithmDto, TotpUpdateDto};

// ---- IndexEntryDto -----------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntryDto {
    pub id: String,
    pub name: String,
    pub entry_type: EntryTypeDto,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub favicon_url: Option<String>,
    #[serde(default)]
    pub tag_ids: Vec<String>,
    #[serde(default)]
    pub folder_id: Option<String>,
    pub is_favorite: bool,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub sort_order: u32,
    pub is_trashed: bool,
    pub cipher_suite: i32,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub accessed_at: Option<String>,
}

// ---- HistoryEntryDto ---------------------------------------------------------

/// One version in an entry's history timeline — metadata only, no secret values.
///
/// `history_id = None` marks the current live version; the rest are snapshots.
/// `changed_fields` are field-name keys that differ from the next-older version
/// (the changed-field chips); empty for the origin version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntryDto {
    #[serde(default)]
    pub history_id: Option<String>,
    pub version: u32,
    pub changed_at: String,
    #[serde(default)]
    pub changed_fields: Vec<String>,
}

// ---- AddressDto --------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressDto {
    pub line1: String,
    #[serde(default)]
    pub line2: Option<String>,
    pub city: String,
    #[serde(default)]
    pub state: Option<String>,
    pub postal_code: String,
    pub country: String,
}

// ---- EnvVarDto ---------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVarDto {
    pub key: String,
    pub value: String,
}

// ---- Per-variant payload DTOs -----------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginPayloadDto {
    pub meta: CommonMetaDto,
    pub username: String,
    pub password: String,
    /// Non-invertible presence flag — the seed itself does **not** cross to WASM
    /// (the 4.2 door). Drives the detail view (show TOTP?) and the edit form
    /// (add vs replace/remove).
    #[serde(default)]
    pub has_totp: bool,
    /// TOTP metadata — safe to cross (the form must edit it).
    #[serde(default)]
    pub totp_algorithm: TotpAlgorithmDto,
    #[serde(default = "default_totp_digits")]
    pub totp_digits: u8,
    #[serde(default = "default_totp_period")]
    pub totp_period: u32,
    /// Inbound enrolment intent; ignored on the outbound (reveal) direction.
    #[serde(default)]
    pub totp: TotpUpdateDto,
    #[serde(default)]
    pub recovery_codes: Vec<String>,
}

const fn default_totp_digits() -> u8 {
    6
}

const fn default_totp_period() -> u32 {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardPayloadDto {
    pub meta: CommonMetaDto,
    pub cardholder_name: String,
    pub number: String,
    pub expiry_month: u8,
    pub expiry_year: u16,
    pub cvv: String,
    #[serde(default)]
    pub pin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshKeyPayloadDto {
    pub meta: CommonMetaDto,
    pub private_key_pem: String,
    #[serde(default)]
    pub passphrase: Option<String>,
    pub public_key: String,
    pub fingerprint: String,
    pub key_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyPayloadDto {
    pub meta: CommonMetaDto,
    pub key: String,
    #[serde(default)]
    pub secret: Option<String>,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub expiry: Option<String>,
    #[serde(default)]
    pub key_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVarsPayloadDto {
    pub meta: CommonMetaDto,
    #[serde(default)]
    pub vars: Vec<EnvVarDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotePayloadDto {
    pub meta: CommonMetaDto,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentPayloadDto {
    pub meta: CommonMetaDto,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: u64,
    /// Base64 of the 24-byte nonce used to encrypt the sidecar blob.
    pub blob_nonce_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityPayloadDto {
    pub meta: CommonMetaDto,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub address: Option<AddressDto>,
    #[serde(default)]
    pub date_of_birth: Option<String>,
    #[serde(default)]
    pub national_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderPayloadDto {
    pub meta: CommonMetaDto,
}

// ---- PayloadDto (adjacently tagged union) -----------------------------------

/// Adjacently-tagged write-side union: `{ "entry_type": "Login",
/// "data": { ... } }`. `Unknown` has no DTO — it's a read-side-only
/// domain variant; `create_entry` / `update_entry` can't produce one.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "entry_type", content = "data")]
pub enum PayloadDto {
    Login(LoginPayloadDto),
    Card(CardPayloadDto),
    SshKey(SshKeyPayloadDto),
    ApiKey(ApiKeyPayloadDto),
    EnvVars(EnvVarsPayloadDto),
    Note(NotePayloadDto),
    Document(DocumentPayloadDto),
    Identity(IdentityPayloadDto),
    Folder(FolderPayloadDto),
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::{HistoryEntryDto, LoginPayloadDto};

    /// The door's wire test: the outbound Login DTO exposes `has_totp` but never
    /// the raw seed. A stray `totp_secret` key would reopen it.
    #[test]
    fn login_dto_omits_totp_secret() {
        let dto: LoginPayloadDto = serde_json::from_value(serde_json::json!({
            "meta": { "name": "gh", "entry_type": "Login" },
            "username": "alice",
            "password": "pw",
            "has_totp": true,
            "totp_algorithm": "Sha1",
            "totp_digits": 6,
            "totp_period": 30,
        }))
        .unwrap();
        let out = serde_json::to_value(&dto).unwrap();
        assert!(
            out.get("totp_secret").is_none(),
            "the seed must not cross the door"
        );
        assert_eq!(out.get("has_totp"), Some(&serde_json::Value::Bool(true)));
        assert_eq!(out.get("totp_digits"), Some(&serde_json::json!(6)));
    }

    #[test]
    fn history_entry_snapshot_round_trips() {
        let dto = HistoryEntryDto {
            history_id: Some("01J8XH".into()),
            version: 3,
            changed_at: "2026-07-03T09:00:00+00:00".into(),
            changed_fields: vec!["password".into(), "number".into()],
        };
        let json = serde_json::to_string(&dto).unwrap();
        let back: HistoryEntryDto = serde_json::from_str(&json).unwrap();
        assert_eq!(back.history_id, dto.history_id);
        assert_eq!(back.version, dto.version);
        assert_eq!(back.changed_at, dto.changed_at);
        assert_eq!(back.changed_fields, dto.changed_fields);
    }

    #[test]
    fn history_entry_current_row_defaults() {
        // The current live version rides as `history_id = None`; `changed_fields`
        // defaults to empty when omitted.
        let back: HistoryEntryDto =
            serde_json::from_str(r#"{"version":5,"changed_at":"2026-07-08T00:00:00Z"}"#).unwrap();
        assert!(back.history_id.is_none());
        assert!(back.changed_fields.is_empty());
        assert_eq!(back.version, 5);
    }
}
