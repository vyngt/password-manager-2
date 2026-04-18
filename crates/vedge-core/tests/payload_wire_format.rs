//! Golden wire-format tests. These pin the JSON layout of every payload variant.
//! A diff here is a schema change and must trigger a `payload_schema` discussion.

use secrecy::{ExposeSecret, SecretString};
use serde_json::{Value, json};

use vedge_core::domain::shared::{EntryId, TagId};
use vedge_core::domain::vault::payloads::{
    ApiKeyPayload, CardPayload, CommonMeta, DocumentPayload, EntryPayload, EntryType, EnvVar,
    EnvVarsPayload, FolderPayload, IdentityPayload, LoginPayload, NotePayload, SshKeyPayload,
    TagPayload,
};
use vedge_core::domain::vault::payloads::identity::Address;

fn meta(name: &str, ty: EntryType) -> CommonMeta {
    CommonMeta {
        name: name.into(),
        entry_type: ty,
        url: Some("https://example.com".into()),
        favicon_url: None,
        tag_ids: vec![TagId::from_raw("01H0000000000000000000TAG1")],
        folder_id: Some(EntryId::from_raw("01ARZ3NDEKTSV4RRFFQ69G5FAV")),
        is_favorite: true,
        notes: Some("hello".into()),
        payload_schema: 1,
    }
}

fn to_value(p: &EntryPayload) -> Value {
    serde_json::to_value(p).expect("serialize")
}

#[test]
fn login_wire_format() {
    let p = EntryPayload::Login(LoginPayload {
        meta: meta("github", EntryType::Login),
        username: "alice".into(),
        password: SecretString::from("hunter2"),
        totp_secret: Some(SecretString::from("JBSWY3DPEHPK3PXP")),
        recovery_codes: vec![SecretString::from("code-1"), SecretString::from("code-2")],
    });

    let expected = json!({
        "name": "github",
        "entry_type": "Login",
        "url": "https://example.com",
        "tag_ids": ["01H0000000000000000000TAG1"],
        "folder_id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        "is_favorite": true,
        "notes": "hello",
        "payload_schema": 1,
        "username": "alice",
        "password": "hunter2",
        "totp_secret": "JBSWY3DPEHPK3PXP",
        "recovery_codes": ["code-1", "code-2"],
    });
    assert_eq!(to_value(&p), expected);
}

#[test]
fn card_wire_format() {
    let p = EntryPayload::Card(CardPayload {
        meta: meta("visa", EntryType::Card),
        cardholder_name: "Alice Smith".into(),
        number: SecretString::from("4242424242424242"),
        expiry_month: 12,
        expiry_year: 2030,
        cvv: SecretString::from("123"),
        pin: Some(SecretString::from("4321")),
    });

    let expected = json!({
        "name": "visa", "entry_type": "Card",
        "url": "https://example.com",
        "tag_ids": ["01H0000000000000000000TAG1"],
        "folder_id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        "is_favorite": true, "notes": "hello", "payload_schema": 1,
        "cardholder_name": "Alice Smith",
        "number": "4242424242424242",
        "expiry_month": 12, "expiry_year": 2030,
        "cvv": "123", "pin": "4321",
    });
    assert_eq!(to_value(&p), expected);
}

#[test]
fn ssh_key_wire_format() {
    let p = EntryPayload::SshKey(SshKeyPayload {
        meta: meta("prod-server", EntryType::SshKey),
        private_key_pem: SecretString::from("-----BEGIN OPENSSH PRIVATE KEY-----\n..."),
        passphrase: None,
        public_key: "ssh-ed25519 AAAA...".into(),
        fingerprint: "SHA256:abc...".into(),
        key_type: "ed25519".into(),
    });
    let expected = json!({
        "name": "prod-server", "entry_type": "SshKey",
        "url": "https://example.com",
        "tag_ids": ["01H0000000000000000000TAG1"],
        "folder_id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        "is_favorite": true, "notes": "hello", "payload_schema": 1,
        "private_key_pem": "-----BEGIN OPENSSH PRIVATE KEY-----\n...",
        "public_key": "ssh-ed25519 AAAA...",
        "fingerprint": "SHA256:abc...",
        "key_type": "ed25519",
    });
    assert_eq!(to_value(&p), expected);
}

#[test]
fn api_key_wire_format() {
    let p = EntryPayload::ApiKey(ApiKeyPayload {
        meta: meta("aws", EntryType::ApiKey),
        key: SecretString::from("AKIA..."),
        secret: Some(SecretString::from("secret...")),
        endpoint: Some("https://ec2.amazonaws.com".into()),
        expiry: Some("2027-01-01".into()),
        key_type: Some("HMAC-SHA256".into()),
    });
    let expected = json!({
        "name": "aws", "entry_type": "ApiKey",
        "url": "https://example.com",
        "tag_ids": ["01H0000000000000000000TAG1"],
        "folder_id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        "is_favorite": true, "notes": "hello", "payload_schema": 1,
        "key": "AKIA...",
        "secret": "secret...",
        "endpoint": "https://ec2.amazonaws.com",
        "expiry": "2027-01-01",
        "key_type": "HMAC-SHA256",
    });
    assert_eq!(to_value(&p), expected);
}

#[test]
fn env_vars_wire_format() {
    let p = EntryPayload::EnvVars(EnvVarsPayload {
        meta: meta("staging", EntryType::EnvVars),
        vars: vec![
            EnvVar { key: "DATABASE_URL".into(), value: SecretString::from("postgres://...") },
            EnvVar { key: "API_KEY".into(), value: SecretString::from("xyz") },
        ],
    });
    let expected = json!({
        "name": "staging", "entry_type": "EnvVars",
        "url": "https://example.com",
        "tag_ids": ["01H0000000000000000000TAG1"],
        "folder_id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        "is_favorite": true, "notes": "hello", "payload_schema": 1,
        "vars": [
            {"key": "DATABASE_URL", "value": "postgres://..."},
            {"key": "API_KEY", "value": "xyz"},
        ],
    });
    assert_eq!(to_value(&p), expected);
}

