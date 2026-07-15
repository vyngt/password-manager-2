//! Delete a vault — destroy its files, all three OS credentials, and its registry row (5.2.4).
//!
//! A FILE operation: no session, no KEK. The vault must be LOCKED (the command refuses an
//! unlocked target). The order is load-bearing (Decision ③) and crash-safe:
//!
//! 1. Resolve the `vault_uuid` (the config file is the authority; the registry row is the
//!    corrupt-vault fallback) and **backfill it onto the row before any destruction** — so a
//!    crash between the file delete and the credential delete leaves a row that still knows the
//!    uuid, and a re-run (the picker offers Delete again on the now-missing row) finishes the
//!    job idempotently (the P0-1 keystone).
//! 2. Delete the home directory (`vault.vdb` + `blobs/` + in-home `snapshots/`), retrying the
//!    Windows lingering-handle window on a blocking thread. **This is the one hard error.**
//! 3. Delete the three credentials, most-dangerous-first: the biometric KEK, then the rollback
//!    baseline, then the Secret Key. Best-effort — a keychain failure sets `credentials_cleaned
//!    = false` but never fails the (already durable) delete (L1).
//! 4. Delete the registry row LAST — the tombstone. Swallowed on error (L1).

use std::path::{Path, PathBuf};

use tracing::{instrument, warn};

use crate::application::app::ports::VaultRegistry;
use crate::application::app::use_cases::vault_registry_ops::{deregister_vault, record_vault_uuid};
use crate::application::vault::ports::biometric::BiometricAuthenticator;
use crate::application::vault::ports::keychain::KeychainProvider;
use crate::domain::shared::{StorageError, VaultId};
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::backup::journal::remove_dir_all_retrying;
use crate::infrastructure::backup::target::read_target_identity;

/// Input to [`delete_vault`].
pub struct DeleteVaultInput {
    /// The vault HOME directory (`<name>.vedge/`).
    pub home: PathBuf,
    /// The registry row id, if the vault is registered. `None` for a vault opened by a raw file
    /// path that was never registered — files + credentials still go, there is just no row to
    /// remove.
    pub registry_id: Option<String>,
}

/// Outcome of [`delete_vault`].
pub struct DeleteVaultReport {
    /// Snapshots that were destroyed with the home (for the confirmation copy).
    pub snapshots_deleted: u64,
    /// Approximate bytes reclaimed.
    pub bytes_freed: u64,
    /// `false` ⇒ the uuid was unknowable or a credential delete failed. The UI MUST say so and
    /// point at `mise keychain-audit` — silence here recreates the orphan this slice prevents.
    pub credentials_cleaned: bool,
}

/// Destroy a vault: files, all three OS credentials, and its registry row (Decision ③ order).
#[instrument(skip_all, fields(home = %input.home.display()))]
pub async fn delete_vault(
    keychain: &dyn KeychainProvider,
    biometric: &dyn BiometricAuthenticator,
    registry: &dyn VaultRegistry,
    input: DeleteVaultInput,
) -> Result<DeleteVaultReport, VaultError> {
    let DeleteVaultInput { home, registry_id } = input;

    // 1-2. Resolve the uuid (config authority, row fallback) and durably backfill it onto the
    //      row BEFORE any destruction (the P0-1 keystone). See [`resolve_and_backfill`].
    let resolved = resolve_and_backfill(registry, registry_id.as_deref(), &home).await;

    // 3. Report stats — computed BEFORE deletion (best-effort; 0 on error).
    let snapshots_dir = VaultId::new(&home).snapshots_dir();
    let snapshots_deleted = count_subdirs(&snapshots_dir);
    let bytes_freed = dir_size(&home);

    // 4. Destroy the files — the ONE hard error. Retry the Windows lingering-handle window on a
    //    blocking thread so the async runtime stays free to finish any pending pool close.
    {
        let home = home.clone();
        tokio::task::spawn_blocking(move || remove_dir_all_retrying(&home))
            .await
            .map_err(|e| VaultError::Storage(StorageError::Io(format!("delete join: {e}"))))??;
    }

    // 5. Credentials, most-dangerous-first — only if the uuid is known. Never guess.
    let credentials_cleaned = resolved
        .as_deref()
        .is_some_and(|uuid| clean_credentials(keychain, biometric, uuid));

    // 6. Tombstone LAST — swallowed on error (L1: the vault is already destroyed, so nothing
    //    downstream may turn a completed delete into a user-visible failure).
    if let Some(id) = registry_id.as_deref() {
        if let Err(e) = deregister_vault(registry, id).await {
            warn!(error = %e, "vault destroyed, but its registry row could not be removed");
        }
    }

    Ok(DeleteVaultReport {
        snapshots_deleted,
        bytes_freed,
        credentials_cleaned,
    })
}

