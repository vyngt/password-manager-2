//! The export DTO — `VEdge`'s **one permanent on-disk format** (slice 5.3).
//!
//! # 🔴 Decision ①: this is NOT `EntryPayload`, and it must never become it
//!
//! Once a single export file exists on a user's disk, **every future `VEdge` must
//! read it — forever.** The at-rest `EntryPayload` JSON, by contrast, is pinned
//! by golden tests (`tests/payload_wire_format.rs`) precisely so it stays *free
//! to change* (the JSON→binary decision — see `05 Research/Core Hardening/04 -
//! Memory Hygiene Gaps` item 4). If export re-used `EntryPayload`, every future
//! change to the on-disk encoding would break every export ever written, and
//! that decision would be frozen forever by a *file format*.
//!
//! So the types here bind **zero** vault schema. They deliberately duplicate the
//! shape of the payload structs ([`ExportMeta`] vs `CommonMeta`,
//! [`ExportTotpParams`] vs `TotpParams`, [`ExportAddress`] vs `Address`) rather
//! than reference them. **The duplication IS the decoupling — do not "DRY" the
//! two together.** The only external types that appear here are `SecretString`
//! (a hygiene wrapper, not a schema) and `serde_json::Value` (the opaque carrier
//! for [`ExportUnknown`]).
//!
//! # Shape
//!
//! [`ExportBundle`] is the top-level object serialized as the `entries.json`
//! member of the sealed envelope's tar. Each [`ExportEntry`] is an
//! internally-tagged (`type`) newtype variant wrapping a per-type struct with a
//! **nested** [`ExportMeta`] — deliberately not `#[serde(flatten)]`, which is a
//! known footgun combined with internally-tagged enums; a permanent format
//! values robustness over a flatter object.
//!
//! Document *bytes* do not live in this DTO. An [`ExportEntry::Document`] carries
//! only its metadata; its decrypted plaintext travels as a separate tar member
//! named `blobs/{meta.id}` (see the export use case). Import re-encrypts it under
//! a fresh DEK.
//!
//! `meta.id` / `meta.folder_id` carry the **source** ULIDs verbatim, as
//! batch-local keys. Import (5.3b) mints fresh ULIDs and remaps `folder_id`
//! through the batch; a `folder_id` absent from the batch imports flat.

use secrecy::SecretString;
use serde::{Deserialize, Serialize};

use crate::domain::vault::payloads::serde_secret::{
    expose_optional_secret_string, expose_secret_string, expose_secret_string_vec,
};

/// Bumped **only** on a breaking change to the export format.
///
/// 🔴 Read with `>` (accept older, refuse newer) — never `!=`. See the envelope
/// header's version guard; H1 has shipped twice in this phase.
pub const EXPORT_FORMAT_VERSION: u32 = 1;

/// The top-level bundle, serialized as the `entries.json` member of the tar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportBundle {
    /// A reader that does not recognise this version MUST refuse (never guess).
    pub format_version: u32,
    pub entries: Vec<ExportEntry>,
}

impl ExportBundle {
    /// A bundle at the current format version.
    #[must_use]
    pub const fn new(entries: Vec<ExportEntry>) -> Self {
        Self {
            format_version: EXPORT_FORMAT_VERSION,
            entries,
        }
    }
}

// ---------------------------------------------------------------------------
// Shared, export-owned metadata (mirrors `CommonMeta` — deliberately separate).
// ---------------------------------------------------------------------------

#[allow(clippy::trivially_copy_pass_by_ref)] // serde `skip_serializing_if` requires `&T`
const fn is_false(b: &bool) -> bool {
    !*b
}

#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_zero_u32(n: &u32) -> bool {
    *n == 0
}

/// Non-secret metadata shared by every entry type.
///
/// Duplicates `CommonMeta`'s user-authored fields; deliberately **omits**
/// `entry_type` (the enum tag carries it), `payload_schema` (export has its own
/// `format_version`), and `secret_changed_at` (health-derived, owned by
/// `create_entry` — a fresh import legitimately resets a secret's apparent age).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportMeta {
    /// The **source** entry ULID, verbatim — a batch-local key so `folder_id`
    /// references resolve within the export. Import mints fresh ULIDs.
    pub id: String,
    pub name: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favicon_url: Option<String>,

    /// Tag **names** (not IDs) — import matches-or-creates by name (Decision ④).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// The **source** ULID of the parent folder, if any. Remapped on import.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,

    #[serde(default, skip_serializing_if = "is_false")]
    pub is_favorite: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub sort_order: u32,
}

// ---------------------------------------------------------------------------
// The entry enum + per-type payloads.
// ---------------------------------------------------------------------------

