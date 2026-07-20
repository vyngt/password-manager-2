#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::needless_pass_by_value
)]

mod common;

use chrono::Duration;
use common::{Harness, build_unlock};
use secrecy::SecretString;

use vedge_core::application::vault::ports::VaultRepository;
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::{
    CreateEntryInput, UnlockVaultInput, create_entry, run_maintenance, soft_delete_entry,
};
use vedge_core::domain::shared::{EntryId, now};
use vedge_core::domain::vault::entities::{AuditAction, AuditEvent};
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::domain::vault::payloads::{CommonMeta, EntryPayload, EntryType, LoginPayload};

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

fn login(name: &str) -> EntryPayload {
    EntryPayload::Login(LoginPayload {
        meta: CommonMeta::new(name, EntryType::Login),
        username: "u".into(),
        password: SecretString::from("p"),
        totp_secret: None,
        totp_params: vedge_core::TotpParams::default(),
        recovery_codes: vec![],
    })
}

#[tokio::test]
async fn trashed_entry_past_retention_is_hard_deleted() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("gh"),
        },
    )
    .await
    .unwrap()
    .entry_id;
    soft_delete_entry(&mut session, &id).await.unwrap();

    // Push trashed_at back to 60 days ago (> default 30-day retention).
    let mut row = h.repo.get_entry(&id).await.unwrap();
    row.trashed_at = Some(now() - Duration::days(60));
    h.repo.update_entry(&row).await.unwrap();

    let report = run_maintenance(&session).await.unwrap();
    assert_eq!(report.trashed_entries_deleted, 1);
    assert!(matches!(
        h.repo.get_entry(&id).await.unwrap_err(),
        VaultError::EntryNotFound(_)
    ));

    h.assert_coherent().await;
}

#[tokio::test]
async fn fresh_trashed_entry_is_kept() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("gh"),
        },
    )
    .await
    .unwrap()
    .entry_id;
    soft_delete_entry(&mut session, &id).await.unwrap();

    let report = run_maintenance(&session).await.unwrap();
    assert_eq!(report.trashed_entries_deleted, 0);
    h.repo.get_entry(&id).await.unwrap(); // still present
}

#[tokio::test]
async fn old_audit_rows_are_deleted() {
    let h = Harness::fresh().await;
    let session = unlock(&h).await;

    // Insert an old audit event (default retention = 90 days).
    let old = AuditEvent {
        id: ulid::Ulid::new().to_string(),
        entry_id: None,
        action: AuditAction::Unlocked,
        occurred_at: now() - Duration::days(120),
        device_id: None,
    };
    h.repo.append_audit(&old).await.unwrap();

    let recent_before = h.repo.recent_audit(100).await.unwrap().len();

    let report = run_maintenance(&session).await.unwrap();
    assert!(report.audit_events_deleted >= 1);

    let recent_after = h.repo.recent_audit(100).await.unwrap().len();
    assert!(recent_after < recent_before);
}

#[tokio::test]
async fn orphan_blobs_are_reaped() {
    let h = Harness::fresh().await;
    let session = unlock(&h).await;

    // Manually write a blob for a non-existent entry.
    let ghost = EntryId::new();
    let blob_path = h.blob.root().join(format!("{}.blob", ghost.as_str()));
    tokio::fs::write(&blob_path, b"fake ciphertext")
        .await
        .unwrap();
    assert!(blob_path.exists());

    let report = run_maintenance(&session).await.unwrap();
    assert!(report.orphaned_blobs_deleted >= 1);
    assert!(!blob_path.exists());

    // The orphan is reaped → no orphan blob remains (invariant #4).
    h.assert_coherent().await;
}