/// Resolve the vault's `vault_uuid` and durably backfill it onto the registry row **before** the
/// caller destroys the home (the P0-1 keystone).
///
/// The config file is the authority (`record_vault_uuid`'s rule); the registry row is the
/// fallback that survives a **corrupt** vault (whose config is unreadable — exactly the vault you
/// delete). Backfilling the resolved uuid onto the row before destruction means a crash in the
/// file→credentials window leaves a row that still knows the uuid, so a re-run (Delete offered
/// again on the now-missing row) can finish the credential cleanup. Returns the resolved uuid, or
/// `None` when neither source knows it (then the caller cleans nothing and reports it).
async fn resolve_and_backfill(
    registry: &dyn VaultRegistry,
    registry_id: Option<&str>,
    home: &Path,
) -> Option<String> {
    let row_uuid = match registry_id {
        Some(id) => registry.get(id).await.ok().and_then(|row| row.vault_uuid),
        None => None,
    };
    let config_uuid = read_target_identity(home).await.and_then(|id| id.vault_uuid);
    let resolved = config_uuid.or_else(|| row_uuid.clone());

    if let (Some(id), Some(uuid)) = (registry_id, resolved.as_deref()) {
        if row_uuid.as_deref() != Some(uuid) {
            if let Err(e) = record_vault_uuid(registry, id, uuid).await {
                warn!(error = %e, "could not backfill the registry uuid before delete");
            }
        }
    }
    resolved
}

/// Delete all three per-vault OS credentials, the biometric KEK first. Idempotent-tolerant: a
/// missing entry counts as clean. Returns whether all three are gone.
fn clean_credentials(
    keychain: &dyn KeychainProvider,
    biometric: &dyn BiometricAuthenticator,
    uuid: &str,
) -> bool {
    // `disable` and `delete_commit_baseline` are idempotent (missing = `Ok`).
    let bio_ok = biometric.disable(uuid).is_ok();
    let baseline_ok = keychain.delete_commit_baseline(uuid).is_ok();
    // `delete_secret_key` is NOT idempotent — a missing entry (crash-resume, or an SK never
    // stored on this device) still counts as clean.
    let secret_ok = matches!(
        keychain.delete_secret_key(uuid),
        Ok(()) | Err(VaultError::KeychainEntryNotFound)
    );
    bio_ok && baseline_ok && secret_ok
}

/// Count the immediate subdirectories of a directory (the snapshot count). Best-effort → 0.
fn count_subdirs(dir: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let count = entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .count();
    u64::try_from(count).unwrap_or(u64::MAX)
}