#[test]
fn note_wire_format() {
    let p = EntryPayload::Note(NotePayload {
        meta: meta("meeting-notes", EntryType::Note),
        content: SecretString::from("# confidential\nsecret stuff"),
    });
    let expected = json!({
        "name": "meeting-notes", "entry_type": "Note",
        "url": "https://example.com",
        "tag_ids": ["01H0000000000000000000TAG1"],
        "folder_id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        "is_favorite": true, "notes": "hello", "payload_schema": 1,
        "content": "# confidential\nsecret stuff",
    });
    assert_eq!(to_value(&p), expected);
}

#[test]
fn document_wire_format() {
    let p = EntryPayload::Document(DocumentPayload {
        meta: meta("passport.pdf", EntryType::Document),
        filename: "passport.pdf".into(),
        mime_type: "application/pdf".into(),
        size_bytes: 102_400,
        blob_nonce: [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        ],
    });
    // base64 of [1..=24] = "AQIDBAUGBwgJCgsMDQ4PEBESExQVFhcY"
    let expected = json!({
        "name": "passport.pdf", "entry_type": "Document",
        "url": "https://example.com",
        "tag_ids": ["01H0000000000000000000TAG1"],
        "folder_id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        "is_favorite": true, "notes": "hello", "payload_schema": 1,
        "filename": "passport.pdf",
        "mime_type": "application/pdf",
        "size_bytes": 102_400,
        "blob_nonce": "AQIDBAUGBwgJCgsMDQ4PEBESExQVFhcY",
    });
    assert_eq!(to_value(&p), expected);
}

#[test]
fn identity_wire_format() {
    let p = EntryPayload::Identity(IdentityPayload {
        meta: meta("me", EntryType::Identity),
        first_name: "Alice".into(),
        last_name: "Smith".into(),
        email: "alice@example.com".into(),
        phone: Some("+84-900-000-000".into()),
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
    });
    let expected = json!({
        "name": "me", "entry_type": "Identity",
        "url": "https://example.com",
        "tag_ids": ["01H0000000000000000000TAG1"],
        "folder_id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        "is_favorite": true, "notes": "hello", "payload_schema": 1,
        "first_name": "Alice", "last_name": "Smith",
        "email": "alice@example.com",
        "phone": "+84-900-000-000",
        "address": {
            "line1": "1 Main St", "city": "Hanoi", "postal_code": "10000", "country": "VN",
        },
        "date_of_birth": "1990-01-15",
        "national_id": "CCCD-1234",
    });
    assert_eq!(to_value(&p), expected);
}

#[test]
fn folder_wire_format() {
    let p = EntryPayload::Folder(FolderPayload {
        meta: meta("Work", EntryType::Folder),
    });
    let expected = json!({
        "name": "Work", "entry_type": "Folder",
        "url": "https://example.com",
        "tag_ids": ["01H0000000000000000000TAG1"],
        "folder_id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        "is_favorite": true, "notes": "hello", "payload_schema": 1,
    });
    assert_eq!(to_value(&p), expected);
}

#[test]
fn tag_payload_wire_format() {
    let t = TagPayload {
        name: "github".into(),
        color: Some("#1D9E75".into()),
        sort_order: 3,
    };
    let expected = json!({"name": "github", "color": "#1D9E75", "sort_order": 3});
    assert_eq!(serde_json::to_value(&t).unwrap(), expected);
}

#[test]
fn payload_schema_too_high_rejects() {
    let json_bytes = br#"{"name":"x","entry_type":"Note","content":"c","payload_schema":2}"#;
    let err = EntryPayload::from_decrypted_json(json_bytes).unwrap_err();
    match err {
        vedge_core::domain::vault::errors::VaultError::UnsupportedPayloadSchema(2) => {}
        other => panic!("expected UnsupportedPayloadSchema(2), got {other:?}"),
    }
}

#[test]
fn unknown_entry_type_goes_to_unknown_variant() {
    let json_bytes =
        br#"{"name":"x","entry_type":"Passkey","credential_id":"abc","payload_schema":1}"#;
    let p = EntryPayload::from_decrypted_json(json_bytes).unwrap();
    match p {
        EntryPayload::Unknown(u) => {
            assert_eq!(u.meta.entry_type, EntryType::Unknown("Passkey".into()));
            assert_eq!(u.unknown_fields["credential_id"], "abc");
        }
        other => panic!("expected Unknown, got {other:?}"),
    }
}

#[test]
fn unknown_variant_refuses_to_serialize() {
    let json_bytes = br#"{"name":"x","entry_type":"Passkey"}"#;
    let p = EntryPayload::from_decrypted_json(json_bytes).unwrap();
    assert!(p.to_encryptable_json().is_err());
}

#[test]
fn login_debug_does_not_leak_password() {
    let p = LoginPayload {
        meta: meta("github", EntryType::Login),
        username: "alice".into(),
        password: SecretString::from("hunter2"),
        totp_secret: None,
        recovery_codes: vec![],
    };
    let debug = format!("{p:?}");
    assert!(!debug.contains("hunter2"), "Debug leaked password: {debug}");
    // Sanity check that ExposeSecret still works on the struct.
    assert_eq!(p.password.expose_secret(), "hunter2");
}