/// One exported entry. Internally tagged by `type`; each known variant nests an
/// [`ExportMeta`]. `Unknown` is the forward-compat carrier (Decision ③) and
/// holds the original JSON object verbatim.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExportEntry {
    Login(ExportLogin),
    Card(ExportCard),
    SshKey(ExportSshKey),
    ApiKey(ExportApiKey),
    EnvVars(ExportEnvVars),
    Note(ExportNote),
    Document(ExportDocument),
    Identity(ExportIdentity),
    Folder(ExportFolder),
    Unknown(ExportUnknown),
}

impl ExportEntry {
    /// The batch-local id (source ULID) for known variants; `None` for
    /// [`ExportEntry::Unknown`], whose id lives opaquely inside `raw`.
    #[must_use]
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Login(e) => Some(&e.meta.id),
            Self::Card(e) => Some(&e.meta.id),
            Self::SshKey(e) => Some(&e.meta.id),
            Self::ApiKey(e) => Some(&e.meta.id),
            Self::EnvVars(e) => Some(&e.meta.id),
            Self::Note(e) => Some(&e.meta.id),
            Self::Document(e) => Some(&e.meta.id),
            Self::Identity(e) => Some(&e.meta.id),
            Self::Folder(e) => Some(&e.meta.id),
            Self::Unknown(_) => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportLogin {
    pub meta: ExportMeta,
    pub username: String,
    #[serde(serialize_with = "expose_secret_string")]
    pub password: SecretString,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "expose_optional_secret_string"
    )]
    pub totp_secret: Option<SecretString>,
    #[serde(default, skip_serializing_if = "ExportTotpParams::is_default")]
    pub totp_params: ExportTotpParams,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "expose_secret_string_vec"
    )]
    pub recovery_codes: Vec<SecretString>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportCard {
    pub meta: ExportMeta,
    pub cardholder_name: String,
    #[serde(serialize_with = "expose_secret_string")]
    pub number: SecretString,
    pub expiry_month: u8,
    pub expiry_year: u16,
    #[serde(serialize_with = "expose_secret_string")]
    pub cvv: SecretString,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "expose_optional_secret_string"
    )]
    pub pin: Option<SecretString>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSshKey {
    pub meta: ExportMeta,
    #[serde(serialize_with = "expose_secret_string")]
    pub private_key_pem: SecretString,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "expose_optional_secret_string"
    )]
    pub passphrase: Option<SecretString>,
    pub public_key: String,
    pub fingerprint: String,
    pub key_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportApiKey {
    pub meta: ExportMeta,
    #[serde(serialize_with = "expose_secret_string")]
    pub key: SecretString,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "expose_optional_secret_string"
    )]
    pub secret: Option<SecretString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expiry: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportEnvVar {
    pub key: String,
    #[serde(serialize_with = "expose_secret_string")]
    pub value: SecretString,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportEnvVars {
    pub meta: ExportMeta,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vars: Vec<ExportEnvVar>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportNote {
    pub meta: ExportMeta,
    #[serde(serialize_with = "expose_secret_string")]
    pub content: SecretString,
}

/// A document's metadata. Its decrypted bytes travel as the tar member
/// `blobs/{meta.id}` — never inline here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportDocument {
    pub meta: ExportMeta,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportAddress {
    pub line1: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line2: Option<String>,
    pub city: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    pub postal_code: String,
    pub country: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportIdentity {
    pub meta: ExportMeta,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<ExportAddress>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_of_birth: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "expose_optional_secret_string"
    )]
    pub national_id: Option<SecretString>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportFolder {
    pub meta: ExportMeta,
}

/// 🔴 Decision ③: the forward-compat carrier.
///
/// Holds the original JSON object of an entry whose `entry_type` this build does
/// not recognise, byte-faithfully. Import writes it back as
/// `EntryPayload::Unknown`. **Never** parse, "fix", or drop it — doing so
/// silently loses the data of a vault written by a newer build.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportUnknown {
    pub raw: serde_json::Value,
}

// ---------------------------------------------------------------------------
// TOTP params — export's own copy (mirrors `domain::vault::totp::TotpParams`).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ExportTotpAlgorithm {
    #[default]
    Sha1,
    Sha256,
    Sha512,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportTotpParams {
    pub algorithm: ExportTotpAlgorithm,
    pub digits: u8,
    pub period: u32,
}

impl Default for ExportTotpParams {
    fn default() -> Self {
        Self {
            algorithm: ExportTotpAlgorithm::Sha1,
            digits: 6,
            period: 30,
        }
    }
}

impl ExportTotpParams {
    /// The near-universal SHA-1 / 6 / 30 default — omitted from the JSON.
    ///
    /// `&self` (not `self`) because serde's `skip_serializing_if` calls it as
    /// `fn(&T) -> bool`.
    #[must_use]
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}
