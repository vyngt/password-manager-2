//! Integration tests for the `export_entries` use case (slice 5.3a): the full
//! path from an unlocked vault through the real decrypt → map → seal pipeline.
//!
//! The DTO/envelope round-trip itself is proven in `export_format.rs`; this
//! proves the *use case* — tag/folder resolution from the live index, both
//! output formats, the no-plaintext-temp-file discipline (Decision ②), and the
//! single `Exported` audit row.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use secrecy::{ExposeSecret, SecretString};
use zeroize::Zeroizing;

use common::Harness;
use vedge_core::application::vault::use_cases::{
    CreateEntryInput, ExportEntriesInput, ExportFormat, create_entry, create_tag, export_entries,
};
use vedge_core::domain::export::{ExportBundle, ExportEntry};
use vedge_core::domain::vault::entities::AuditAction;
use vedge_core::domain::vault::payloads::{
    CommonMeta, EntryPayload, EntryType, FolderPayload, LoginPayload,
};
use vedge_core::infrastructure::export::{archive, csv, envelope};

use vedge_core::application::vault::ports::VaultRepository;
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::unlock_vault::UnlockVaultInput;

const PASSPHRASE: &str = "a strong export passphrase";

async fn unlock(h: &Harness) -> VaultSession {
    common::build_unlock(h)
        .execute(UnlockVaultInput {
            vault_path: h.home.clone(),
            master_password: h.master_password.clone(),
            secret_key: None, // resolved from the harness's memory keychain
        })
        .await
        .unwrap()
}

fn login_payload(name: &str, username: &str, password: &str) -> EntryPayload {
    EntryPayload::Login(LoginPayload {
        meta: CommonMeta::new(name, EntryType::Login),
        username: username.to_owned(),
        password: SecretString::from(password.to_owned()),
        totp_secret: None,
        totp_params: vedge_core::TotpParams::default(),
        recovery_codes: vec![],
    })
}

/// Build a vault with: a seeded login (leading-`=` password), a seeded Unknown, a
/// tagged login, a folder, and a login inside that folder. Returns the session
/// plus the folder's minted ULID (to assert the remap-source id survives).
async fn seed_vault(h: &Harness) -> (VaultSession, String) {
    h.seed_login("github", "octocat", "=hunter2").await;
    h.seed_unknown("a passkey", "Passkey").await;
    let mut session = unlock(h).await;

    // A tagged login through the real write path (exercises tag-name resolution).
    let tag = create_tag(&mut session, "work", None).await.unwrap();
    let mut gitlab = login_payload("gitlab", "octo2", "p2");
    gitlab.meta_mut().tag_ids = vec![tag];
    create_entry(&mut session, CreateEntryInput { payload: gitlab })
        .await
        .unwrap();

    // A folder + a child login (exercises folder_id passthrough).
    let folder = create_entry(
        &mut session,
        CreateEntryInput {
            payload: EntryPayload::Folder(FolderPayload {
                meta: CommonMeta::new("Personal", EntryType::Folder),
            }),
        },
    )
    .await
    .unwrap()
    .entry_id;
    let mut bank = login_payload("bank", "me", "p3");
    bank.meta_mut().folder_id = Some(folder.clone());
    create_entry(&mut session, CreateEntryInput { payload: bank })
        .await
        .unwrap();

    (session, folder.as_str().to_owned())
}

fn find_login<'a>(
    bundle: &'a ExportBundle,
    name: &str,
) -> &'a vedge_core::domain::export::ExportLogin {
    bundle
        .entries
        .iter()
        .find_map(|e| match e {
            ExportEntry::Login(l) if l.meta.name == name => Some(l),
            _ => None,
        })
        .unwrap_or_else(|| panic!("login {name} missing from export"))
}

#[tokio::test]
async fn encrypted_export_round_trips_all_entries_from_a_live_vault() {
    let h = Harness::fresh().await;
    let (session, folder_ulid) = seed_vault(&h).await;

    // Export into its OWN empty dir so we can assert no stray temp files.
    let out = tempfile::tempdir().unwrap();
    let dest = out.path().join("vault-export.vedgex");
    let report = export_entries(
        &session,
        ExportEntriesInput {
            dest: dest.clone(),
            format: ExportFormat::Encrypted {
                passphrase: Zeroizing::new(PASSPHRASE.to_owned()),
            },
        },
    )
    .await
    .unwrap();
    assert!(report.encrypted);
    assert_eq!(report.entry_count, 5);

    // 🔴 Decision ② — no plaintext temp file: the only thing written is the dest.
    let written: Vec<_> = std::fs::read_dir(out.path()).unwrap().collect();
    assert_eq!(
        written.len(),
        1,
        "an encrypted export must write ONLY its destination"
    );

    // The file is opaque — no secret leaks.
    let sealed = std::fs::read(&dest).unwrap();
    assert!(!sealed.windows(8).any(|w| w == b"=hunter2"));

    // Open + verify the bundle.
    let opened = envelope::open(PASSPHRASE.as_bytes(), &sealed).unwrap();
    let entries_json = archive::read_member(&opened, archive::ENTRIES_MEMBER)
        .unwrap()
        .unwrap();
    let bundle: ExportBundle = serde_json::from_slice(&entries_json).unwrap();
    assert_eq!(bundle.entries.len(), 5);

    // github: leading-`=` password survives verbatim.
    assert_eq!(
        find_login(&bundle, "github").password.expose_secret(),
        "=hunter2"
    );
    // gitlab: tag resolved from the index by NAME.
    assert_eq!(
        find_login(&bundle, "gitlab").meta.tags,
        vec!["work".to_owned()]
    );
    // bank: the source folder ULID rode through untouched (import remaps it in 5.3b).
    assert_eq!(
        find_login(&bundle, "bank").meta.folder_id.as_deref(),
        Some(folder_ulid.as_str())
    );
    // The Unknown survived as Unknown.
    assert_eq!(
        bundle
            .entries
            .iter()
            .filter(|e| matches!(e, ExportEntry::Unknown(_)))
            .count(),
        1
    );

    // A single vault-level `Exported` audit row (read via the harness's connection
    // to the same `.vdb` — the session's repo is crate-private).
    let audit = h.repo.recent_audit(50).await.unwrap();
    assert_eq!(
        audit
            .iter()
            .filter(|e| e.action == AuditAction::Exported)
            .count(),
        1
    );
}

#[tokio::test]
async fn csv_export_is_logins_only_and_plaintext() {
    let h = Harness::fresh().await;
    let (session, _) = seed_vault(&h).await;

    let out = tempfile::tempdir().unwrap();
    let dest = out.path().join("logins.csv");
    let report = export_entries(
        &session,
        ExportEntriesInput {
            dest: dest.clone(),
            format: ExportFormat::Csv {
                spreadsheet_safe: false,
            },
        },
    )
    .await
    .unwrap();
    assert!(!report.encrypted);
    // Logins only: github + gitlab + bank = 3 (folder + unknown excluded).
    assert_eq!(report.entry_count, 3);

    let bytes = std::fs::read(&dest).unwrap();
    // Plaintext by design — the header is readable and the `=`-password is verbatim.
    assert!(bytes.starts_with(b"name,username,password,url,notes,tags"));
    let rows = csv::parse_csv(&bytes);
    let logins: Vec<_> = rows.iter().filter_map(|r| r.result.as_ref().ok()).collect();
    assert_eq!(logins.len(), 3);
    let github = logins.iter().find(|l| l.name == "github").unwrap();
    assert_eq!(github.password.expose_secret(), "=hunter2");
}
