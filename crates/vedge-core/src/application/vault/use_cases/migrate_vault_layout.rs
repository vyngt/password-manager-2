//! Convert a legacy `.vdb` + sibling `<stem>.vedge_blobs/` vault to a `<name>.vedge/`
//! home (slice 5.2.0). A **file** operation — no session, no KEK, no master password.
//!
//! ## Crash safety — cannot strand a vault
//!
//! The home is built by **copy**, so the original `.vdb` + blob sibling stay fully intact
//! until a single **atomic directory rename** commits the home; only then are the
//! originals reaped. Therefore:
//!
//! - **Crash before the commit rename** → the old layout is untouched and still opens;
//!   the `.<stem>.vedge.migrate-tmp` scratch dir is garbage, reaped on the next attempt.
//! - **Crash after the commit rename** → both the home and the old layout exist; re-running
//!   finishes the keychain re-key + reap (idempotent), and the recents picker can re-point
//!   to the home.
//!
//! ## Keychain re-key
//!
//! The Secret Key entry moves from the legacy `vault:{old_path}` account to `secret:{uuid}`
//! via [`KeychainProvider::migrate_secret_key`] (store → verify → delete). The `vault_uuid`
//! is minted first if a pre-4.6 vault lacks one, so the very next unlock finds the key.
//! Best-effort **after** the durable home commit: a keychain hiccup must not fail a
//! migration whose data is already safely committed (the Emergency Kit remains the
//! backstop).

use std::path::{Path, PathBuf};

use tracing::{instrument, warn};

use crate::application::vault::ports::biometric::BiometricAuthenticator;
use crate::application::vault::ports::keychain::KeychainProvider;
use crate::application::vault::ports::repository::VaultRepository;
use crate::domain::shared::{BLOBS_DIR, SNAPSHOTS_DIR, StorageError, VAULT_FILE, VaultId};
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::sqlite::vault::{SqliteVaultRepository, VaultDbConnection};

/// The legacy vault to convert.
#[derive(Debug, Clone)]
pub struct MigrateVaultLayoutInput {
    /// The old `.vdb` path recorded in recents (with a sibling `<stem>.vedge_blobs/`).
    pub legacy_vdb_path: PathBuf,
}

/// Result of a conversion — the new home + whether a keychain entry was re-keyed + whether a
/// legacy biometric enrollment was purged (slice 5.2.3).
#[derive(Debug, Clone)]
pub struct MigrateVaultLayoutOutput {
    pub home: PathBuf,
    pub keychain_migrated: bool,
    /// A legacy path-hashed Windows Hello credential existed and was purged — so biometric
    /// unlock is now OFF and the UI must tell the user to re-enable it (slice 5.2.3). `false`
    /// when none existed, or when the purge failed (it is best-effort and never fails the
    /// migration).
    pub biometric_reset: bool,
}

/// Crash points inside [`migrate_with_hook`]. Production passes a no-op hook; tests inject
/// a crash at a chosen point to prove the copy-then-commit ordering is strand-proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MigrateStep {
    /// Staged home built (copies done); the atomic commit rename is next.
    BeforeCommit,
    /// Home committed (rename done); the re-key + reap are next.
    AfterCommit,
}

fn io_ctx(op: &str, e: &std::io::Error) -> VaultError {
    VaultError::Storage(StorageError::Io(format!("{op}: {e}")))
}

fn io_msg(msg: impl Into<String>) -> VaultError {
    VaultError::Storage(StorageError::Io(msg.into()))
}

fn with_suffix(base: &Path, suffix: &str) -> PathBuf {
    let mut s = base.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

fn remove_file_if_exists(path: &Path) {
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => warn!(error = %e, path = %path.display(), "migration reap: remove file failed"),
    }
}

fn remove_dir_if_exists(path: &Path) -> Result<(), VaultError> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io_ctx("remove dir", &e)),
    }
}

/// Copy every `*.blob` file from `src` into `dst` (both dirs must exist).
fn copy_blob_files(src: &Path, dst: &Path) -> Result<(), VaultError> {
    let rd = match std::fs::read_dir(src) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(io_ctx("read legacy blob dir", &e)),
    };
    for entry in rd {
        let path = entry.map_err(|e| io_ctx("read blob entry", &e))?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("blob") {
            let name = path
                .file_name()
                .ok_or_else(|| io_msg("blob has no file name"))?;
            std::fs::copy(&path, dst.join(name)).map_err(|e| io_ctx("copy blob", &e))?;
        }
    }
    Ok(())
}

