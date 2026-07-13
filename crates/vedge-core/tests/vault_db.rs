#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::needless_pass_by_value
)]

use tempfile::tempdir;

use vedge_core::application::vault::ports::VaultRepository;
use vedge_core::domain::shared::{DeviceId, EntryId, TagId, now};
use vedge_core::domain::vault::entities::{
    AuditAction, AuditEvent, EntryHistoryRow, EntryRow, TagRow, VaultConfig,
};
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::domain::vault::kdf_params::KdfParams;
use vedge_core::infrastructure::sqlite::vault::{SqliteVaultRepository, VaultDbConnection};

async fn open_repo() -> (tempfile::TempDir, SqliteVaultRepository) {
    let dir = tempdir().unwrap();
    let db = VaultDbConnection::open(&dir.path().join("work.vdb"))
        .await
        .unwrap();
    (dir, SqliteVaultRepository::new(db.handle()))
}

fn sample_config() -> VaultConfig {
    VaultConfig {
        id: "default".into(),
        magic: "VEDG".into(),
        schema_version: 1,
        vault_salt: [9u8; 32],
        kdf_params: KdfParams::argon2id_default(),
        verify_hash: [1u8; 32],
        preferred_cipher_suite: 1,
        trash_retention_days: 30,
        audit_retention_days: 90,
        created_at: now(),
        last_unlocked_at: None,
        vault_uuid: None,
        commit_counter: 0,
    }
}

fn sample_entry(id: &EntryId) -> EntryRow {
    EntryRow {
        id: id.clone(),
        version: 1,
        cipher_suite: 1,
        dek_wrapped: [5u8; 40],
        nonce: [2u8; 24],
        ciphertext: vec![0xAB; 64],
        created_at: now(),
        updated_at: now(),
        accessed_at: None,
        is_trashed: false,
        trashed_at: None,
    }
}

fn sample_tag(id: &TagId) -> TagRow {
    TagRow {
        id: id.clone(),
        nonce: [4u8; 24],
        ciphertext: vec![0xCD; 48],
        created_at: now(),
        updated_at: now(),
    }
}

fn sample_history(entry_id: &EntryId, version: i64) -> EntryHistoryRow {
    EntryHistoryRow {
        id: ulid::Ulid::new().to_string(),
        entry_id: entry_id.clone(),
        version,
        cipher_suite: 1,
        nonce: [3u8; 24],
        ciphertext: vec![0x11; 32],
        changed_at: now(),
    }
}

#[tokio::test]
async fn load_config_missing_returns_config_missing() {
    let (_dir, repo) = open_repo().await;
    let err = repo.load_config().await.unwrap_err();
    assert!(matches!(err, VaultError::ConfigMissing));
}

#[tokio::test]
async fn vault_config_save_and_load_round_trip() {
    let (_dir, repo) = open_repo().await;
    let config = sample_config();
    repo.save_config(&config).await.unwrap();

    let loaded = repo.load_config().await.unwrap();
    assert_eq!(loaded.vault_salt, config.vault_salt);
    assert_eq!(loaded.verify_hash, config.verify_hash);
    assert_eq!(loaded.kdf_params, config.kdf_params);
    assert_eq!(loaded.schema_version, 1);
    assert_eq!(loaded.magic, "VEDG");

    // Upsert updates fields.
    let mut updated = loaded.clone();
    updated.trash_retention_days = 60;
    repo.save_config(&updated).await.unwrap();
    let reloaded = repo.load_config().await.unwrap();
    assert_eq!(reloaded.trash_retention_days, 60);
}

#[tokio::test]
async fn entries_crud_round_trip() {
    let (_dir, repo) = open_repo().await;
    let id = EntryId::new();
    let row = sample_entry(&id);
    repo.insert_entry(&row).await.unwrap();

    let fetched = repo.get_entry(&id).await.unwrap();
    assert_eq!(fetched.dek_wrapped, [5u8; 40]);
    assert_eq!(fetched.nonce, [2u8; 24]);
    assert_eq!(fetched.ciphertext.len(), 64);

    // update
    let mut updated = fetched.clone();
    updated.version = 2;
    updated.ciphertext = vec![0xEE; 32];
    updated.updated_at = now();
    repo.update_entry(&updated).await.unwrap();
    let fetched = repo.get_entry(&id).await.unwrap();
    assert_eq!(fetched.version, 2);
    assert_eq!(fetched.ciphertext, vec![0xEE; 32]);

    // soft delete + restore
    repo.soft_delete_entry(&id, now()).await.unwrap();
    let fetched = repo.get_entry(&id).await.unwrap();
    assert!(fetched.is_trashed);
    assert!(fetched.trashed_at.is_some());

    repo.restore_entry(&id, now()).await.unwrap();
    let fetched = repo.get_entry(&id).await.unwrap();
    assert!(!fetched.is_trashed);
    assert!(fetched.trashed_at.is_none());

    // accessed_at touch
    repo.update_accessed_at(&id, now()).await.unwrap();
    let fetched = repo.get_entry(&id).await.unwrap();
    assert!(fetched.accessed_at.is_some());

    // hard delete
    repo.hard_delete_entry(&id).await.unwrap();
    let err = repo.get_entry(&id).await.unwrap_err();
    assert!(matches!(err, VaultError::EntryNotFound(_)));
}

