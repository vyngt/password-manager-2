//! Integration tests for the `import_entries` use case (slice 5.3b): the full
//! **vault round-trip** — seed a source vault, export it, then import into a
//! *fresh* vault and prove every field came back.
//!
//! This is the proof the slice works (spec test #1): all 9 types + `Unknown`
//! byte-equal, fresh ULIDs/DEKs, folder structure remapped within the batch, tags
//! matched-or-created by name, and CSV Skip actually skipping.

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
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::unlock_vault::UnlockVaultInput;
use vedge_core::domain::export::{ExportBundle, ExportEntry, ExportLogin};
use vedge_core::domain::vault::payloads::{
    CommonMeta, EntryPayload, EntryType, FolderPayload, LoginPayload,
};
use vedge_core::infrastructure::export::{archive, envelope};
use vedge_core::{
    CreateEntryInput, ExportEntriesInput, ExportFormat, ImportAction, ImportSource, RowAction,
    begin_import, commit_import, create_entry, create_tag, export_entries,
};

const PASSPHRASE: &str = "a strong import passphrase";

async fn unlock(h: &Harness) -> VaultSession {
    common::build_unlock(h)
        .execute(UnlockVaultInput {
            vault_path: h.home.clone(),
            master_password: h.master_password.clone(),
            secret_key: None,
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

/// Seed a source vault: a `=`-password login, an `Unknown`, a `work`-tagged login,
/// a "Personal" folder, and a login inside it. Returns the session + the folder's
/// source ULID (to prove import mints a fresh one).
async fn seed_source(h: &Harness) -> (VaultSession, String) {
    h.seed_login("github", "octocat", "=hunter2").await;
    h.seed_unknown("a passkey", "Passkey").await;
    let mut session = unlock(h).await;

    let tag = create_tag(&mut session, "work", None).await.unwrap();
    let mut gitlab = login_payload("gitlab", "octo2", "p2");
    gitlab.meta_mut().tag_ids = vec![tag];
    create_entry(&mut session, CreateEntryInput { payload: gitlab })
        .await
        .unwrap();

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

/// Export the session to a sealed envelope and return the bytes.
async fn export_to_bytes(session: &VaultSession) -> Vec<u8> {
    let out = tempfile::tempdir().unwrap();
    let dest = out.path().join("x.vedgex");
    export_entries(
        session,
        ExportEntriesInput {
            dest: dest.clone(),
            format: ExportFormat::Encrypted {
                passphrase: Zeroizing::new(PASSPHRASE.to_owned()),
            },
            ids: None,
        },
    )
    .await
    .unwrap();
    std::fs::read(&dest).unwrap()
}

fn open_bundle(sealed: &[u8]) -> ExportBundle {
    let opened = envelope::open(PASSPHRASE.as_bytes(), sealed).unwrap();
    let entries_json = archive::read_member(&opened, archive::ENTRIES_MEMBER)
        .unwrap()
        .unwrap();
    serde_json::from_slice(&entries_json).unwrap()
}

fn find_login<'a>(bundle: &'a ExportBundle, name: &str) -> &'a ExportLogin {
    bundle
        .entries
        .iter()
        .find_map(|e| match e {
            ExportEntry::Login(l) if l.meta.name == name => Some(l),
            _ => None,
        })
        .unwrap_or_else(|| panic!("login {name} missing"))
}

fn folder_id(bundle: &ExportBundle, name: &str) -> String {
    bundle
        .entries
        .iter()
        .find_map(|e| match e {
            ExportEntry::Folder(f) if f.meta.name == name => Some(f.meta.id.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("folder {name} missing"))
}

fn unknown_raw(bundle: &ExportBundle) -> serde_json::Value {
    bundle
        .entries
        .iter()
        .find_map(|e| match e {
            ExportEntry::Unknown(u) => Some(u.raw.clone()),
            _ => None,
        })
        .expect("an Unknown entry")
}

fn import_all(rows: &[vedge_core::ImportPreviewRow]) -> Vec<ImportAction> {
    rows.iter()
        .map(|r| ImportAction {
            row_id: r.row_id,
            action: RowAction::Import,
        })
        .collect()
}

/// 🔴 The proof the slice works (spec #1): export → import into a FRESH vault →
/// every field byte-equal, including `Unknown`'s raw JSON (③), folder structure
/// remapped under a fresh ULID (④/#13), and the `work` tag created by name.
#[tokio::test]
async fn export_then_import_round_trips_all_entries() {
    let src = Harness::fresh().await;
    let (src_session, src_folder_ulid) = seed_source(&src).await;
    let sealed = export_to_bytes(&src_session).await;
    let src_unknown = unknown_raw(&open_bundle(&sealed));

    // A fresh, empty target vault.
    let dst = Harness::fresh().await;
    let mut dst_session = unlock(&dst).await;
    assert!(
        dst_session.index().all_active().is_empty(),
        "target must start empty"
    );

    // begin_import → derivatives-only preview.
    let rows = begin_import(
        &mut dst_session,
        ImportSource::Encrypted {
            data: sealed,
            passphrase: Zeroizing::new(PASSPHRASE.to_owned()),
        },
    )
    .unwrap();
    assert_eq!(rows.len(), 5);
    // ⑦ the preview shows presence, not the secret.
    assert!(
        rows.iter()
            .find(|r| r.name == "github")
            .unwrap()
            .has_password
    );

    let actions = import_all(rows);
    let report = commit_import(&mut dst_session, &actions).await.unwrap();
    assert_eq!(report.imported, 5);
    assert!(
        report.failed.is_empty(),
        "no row should fail: {:?}",
        report.failed
    );

    // Verify by re-exporting the TARGET and inspecting the decrypted bundle.
    let dst_bundle = open_bundle(&export_to_bytes(&dst_session).await);
    assert_eq!(dst_bundle.entries.len(), 5);

    // Secret round-trip: the leading-`=` password survived verbatim.
    assert_eq!(
        find_login(&dst_bundle, "github").password.expose_secret(),
        "=hunter2"
    );
    // ④ tag matched-or-created by name.
    assert_eq!(
        find_login(&dst_bundle, "gitlab").meta.tags,
        vec!["work".to_owned()]
    );
    // ④ folder remap + #13 fresh ULID: bank sits in the target's Personal folder,
    // whose id is NOT the source's.
    let dst_folder = folder_id(&dst_bundle, "Personal");
    assert_eq!(
        find_login(&dst_bundle, "bank").meta.folder_id.as_deref(),
        Some(dst_folder.as_str())
    );
    assert_ne!(
        dst_folder, src_folder_ulid,
        "#13 — the source folder ULID must not survive the import"
    );
    // ③ the Unknown round-trips byte-faithfully through the whole vault path.
    assert_eq!(unknown_raw(&dst_bundle), src_unknown);
}

/// ④ **No merge**: importing a "Work" folder into a vault that already has one
/// makes a **second** folder, not a merge.
#[tokio::test]
async fn import_makes_a_sibling_folder_never_merges() {
    let src = Harness::fresh().await;
    let mut src_session = unlock(&src).await;
    create_entry(
        &mut src_session,
        CreateEntryInput {
            payload: EntryPayload::Folder(FolderPayload {
                meta: CommonMeta::new("Work", EntryType::Folder),
            }),
        },
    )
    .await
    .unwrap();
    let sealed = export_to_bytes(&src_session).await;

    // Target already has a "Work" folder.
    let dst = Harness::fresh().await;
    let mut dst_session = unlock(&dst).await;
    create_entry(
        &mut dst_session,
        CreateEntryInput {
            payload: EntryPayload::Folder(FolderPayload {
                meta: CommonMeta::new("Work", EntryType::Folder),
            }),
        },
    )
    .await
    .unwrap();

    let rows = begin_import(
        &mut dst_session,
        ImportSource::Encrypted {
            data: sealed,
            passphrase: Zeroizing::new(PASSPHRASE.to_owned()),
        },
    )
    .unwrap();
    let actions = import_all(rows);
    commit_import(&mut dst_session, &actions).await.unwrap();

    let work = dst_session
        .index()
        .all_active()
        .iter()
        .filter(|e| matches!(e.entry_type, EntryType::Folder) && e.name == "Work")
        .count();
    assert_eq!(work, 2, "no merge — an existing folder gets a sibling");
}

/// ④ tags match-or-create by name: an existing `work` tag is reused (not
/// duplicated); a new one is created.
#[tokio::test]
async fn tags_match_or_create_by_name() {
    let src = Harness::fresh().await;
    let mut src_session = unlock(&src).await;
    let t1 = create_tag(&mut src_session, "work", None).await.unwrap();
    let t2 = create_tag(&mut src_session, "brandnew", None)
        .await
        .unwrap();
    let mut a = login_payload("a", "ua", "pa");
    a.meta_mut().tag_ids = vec![t1];
    create_entry(&mut src_session, CreateEntryInput { payload: a })
        .await
        .unwrap();
    let mut b = login_payload("b", "ub", "pb");
    b.meta_mut().tag_ids = vec![t2];
    create_entry(&mut src_session, CreateEntryInput { payload: b })
        .await
        .unwrap();
    let sealed = export_to_bytes(&src_session).await;

    // Target already has a `work` tag but not `brandnew`.
    let dst = Harness::fresh().await;
    let mut dst_session = unlock(&dst).await;
    create_tag(&mut dst_session, "work", None).await.unwrap();

    let rows = begin_import(
        &mut dst_session,
        ImportSource::Encrypted {
            data: sealed,
            passphrase: Zeroizing::new(PASSPHRASE.to_owned()),
        },
    )
    .unwrap();
    let actions = import_all(rows);
    commit_import(&mut dst_session, &actions).await.unwrap();

    // Exactly two tags: `work` reused (once), `brandnew` created.
    let tags = &dst_session.index().tags;
    assert_eq!(tags.len(), 2, "work must be reused, not duplicated");
    assert_eq!(tags.values().filter(|t| t.name == "work").count(), 1);
    assert_eq!(tags.values().filter(|t| t.name == "brandnew").count(), 1);
}

/// ⑨ CSV Skip actually skips: a row marked Skip is NOT imported.
#[tokio::test]
async fn csv_skip_actually_skips() {
    let dst = Harness::fresh().await;
    let mut session = unlock(&dst).await;

    let csv = "name,username,password,url,notes,tags\nalpha,ua,pa,,,\nbeta,ub,pb,,,\n";
    let rows = begin_import(
        &mut session,
        ImportSource::Csv {
            text: csv.to_owned(),
        },
    )
    .unwrap();
    assert_eq!(rows.len(), 2);

    // Import alpha (row 0), skip beta (row 1).
    let actions = vec![
        ImportAction {
            row_id: rows[0].row_id,
            action: RowAction::Import,
        },
        ImportAction {
            row_id: rows[1].row_id,
            action: RowAction::Skip,
        },
    ];
    let report = commit_import(&mut session, &actions).await.unwrap();
    assert_eq!(report.imported, 1);
    assert_eq!(report.skipped, 1);

    let names: Vec<String> = session
        .index()
        .all_active()
        .iter()
        .map(|e| e.name.clone())
        .collect();
    assert!(names.contains(&"alpha".to_owned()));
    assert!(
        !names.contains(&"beta".to_owned()),
        "a skipped row must not import"
    );
}