/// Ensure the migrated home carries a `vault_uuid` (mint one for a pre-4.6 vault) and
/// return it — the keychain re-key keys on it. Opens `home/vault.vdb` directly (running
/// migrations) and **closes** the connection before returning, so the home is not left
/// locked (Windows) — a caller may move it right after.
async fn ensure_home_uuid(home: &Path) -> Result<String, VaultError> {
    let db = VaultDbConnection::open(&home.join(VAULT_FILE))
        .await
        .map_err(VaultError::Storage)?;
    let uuid = {
        let repo = SqliteVaultRepository::new(db.handle());
        let mut config = repo.load_config().await?;
        if let Some(uuid) = config.vault_uuid.clone() {
            uuid
        } else {
            let uuid = ulid::Ulid::new().to_string();
            config.vault_uuid = Some(uuid.clone());
            repo.save_config(&config).await?;
            uuid
        }
    };
    db.close().await.map_err(VaultError::Storage)?;
    Ok(uuid)
}

/// Convert a legacy-layout vault to a home. See the module docs for the crash-safety
/// contract.
#[instrument(skip_all, fields(legacy = %input.legacy_vdb_path.display()))]
pub async fn migrate_vault_layout(
    keychain: &dyn KeychainProvider,
    biometric: &dyn BiometricAuthenticator,
    input: MigrateVaultLayoutInput,
) -> Result<MigrateVaultLayoutOutput, VaultError> {
    migrate_with_hook(keychain, biometric, input, &mut |_step| Ok(())).await
}