#[tokio::test]
async fn hard_delete_trashed_before_cutoff() {
    let (_dir, repo) = open_repo().await;
    let id_old = EntryId::new();
    let id_fresh = EntryId::new();
    repo.insert_entry(&sample_entry(&id_old)).await.unwrap();
    repo.insert_entry(&sample_entry(&id_fresh)).await.unwrap();

    let old_trashed_at = chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00+00:00")
        .unwrap()
        .with_timezone(&chrono::Utc);
    repo.soft_delete_entry(&id_old, old_trashed_at)
        .await
        .unwrap();
    repo.soft_delete_entry(&id_fresh, now()).await.unwrap();

    let cutoff = chrono::DateTime::parse_from_rfc3339("2026-02-01T00:00:00+00:00")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let deleted = repo.hard_delete_trashed_before(cutoff).await.unwrap();
    assert_eq!(deleted, 1);
    assert!(matches!(
        repo.get_entry(&id_old).await.unwrap_err(),
        VaultError::EntryNotFound(_)
    ));
    // Fresh one still around.
    repo.get_entry(&id_fresh).await.unwrap();
}

#[tokio::test]
async fn tags_crud_round_trip() {
    let (_dir, repo) = open_repo().await;
    let id = TagId::new();
    let tag = sample_tag(&id);
    repo.insert_tag(&tag).await.unwrap();

    let fetched = repo.get_tag(&id).await.unwrap();
    assert_eq!(fetched.nonce, [4u8; 24]);

    let mut updated = fetched.clone();
    updated.ciphertext = vec![0xFF; 8];
    updated.updated_at = now();
    repo.update_tag(&updated).await.unwrap();
    let fetched = repo.get_tag(&id).await.unwrap();
    assert_eq!(fetched.ciphertext, vec![0xFF; 8]);

    let all = repo.all_tags().await.unwrap();
    assert_eq!(all.len(), 1);

    repo.delete_tag(&id).await.unwrap();
    assert!(matches!(
        repo.get_tag(&id).await.unwrap_err(),
        VaultError::TagNotFound(_)
    ));
}

#[tokio::test]
async fn audit_log_append_and_filter() {
    let (_dir, repo) = open_repo().await;
    let entry_id = EntryId::new();

    let events = [
        AuditEvent {
            id: ulid::Ulid::new().to_string(),
            entry_id: None,
            action: AuditAction::Unlocked,
            occurred_at: now(),
            device_id: Some(DeviceId::from_raw("laptop")),
        },
        AuditEvent {
            id: ulid::Ulid::new().to_string(),
            entry_id: Some(entry_id.clone()),
            action: AuditAction::Created,
            occurred_at: now(),
            device_id: None,
        },
        AuditEvent {
            id: ulid::Ulid::new().to_string(),
            entry_id: Some(entry_id.clone()),
            action: AuditAction::Viewed,
            occurred_at: now(),
            device_id: None,
        },
    ];
    for ev in &events {
        repo.append_audit(ev).await.unwrap();
    }

    let recent = repo.recent_audit(10).await.unwrap();
    assert_eq!(recent.len(), 3);

    let by_entry = repo.audit_by_entry(&entry_id, 10).await.unwrap();
    assert_eq!(by_entry.len(), 2);
    assert!(
        by_entry
            .iter()
            .all(|e| e.entry_id.as_ref() == Some(&entry_id))
    );

    let cutoff = chrono::DateTime::parse_from_rfc3339("2099-01-01T00:00:00+00:00")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let deleted = repo.delete_audit_before(cutoff).await.unwrap();
    assert_eq!(deleted, 3);
    assert!(repo.recent_audit(10).await.unwrap().is_empty());
}

async fn commit_counter(repo: &SqliteVaultRepository) -> i64 {
    repo.load_config().await.unwrap().commit_counter
}

