//! Full round-trip: payload → JSON → ciphertext → JSON → payload, under the real
//! `XChaCha20CryptoProvider`. Ensures the serde + crypto layers compose correctly.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::needless_pass_by_value
)]

use secrecy::{ExposeSecret, SecretString};

use vedge_core::application::vault::ports::CryptoProvider;
use vedge_core::domain::shared::{EntryId, TagId};
use vedge_core::domain::vault::aad::{entry_aad, tag_aad};
use vedge_core::domain::vault::payloads::identity::Address;
use vedge_core::domain::vault::payloads::{
    ApiKeyPayload, CardPayload, CommonMeta, DocumentPayload, EntryPayload, EntryType, EnvVar,
    EnvVarsPayload, FolderPayload, IdentityPayload, LoginPayload, NotePayload, SshKeyPayload,
    TagPayload,
};
use vedge_core::infrastructure::crypto::XChaCha20CryptoProvider;

fn roundtrip_entry(payload: EntryPayload) -> EntryPayload {
    let crypto = XChaCha20CryptoProvider::new();
    let dek = crypto.generate_dek();
    let id = EntryId::new();
    let aad = entry_aad(&id, 1).unwrap();

    let bytes = payload.to_encryptable_json().unwrap();
    let (nonce, ct) = crypto.encrypt_entry(&dek, &bytes, &aad).unwrap();
    let pt = crypto.decrypt_entry(&dek, &nonce, &ct, &aad).unwrap();
    EntryPayload::from_decrypted_json(&pt).unwrap()
}

fn simple_meta(name: &str, ty: EntryType) -> CommonMeta {
    CommonMeta::new(name, ty)
}

#[test]
fn login_round_trip() {
    let out = roundtrip_entry(EntryPayload::Login(LoginPayload {
        meta: simple_meta("github", EntryType::Login),
        username: "alice".into(),
        password: SecretString::from("hunter2"),
        totp_secret: Some(SecretString::from("JBSWY3DPEHPK3PXP")),
        recovery_codes: vec![SecretString::from("code-1"), SecretString::from("code-2")],
    }));
    let EntryPayload::Login(l) = out else {
        panic!("wrong variant")
    };
    assert_eq!(l.username, "alice");
    assert_eq!(l.password.expose_secret(), "hunter2");
    assert_eq!(
        l.totp_secret.as_ref().unwrap().expose_secret(),
        "JBSWY3DPEHPK3PXP"
    );
    assert_eq!(l.recovery_codes.len(), 2);
    assert_eq!(l.recovery_codes[0].expose_secret(), "code-1");
}

#[test]
fn card_round_trip() {
    let out = roundtrip_entry(EntryPayload::Card(CardPayload {
        meta: simple_meta("visa", EntryType::Card),
        cardholder_name: "Alice".into(),
        number: SecretString::from("4242424242424242"),
        expiry_month: 12,
        expiry_year: 2030,
        cvv: SecretString::from("123"),
        pin: None,
    }));
    let EntryPayload::Card(c) = out else { panic!() };
    assert_eq!(c.cardholder_name, "Alice");
    assert_eq!(c.number.expose_secret(), "4242424242424242");
    assert_eq!(c.cvv.expose_secret(), "123");
    assert!(c.pin.is_none());
}

#[test]
fn ssh_key_round_trip() {
    let out = roundtrip_entry(EntryPayload::SshKey(SshKeyPayload {
        meta: simple_meta("prod", EntryType::SshKey),
        private_key_pem: SecretString::from("-----BEGIN KEY-----\nabc\n-----END KEY-----"),
        passphrase: Some(SecretString::from("phrase")),
        public_key: "ssh-ed25519 AAAA".into(),
        fingerprint: "SHA256:...".into(),
        key_type: "ed25519".into(),
    }));
    let EntryPayload::SshKey(k) = out else {
        panic!()
    };
    assert_eq!(
        k.private_key_pem.expose_secret(),
        "-----BEGIN KEY-----\nabc\n-----END KEY-----"
    );
    assert_eq!(k.passphrase.as_ref().unwrap().expose_secret(), "phrase");
    assert_eq!(k.public_key, "ssh-ed25519 AAAA");
}