async fn migrate_with_hook(
    keychain: &dyn KeychainProvider,
    biometric: &dyn BiometricAuthenticator,
    input: MigrateVaultLayoutInput,
    hook: &mut (dyn FnMut(MigrateStep) -> Result<(), VaultError> + Send),
) -> Result<MigrateVaultLayoutOutput, VaultError> {
    let old_vdb = input.legacy_vdb_path;
    let parent = old_vdb
        .parent()
        .ok_or_else(|| io_msg("legacy vault path has no parent directory"))?
        .to_owned();
    let stem = old_vdb
        .file_stem()
        .map_or_else(|| "vault".to_owned(), |s| s.to_string_lossy().into_owned());
    let home = parent.join(format!("{stem}.vedge"));
    let old_blobs = parent.join(format!("{stem}.vedge_blobs"));
    let tmp = parent.join(format!(".{stem}.vedge.migrate-tmp"));

    // ---- BUILD (skipped when the home already exists — a resumed migration) ----------
    if !home.exists() {
        if !old_vdb.is_file() {
            return Err(io_msg(format!(
                "not a legacy vault: {} is not a file",
                old_vdb.display()
            )));
        }

        // Clear any stale scratch from an aborted attempt, then build the staged home by
        // COPY — the originals are not touched until the commit rename below.
        remove_dir_if_exists(&tmp)?;
        std::fs::create_dir_all(tmp.join(BLOBS_DIR)).map_err(|e| io_ctx("create tmp blobs", &e))?;
        std::fs::create_dir_all(tmp.join(SNAPSHOTS_DIR))
            .map_err(|e| io_ctx("create tmp snapshots", &e))?;

        // Copy the DB (+ its `-wal`, so uncheckpointed writes survive; the `-shm` is a
        // rebuildable cache — skip it). SQLite replays the WAL on the next open.
        std::fs::copy(&old_vdb, tmp.join(VAULT_FILE)).map_err(|e| io_ctx("copy vault db", &e))?;
        let old_wal = with_suffix(&old_vdb, "-wal");
        if old_wal.exists() {
            std::fs::copy(&old_wal, with_suffix(&tmp.join(VAULT_FILE), "-wal"))
                .map_err(|e| io_ctx("copy vault wal", &e))?;
        }
        copy_blob_files(&old_blobs, &tmp.join(BLOBS_DIR))?;

        hook(MigrateStep::BeforeCommit)?;

        // ---- COMMIT: the single atomic directory rename ------------------------------
        std::fs::rename(&tmp, &home).map_err(|e| io_ctx("commit home rename", &e))?;
    }

    hook(MigrateStep::AfterCommit)?;

    // ---- RE-KEY + REAP (best-effort, the home is already durably committed) -----------
    let uuid = ensure_home_uuid(&home).await?;
    let legacy_id = VaultId::new(old_vdb.clone());
    let keychain_migrated = match keychain.migrate_secret_key(&legacy_id, &uuid) {
        Ok(migrated) => migrated,
        Err(e) => {
            warn!(error = %e, "layout migration committed but the keychain re-key failed");
            false
        }
    };

    // 🔴 Purge the LEGACY path-hashed biometric credential (slice 5.2.3). Best-effort, beside
    // the keychain re-key. This is the ONLY thing that will ever reach that credential — the
    // vault's KEK is sealed behind it, and once the home is renamed the path it was keyed on
    // is gone forever. We do NOT re-enroll (that needs a Hello prompt mid-migration); we report
    // the reset so the UI can tell the user to re-enable biometric unlock in Settings.
    let biometric_reset = match biometric.purge_legacy(&legacy_id) {
        Ok(purged) => purged,
        Err(e) => {
            warn!(error = %e, "layout migration committed but the legacy biometric purge failed");
            false
        }
    };

    // Reap the old layout LAST (after the home is committed + the key re-keyed).
    remove_file_if_exists(&old_vdb);
    remove_file_if_exists(&with_suffix(&old_vdb, "-wal"));
    remove_file_if_exists(&with_suffix(&old_vdb, "-shm"));
    if let Err(e) = remove_dir_if_exists(&old_blobs) {
        warn!(error = %e, "layout migration committed but reaping the old blob dir failed");
    }

    Ok(MigrateVaultLayoutOutput {
        home,
        keychain_migrated,
        biometric_reset,
    })
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::let_underscore_must_use
    )]

    use super::*;
    use crate::application::vault::ports::repository::VaultRepository;
    use crate::domain::vault::crypto_constants::{KEK_LEN, SECRET_KEY_LEN};
    use crate::domain::vault::entities::VaultConfig;
    use crate::domain::vault::kdf_params::KdfParams;
    use crate::infrastructure::biometric::MemoryBiometricAuthenticator;
    use crate::infrastructure::keychain::MemoryKeychainProvider;
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

    /// Build an OLD-layout vault: `<dir>/work.vdb` (valid config) + `<dir>/work.vedge_blobs/`
    /// with one blob. The DB connection is dropped so nothing holds the file open. Seeds the
    /// legacy path-keyed keychain entry. Returns the `.vdb` path.
    async fn build_old_layout(
        dir: &Path,
        keychain: &MemoryKeychainProvider,
        uuid: Option<String>,
    ) -> PathBuf {
        let old_vdb = dir.join("work.vdb");
        let db = VaultDbConnection::open(&old_vdb).await.unwrap();
        {
            let repo =
                crate::infrastructure::sqlite::vault::SqliteVaultRepository::new(db.handle());
            repo.save_config(&minimal_config(uuid)).await.unwrap();
        }
        // Close cleanly so the file is released (Windows) and the WAL is checkpointed —
        // exactly the state a locked, previously-closed legacy vault would be in.
        db.close().await.unwrap();
        let blobs = dir.join("work.vedge_blobs");
        std::fs::create_dir_all(&blobs).unwrap();
        std::fs::write(blobs.join("01BLOB.blob"), b"ciphertext bytes").unwrap();
        keychain.seed_legacy_secret_key(&VaultId::new(&old_vdb), &SK);
        old_vdb
    }

    #[tokio::test]
    async fn happy_path_converts_reaps_and_rekeys() {
        let dir = tempfile::tempdir().unwrap();
        let kc = MemoryKeychainProvider::new();
        let bio = MemoryBiometricAuthenticator::new();
        let old_vdb = build_old_layout(dir.path(), &kc, Some("01UUIDHOME".to_owned())).await;

        let out = migrate_vault_layout(
            &kc,
            &bio,
            MigrateVaultLayoutInput {
                legacy_vdb_path: old_vdb.clone(),
            },
        )
        .await
        .unwrap();

        let home = dir.path().join("work.vedge");
        assert_eq!(out.home, home);
        assert!(home.join("vault.vdb").is_file(), "home has vault.vdb");
        assert!(
            home.join("blobs").join("01BLOB.blob").is_file(),
            "blob copied"
        );
        assert!(home.join("snapshots").is_dir(), "snapshots reserved");
        // Old layout reaped.
        assert!(!old_vdb.exists(), "old .vdb reaped");
        assert!(
            !dir.path().join("work.vedge_blobs").exists(),
            "old blobs reaped"
        );
        // Keychain re-keyed onto the uuid.
        assert!(out.keychain_migrated);
        assert_eq!(*kc.read_secret_key("01UUIDHOME").unwrap(), SK);
    }

    #[tokio::test]
    async fn crash_before_commit_leaves_the_old_layout_intact() {
        let dir = tempfile::tempdir().unwrap();
        let kc = MemoryKeychainProvider::new();
        let bio = MemoryBiometricAuthenticator::new();
        let old_vdb = build_old_layout(dir.path(), &kc, Some("01UUIDX".to_owned())).await;

        let err = migrate_with_hook(
            &kc,
            &bio,
            MigrateVaultLayoutInput {
                legacy_vdb_path: old_vdb.clone(),
            },
            &mut |step| {
                if step == MigrateStep::BeforeCommit {
                    Err(io_msg("injected crash"))
                } else {
                    Ok(())
                }
            },
        )
        .await;
        assert!(err.is_err());
        // The original is fully intact; the home was never committed.
        assert!(old_vdb.is_file(), "old .vdb intact");
        assert!(
            dir.path().join("work.vedge_blobs").is_dir(),
            "old blobs intact"
        );
        assert!(!dir.path().join("work.vedge").exists(), "no home committed");
        // The legacy keychain entry is untouched (re-key never ran).
        assert!(kc.read_secret_key("01UUIDX").is_err());

        // A clean retry now succeeds (stale tmp is cleared).
        let out = migrate_vault_layout(
            &kc,
            &bio,
            MigrateVaultLayoutInput {
                legacy_vdb_path: old_vdb,
            },
        )
        .await
        .unwrap();
        assert!(out.home.join("vault.vdb").is_file());
        assert!(out.keychain_migrated);
    }

    #[tokio::test]
    async fn crash_after_commit_is_finished_by_a_rerun() {
        let dir = tempfile::tempdir().unwrap();
        let kc = MemoryKeychainProvider::new();
        let bio = MemoryBiometricAuthenticator::new();
        let old_vdb = build_old_layout(dir.path(), &kc, Some("01UUIDR".to_owned())).await;

        // Crash right after the commit rename: the home exists, but the re-key + reap
        // never ran.
        let _ = migrate_with_hook(
            &kc,
            &bio,
            MigrateVaultLayoutInput {
                legacy_vdb_path: old_vdb.clone(),
            },
            &mut |step| {
                if step == MigrateStep::AfterCommit {
                    Err(io_msg("injected crash"))
                } else {
                    Ok(())
                }
            },
        )
        .await;
        let home = dir.path().join("work.vedge");
        assert!(home.join("vault.vdb").is_file(), "home committed");
        assert!(old_vdb.exists(), "old not yet reaped");
        assert!(kc.read_secret_key("01UUIDR").is_err(), "not yet re-keyed");

        // Re-running finishes the migration (home already present → build is skipped).
        let out = migrate_vault_layout(
            &kc,
            &bio,
            MigrateVaultLayoutInput {
                legacy_vdb_path: old_vdb.clone(),
            },
        )
        .await
        .unwrap();
        assert_eq!(out.home, home);
        assert!(out.keychain_migrated, "re-key finished on the rerun");
        assert_eq!(*kc.read_secret_key("01UUIDR").unwrap(), SK);
        assert!(!old_vdb.exists(), "old reaped on the rerun");
    }

    #[tokio::test]
    async fn a_moved_home_keeps_its_keychain_entry() {
        // The whole point: keying on `vault_uuid` (not the path) means moving the home
        // does not lose the Secret Key.
        let dir = tempfile::tempdir().unwrap();
        let kc = MemoryKeychainProvider::new();
        let bio = MemoryBiometricAuthenticator::new();
        let old_vdb = build_old_layout(dir.path(), &kc, Some("01UUIDMOVE".to_owned())).await;

        migrate_vault_layout(
            &kc,
            &bio,
            MigrateVaultLayoutInput {
                legacy_vdb_path: old_vdb,
            },
        )
        .await
        .unwrap();

        // Move the home to a new name/place — the keychain entry is untouched.
        let moved = dir.path().join("personal.vedge");
        std::fs::rename(dir.path().join("work.vedge"), &moved).unwrap();
        assert_eq!(
            *kc.read_secret_key("01UUIDMOVE").unwrap(),
            SK,
            "a moved vault still resolves its Secret Key"
        );
    }

    #[tokio::test]
    async fn migration_mints_a_uuid_for_a_pre_46_vault() {
        // A legacy vault with no `vault_uuid` gets one minted so unlock can find the key.
        let dir = tempfile::tempdir().unwrap();
        let kc = MemoryKeychainProvider::new();
        let bio = MemoryBiometricAuthenticator::new();
        let old_vdb = build_old_layout(dir.path(), &kc, None).await;

        let out = migrate_vault_layout(
            &kc,
            &bio,
            MigrateVaultLayoutInput {
                legacy_vdb_path: old_vdb,
            },
        )
        .await
        .unwrap();
        assert!(out.keychain_migrated);
        // The home now carries a uuid, and the key is stored under it.
        let db = VaultDbConnection::open(&out.home.join(VAULT_FILE))
            .await
            .unwrap();
        let repo = SqliteVaultRepository::new(db.handle());
        let uuid = repo.load_config().await.unwrap().vault_uuid.unwrap();
        assert_eq!(*kc.read_secret_key(&uuid).unwrap(), SK);
    }

    /// 🔴 Slice 5.2.3 — B1: converting an enrolled legacy vault PURGES the old path-hashed
    /// biometric credential (the KEK-holding one) and REPORTS the reset. Without the purge that
    /// credential is stranded in the TPM forever, unreachable once the home is renamed.
    #[tokio::test]
    async fn migration_purges_a_legacy_biometric_credential_and_reports_it() {
        let dir = tempfile::tempdir().unwrap();
        let kc = MemoryKeychainProvider::new();
        let bio = MemoryBiometricAuthenticator::new();
        let old_vdb = build_old_layout(dir.path(), &kc, Some("01UUIDBIO".to_owned())).await;
        // The user was enrolled under the OLD path-hashed credential.
        bio.seed_legacy_credential(&VaultId::new(&old_vdb), &BIO_KEK);

        let out = migrate_vault_layout(
            &kc,
            &bio,
            MigrateVaultLayoutInput {
                legacy_vdb_path: old_vdb.clone(),
            },
        )
        .await
        .unwrap();

        assert!(
            out.biometric_reset,
            "the legacy credential was found and the reset reported"
        );
        // The stranded credential is actually gone — a second purge finds nothing.
        assert!(
            !bio.purge_legacy(&VaultId::new(&old_vdb)).unwrap(),
            "the legacy credential was deleted, not merely flagged"
        );
    }

    /// B1: the purge is idempotent — a vault that was never enrolled (or a second convert)
    /// reports no reset and does not error.
    #[tokio::test]
    async fn migration_biometric_purge_is_idempotent_when_not_enrolled() {
        let dir = tempfile::tempdir().unwrap();
        let kc = MemoryKeychainProvider::new();
        let bio = MemoryBiometricAuthenticator::new();
        let old_vdb = build_old_layout(dir.path(), &kc, Some("01UUIDNOBIO".to_owned())).await;

        let out = migrate_vault_layout(
            &kc,
            &bio,
            MigrateVaultLayoutInput {
                legacy_vdb_path: old_vdb,
            },
        )
        .await
        .unwrap();

        assert!(
            !out.biometric_reset,
            "nothing to purge → no reset reported, no error"
        );
    }

    /// 🔴 B1 (L1): a purge failure must never fail the migration — the data is already durably
    /// committed. The convert succeeds; a reset it could not complete is not falsely claimed.
    #[tokio::test]
    async fn a_failed_biometric_purge_never_fails_the_migration() {
        let dir = tempfile::tempdir().unwrap();
        let kc = MemoryKeychainProvider::new();
        let bio = MemoryBiometricAuthenticator::new();
        let old_vdb = build_old_layout(dir.path(), &kc, Some("01UUIDPURGEFAIL".to_owned())).await;
        bio.seed_legacy_credential(&VaultId::new(&old_vdb), &BIO_KEK);
        bio.set_purge_fails(true);

        let out = migrate_vault_layout(
            &kc,
            &bio,
            MigrateVaultLayoutInput {
                legacy_vdb_path: old_vdb.clone(),
            },
        )
        .await
        .unwrap();

        assert!(
            out.home.join("vault.vdb").is_file(),
            "home committed despite the purge failure"
        );
        assert!(out.keychain_migrated, "keychain re-key still ran");
        assert!(
            !out.biometric_reset,
            "a failed purge does not claim a reset it could not complete"
        );
        // The credential really did survive the failed purge (proving the failure was real).
        bio.set_purge_fails(false);
        assert!(
            bio.purge_legacy(&VaultId::new(&old_vdb)).unwrap(),
            "the legacy credential survived the failed purge"
        );
    }
}
