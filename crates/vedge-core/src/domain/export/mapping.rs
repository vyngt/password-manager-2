//! The mapping between `EntryPayload` and the permanent [`ExportEntry`] DTO —
//! **both directions**, hand-written (slices 5.3a forward, 5.3b reverse).
//!
//! Boring, mechanical, per-variant — and the most valuable code in the slice.
//! It is the seam that keeps the permanent export format decoupled from the
//! at-rest schema (Decision ①). **The two directions are deliberately separate
//! and NOT DRY'd — the duplication IS the decoupling.** `payload_to_export` is the
//! export half; `export_to_payload` (+ `common_meta_from_export`) is the import
//! half a reviewer should read side-by-side against it.
//!
//! Pure: tag **names** cross the boundary (a payload carries only tag IDs, which
//! are vault-local), so the forward map takes pre-resolved names and the reverse
//! hands names back for the caller to match-or-create; `meta.id`/`folder_id` are
//! the source ULIDs verbatim, remapped by the import batch, not here.

use super::dto::{
    ExportAddress, ExportApiKey, ExportCard, ExportDocument, ExportEntry, ExportEnvVar,
    ExportEnvVars, ExportIdentity, ExportLogin, ExportMeta, ExportNote, ExportSshKey,
    ExportTotpAlgorithm, ExportTotpParams, ExportUnknown,
};
use crate::domain::shared::{EntryId, TagId};
use crate::domain::vault::payloads::common_meta::CURRENT_PAYLOAD_SCHEMA;
use crate::domain::vault::payloads::{
    Address, ApiKeyPayload, CardPayload, CommonMeta, EntryPayload, EntryType, EnvVar,
    EnvVarsPayload, FolderPayload, IdentityPayload, LoginPayload, NotePayload, SshKeyPayload,
};
use crate::domain::vault::totp::{TotpAlgorithm, TotpParams};

use super::dto::ExportFolder;

fn export_meta(id: &EntryId, meta: &CommonMeta, tag_names: Vec<String>) -> ExportMeta {
    ExportMeta {
        id: id.as_str().to_owned(),
        name: meta.name.clone(),
        url: meta.url.clone(),
        favicon_url: meta.favicon_url.clone(),
        tags: tag_names,
        folder_id: meta.folder_id.as_ref().map(|f| f.as_str().to_owned()),
        is_favorite: meta.is_favorite,
        notes: meta.notes.clone(),
        color: meta.color.clone(),
        icon: meta.icon.clone(),
        sort_order: meta.sort_order,
    }
}

const fn export_algorithm(a: TotpAlgorithm) -> ExportTotpAlgorithm {
    match a {
        TotpAlgorithm::Sha1 => ExportTotpAlgorithm::Sha1,
        TotpAlgorithm::Sha256 => ExportTotpAlgorithm::Sha256,
        TotpAlgorithm::Sha512 => ExportTotpAlgorithm::Sha512,
    }
}

const fn export_totp(p: TotpParams) -> ExportTotpParams {
    ExportTotpParams {
        algorithm: export_algorithm(p.algorithm),
        digits: p.digits,
        period: p.period,
    }
}

fn export_address(a: &Address) -> ExportAddress {
    ExportAddress {
        line1: a.line1.clone(),
        line2: a.line2.clone(),
        city: a.city.clone(),
        state: a.state.clone(),
        postal_code: a.postal_code.clone(),
        country: a.country.clone(),
    }
}