#[test]
fn api_key_round_trip() {
    let out = roundtrip_entry(EntryPayload::ApiKey(ApiKeyPayload {
        meta: simple_meta("aws", EntryType::ApiKey),
        key: SecretString::from("AKIA..."),
        secret: Some(SecretString::from("secret...")),
        endpoint: None,
        expiry: None,
        key_type: Some("Bearer".into()),
    }));
    let EntryPayload::ApiKey(a) = out else {
        panic!()
    };
    assert_eq!(a.key.expose_secret(), "AKIA...");
    assert_eq!(a.secret.as_ref().unwrap().expose_secret(), "secret...");
    assert_eq!(a.key_type.as_deref(), Some("Bearer"));
}

#[test]
fn env_vars_round_trip() {
    let out = roundtrip_entry(EntryPayload::EnvVars(EnvVarsPayload {
        meta: simple_meta("staging", EntryType::EnvVars),
        vars: vec![
            EnvVar {
                key: "A".into(),
                value: SecretString::from("1"),
            },
            EnvVar {
                key: "B".into(),
                value: SecretString::from("2"),
            },
        ],
    }));
    let EntryPayload::EnvVars(e) = out else {
        panic!()
    };
    assert_eq!(e.vars.len(), 2);
    assert_eq!(e.vars[0].key, "A");
    assert_eq!(e.vars[0].value.expose_secret(), "1");
    assert_eq!(e.vars[1].value.expose_secret(), "2");
}

#[test]
fn note_round_trip() {
    let out = roundtrip_entry(EntryPayload::Note(NotePayload {
        meta: simple_meta("journal", EntryType::Note),
        content: SecretString::from("# secret\nstuff"),
    }));
    let EntryPayload::Note(n) = out else { panic!() };
    assert_eq!(n.content.expose_secret(), "# secret\nstuff");
}

#[test]
fn document_round_trip_preserves_blob_nonce_bytes() {
    let nonce: [u8; 24] = [7; 24];
    let out = roundtrip_entry(EntryPayload::Document(DocumentPayload {
        meta: simple_meta("passport.pdf", EntryType::Document),
        filename: "passport.pdf".into(),
        mime_type: "application/pdf".into(),
        size_bytes: 102_400,
        blob_nonce: nonce,
    }));
    let EntryPayload::Document(d) = out else {
        panic!()
    };
    assert_eq!(d.blob_nonce, nonce);
    assert_eq!(d.size_bytes, 102_400);
}

#[test]
fn identity_round_trip() {
    let out = roundtrip_entry(EntryPayload::Identity(IdentityPayload {
        meta: simple_meta("me", EntryType::Identity),
        first_name: "Alice".into(),
        last_name: "Smith".into(),
        email: "alice@example.com".into(),
        phone: None,
        address: Some(Address {
            line1: "1 Main St".into(),
            line2: None,
            city: "Hanoi".into(),
            state: None,
            postal_code: "10000".into(),
            country: "VN".into(),
        }),
        date_of_birth: Some("1990-01-15".into()),
        national_id: Some(SecretString::from("CCCD-1234")),
    }));
    let EntryPayload::Identity(i) = out else {
        panic!()
    };
    assert_eq!(i.first_name, "Alice");
    assert_eq!(i.address.as_ref().unwrap().country, "VN");
    assert_eq!(i.national_id.as_ref().unwrap().expose_secret(), "CCCD-1234");
}

#[test]
fn folder_round_trip() {
    let out = roundtrip_entry(EntryPayload::Folder(FolderPayload {
        meta: simple_meta("Work", EntryType::Folder),
    }));
    let EntryPayload::Folder(f) = out else {
        panic!()
    };
    assert_eq!(f.meta.name, "Work");
}

#[test]
fn tag_round_trip_under_kek() {
    // TagPayload is encrypted directly under KEK (no per-row DEK), using tag_aad.
    let crypto = XChaCha20CryptoProvider::new();
    let kek: [u8; 32] = [0x99; 32];
    let id = TagId::new();
    let aad = tag_aad(&id).unwrap();

    let tag = TagPayload {
        name: "github".into(),
        color: Some("#1D9E75".into()),
        sort_order: 3,
    };
    let bytes = serde_json::to_vec(&tag).unwrap();
    let (nonce, ct) = crypto.encrypt_tag(&kek, &bytes, &aad).unwrap();
    let pt = crypto.decrypt_tag(&kek, &nonce, &ct, &aad).unwrap();
    let back: TagPayload = serde_json::from_slice(&pt).unwrap();
    assert_eq!(back, tag);
}
