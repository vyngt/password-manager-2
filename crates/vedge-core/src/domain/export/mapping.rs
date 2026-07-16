//! The forward mapping `EntryPayload` → [`ExportEntry`] (slice 5.3a).
//!
//! Boring, mechanical, per-variant — and the most valuable code in the slice.
//! It is the seam that keeps the permanent export format decoupled from the
//! at-rest schema (Decision ①). The reverse mapping (import) lands in 5.3b.
//!
//! Pure: tag **names** arrive pre-resolved from the caller's index (a payload
//! carries only tag IDs); `meta.id`/`folder_id` are the source ULIDs verbatim.

use super::dto::{
    ExportAddress, ExportApiKey, ExportCard, ExportDocument, ExportEntry, ExportEnvVar,
    ExportEnvVars, ExportIdentity, ExportLogin, ExportMeta, ExportNote, ExportSshKey,
    ExportTotpAlgorithm, ExportTotpParams, ExportUnknown,
};
use crate::domain::shared::EntryId;
use crate::domain::vault::payloads::{Address, CommonMeta, EntryPayload};
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
            totp_secret: Some(SecretString::from("JBSWY3DPEHPK3PXP")),
            totp_params: TotpParams::default(),
            recovery_codes: vec![SecretString::from("code-1"), SecretString::from("code-2")],
        });
        let card = EntryPayload::Card(CardPayload {
            meta: meta("visa", EntryType::Card),
            cardholder_name: "A Cardholder".into(),
            number: SecretString::from("4111111111111111"),
            expiry_month: 12,
            expiry_year: 2030,
            cvv: SecretString::from("123"),
            pin: Some(SecretString::from("0000")),
        });
        let ssh = EntryPayload::SshKey(SshKeyPayload {
            meta: meta("prod key", EntryType::SshKey),
            private_key_pem: SecretString::from("-----BEGIN-----\nx\n-----END-----"),
            passphrase: None,
            public_key: "ssh-ed25519 AAAA".into(),
            fingerprint: "SHA256:abc".into(),
            key_type: "ed25519".into(),
        });
        let api = EntryPayload::ApiKey(ApiKeyPayload {
            meta: meta("stripe", EntryType::ApiKey),
            key: SecretString::from("sk_live_x"),
            secret: Some(SecretString::from("whsec_y")),
            endpoint: Some("https://api.stripe.com".into()),
            expiry: None,
            key_type: Some("secret".into()),
        });
        let env = EntryPayload::EnvVars(EnvVarsPayload {
            meta: meta("dotenv", EntryType::EnvVars),
            vars: vec![EnvVar {
                key: "DATABASE_URL".into(),
                value: SecretString::from("postgres://u:p@h/db"),
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
}