/// Map a decrypted `EntryPayload` — plus its row `id` and resolved `tag_names` —
/// into the permanent export DTO.
///
/// `EntryPayload::Unknown` is preserved byte-faithfully (Decision ③): its
/// original JSON object is carried verbatim in [`ExportUnknown::raw`], and
/// `id`/`tag_names` are ignored (the id lives inside `raw`; the tags cannot be
/// resolved from an unparsed payload).
#[must_use]
pub fn payload_to_export(
    id: &EntryId,
    payload: &EntryPayload,
    tag_names: Vec<String>,
) -> ExportEntry {
    match payload {
        EntryPayload::Login(p) => ExportEntry::Login(ExportLogin {
            meta: export_meta(id, &p.meta, tag_names),
            username: p.username.clone(),
            password: p.password.clone(),
            totp_secret: p.totp_secret.clone(),
            totp_params: export_totp(p.totp_params),
            recovery_codes: p.recovery_codes.clone(),
        }),
        EntryPayload::Card(p) => ExportEntry::Card(ExportCard {
            meta: export_meta(id, &p.meta, tag_names),
            cardholder_name: p.cardholder_name.clone(),
            number: p.number.clone(),
            expiry_month: p.expiry_month,
            expiry_year: p.expiry_year,
            cvv: p.cvv.clone(),
            pin: p.pin.clone(),
        }),
        EntryPayload::SshKey(p) => ExportEntry::SshKey(ExportSshKey {
            meta: export_meta(id, &p.meta, tag_names),
            private_key_pem: p.private_key_pem.clone(),
            passphrase: p.passphrase.clone(),
            public_key: p.public_key.clone(),
            fingerprint: p.fingerprint.clone(),
            key_type: p.key_type.clone(),
        }),
        EntryPayload::ApiKey(p) => ExportEntry::ApiKey(ExportApiKey {
            meta: export_meta(id, &p.meta, tag_names),
            key: p.key.clone(),
            secret: p.secret.clone(),
            endpoint: p.endpoint.clone(),
            expiry: p.expiry.clone(),
            key_type: p.key_type.clone(),
        }),
        EntryPayload::EnvVars(p) => ExportEntry::EnvVars(ExportEnvVars {
            meta: export_meta(id, &p.meta, tag_names),
            vars: p
                .vars
                .iter()
                .map(|v| ExportEnvVar {
                    key: v.key.clone(),
                    value: v.value.clone(),
                })
                .collect(),
        }),
        EntryPayload::Note(p) => ExportEntry::Note(ExportNote {
            meta: export_meta(id, &p.meta, tag_names),
            content: p.content.clone(),
        }),
        EntryPayload::Document(p) => ExportEntry::Document(ExportDocument {
            meta: export_meta(id, &p.meta, tag_names),
            filename: p.filename.clone(),
            mime_type: p.mime_type.clone(),
            size_bytes: p.size_bytes,
        }),
        EntryPayload::Identity(p) => ExportEntry::Identity(ExportIdentity {
            meta: export_meta(id, &p.meta, tag_names),
            first_name: p.first_name.clone(),
            last_name: p.last_name.clone(),
            email: p.email.clone(),
            phone: p.phone.clone(),
            address: p.address.as_ref().map(export_address),
            date_of_birth: p.date_of_birth.clone(),
            national_id: p.national_id.clone(),
        }),
        EntryPayload::Folder(p) => ExportEntry::Folder(ExportFolder {
            meta: export_meta(id, &p.meta, tag_names),
        }),
        EntryPayload::Unknown(p) => ExportEntry::Unknown(ExportUnknown {
            raw: p.unknown_fields.clone(),
        }),
    }
}

// ---------------------------------------------------------------------------
// The reverse mapping [`ExportEntry`] → import payloads (slice 5.3b).
//
// The hand-written twin of `payload_to_export`. Separate, mechanical, NOT DRY'd —
// same reason as the forward map (Decision ①): a permanent format welded to the
// at-rest schema would freeze the JSON→binary decision forever.
// ---------------------------------------------------------------------------

/// One reversed [`ExportEntry`], routed to the write path that can persist it.
///
/// Three arms because `create_entry` is **not** universal: it refuses a `Document`
/// (whose blob must be written first, keyed to a fresh DEK) and refuses an
/// `Unknown` (which it cannot re-serialize — `entry_payload.rs`).
#[allow(clippy::large_enum_variant)] // short-lived: built then immediately consumed by the commit loop
pub enum ImportedEntry {
    /// Persist via `create_entry` — mints a fresh ULID + DEK, one `Created` audit.
    Standard(EntryPayload),
    /// Persist via `import_document`; the caller supplies the plaintext bytes from
    /// the session's staged blob (keyed by the source id).
    Document(ImportedDocument),
    /// Persist via the low-level unknown write. The unknown *schema* stays byte-
    /// faithful (Decision ③); only `folder_id` — a `CommonMeta` reference we own,
    /// and the one that would orphan the entry in the index's folder map — is
    /// remapped through the batch (source tag IDs are left as-is: unresolvable in
    /// the target, they simply don't render, and rewriting them would break the ③
    /// byte-faithfulness the round-trip test pins).
    Unknown(serde_json::Value),
}

