#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::needless_pass_by_value
)]

mod common;

use std::time::Duration;

use common::{Harness, build_unlock};
use secrecy::SecretString;

use vedge_core::application::vault::ports::VaultRepository;
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::{
    CopyFieldInput, CreateEntryInput, FieldSelector, UnlockVaultInput, copy_field, create_entry,
    move_entry,
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

fn login(name: &str, pw: &str, totp: Option<&str>) -> EntryPayload {
    EntryPayload::Login(LoginPayload {
        meta: CommonMeta::new(name, EntryType::Login),
        username: "alice".into(),
        password: SecretString::from(pw),
        totp_secret: totp.map(|s| SecretString::from(s.to_owned())),
        recovery_codes: vec![],
    })
}

#[tokio::test]
async fn copy_password_puts_value_on_clipboard() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("gh", "hunter2", None),
        },
    )
    .await
    .unwrap()
    .entry_id;

    let mut input = CopyFieldInput::new(id, FieldSelector::Password);
    input.clear_after_secs = 9999; // we don't want the clear to fire during the test
    copy_field(&mut session, input).await.unwrap();

    assert_eq!(h.clipboard.peek().as_deref(), Some("hunter2"));
}

#[tokio::test]
async fn copy_wrong_field_for_type_errors() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: EntryPayload::Note(NotePayload {
                meta: CommonMeta::new("note", EntryType::Note),
                content: SecretString::from("body"),
            }),
        },
    )
    .await
    .unwrap()
    .entry_id;

    let mut input = CopyFieldInput::new(id, FieldSelector::Password);
    input.clear_after_secs = 9999;
    let err = copy_field(&mut session, input).await.unwrap_err();
    assert!(matches!(err, VaultError::FieldNotApplicable));
}

#[tokio::test]
async fn copy_totp_produces_six_digits() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    // RFC 6238 test-vector secret ("12345678901234567890", 20 bytes / 160 bits)
    // encoded as base32 — totp-rs rejects secrets under 128 bits.
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("gh", "pw", Some("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ")),
        },
    )
    .await
    .unwrap()
    .entry_id;

    let mut input = CopyFieldInput::new(id, FieldSelector::TotpCode);
    input.clear_after_secs = 9999;
    copy_field(&mut session, input).await.unwrap();

    let code = h.clipboard.peek().unwrap();
    assert_eq!(code.len(), 6);
    assert!(code.chars().all(|c| c.is_ascii_digit()));
}

#[tokio::test]
async fn copy_totp_without_secret_errors() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("gh", "pw", None),
        },
    )
    .await
    .unwrap()
    .entry_id;

    let mut input = CopyFieldInput::new(id, FieldSelector::TotpCode);
    input.clear_after_secs = 9999;
    let err = copy_field(&mut session, input).await.unwrap_err();
    assert!(matches!(err, VaultError::FieldNotApplicable));
}

#[tokio::test]
async fn clipboard_clears_after_delay() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("gh", "hunter2", None),
        },
    )
    .await
    .unwrap()
    .entry_id;

    copy_field(
        &mut session,
        CopyFieldInput {
            entry_id: id,
            field: FieldSelector::Password,
            clear_after_secs: 1,
        },
    )
    .await
    .unwrap();
    assert_eq!(h.clipboard.peek().as_deref(), Some("hunter2"));

    // Wait long enough for the spawned task to fire. The test runtime is
    // unpaused; we sleep a bit beyond the configured clear delay.
    tokio::time::sleep(Duration::from_millis(1250)).await;
    assert_eq!(h.clipboard.peek(), None);
}

#[tokio::test]
async fn move_entry_updates_folder_and_bumps_version() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let folder_id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: EntryPayload::Folder(FolderPayload {
                meta: CommonMeta::new("Work", EntryType::Folder),
            }),
        },
    )
    .await
    .unwrap()
    .entry_id;

    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("gh", "pw", None),
        },
    )
    .await
    .unwrap()
    .entry_id;

    let v1 = h.repo.get_entry(&id).await.unwrap();
    assert_eq!(v1.version, 1);

    move_entry(&mut session, &id, Some(&folder_id))
        .await
        .unwrap();

    let v2 = h.repo.get_entry(&id).await.unwrap();
    assert_eq!(v2.version, 2);
    assert_ne!(v1.nonce, v2.nonce);
    assert_eq!(
        session.index().entries.get(&id).unwrap().folder_id.as_ref(),
        Some(&folder_id)
    );
}

#[tokio::test]
async fn move_entry_rejects_missing_folder() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("gh", "pw", None),
        },
    )
    .await
    .unwrap()
    .entry_id;

    let ghost = EntryId::new();
    let err = move_entry(&mut session, &id, Some(&ghost))
        .await
        .unwrap_err();
    assert!(matches!(err, VaultError::FolderNotFound(_)));
}

#[tokio::test]
async fn move_entry_to_root_clears_folder() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let folder_id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: EntryPayload::Folder(FolderPayload {
                meta: CommonMeta::new("Work", EntryType::Folder),
            }),
        },
    )
    .await
    .unwrap()
    .entry_id;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("gh", "pw", None),
        },
    )
    .await
    .unwrap()
    .entry_id;
    move_entry(&mut session, &id, Some(&folder_id))
        .await
        .unwrap();
    move_entry(&mut session, &id, None).await.unwrap();
    assert!(
        session
            .index()
            .entries
            .get(&id)
            .unwrap()
            .folder_id
            .is_none()
    );
}
