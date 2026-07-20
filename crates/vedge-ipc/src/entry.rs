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
    pub is_trashed: bool,
    pub cipher_suite: i32,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub accessed_at: Option<String>,
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
    #[serde(default)]
    pub totp_secret: Option<String>,
    #[serde(default)]
    pub recovery_codes: Vec<String>,
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
