//! Snapshot host suite (slice 5.2.1): snapshot creation, dedup, the M2 active-count fix,
//! the ⑧ two-timestamp rule, and the ⑮-A exclusion (a snapshot never enters a `.vbk`).
//!
//! The mark-and-sweep GC and CAS pool have their own unit tests in
//! `infrastructure::snapshot::store`; this suite exercises the end-to-end use case over a
//! real unlocked session. Revert + auto-snapshot live in `snapshots_revert.rs`.

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

use vedge_core::SnapshotReason;
use vedge_core::application::vault::ports::VaultRepository;
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::{
    BackupVaultInput, CreateEntryInput, UnlockVaultInput, backup_vault, create_entry,
    create_snapshot, soft_delete_entry,
};
use vedge_core::domain::shared::{EntryId, SNAPSHOTS_DIR};
use vedge_core::domain::vault::payloads::{CommonMeta, EntryPayload, EntryType, LoginPayload};
use vedge_core::infrastructure::backup::archive;
use vedge_core::infrastructure::snapshot::store;

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
        password: secrecy::SecretString::from("p"),
        totp_secret: None,
        totp_params: vedge_core::TotpParams::default(),
        recovery_codes: vec![],
    })
}

async fn add(session: &mut VaultSession, name: &str) -> EntryId {
    create_entry(
        session,
        CreateEntryInput {
            payload: login(name),
        },
    )
    .await
    .unwrap()
    .entry_id
}

/// Write a raw blob file straight into the vault home's `blobs/` dir.
fn seed_blob(h: &Harness, bytes: &[u8]) {
    let path = h
        .blob
        .root()
        .join(format!("{}.blob", EntryId::new().as_str()));
    std::fs::write(path, bytes).unwrap();
}

fn object_count(h: &Harness) -> usize {
    let objects = h.home.join(SNAPSHOTS_DIR).join("objects");
    match std::fs::read_dir(&objects) {
        Ok(rd) => rd.filter(|e| e.as_ref().unwrap().path().is_file()).count(),
        Err(_) => 0,
    }
}

/// A snapshot captures the vault + blobs; a second snapshot of an unchanged vault dedups
/// (adds ZERO objects). Tests 2/3 at the integration level.
#[tokio::test]
async fn snapshot_captures_and_dedups() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    add(&mut session, "keeper").await;
    seed_blob(&h, b"blob one -- nonce and ciphertext");
    seed_blob(&h, b"blob two -- nonce and ciphertext");

    let report = create_snapshot(&session, SnapshotReason::Manual).await.unwrap();
    assert_eq!(report.entry_count, 1);
    assert_eq!(report.blob_count, 2);
    assert_eq!(report.reason, SnapshotReason::Manual);

    let store_dir = h.home.join(SNAPSHOTS_DIR);
    let snaps = store::list_snapshots(&store_dir).unwrap();
    assert_eq!(snaps.len(), 1, "one committed snapshot");
    assert_eq!(snaps[0].manifest.blob_count, 2);
    assert_eq!(snaps[0].manifest.objects.len(), 2);
    assert_eq!(snaps[0].manifest.reason, SnapshotReason::Manual);
    assert_eq!(object_count(&h), 2, "two objects pooled");

    // Snapshot again with nothing changed → the pool gains ZERO objects (dedup).
    create_snapshot(&session, SnapshotReason::Manual).await.unwrap();
    assert_eq!(object_count(&h), 2, "dedup: no new objects for an unchanged vault");
    assert_eq!(store::list_snapshots(&store_dir).unwrap().len(), 2);
}

/// Test 15 (M2): the snapshot's `entry_count` counts ACTIVE entries only — a trashed entry
/// must not inflate it.
#[tokio::test]
async fn snapshot_entry_count_excludes_trashed() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    add(&mut session, "a").await;
    add(&mut session, "b").await;
    let trashed = add(&mut session, "c").await;
    soft_delete_entry(&mut session, &trashed).await.unwrap();

    let report = create_snapshot(&session, SnapshotReason::Manual).await.unwrap();
    assert_eq!(report.entry_count, 2, "3 active + 1 trashed → 2 active");
}

/// Test 14 (⑧): `create_snapshot` moves `last_snapshot_at` and leaves `last_backup_at`
/// alone; a `.vbk` backup then moves `last_backup_at` without disturbing `last_snapshot_at`.
#[tokio::test]
async fn snapshot_and_backup_touch_separate_timestamps() {
    let h = Harness::fresh().await;
    let session = unlock(&h).await;

    let before = h.repo.load_config().await.unwrap();
    assert!(before.last_snapshot_at.is_none());
    assert!(before.last_backup_at.is_none());

    create_snapshot(&session, SnapshotReason::Manual).await.unwrap();
    let after_snap = h.repo.load_config().await.unwrap();
    assert!(after_snap.last_snapshot_at.is_some(), "snapshot set last_snapshot_at");
    assert!(
        after_snap.last_backup_at.is_none(),
        "🔴 a snapshot is NOT a backup (⑧): last_backup_at untouched"
    );

    let out = tempfile::tempdir().unwrap();
    backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: out.path().join("v.vbk"),
        },
    )
    .await
    .unwrap();
    let after_backup = h.repo.load_config().await.unwrap();
    assert!(after_backup.last_backup_at.is_some(), "backup set last_backup_at");
    assert_eq!(
        after_backup.last_snapshot_at, after_snap.last_snapshot_at,
        "backup left last_snapshot_at unchanged"
    );
}

/// Test 6 (⑮-A): a snapshot NEVER enters a `.vbk`. A backup taken with a populated snapshot
/// store has no `snapshots/` members — only `vault.vdb` + `blobs/*`.
#[tokio::test]
async fn snapshot_never_enters_a_vbk() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    add(&mut session, "keeper").await;
    seed_blob(&h, b"one blob");

    // Populate the snapshot store.
    create_snapshot(&session, SnapshotReason::Manual).await.unwrap();
    create_snapshot(&session, SnapshotReason::Manual).await.unwrap();
    assert!(!store::list_snapshots(&h.home.join(SNAPSHOTS_DIR)).unwrap().is_empty());

    let out = tempfile::tempdir().unwrap();
    let dest = out.path().join("v.vbk");
    backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: dest.clone(),
        },
    )
    .await
    .unwrap();

    let manifest = archive::verify_archive(&dest).unwrap();
    assert!(
        manifest.files.iter().all(|m| !m.name.contains(SNAPSHOTS_DIR)),
        "🔴 ⑮-A: a .vbk must never contain the snapshot store"
    );
    // Only the vault member + the one blob.
    assert_eq!(manifest.blob_count, 1);
    assert!(manifest.files.iter().any(|m| m.name == "vault.vdb"));
}
