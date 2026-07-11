#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::needless_pass_by_value
)]

mod common;

use common::{Harness, build_unlock};
use secrecy::SecretString;

use vedge_core::application::vault::ports::VaultRepository;
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::{
    CreateEntryInput, UnlockVaultInput, UpdateEntryInput, create_entry, hard_delete_entry,
    restore_entry, soft_delete_entry, update_entry,
};
use vedge_core::domain::shared::EntryId;
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::domain::vault::payloads::{
    CommonMeta, EntryPayload, EntryType, FolderPayload, LoginPayload, NotePayload,
};

async fn unlock(h: &Harness) -> VaultSession {
    build_unlock(h)
        .execute(UnlockVaultInput {
            vault_path: h.vdb_path.clone(),
            master_password: h.master_password.clone(),
            secret_key: None,
        })
        .await
        .unwrap()
}

fn login_payload(name: &str, pw: &str) -> EntryPayload {
    EntryPayload::Login(LoginPayload {
        meta: CommonMeta::new(name, EntryType::Login),
        username: "alice".into(),
        password: SecretString::from(pw),
        totp_secret: None,
        totp_params: vedge_core::TotpParams::default(),
        recovery_codes: vec![],
    })
}

fn folder_payload(name: &str) -> EntryPayload {
    EntryPayload::Folder(FolderPayload {
        meta: CommonMeta::new(name, EntryType::Folder),
    })
}

fn note_with_folder(name: &str, folder: Option<EntryId>) -> EntryPayload {
    let mut meta = CommonMeta::new(name, EntryType::Note);
    meta.folder_id = folder;
    EntryPayload::Note(NotePayload {
        meta,
        content: SecretString::from("body"),
    })
}

#[tokio::test]
async fn create_then_index_reflects() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;

    let out = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login_payload("gh", "pw"),
        },
    )
    .await
    .unwrap();

    let fetched = session.index().entries.get(&out.entry_id).unwrap();
    assert_eq!(fetched.name, "gh");
}

#[tokio::test]
async fn update_bumps_version_and_refreshes_nonce() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login_payload("gh", "pw"),
        },
    )
    .await
    .unwrap()
    .entry_id;

    let row_v1 = h.repo.get_entry(&id).await.unwrap();
    assert_eq!(row_v1.version, 1);

    update_entry(
        &mut session,
        UpdateEntryInput {
            entry_id: id.clone(),
            payload: login_payload("gh", "pw2"),
            totp: vedge_core::TotpUpdate::Unchanged,
        },
    )
    .await
    .unwrap();

    let row_v2 = h.repo.get_entry(&id).await.unwrap();
    assert_eq!(row_v2.version, 2);
    assert_ne!(
        row_v1.nonce, row_v2.nonce,
        "nonce must rotate on every write"
    );
    assert_ne!(row_v1.ciphertext, row_v2.ciphertext);
    // DEK is stable across updates — the wrapped blob is the same.
    assert_eq!(row_v1.dek_wrapped, row_v2.dek_wrapped);
}

#[tokio::test]
async fn soft_delete_then_restore() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login_payload("gh", "pw"),
        },
    )
    .await
    .unwrap()
    .entry_id;

    soft_delete_entry(&mut session, &id).await.unwrap();
    assert!(session.index().all_active().is_empty());
    assert_eq!(session.index().all_trashed().len(), 1);

    restore_entry(&mut session, &id).await.unwrap();
    assert_eq!(session.index().all_active().len(), 1);
    assert!(session.index().all_trashed().is_empty());
}

#[tokio::test]
async fn hard_delete_removes_from_db_and_index() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login_payload("gh", "pw"),
        },
    )
    .await
    .unwrap()
    .entry_id;

    hard_delete_entry(&mut session, &id).await.unwrap();
    assert!(session.index().entries.get(&id).is_none());
    let err = h.repo.get_entry(&id).await.unwrap_err();
    assert!(matches!(err, VaultError::EntryNotFound(_)));
}

#[tokio::test]
async fn hard_delete_folder_with_children_is_refused() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let folder_id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: folder_payload("Work"),
        },
    )
    .await
    .unwrap()
    .entry_id;
    let _ = create_entry(
        &mut session,
        CreateEntryInput {
            payload: note_with_folder("child", Some(folder_id.clone())),
        },
    )
    .await
    .unwrap();

    let err = hard_delete_entry(&mut session, &folder_id)
        .await
        .unwrap_err();
    assert!(matches!(err, VaultError::FolderNotEmpty));
}

#[tokio::test]
async fn create_rejects_missing_folder() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let ghost = EntryId::new();
    let err = create_entry(
        &mut session,
        CreateEntryInput {
            payload: note_with_folder("orphan", Some(ghost.clone())),
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(err, VaultError::FolderNotFound(id) if id == ghost));
}

#[tokio::test]
async fn update_rejects_nonexistent_entry() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let err = update_entry(
        &mut session,
        UpdateEntryInput {
            entry_id: EntryId::new(),
            payload: login_payload("gh", "pw"),
            totp: vedge_core::TotpUpdate::Unchanged,
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(err, VaultError::EntryNotFound(_)));
}