/// Slice 5.2c (T1): every content-mutating write advances `commit_counter` by
/// exactly one; the metadata/bookkeeping writes (`save_config`, `update_accessed_at`,
/// `append_audit`) leave it untouched. Delta-based so a write that forgot its bump
/// (or an excluded one that wrongly gained a bump) names itself.
#[tokio::test]
async fn commit_counter_bumps_on_content_writes_only() {
    let (_dir, repo) = open_repo().await;
    // Seed via a save_config INSERT: the counter starts at 0 and save_config,
    // being metadata, never bumps.
    repo.save_config(&sample_config()).await.unwrap();
    assert_eq!(commit_counter(&repo).await, 0, "seed starts at 0");

    let e = EntryId::new();
    let mut n = 0i64;

    // ---- entries (4 of the 13; hard-delete + trashed-purge are exercised below) ----
    repo.insert_entry(&sample_entry(&e)).await.unwrap();
    n += 1;
    assert_eq!(commit_counter(&repo).await, n, "insert_entry");

    let mut upd = sample_entry(&e);
    upd.version = 2;
    repo.update_entry(&upd).await.unwrap();
    n += 1;
    assert_eq!(commit_counter(&repo).await, n, "update_entry");

    repo.soft_delete_entry(&e, now()).await.unwrap();
    n += 1;
    assert_eq!(commit_counter(&repo).await, n, "soft_delete_entry");

    repo.restore_entry(&e, now()).await.unwrap();
    n += 1;
    assert_eq!(commit_counter(&repo).await, n, "restore_entry");

    // ---- history (3 of the 13) ----
    repo.insert_history(&sample_history(&e, 1)).await.unwrap();
    n += 1;
    assert_eq!(commit_counter(&repo).await, n, "insert_history");
    repo.insert_history(&sample_history(&e, 2)).await.unwrap();
    n += 1; // same method, second call — still one bump
    let pruned = repo.prune_history_keep(&e, 1).await.unwrap();
    assert_eq!(pruned, 1);
    n += 1;
    assert_eq!(commit_counter(&repo).await, n, "prune_history_keep");
    let removed = repo.delete_history_for_entry(&e).await.unwrap();
    assert_eq!(removed, 1);
    n += 1;
    assert_eq!(commit_counter(&repo).await, n, "delete_history_for_entry");

    // ---- tags (3 of the 13) ----
    let t = TagId::new();
    repo.insert_tag(&sample_tag(&t)).await.unwrap();
    n += 1;
    assert_eq!(commit_counter(&repo).await, n, "insert_tag");
    let mut tu = sample_tag(&t);
    tu.ciphertext = vec![0x22; 8];
    repo.update_tag(&tu).await.unwrap();
    n += 1;
    assert_eq!(commit_counter(&repo).await, n, "update_tag");
    repo.delete_tag(&t).await.unwrap();
    n += 1;
    assert_eq!(commit_counter(&repo).await, n, "delete_tag");

    // ---- rewrap_all_deks (the ONE transactional write; bump rides its txn) ----
    let cfg = repo.load_config().await.unwrap();
    repo.rewrap_all_deks(&[], &cfg).await.unwrap();
    n += 1;
    assert_eq!(commit_counter(&repo).await, n, "rewrap_all_deks");

    // ---- hard_delete_entry (1) ----
    repo.hard_delete_entry(&e).await.unwrap();
    n += 1;
    assert_eq!(commit_counter(&repo).await, n, "hard_delete_entry");

    // ---- hard_delete_trashed_before (1) ----
    let e2 = EntryId::new();
    repo.insert_entry(&sample_entry(&e2)).await.unwrap();
    n += 1;
    let old = chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00+00:00")
        .unwrap()
        .with_timezone(&chrono::Utc);
    repo.soft_delete_entry(&e2, old).await.unwrap();
    n += 1;
    let cutoff = chrono::DateTime::parse_from_rfc3339("2026-02-01T00:00:00+00:00")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let purged = repo.hard_delete_trashed_before(cutoff).await.unwrap();
    assert_eq!(purged, 1);
    n += 1;
    assert_eq!(commit_counter(&repo).await, n, "hard_delete_trashed_before");

    // ---- exclusions: only the insert bumps; accessed_at / audit / save_config do not ----
    let before = commit_counter(&repo).await;
    let e3 = EntryId::new();
    repo.insert_entry(&sample_entry(&e3)).await.unwrap();
    repo.update_accessed_at(&e3, now()).await.unwrap();
    repo.append_audit(&AuditEvent {
        id: ulid::Ulid::new().to_string(),
        entry_id: None,
        action: AuditAction::Unlocked,
        occurred_at: now(),
        device_id: None,
    })
    .await
    .unwrap();
    let mut cfg2 = repo.load_config().await.unwrap();
    cfg2.last_unlocked_at = Some(now());
    repo.save_config(&cfg2).await.unwrap();
    assert_eq!(
        commit_counter(&repo).await,
        before + 1,
        "only insert_entry bumped; update_accessed_at / append_audit / save_config did not"
    );
}
