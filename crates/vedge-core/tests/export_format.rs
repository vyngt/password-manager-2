//! End-to-end **format** round-trip for the export DTO + envelope (slice 5.3a).
//!
//! This is the refinement that keeps the permanent format honest: it proves the
//! whole pipeline — `EntryPayload` → `ExportEntry` → `entries.json` → tar →
//! sealed envelope → open → tar → `entries.json` → `ExportEntry` — round-trips
//! byte-for-byte, **inside the PR that freezes the format**. The vault-level
//! round-trip (through the real import write path) lands in 5.3b.
//!
//! Uses the real `seal`/`open` (64 MiB Argon2id), so it also exercises the
//! production envelope, not a fast test profile.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use secrecy::{ExposeSecret, SecretString};
use serde_json::json;

use vedge_core::domain::export::{ExportBundle, ExportEntry, payload_to_export};
use vedge_core::domain::shared::EntryId;
use vedge_core::domain::vault::payloads::{
    Address, ApiKeyPayload, CardPayload, CommonMeta, DocumentPayload, EntryPayload, EntryType,
    EnvVar, EnvVarsPayload, FolderPayload, IdentityPayload, LoginPayload, NotePayload,
    SshKeyPayload, UnknownPayload,
};
use vedge_core::domain::vault::totp::TotpParams;
use vedge_core::infrastructure::export::{archive, envelope};

fn id(n: u8) -> EntryId {
    // Distinct valid ULIDs per entry so folder_id references stay meaningful.
    EntryId::from_raw(format!(
        "01ARZ3NDEKTSV4RRFFQ69G5FA{}",
        char::from(b'A'.wrapping_add(n))
    ))
}