/// A document to import — metadata only. The commit loop supplies the plaintext
/// bytes (from the session's staged blob) before calling `import_document`.
pub struct ImportedDocument {
    pub filename: String,
    pub mime_type: String,
    pub meta: CommonMeta,
}

const fn import_algorithm(a: ExportTotpAlgorithm) -> TotpAlgorithm {
    match a {
        ExportTotpAlgorithm::Sha1 => TotpAlgorithm::Sha1,
        ExportTotpAlgorithm::Sha256 => TotpAlgorithm::Sha256,
        ExportTotpAlgorithm::Sha512 => TotpAlgorithm::Sha512,
    }
}

const fn import_totp(p: ExportTotpParams) -> TotpParams {
    TotpParams {
        algorithm: import_algorithm(p.algorithm),
        digits: p.digits,
        period: p.period,
    }
}

fn import_address(a: ExportAddress) -> Address {
    Address {
        line1: a.line1,
        line2: a.line2,
        city: a.city,
        state: a.state,
        postal_code: a.postal_code,
        country: a.country,
    }
}

/// Reverse of `export_meta`.
///
/// `tag_ids` are resolved by the caller (match-or-create by name — Decision ④);
/// `folder_id` is the remapped in-batch id, or `None` for a folder absent from the
/// batch (the entry imports flat). `secret_changed_at` stays `None` —
/// `create_entry` owns it, so a fresh import legitimately resets a secret's age.
#[must_use]
pub fn common_meta_from_export(
    m: &ExportMeta,
    entry_type: EntryType,
    tag_ids: Vec<TagId>,
    folder_id: Option<EntryId>,
) -> CommonMeta {
    CommonMeta {
        name: m.name.clone(),
        entry_type,
        url: m.url.clone(),
        favicon_url: m.favicon_url.clone(),
        tag_ids,
        folder_id,
        is_favorite: m.is_favorite,
        notes: m.notes.clone(),
        color: m.color.clone(),
        icon: m.icon.clone(),
        sort_order: m.sort_order,
        secret_changed_at: None,
        payload_schema: CURRENT_PAYLOAD_SCHEMA,
    }
}

/// The [`EntryType`] a given [`ExportEntry`] reconstitutes to, or `None` for
/// `Unknown`.
///
/// An `Unknown`'s original type string was consumed with the enum tag; the caller
/// reads it from the `raw` object instead.
#[must_use]
pub const fn export_entry_type(entry: &ExportEntry) -> Option<EntryType> {
    Some(match entry {
        ExportEntry::Login(_) => EntryType::Login,
        ExportEntry::Card(_) => EntryType::Card,
        ExportEntry::SshKey(_) => EntryType::SshKey,
        ExportEntry::ApiKey(_) => EntryType::ApiKey,
        ExportEntry::EnvVars(_) => EntryType::EnvVars,
        ExportEntry::Note(_) => EntryType::Note,
        ExportEntry::Document(_) => EntryType::Document,
        ExportEntry::Identity(_) => EntryType::Identity,
        ExportEntry::Folder(_) => EntryType::Folder,
        ExportEntry::Unknown(_) => return None,
    })
}

