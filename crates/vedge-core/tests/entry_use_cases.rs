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
use secrecy::{ExposeSecret, SecretString};

use vedge_core::application::vault::ports::VaultRepository;
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::{
    CreateEntryInput, GetEntryInput, UnlockVaultInput, UpdateEntryInput, create_entry, get_entry,
    hard_delete_entry, restore_entry, soft_delete_entry, update_entry,
};
use vedge_core::domain::shared::EntryId;
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::domain::vault::payloads::{
    CommonMeta, EntryPayload, EntryType, FolderPayload, LoginPayload, NotePayload,
};
use vedge_core::{SecretListUpdate, SecretUpdate, SecretUpdates, TotpUpdate};

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

    h.assert_coherent().await;
}

#[tokio::test]
async fn edit_name_only_does_not_change_the_password() {
    // Spec #5 — the one a user notices. Simulate a sealed edit-form save where the
    // form changed only the NAME: the password field is an empty placeholder and
    // the intent is Unchanged. The password must survive; the ciphertext rotates.
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login_payload("gh", "hunter2"),
        },
    )
    .await
    .unwrap()
    .entry_id;
    let v1 = h.repo.get_entry(&id).await.unwrap();

    update_entry(
        &mut session,
        UpdateEntryInput {
            entry_id: id.clone(),
            payload: login_payload("gh-renamed", ""), // placeholder password
            secrets: SecretUpdates::Login {
                password: SecretUpdate::Unchanged,
                recovery_codes: SecretListUpdate::Unchanged,
                totp: TotpUpdate::Unchanged,
            },
        },
    )
    .await
    .unwrap();

    // The ciphertext rotated (fresh nonce + bumped version)...
    let v2 = h.repo.get_entry(&id).await.unwrap();
    assert!(v2.version > v1.version);
    assert_ne!(v1.nonce, v2.nonce, "a fresh nonce on every write");

    // ...but the decrypted password is unchanged, and the name DID change.
    let payload = get_entry(
        &mut session,
        GetEntryInput {
            entry_id: id.clone(),
        },
    )
    .await
    .unwrap();
    let EntryPayload::Login(l) = payload else {
        panic!("expected Login")
    };
    assert_eq!(
        l.password.expose_secret(),
        "hunter2",
        "the password survives a name-only edit"
    );
    assert_eq!(l.meta.name, "gh-renamed");
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
        UpdateEntryInput::full(id.clone(), login_payload("gh", "pw2")),
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

    h.assert_coherent().await;
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

    h.assert_coherent().await;
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

    h.assert_coherent().await;
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

    // A refused delete must not have left the folder or its child inconsistent.
    h.assert_coherent().await;
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
        UpdateEntryInput::full(EntryId::new(), login_payload("gh", "pw")),
    )
    .await
    .unwrap_err();
    assert!(matches!(err, VaultError::EntryNotFound(_)));
}
