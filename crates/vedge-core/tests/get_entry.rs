#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::too_many_lines,
    clippy::needless_pass_by_value
)]

mod common;

use common::{Harness, build_unlock};
use secrecy::{ExposeSecret, SecretString};

use vedge_core::application::vault::ports::VaultRepository;
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::{
    CreateEntryInput, GetEntryInput, UnlockVaultInput, create_entry, get_entry,
};
use vedge_core::domain::shared::EntryId;
use vedge_core::domain::vault::entities::AuditAction;
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::domain::vault::payloads::{
    Address, ApiKeyPayload, CardPayload, CommonMeta, EntryPayload, EntryType, EnvVar,
    EnvVarsPayload, FolderPayload, IdentityPayload, LoginPayload, NotePayload, SshKeyPayload,
};

async fn unlock(h: &Harness) -> VaultSession {
    build_unlock(h)
        .execute(UnlockVaultInput {
            vault_path: h.home.clone(),
            master_password: h.master_password.clone(),
            secret_key: None,
        })
        .await
        .unwrap()
}

async fn create(session: &mut VaultSession, payload: EntryPayload) -> EntryId {
    create_entry(session, CreateEntryInput { payload })
        .await
        .unwrap()
        .entry_id
}

async fn reveal(session: &mut VaultSession, id: EntryId) -> EntryPayload {
    get_entry(session, GetEntryInput { entry_id: id })
        .await
        .unwrap()
}

#[tokio::test]
async fn get_entry_roundtrips_login() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create(
        &mut session,
        EntryPayload::Login(LoginPayload {
            meta: CommonMeta::new("GitHub", EntryType::Login),
            username: "alice".into(),
            password: SecretString::from("hunter2"),
            totp_secret: Some(SecretString::from("GEZDGNBVGY3TQOJQ".to_owned())),
            totp_params: vedge_core::TotpParams::default(),
            recovery_codes: vec![SecretString::from("code-1"), SecretString::from("code-2")],
        }),
    )
    .await;

    let EntryPayload::Login(p) = reveal(&mut session, id).await else {
        panic!("expected Login");
    };
    assert_eq!(p.meta.name, "GitHub");
    assert_eq!(p.username, "alice");
    assert_eq!(p.password.expose_secret(), "hunter2");
    assert_eq!(
        p.totp_secret.as_ref().unwrap().expose_secret(),
        "GEZDGNBVGY3TQOJQ"
    );
    assert_eq!(p.recovery_codes.len(), 2);
    assert_eq!(p.recovery_codes[0].expose_secret(), "code-1");
}