/// Reverse of `payload_to_export`.
///
/// `meta` is the fully-resolved [`CommonMeta`] the caller has already built (tags
/// resolved, folder remapped, `entry_type` set); for `Unknown` only
/// `meta.folder_id` is consulted (written back into the byte-faithful `raw`).
/// Secrets are **moved** out of the export DTO — no lingering copy.
#[must_use]
pub fn export_to_payload(entry: ExportEntry, meta: CommonMeta) -> ImportedEntry {
    match entry {
        ExportEntry::Login(e) => ImportedEntry::Standard(EntryPayload::Login(LoginPayload {
            meta,
            username: e.username,
            password: e.password,
            totp_secret: e.totp_secret,
            totp_params: import_totp(e.totp_params),
            recovery_codes: e.recovery_codes,
        })),
        ExportEntry::Card(e) => ImportedEntry::Standard(EntryPayload::Card(CardPayload {
            meta,
            cardholder_name: e.cardholder_name,
            number: e.number,
            expiry_month: e.expiry_month,
            expiry_year: e.expiry_year,
            cvv: e.cvv,
            pin: e.pin,
        })),
        ExportEntry::SshKey(e) => ImportedEntry::Standard(EntryPayload::SshKey(SshKeyPayload {
            meta,
            private_key_pem: e.private_key_pem,
            passphrase: e.passphrase,
            public_key: e.public_key,
            fingerprint: e.fingerprint,
            key_type: e.key_type,
        })),
        ExportEntry::ApiKey(e) => ImportedEntry::Standard(EntryPayload::ApiKey(ApiKeyPayload {
            meta,
            key: e.key,
            secret: e.secret,
            endpoint: e.endpoint,
            expiry: e.expiry,
            key_type: e.key_type,
        })),
        ExportEntry::EnvVars(e) => ImportedEntry::Standard(EntryPayload::EnvVars(EnvVarsPayload {
            meta,
            vars: e
                .vars
                .into_iter()
                .map(|v| EnvVar {
                    key: v.key,
                    value: v.value,
                })
                .collect(),
        })),
        ExportEntry::Note(e) => ImportedEntry::Standard(EntryPayload::Note(NotePayload {
            meta,
            content: e.content,
        })),
        ExportEntry::Identity(e) => {
            ImportedEntry::Standard(EntryPayload::Identity(IdentityPayload {
                meta,
                first_name: e.first_name,
                last_name: e.last_name,
                email: e.email,
                phone: e.phone,
                address: e.address.map(import_address),
                date_of_birth: e.date_of_birth,
                national_id: e.national_id,
            }))
        }
        ExportEntry::Folder(_e) => {
            ImportedEntry::Standard(EntryPayload::Folder(FolderPayload { meta }))
        }
        ExportEntry::Document(e) => ImportedEntry::Document(ImportedDocument {
            filename: e.filename,
            mime_type: e.mime_type,
            meta,
        }),
        ExportEntry::Unknown(e) => {
            let mut raw = e.raw;
            // Remap ONLY folder_id — the one cross-reference that would orphan the
            // entry in the index. Never touch the unknown schema (Decision ③).
            if let Some(obj) = raw.as_object_mut() {
                match &meta.folder_id {
                    Some(id) => {
                        obj.insert(
                            "folder_id".to_owned(),
                            serde_json::Value::String(id.as_str().to_owned()),
                        );
                    }
                    // Absent from the batch (or the source had none) → flat. Only
                    // strip an existing key; never add one (keeps ③ byte-identical
                    // for an Unknown that had no folder_id to begin with).
                    None => {
                        obj.remove("folder_id");
                    }
                }
            }
            ImportedEntry::Unknown(raw)
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use secrecy::{ExposeSecret, SecretString};
    use serde_json::json;

    use super::*;
    use crate::domain::export::dto::{EXPORT_FORMAT_VERSION, ExportBundle, ExportEntry};
    use crate::domain::vault::payloads::{
        ApiKeyPayload, CardPayload, CommonMeta, DocumentPayload, EntryType, EnvVar, EnvVarsPayload,
        IdentityPayload, LoginPayload, NotePayload, SshKeyPayload, UnknownPayload,
    };
    use crate::domain::vault::totp::TotpParams;

    fn meta(name: &str, ty: EntryType) -> CommonMeta {
        CommonMeta::new(name, ty)
    }

    fn id() -> EntryId {
        EntryId::from_raw("01ARZ3NDEKTSV4RRFFQ69G5FAV")
    }

    /// 🔴 Decision ③ — the test the tracking note says to write FIRST: an entry this
    /// build cannot parse survives export byte-faithfully. Skip it and export silently
    /// drops the user's data.
    #[test]
    fn unknown_round_trips_byte_faithfully() {
        // A payload from a NEWER build: an unrecognised `entry_type` plus fields we
        // don't model. This is exactly what `EntryPayload::Unknown` preserves at rest.
        let raw = json!({
            "entry_type": "Passkey",
            "name": "my passkey",
            "payload_schema": 1,
            "credential_id": "AAAA-not-a-field-we-know",
            "nested": { "rp_id": "example.com", "counts": [1, 2, 3] }
        });
        let payload = EntryPayload::Unknown(UnknownPayload {
            meta: CommonMeta::new("my passkey", EntryType::Unknown("Passkey".into())),
            unknown_fields: raw.clone(),
        });

        let export = payload_to_export(&id(), &payload, vec![]);
        assert!(matches!(export, ExportEntry::Unknown(_)));

        // Through the full bundle serde round-trip, `raw` must be identical.
        let bundle = ExportBundle::new(vec![export]);
        let bytes = serde_json::to_vec(&bundle).unwrap();
        let back: ExportBundle = serde_json::from_slice(&bytes).unwrap();
        let ExportEntry::Unknown(u) = &back.entries[0] else {
            panic!("Unknown variant must survive as Unknown");
        };
        assert_eq!(
            u.raw, raw,
            "the unknown payload's raw JSON must be identical"
        );
    }

    /// One of every payload variant maps to its export twin and survives a serde
    /// round-trip byte-for-byte. Proves the DTO serialization is stable (the
    /// envelope-level round-trip lives in `tests/export_format.rs`).
    #[test]
    fn all_variants_survive_a_format_round_trip() {
        let login = EntryPayload::Login(LoginPayload {
            meta: meta("github", EntryType::Login),
            username: "octocat".into(),
            password: SecretString::from("=hunter2"), // leading `=` — must survive verbatim
            totp_secret: Some(SecretString::from("ABCDEFGH23456789")),
            totp_params: TotpParams::default(),
            recovery_codes: vec![SecretString::from("code-1"), SecretString::from("code-2")],
        });
        let card = EntryPayload::Card(CardPayload {
            meta: meta("visa", EntryType::Card),
            cardholder_name: "A Cardholder".into(),
            number: SecretString::from("card-number-placeholder"),
            expiry_month: 12,
            expiry_year: 2030,
            cvv: SecretString::from("123"),
            pin: Some(SecretString::from("0000")),
        });
        let ssh = EntryPayload::SshKey(SshKeyPayload {
            meta: meta("prod key", EntryType::SshKey),
            private_key_pem: SecretString::from("ssh-private-key-placeholder"),
            passphrase: None,
            public_key: "ssh-ed25519 AAAA".into(),
            fingerprint: "SHA256:abc".into(),
            key_type: "ed25519".into(),
        });
        let api = EntryPayload::ApiKey(ApiKeyPayload {
            meta: meta("stripe", EntryType::ApiKey),
            key: SecretString::from("api-key-placeholder"),
            secret: Some(SecretString::from("api-secret-placeholder")),
            endpoint: Some("https://api.example.com".into()),
            expiry: None,
            key_type: Some("secret".into()),
        });
        let env = EntryPayload::EnvVars(EnvVarsPayload {
            meta: meta("dotenv", EntryType::EnvVars),
            vars: vec![EnvVar {
                key: "DATABASE_URL".into(),
                value: SecretString::from("connection-string-placeholder"),
            }],
        });
        let note = EntryPayload::Note(NotePayload {
            meta: meta("secret note", EntryType::Note),
            content: SecretString::from("multi\nline\nmarkdown"),
        });
        let doc = EntryPayload::Document(DocumentPayload {
            meta: meta("passport.pdf", EntryType::Document),
            filename: "passport.pdf".into(),
            mime_type: "application/pdf".into(),
            size_bytes: 4096,
            blob_nonce: [7u8; 24],
        });
        let ident = EntryPayload::Identity(IdentityPayload {
            meta: meta("me", EntryType::Identity),
            first_name: "Ada".into(),
            last_name: "Lovelace".into(),
            email: "ada@example.com".into(),
            phone: Some("+1-555".into()),
            address: Some(Address {
                line1: "1 Analytical Way".into(),
                line2: None,
                city: "London".into(),
                state: None,
                postal_code: "SW1".into(),
                country: "GB".into(),
            }),
            date_of_birth: Some("1815-12-10".into()),
            national_id: Some(SecretString::from("ID-123")),
        });
        let folder = EntryPayload::Folder(crate::domain::vault::payloads::FolderPayload {
            meta: meta("Work", EntryType::Folder),
        });

        let payloads = [login, card, ssh, api, env, note, doc, ident, folder];
        let entries: Vec<ExportEntry> = payloads
            .iter()
            .map(|p| payload_to_export(&id(), p, vec!["work".into(), "secrets".into()]))
            .collect();

        let bundle = ExportBundle::new(entries);
        let bytes1 = serde_json::to_vec(&bundle).unwrap();
        let back: ExportBundle = serde_json::from_slice(&bytes1).unwrap();
        let bytes2 = serde_json::to_vec(&back).unwrap();
        assert_eq!(
            bytes1, bytes2,
            "the export DTO must round-trip byte-for-byte"
        );
        assert_eq!(back.format_version, EXPORT_FORMAT_VERSION);
        assert_eq!(back.entries.len(), 9);
    }

    /// The mapping actually copies secret material through (not just metadata).
    #[test]
    fn login_secrets_and_tags_are_carried() {
        let payload = EntryPayload::Login(LoginPayload {
            meta: meta("acct", EntryType::Login),
            username: "u".into(),
            password: SecretString::from("p@ss"),
            totp_secret: None,
            totp_params: TotpParams::default(),
            recovery_codes: vec![],
        });
        let export = payload_to_export(&id(), &payload, vec!["personal".into()]);
        let ExportEntry::Login(l) = export else {
            panic!("expected Login");
        };
        assert_eq!(l.meta.id, "01ARZ3NDEKTSV4RRFFQ69G5FAV");
        assert_eq!(l.meta.tags, vec!["personal".to_owned()]);
        assert_eq!(l.username, "u");
        assert_eq!(l.password.expose_secret(), "p@ss");
    }

    // ---- the reverse mapping (5.3b) --------------------------------------

    /// A Login survives forward → reverse with its secrets intact — the mechanical
    /// twin of `login_secrets_and_tags_are_carried`, closing the loop.
    #[test]
    fn login_survives_forward_then_reverse() {
        let payload = EntryPayload::Login(LoginPayload {
            meta: meta("github", EntryType::Login),
            username: "octocat".into(),
            password: SecretString::from("=hunter2"), // leading `=` must survive
            totp_secret: Some(SecretString::from("ABCDEFGH23456789")),
            totp_params: TotpParams::default(),
            recovery_codes: vec![SecretString::from("rc-1")],
        });
        let export = payload_to_export(&id(), &payload, vec!["work".into()]);

        // Build the resolved meta the commit loop would (tags → fresh ids, no folder).
        let entry_type = export_entry_type(&export).unwrap();
        let cm = match &export {
            ExportEntry::Login(e) => common_meta_from_export(&e.meta, entry_type, vec![], None),
            _ => panic!("expected Login"),
        };
        let ImportedEntry::Standard(EntryPayload::Login(back)) = export_to_payload(export, cm)
        else {
            panic!("expected a standard Login");
        };
        assert_eq!(back.username, "octocat");
        assert_eq!(back.password.expose_secret(), "=hunter2");
        assert_eq!(
            back.totp_secret.unwrap().expose_secret(),
            "ABCDEFGH23456789"
        );
        assert_eq!(back.recovery_codes.len(), 1);
        assert_eq!(back.meta.name, "github");
        assert_eq!(back.meta.entry_type, EntryType::Login);
        // The health-owned field is reset — `create_entry` will stamp it.
        assert!(back.meta.secret_changed_at.is_none());
    }

    /// A Document reverses to the `Document` arm (routed to `import_document`, since
    /// `create_entry` can't write a blob) — metadata only, no bytes.
    #[test]
    fn document_reverses_to_the_document_arm() {
        let payload = EntryPayload::Document(DocumentPayload {
            meta: meta("passport.pdf", EntryType::Document),
            filename: "passport.pdf".into(),
            mime_type: "application/pdf".into(),
            size_bytes: 4096,
            blob_nonce: [7u8; 24],
        });
        let export = payload_to_export(&id(), &payload, vec![]);
        let cm = match &export {
            ExportEntry::Document(e) => {
                common_meta_from_export(&e.meta, EntryType::Document, vec![], None)
            }
            _ => panic!("expected Document"),
        };
        let ImportedEntry::Document(doc) = export_to_payload(export, cm) else {
            panic!("expected the Document arm");
        };
        assert_eq!(doc.filename, "passport.pdf");
        assert_eq!(doc.mime_type, "application/pdf");
        assert_eq!(doc.meta.entry_type, EntryType::Document);
    }

    /// 🔴 Decision ③ — an Unknown with no cross-references reverses **byte-identical**.
    #[test]
    fn unknown_with_no_folder_reverses_byte_identical() {
        let raw = json!({
            "entry_type": "Passkey",
            "name": "my passkey",
            "payload_schema": 1,
            "credential_id": "AAAA-not-a-field-we-know",
            "nested": { "rp_id": "example.com" }
        });
        let entry =
            ExportEntry::Unknown(crate::domain::export::dto::ExportUnknown { raw: raw.clone() });
        // No folder in the batch → None; adds nothing, strips nothing.
        let cm = CommonMeta::new("my passkey", EntryType::Unknown("Passkey".into()));
        let ImportedEntry::Unknown(back) = export_to_payload(entry, cm) else {
            panic!("expected Unknown");
        };
        assert_eq!(
            back, raw,
            "an Unknown with no folder_id must be byte-identical"
        );
    }

    /// An Unknown whose folder IS in the batch has ONLY its `folder_id` remapped;
    /// every unknown-schema field is preserved.
    #[test]
    fn unknown_remaps_only_folder_id() {
        let raw = json!({
            "entry_type": "Passkey",
            "name": "my passkey",
            "payload_schema": 1,
            "folder_id": "01ARZ3NDEKTSV4RRFFQ69G5FA1",
            "credential_id": "AAAA-not-a-field-we-know"
        });
        let entry = ExportEntry::Unknown(crate::domain::export::dto::ExportUnknown { raw });
        let mut cm = CommonMeta::new("my passkey", EntryType::Unknown("Passkey".into()));
        cm.folder_id = Some(EntryId::from_raw("01ARZ3NDEKTSV4RRFFQ69G5FA2"));
        let ImportedEntry::Unknown(back) = export_to_payload(entry, cm) else {
            panic!("expected Unknown");
        };
        assert_eq!(back["folder_id"], json!("01ARZ3NDEKTSV4RRFFQ69G5FA2"));
        assert_eq!(back["credential_id"], json!("AAAA-not-a-field-we-know"));
        assert_eq!(back["entry_type"], json!("Passkey"));
    }

    /// An Unknown whose folder is ABSENT from the batch imports flat: an existing
    /// `folder_id` is stripped (not left dangling).
    #[test]
    fn unknown_absent_folder_imports_flat() {
        let raw = json!({
            "entry_type": "Passkey",
            "name": "pk",
            "payload_schema": 1,
            "folder_id": "01ARZ3NDEKTSV4RRFFQ69G5FA1"
        });
        let entry = ExportEntry::Unknown(crate::domain::export::dto::ExportUnknown { raw });
        // Folder not in batch → None → strip.
        let cm = CommonMeta::new("pk", EntryType::Unknown("Passkey".into()));
        let ImportedEntry::Unknown(back) = export_to_payload(entry, cm) else {
            panic!("expected Unknown");
        };
        assert!(
            back.get("folder_id").is_none(),
            "a folder absent from the batch must import flat (folder_id stripped)"
        );
    }
}