/// Recursive on-disk byte size of a directory. Best-effort; saturating (the workspace denies
/// `arithmetic_side_effects`).
fn dir_size(path: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    let mut total: u64 = 0;
    for entry in entries.filter_map(Result::ok) {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let add = if file_type.is_dir() {
            dir_size(&entry.path())
        } else {
            entry.metadata().map_or(0, |m| m.len())
        };
        total = total.saturating_add(add);
    }
    total
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )]

    use std::sync::Arc;

    use super::*;
    use crate::application::vault::ports::repository::VaultRepository;
    use crate::domain::app::entities::RegisteredVault;
    use crate::domain::vault::crypto_constants::{KEK_LEN, SECRET_KEY_LEN};
    use crate::domain::vault::entities::VaultConfig;
    use crate::domain::vault::kdf_params::KdfParams;
    use crate::infrastructure::biometric::MemoryBiometricAuthenticator;
    use crate::infrastructure::keychain::MemoryKeychainProvider;
    use crate::infrastructure::sqlite::app::{AppDbConnection, SqliteVaultRegistry};
    use crate::infrastructure::sqlite::vault::{SqliteVaultRepository, VaultDbConnection};

    const SK: [u8; SECRET_KEY_LEN] = [0x5Au8; SECRET_KEY_LEN];
    const BIO_KEK: [u8; KEK_LEN] = [0x3Cu8; KEK_LEN];

    fn minimal_config(uuid: Option<String>) -> VaultConfig {
        VaultConfig {
            id: "default".to_owned(),
            magic: "VEDG".to_owned(),
            schema_version: 1,
            vault_salt: [0u8; 32],
            kdf_params: KdfParams {
                alg: "argon2id".into(),
                m: 8,
                t: 1,
                p: 1,
                version: 1,
            },
            verify_hash: [0u8; 32],
            preferred_cipher_suite: 1,
            trash_retention_days: 30,
            audit_retention_days: 90,
            created_at: crate::domain::shared::now(),
            last_unlocked_at: None,
            vault_uuid: uuid,
            commit_counter: 0,
            backup_dir: None,
            backup_keep_count: None,
            last_snapshot_at: None,
            last_backup_at: None,
        }
    }

    /// Build a real vault home `<dir>/work.vedge/` — `vault.vdb` (valid config) + `blobs/` + a
    /// `snapshots/` dir holding `snapshots` subdirectories. The DB is closed so nothing holds it.
    async fn build_home(dir: &Path, uuid: Option<String>, snapshots: usize) -> PathBuf {
        let home = dir.join("work.vedge");
        std::fs::create_dir_all(home.join("blobs")).unwrap();
        std::fs::write(home.join("blobs").join("01BLOB.blob"), b"ciphertext").unwrap();
        let snaps = home.join("snapshots");
        std::fs::create_dir_all(&snaps).unwrap();
        for i in 0..snapshots {
            std::fs::create_dir_all(snaps.join(format!("snap{i}"))).unwrap();
        }
        let db = VaultDbConnection::open(&home.join("vault.vdb")).await.unwrap();
        {
            let repo = SqliteVaultRepository::new(db.handle());
            repo.save_config(&minimal_config(uuid)).await.unwrap();
        }
        db.close().await.unwrap();
        home
    }

    async fn registry() -> (tempfile::TempDir, Arc<SqliteVaultRegistry>) {
        let dir = tempfile::tempdir().unwrap();
        let db = AppDbConnection::open(&dir.path().join("app.db"))
            .await
            .unwrap();
        (dir, Arc::new(SqliteVaultRegistry::new(db.handle())))
    }

    async fn seed_row(repo: &dyn VaultRegistry, id: &str, home: &Path, uuid: Option<String>) {
        repo.upsert(&RegisteredVault {
            id: id.to_owned(),
            path: home.to_path_buf(),
            display_name: "Work".to_owned(),
            last_opened: None,
            sort_order: 0,
            vault_uuid: uuid,
        })
        .await
        .unwrap();
    }

    /// The happy path: home + snapshots + all three credentials + the row all gone.
    #[tokio::test]
    async fn deletes_home_snapshots_credentials_and_row() {
        let (dir, repo) = registry().await;
        let uuid = "01HAPPYUUID0";
        let home = build_home(dir.path(), Some(uuid.to_owned()), 3).await;
        seed_row(&*repo, "row1", &home, Some(uuid.to_owned())).await;
        let kc = MemoryKeychainProvider::new();
        kc.store_secret_key(uuid, &SK).unwrap();
        kc.store_commit_baseline(uuid, 7).unwrap();
        let bio = MemoryBiometricAuthenticator::new();
        bio.enroll(uuid, &BIO_KEK).unwrap();

        let report = delete_vault(
            &kc,
            &bio,
            &*repo,
            DeleteVaultInput {
                home: home.clone(),
                registry_id: Some("row1".to_owned()),
            },
        )
        .await
        .unwrap();

        assert!(!home.exists(), "home destroyed");
        assert_eq!(report.snapshots_deleted, 3);
        assert!(report.credentials_cleaned);
        assert!(kc.read_secret_key(uuid).is_err(), "secret gone");
        assert_eq!(kc.read_commit_baseline(uuid).unwrap(), None, "baseline gone");
        assert!(!bio.is_enrolled(uuid).unwrap(), "biometric gone");
        assert!(repo.get("row1").await.is_err(), "row gone");
    }

    /// 🔴 P0-1: `resolve_and_backfill` writes the config uuid onto a NULL row BEFORE any
    /// destruction, so a crash-resume can still find it.
    #[tokio::test]
    async fn resolve_and_backfill_writes_the_config_uuid_onto_a_null_row() {
        let (dir, repo) = registry().await;
        let uuid = "01BACKFILLX0";
        let home = build_home(dir.path(), Some(uuid.to_owned()), 0).await;
        seed_row(&*repo, "row1", &home, None).await; // row uuid = NULL

        let resolved = resolve_and_backfill(&*repo, Some("row1"), &home).await;

        assert_eq!(resolved.as_deref(), Some(uuid), "resolved from config");
        assert_eq!(
            repo.get("row1").await.unwrap().vault_uuid.as_deref(),
            Some(uuid),
            "the row was backfilled before destruction"
        );
    }

    /// 🔴 P0-1 resume: with the home already gone (a crashed run), the uuid comes from the
    /// backfilled row and the credentials are finally cleaned.
    #[tokio::test]
    async fn a_resume_after_a_crashed_delete_cleans_from_the_row_uuid() {
        let (dir, repo) = registry().await;
        let uuid = "01RESUMEUUID";
        let home = build_home(dir.path(), Some(uuid.to_owned()), 0).await;
        seed_row(&*repo, "row1", &home, Some(uuid.to_owned())).await;
        std::fs::remove_dir_all(&home).unwrap(); // "crashed after the home delete"
        let kc = MemoryKeychainProvider::new();
        kc.store_secret_key(uuid, &SK).unwrap();
        kc.store_commit_baseline(uuid, 5).unwrap();
        let bio = MemoryBiometricAuthenticator::new();
        bio.enroll(uuid, &BIO_KEK).unwrap();

        let report = delete_vault(
            &kc,
            &bio,
            &*repo,
            DeleteVaultInput {
                home,
                registry_id: Some("row1".to_owned()),
            },
        )
        .await
        .unwrap();

        assert!(report.credentials_cleaned, "resume cleaned from the row uuid");
        assert!(kc.read_secret_key(uuid).is_err());
        assert_eq!(kc.read_commit_baseline(uuid).unwrap(), None);
        assert!(!bio.is_enrolled(uuid).unwrap());
        assert!(repo.get("row1").await.is_err(), "row finally removed");
    }

    /// A CORRUPT vault (unreadable config) is deleted with credentials resolved from the row —
    /// the headline case, since you delete vaults precisely when they are broken.
    #[tokio::test]
    async fn a_corrupt_vault_is_deleted_with_credentials_from_the_row_uuid() {
        let (dir, repo) = registry().await;
        let uuid = "01CORRUPTXX0";
        let home = build_home(dir.path(), Some(uuid.to_owned()), 1).await;
        std::fs::write(home.join("vault.vdb"), b"not a database").unwrap();
        seed_row(&*repo, "row1", &home, Some(uuid.to_owned())).await;
        let kc = MemoryKeychainProvider::new();
        kc.store_secret_key(uuid, &SK).unwrap();
        let bio = MemoryBiometricAuthenticator::new();
        bio.enroll(uuid, &BIO_KEK).unwrap();

        let report = delete_vault(
            &kc,
            &bio,
            &*repo,
            DeleteVaultInput {
                home: home.clone(),
                registry_id: Some("row1".to_owned()),
            },
        )
        .await
        .unwrap();

        assert!(!home.exists());
        assert!(report.credentials_cleaned, "cleaned from the row uuid");
        assert!(kc.read_secret_key(uuid).is_err());
        assert!(!bio.is_enrolled(uuid).unwrap());
    }

    /// The config uuid wins over a stale row uuid — an unrelated vault keyed on the stale uuid is
    /// left untouched.
    #[tokio::test]
    async fn the_config_uuid_wins_over_a_stale_row_uuid() {
        let (dir, repo) = registry().await;
        let config_uuid = "01CONFIGUUID";
        let stale_uuid = "01STALEUUID0";
        let home = build_home(dir.path(), Some(config_uuid.to_owned()), 0).await;
        seed_row(&*repo, "row1", &home, Some(stale_uuid.to_owned())).await;
        let kc = MemoryKeychainProvider::new();
        kc.store_secret_key(config_uuid, &SK).unwrap();
        kc.store_secret_key(stale_uuid, &SK).unwrap(); // a DIFFERENT vault's key
        let bio = MemoryBiometricAuthenticator::new();

        let report = delete_vault(
            &kc,
            &bio,
            &*repo,
            DeleteVaultInput {
                home,
                registry_id: Some("row1".to_owned()),
            },
        )
        .await
        .unwrap();

        assert!(report.credentials_cleaned);
        assert!(
            kc.read_secret_key(config_uuid).is_err(),
            "config-uuid secret cleaned"
        );
        assert!(
            kc.read_secret_key(stale_uuid).is_ok(),
            "stale-uuid vault untouched (config won)"
        );
    }

    /// L1: a keychain failure never fails the (already durable) delete — files go, and the
    /// report says the credentials were left.
    #[tokio::test]
    async fn a_keychain_failure_never_fails_the_delete_and_reports_it() {
        let (dir, repo) = registry().await;
        let uuid = "01L1UUID0000";
        let home = build_home(dir.path(), Some(uuid.to_owned()), 0).await;
        seed_row(&*repo, "row1", &home, Some(uuid.to_owned())).await;
        let kc = MemoryKeychainProvider::new();
        kc.store_secret_key(uuid, &SK).unwrap();
        kc.set_delete_fails(true);
        let bio = MemoryBiometricAuthenticator::new();

        let report = delete_vault(
            &kc,
            &bio,
            &*repo,
            DeleteVaultInput {
                home: home.clone(),
                registry_id: Some("row1".to_owned()),
            },
        )
        .await
        .unwrap();

        assert!(!home.exists(), "files gone despite the keychain failure");
        assert!(!report.credentials_cleaned, "reports the creds were left");
        kc.set_delete_fails(false);
        assert!(
            kc.read_secret_key(uuid).is_ok(),
            "secret survived the failed delete (keychain-audit is the cleanup path)"
        );
    }

    /// An unknowable uuid: the files still go, nothing is guessed, and an unrelated vault's key
    /// is untouched.
    #[tokio::test]
    async fn an_unknown_uuid_deletes_files_and_guesses_nothing() {
        let (dir, repo) = registry().await;
        let home = build_home(dir.path(), None, 0).await; // config has no uuid
        seed_row(&*repo, "row1", &home, None).await; // row has no uuid
        let kc = MemoryKeychainProvider::new();
        kc.store_secret_key("01OTHERVAULT", &SK).unwrap();
        let bio = MemoryBiometricAuthenticator::new();

        let report = delete_vault(
            &kc,
            &bio,
            &*repo,
            DeleteVaultInput {
                home: home.clone(),
                registry_id: Some("row1".to_owned()),
            },
        )
        .await
        .unwrap();

        assert!(!home.exists(), "files still go");
        assert!(!report.credentials_cleaned, "nothing to clean");
        assert!(
            kc.read_secret_key("01OTHERVAULT").is_ok(),
            "unrelated vault untouched — no guessing"
        );
        assert!(repo.get("row1").await.is_err(), "row removed");
    }

    /// Idempotent: a second delete on an already-deleted vault does not error.
    #[tokio::test]
    async fn a_second_delete_is_idempotent() {
        let (dir, repo) = registry().await;
        let uuid = "01IDEMPOTENT";
        let home = build_home(dir.path(), Some(uuid.to_owned()), 0).await;
        seed_row(&*repo, "row1", &home, Some(uuid.to_owned())).await;
        let kc = MemoryKeychainProvider::new();
        kc.store_secret_key(uuid, &SK).unwrap();
        let bio = MemoryBiometricAuthenticator::new();

        delete_vault(
            &kc,
            &bio,
            &*repo,
            DeleteVaultInput {
                home: home.clone(),
                registry_id: Some("row1".to_owned()),
            },
        )
        .await
        .unwrap();

        // Second run: home gone, creds gone, row gone → Ok, nothing to clean.
        let report = delete_vault(
            &kc,
            &bio,
            &*repo,
            DeleteVaultInput {
                home,
                registry_id: Some("row1".to_owned()),
            },
        )
        .await
        .unwrap();
        assert!(!report.credentials_cleaned);
    }
}