#[tokio::test]
async fn get_entry_roundtrips_each_type() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;

    // Card
    let id = create(
        &mut session,
        EntryPayload::Card(CardPayload {
            meta: CommonMeta::new("Visa", EntryType::Card),
            cardholder_name: "Alice A".into(),
            number: SecretString::from("4111111111111111"),
            expiry_month: 12,
            expiry_year: 2030,
            cvv: SecretString::from("123"),
            pin: Some(SecretString::from("4321")),
        }),
    )
    .await;
    let EntryPayload::Card(p) = reveal(&mut session, id).await else {
        panic!("card");
    };
    assert_eq!(p.cardholder_name, "Alice A");
    assert_eq!(p.number.expose_secret(), "4111111111111111");
    assert_eq!(p.expiry_month, 12);
    assert_eq!(p.expiry_year, 2030);
    assert_eq!(p.cvv.expose_secret(), "123");
    assert_eq!(p.pin.as_ref().unwrap().expose_secret(), "4321");

    // SshKey
    let id = create(
        &mut session,
        EntryPayload::SshKey(SshKeyPayload {
            meta: CommonMeta::new("id_ed25519", EntryType::SshKey),
            private_key_pem: SecretString::from("-----BEGIN KEY-----"),
            passphrase: Some(SecretString::from("pp")),
            public_key: "ssh-ed25519 AAAA".into(),
            fingerprint: "SHA256:abc".into(),
            key_type: "ed25519".into(),
        }),
    )
    .await;
    let EntryPayload::SshKey(p) = reveal(&mut session, id).await else {
        panic!("ssh");
    };
    assert_eq!(p.private_key_pem.expose_secret(), "-----BEGIN KEY-----");
    assert_eq!(p.passphrase.as_ref().unwrap().expose_secret(), "pp");
    assert_eq!(p.public_key, "ssh-ed25519 AAAA");
    assert_eq!(p.key_type, "ed25519");

    // ApiKey
    let id = create(
        &mut session,
        EntryPayload::ApiKey(ApiKeyPayload {
            meta: CommonMeta::new("OpenAI", EntryType::ApiKey),
            key: SecretString::from("sk-123"),
            secret: Some(SecretString::from("shh")),
            endpoint: Some("https://api".into()),
            expiry: None,
            key_type: Some("bearer".into()),
        }),
    )
    .await;
    let EntryPayload::ApiKey(p) = reveal(&mut session, id).await else {
        panic!("api");
    };
    assert_eq!(p.key.expose_secret(), "sk-123");
    assert_eq!(p.secret.as_ref().unwrap().expose_secret(), "shh");
    assert_eq!(p.endpoint.as_deref(), Some("https://api"));
    assert_eq!(p.key_type.as_deref(), Some("bearer"));

    // EnvVars
    let id = create(
        &mut session,
        EntryPayload::EnvVars(EnvVarsPayload {
            meta: CommonMeta::new(".env", EntryType::EnvVars),
            vars: vec![
                EnvVar {
                    key: "DB_URL".into(),
                    value: SecretString::from("postgres://"),
                },
                EnvVar {
                    key: "TOKEN".into(),
                    value: SecretString::from("abc"),
                },
            ],
        }),
    )
    .await;
    let EntryPayload::EnvVars(p) = reveal(&mut session, id).await else {
        panic!("env");
    };
    assert_eq!(p.vars.len(), 2);
    assert_eq!(p.vars[0].key, "DB_URL");
    assert_eq!(p.vars[0].value.expose_secret(), "postgres://");

    // Note
    let id = create(
        &mut session,
        EntryPayload::Note(NotePayload {
            meta: CommonMeta::new("Recovery", EntryType::Note),
            content: SecretString::from("secret body"),
        }),
    )
    .await;
    let EntryPayload::Note(p) = reveal(&mut session, id).await else {
        panic!("note");
    };
    assert_eq!(p.content.expose_secret(), "secret body");

    // Identity
    let id = create(
        &mut session,
        EntryPayload::Identity(IdentityPayload {
            meta: CommonMeta::new("Me", EntryType::Identity),
            first_name: "Alice".into(),
            last_name: "Anderson".into(),
            email: "alice@example.com".into(),
            phone: Some("+1".into()),
            address: Some(Address {
                line1: "1 St".into(),
                line2: None,
                city: "Town".into(),
                state: Some("CA".into()),
                postal_code: "90001".into(),
                country: "US".into(),
            }),
            date_of_birth: Some("1990-01-01".into()),
            national_id: Some(SecretString::from("ID-1")),
        }),
    )
    .await;
    let EntryPayload::Identity(p) = reveal(&mut session, id).await else {
        panic!("identity");
    };
    assert_eq!(p.first_name, "Alice");
    assert_eq!(p.email, "alice@example.com");
    assert_eq!(p.address.as_ref().unwrap().city, "Town");
    assert_eq!(p.national_id.as_ref().unwrap().expose_secret(), "ID-1");

    // Folder
    let id = create(
        &mut session,
        EntryPayload::Folder(FolderPayload {
            meta: CommonMeta::new("Work", EntryType::Folder),
        }),
    )
    .await;
    let EntryPayload::Folder(p) = reveal(&mut session, id).await else {
        panic!("folder");
    };
    assert_eq!(p.meta.name, "Work");
}

#[tokio::test]
async fn get_entry_rejects_unknown() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = h.seed_unknown("Legacy", "Passkey").await;

    let err = get_entry(&mut session, GetEntryInput { entry_id: id })
        .await
        .unwrap_err();
    assert!(matches!(err, VaultError::UnsupportedEntryType(_)));
}

#[tokio::test]
async fn get_entry_appends_viewed_audit() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create(
        &mut session,
        EntryPayload::Note(NotePayload {
            meta: CommonMeta::new("n", EntryType::Note),
            content: SecretString::from("x"),
        }),
    )
    .await;

    reveal(&mut session, id.clone()).await;

    let events = h.repo.recent_audit(20).await.unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e.action, AuditAction::Viewed) && e.entry_id.as_ref() == Some(&id))
    );
}

#[tokio::test]
async fn get_entry_wrong_id_is_not_found() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let ghost = EntryId::new();

    let err = get_entry(&mut session, GetEntryInput { entry_id: ghost })
        .await
        .unwrap_err();
    assert!(matches!(err, VaultError::EntryNotFound(_)));
}