/// One of every payload variant, including a leading-`=` password (formula
/// injection: must survive verbatim) and a forward-compat `Unknown`.
fn all_ten_payloads() -> Vec<(EntryId, EntryPayload)> {
    let login = EntryPayload::Login(LoginPayload {
        meta: CommonMeta::new("github", EntryType::Login),
        username: "octocat".into(),
        password: SecretString::from("=hunter2"),
        totp_secret: Some(SecretString::from("JBSWY3DPEHPK3PXP")),
        totp_params: TotpParams::default(),
        recovery_codes: vec![SecretString::from("rc-1"), SecretString::from("rc-2")],
    });
    let card = EntryPayload::Card(CardPayload {
        meta: CommonMeta::new("visa", EntryType::Card),
        cardholder_name: "A Cardholder".into(),
        number: SecretString::from("4111111111111111"),
        expiry_month: 12,
        expiry_year: 2030,
        cvv: SecretString::from("123"),
        pin: Some(SecretString::from("0000")),
    });
    let ssh = EntryPayload::SshKey(SshKeyPayload {
        meta: CommonMeta::new("prod key", EntryType::SshKey),
        private_key_pem: SecretString::from("-----BEGIN-----\nx\n-----END-----"),
        passphrase: None,
        public_key: "ssh-ed25519 AAAA".into(),
        fingerprint: "SHA256:abc".into(),
        key_type: "ed25519".into(),
    });
    let api = EntryPayload::ApiKey(ApiKeyPayload {
        meta: CommonMeta::new("stripe", EntryType::ApiKey),
        key: SecretString::from("sk_live_x"),
        secret: Some(SecretString::from("whsec_y")),
        endpoint: Some("https://api.stripe.com".into()),
        expiry: None,
        key_type: Some("secret".into()),
    });
    let env = EntryPayload::EnvVars(EnvVarsPayload {
        meta: CommonMeta::new("dotenv", EntryType::EnvVars),
        vars: vec![EnvVar {
            key: "DATABASE_URL".into(),
            value: SecretString::from("postgres://u:p@h/db"),
        }],
    });
    let note = EntryPayload::Note(NotePayload {
        meta: CommonMeta::new("secret note", EntryType::Note),
        content: SecretString::from("multi\nline\nmarkdown"),
    });
    let doc = EntryPayload::Document(DocumentPayload {
        meta: CommonMeta::new("passport.pdf", EntryType::Document),
        filename: "passport.pdf".into(),
        mime_type: "application/pdf".into(),
        size_bytes: 4096,
        blob_nonce: [7u8; 24],
    });
    let ident = EntryPayload::Identity(IdentityPayload {
        meta: CommonMeta::new("me", EntryType::Identity),
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
    let folder = EntryPayload::Folder(FolderPayload {
        meta: CommonMeta::new("Work", EntryType::Folder),
    });
    let unknown = EntryPayload::Unknown(UnknownPayload {
        meta: CommonMeta::new("a passkey", EntryType::Unknown("Passkey".into())),
        unknown_fields: json!({
            "entry_type": "Passkey",
            "name": "a passkey",
            "payload_schema": 1,
            "credential_id": "unmodelled-field",
            "nested": { "rp_id": "example.com" }
        }),
    });

    vec![
        login, card, ssh, api, env, note, doc, ident, folder, unknown,
    ]
    .into_iter()
    .enumerate()
    .map(|(i, p)| (id(u8::try_from(i).unwrap()), p))
    .collect()
}

#[test]
fn all_nine_types_plus_unknown_survive_the_full_envelope_round_trip() {
    let payloads = all_ten_payloads();
    let entries: Vec<ExportEntry> = payloads
        .iter()
        .map(|(eid, p)| payload_to_export(eid, p, vec!["work".into()]))
        .collect();
    let bundle = ExportBundle::new(entries);

    // DTO → entries.json → tar → sealed envelope.
    let entries_json = serde_json::to_vec(&bundle).unwrap();
    let members: Vec<(String, &[u8])> =
        vec![(archive::ENTRIES_MEMBER.to_owned(), entries_json.as_slice())];
    let tar = archive::build_tar(&members).unwrap();
    let sealed = envelope::seal(b"a strong export passphrase", &tar).unwrap();

    // The envelope is opaque: no plaintext secret leaks into the ciphertext.
    for needle in [b"=hunter2".as_slice(), b"4111111111111111", b"sk_live_x"] {
        assert!(
            !sealed.windows(needle.len()).any(|w| w == needle),
            "a secret leaked into the sealed envelope"
        );
    }

    // open → tar → entries.json → DTO, then re-serialize and compare byte-for-byte.
    let opened = envelope::open(b"a strong export passphrase", &sealed).unwrap();
    let got_entries = archive::read_member(&opened, archive::ENTRIES_MEMBER)
        .unwrap()
        .expect("entries.json member present");
    let back: ExportBundle = serde_json::from_slice(&got_entries).unwrap();

    let reserialized = serde_json::to_vec(&back).unwrap();
    assert_eq!(
        entries_json, reserialized,
        "the export bundle must round-trip byte-for-byte through the envelope"
    );
    assert_eq!(back.entries.len(), 10);

    // Spot-check the two most safety-critical fidelity claims.
    // 1) The leading-`=` password survives verbatim (③/⑤).
    let ExportEntry::Login(l) = &back.entries[0] else {
        panic!("first entry must be the Login");
    };
    assert_eq!(l.password.expose_secret(), "=hunter2");

    // 2) The Unknown payload's raw JSON is byte-faithful (③).
    let ExportEntry::Unknown(u) = &back.entries[9] else {
        panic!("last entry must be the Unknown");
    };
    assert_eq!(u.raw["credential_id"], json!("unmodelled-field"));
    assert_eq!(u.raw["nested"]["rp_id"], json!("example.com"));
}

/// A document's decrypted bytes travel as a `blobs/{id}` tar member and survive
/// the envelope byte-for-byte (the metadata rides in `entries.json`).
#[test]
fn document_bytes_survive_as_a_blob_member() {
    let doc_bytes = b"\x00\x01\x02 arbitrary binary PDF bytes \xff\xfe".to_vec();
    let member_name = format!("{}01ARZ3NDEKTSV4RRFFQ69G5FAV", archive::BLOBS_PREFIX);
    let members: Vec<(String, &[u8])> = vec![
        (archive::ENTRIES_MEMBER.to_owned(), b"{}".as_slice()),
        (member_name.clone(), doc_bytes.as_slice()),
    ];
    let tar = archive::build_tar(&members).unwrap();
    let sealed = envelope::seal(b"pw", &tar).unwrap();
    let opened = envelope::open(b"pw", &sealed).unwrap();

    let got = archive::read_member(&opened, &member_name)
        .unwrap()
        .expect("blob member present");
    assert_eq!(&*got, doc_bytes.as_slice());
}
