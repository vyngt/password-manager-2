//! Entry wire types.
//!
//! - `IndexEntryDto` — read-side projection, non-secret only.
//! - Per-variant payload DTOs (`LoginPayloadDto`, …) are **bidirectional**: on
//!   the outbound (reveal) direction the sealed-secret fields carry a presence
//!   flag / count (the WASM-Secret Sentinel, slice 5.4); on the inbound
//!   (create/update) direction they carry a [`SecretUpdateDto`] intent. The seed
//!   / secret plaintext does **not** cross on `get_entry`.
//! - `PayloadDto` is the adjacently-tagged write-side union.
//!
//! `Note.content` and the non-`national_id` `Identity` fields are the entry's
//! substance (not credential material) and keep crossing plainly.

use serde::{Deserialize, Serialize};

use crate::common::{CommonMetaDto, EntryTypeDto};
use crate::secret_update::{EnvVarUpdateDto, SecretListUpdateDto, SecretUpdateDto};
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

// ---- Per-variant payload DTOs -----------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginPayloadDto {
    pub meta: CommonMetaDto,
    pub username: String,
    /// Sealed secret. Outbound: `Unchanged` (the plaintext does not cross the
    /// door). Inbound: `Set`/`Clear`/`Unchanged` intent.
    #[serde(default)]
    pub password: SecretUpdateDto,
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
    /// Sealed collection. Outbound: `Unchanged` (values do not cross); the count
    /// crosses as [`recovery_codes_count`](Self::recovery_codes_count).
    #[serde(default)]
    pub recovery_codes: SecretListUpdateDto,
    /// Outbound presence: how many recovery codes are stored (non-invertible).
    #[serde(default)]
    pub recovery_codes_count: u32,
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
    #[serde(default)]
    pub number: SecretUpdateDto,
    pub expiry_month: u8,
    pub expiry_year: u16,
    #[serde(default)]
    pub cvv: SecretUpdateDto,
    /// Optional sealed secret. Outbound presence: [`has_pin`](Self::has_pin).
    #[serde(default)]
    pub pin: SecretUpdateDto,
    #[serde(default)]
    pub has_pin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshKeyPayloadDto {
    pub meta: CommonMetaDto,
    #[serde(default)]
    pub private_key_pem: SecretUpdateDto,
    #[serde(default)]
    pub passphrase: SecretUpdateDto,
    #[serde(default)]
    pub has_passphrase: bool,
    pub public_key: String,
    pub fingerprint: String,
    pub key_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyPayloadDto {
    pub meta: CommonMetaDto,
    #[serde(default)]
    pub key: SecretUpdateDto,
    #[serde(default)]
    pub secret: SecretUpdateDto,
    #[serde(default)]
    pub has_secret: bool,
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
    /// Keys cross plainly (the schema); each value is a sealed-secret intent.
    #[serde(default)]
    pub vars: Vec<EnvVarUpdateDto>,
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
    /// The only sealed Identity field (a government ID is credential material,
    /// `SecretString`-wrapped domain-side). Outbound presence:
    /// [`has_national_id`](Self::has_national_id). The other Identity fields are
    /// PII, not credential material, and keep crossing plainly.
    #[serde(default)]
    pub national_id: SecretUpdateDto,
    #[serde(default)]
    pub has_national_id: bool,
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
    use crate::secret_update::{SecretListUpdateDto, SecretUpdateDto};
    use crate::totp::TotpUpdateDto;

    /// 🔴 Spec #3 — the safe failure mode. An update that never touches a sealed
    /// field sends NO field, so it deserializes as `Unchanged`. Test the ABSENCE.
    #[test]
    fn absent_sealed_fields_default_to_unchanged() {
        let dto: LoginPayloadDto = serde_json::from_value(serde_json::json!({
            "meta": { "name": "gh", "entry_type": "Login" },
            "username": "alice",
        }))
        .unwrap();
        assert!(matches!(dto.password, SecretUpdateDto::Unchanged));
        assert!(matches!(dto.recovery_codes, SecretListUpdateDto::Unchanged));
        assert!(matches!(dto.totp, TotpUpdateDto::Unchanged));
    }

    #[test]
    fn secret_update_set_round_trips() {
        let json = serde_json::to_string(&SecretUpdateDto::Set("v".into())).unwrap();
        assert_eq!(json, r#"{"kind":"Set","value":"v"}"#);
        let back: SecretUpdateDto = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, SecretUpdateDto::Set(s) if s == "v"));
    }

    /// The door's wire test: the outbound Login DTO exposes `has_totp` but never
    /// the raw seed. A stray `totp_secret` key would reopen it.
    #[test]
    fn login_dto_omits_totp_secret() {
        let dto: LoginPayloadDto = serde_json::from_value(serde_json::json!({
            "meta": { "name": "gh", "entry_type": "Login" },
            "username": "alice",
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
