//! Revert host suite (slice 5.2.1): the in-place `revert_to_snapshot` round-trip, the ⑭
//! auto-snapshot (a revert is undoable), and the ⑮-B guarantee (a revert never deletes a
//! snapshot). The crash-safety of the preserving swap itself is proven by the journal unit
//! tests; this suite proves the use-case wiring end-to-end over a real vault.
//!
//! An in-place revert swaps the live home, so — like the product, which reverts a LOCKED
//! vault — the test must hold no open handle on `vault.vdb` during the swap. It builds the
//! vault through the harness, then destructures it to drop the harness's repo connection
//! before reverting (the M1 rename-retry covers sqlx's async close lag on Windows).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::needless_pass_by_value
)]

mod common;

use std::sync::Arc;

use common::{Harness, build_unlock};

use vedge_core::application::vault::ports::{
    BlobStoreFactory, ClipboardProvider, CryptoProvider, KeyDerivationProvider, KeychainProvider,
    VaultRepositoryFactory,
};
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::{
    CreateEntryInput, RevertToSnapshotInput, UnlockVault, UnlockVaultInput, create_entry,
    create_snapshot, revert_to_snapshot,
};
use vedge_core::domain::shared::SNAPSHOTS_DIR;
use vedge_core::domain::vault::crypto_constants::SECRET_KEY_LEN;
use vedge_core::domain::vault::payloads::{CommonMeta, EntryPayload, EntryType, LoginPayload};
use vedge_core::infrastructure::blob::FilesystemBlobStoreFactory;
use vedge_core::infrastructure::keychain::MemoryKeychainProvider;
use vedge_core::infrastructure::snapshot::manifest::SnapshotReason;
use vedge_core::infrastructure::snapshot::store;
use vedge_core::infrastructure::sqlite::vault::SqliteVaultRepositoryFactory;

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

async fn add(session: &mut VaultSession, name: &str) {
    create_entry(
        session,
        CreateEntryInput {
            payload: login(name),
        },
    )
    .await
    .unwrap();
}

/// A re-unlockable context surviving the harness's destructuring: the tempdir keeps the vault
/// files alive; the ports let us build a fresh `UnlockVault` (a NEW connection each time).
struct Reunlock {
    _tempdir: tempfile::TempDir,
    crypto: Arc<dyn CryptoProvider>,
    clipboard: Arc<dyn ClipboardProvider>,
    kdf: Arc<dyn KeyDerivationProvider>,
    keychain: Arc<dyn KeychainProvider>,
    home: std::path::PathBuf,
    master_password: zeroize::Zeroizing<String>,
    secret_key: [u8; SECRET_KEY_LEN],
    vault_uuid: String,
}

impl Reunlock {
    fn unlocker(&self) -> UnlockVault {
        UnlockVault {
            repo_factory: Arc::new(SqliteVaultRepositoryFactory::new())
                as Arc<dyn VaultRepositoryFactory>,
            blob_factory: Arc::new(FilesystemBlobStoreFactory::new()) as Arc<dyn BlobStoreFactory>,
            crypto: Arc::clone(&self.crypto),
            clipboard: Arc::clone(&self.clipboard),
            kdf: Arc::clone(&self.kdf),
            keychain: Arc::clone(&self.keychain),
        }
    }

    async fn active_entry_count(&self) -> usize {
        let session = self
            .unlocker()
            .execute(UnlockVaultInput {
                vault_path: self.home.clone(),
                master_password: self.master_password.clone(),
                secret_key: None,
            })
            .await
            .unwrap();
        let n = session.index().all_active().len();
        drop(session); // release the connection before the next swap
        n
    }

    /// Assert the reverted home is coherent from a fresh open (the swap left every surface
    /// consistent under the current KEK — the 5.7 class of bug).
    async fn assert_coherent(&self) {
        common::coherence::assert_vault_coherent(
            &self.home,
            &common::coherence::Creds {
                master_password: &self.master_password,
                secret_key: &self.secret_key,
                recovery_key: None,
            },
            Some(&common::coherence::OsState {
                vault_uuid: &self.vault_uuid,
                keychain: self.keychain.as_ref(),
            }),
        )
        .await;
    }
}

/// Build a vault with two snapshots (S1 = 1 entry, S2 = 2 entries), then drop the harness's
/// repo handle so an in-place revert can swap the home.
async fn build_two_snapshot_vault() -> (Reunlock, SqliteVaultRepositoryFactory) {
    let h = Harness::fresh().await;
    {
        let mut s = build_unlock(&h)
            .execute(UnlockVaultInput {
                vault_path: h.home.clone(),
                master_password: h.master_password.clone(),
                secret_key: None,
            })
            .await
            .unwrap();
        add(&mut s, "keeper").await;
        create_snapshot(&s, SnapshotReason::Manual).await.unwrap();
        add(&mut s, "extra").await;
        create_snapshot(&s, SnapshotReason::Manual).await.unwrap();
        // session dropped here
    }
    // Take the harness apart, keeping the tempdir + re-unlock ports, DROPPING the repo/blob
    // handles so nothing holds `vault.vdb` open during the swap.
    let Harness {
        tempdir,
        home,
        crypto,
        clipboard,
        kdf,
        keychain,
        master_password,
        secret_key,
        vault_uuid,
        repo,
        blob,
        ..
    } = h;
    drop(repo);
    drop(blob);
    (
        Reunlock {
            _tempdir: tempdir,
            crypto: crypto as Arc<dyn CryptoProvider>,
            clipboard: clipboard as Arc<dyn ClipboardProvider>,
            kdf: kdf as Arc<dyn KeyDerivationProvider>,
            keychain: keychain as Arc<dyn KeychainProvider>,
            home,
            master_password,
            secret_key,
            vault_uuid,
        },
        SqliteVaultRepositoryFactory::new(),
    )
}

/// Tests 7 (⑮-B), 8 (⑭): revert to the older snapshot drops the newer entry, keeps ALL
/// snapshots, auto-captures a `pre-restore` undo point, and reverting to THAT undoes the revert.
#[tokio::test]
async fn revert_is_undoable_and_keeps_every_snapshot() {
    let (ctx, factory) = build_two_snapshot_vault().await;
    let store_dir = ctx.home.join(SNAPSHOTS_DIR);
    let keychain = Arc::clone(&ctx.keychain);

    assert_eq!(
        ctx.active_entry_count().await,
        2,
        "vault starts at 2 entries"
    );
    let before = store::list_snapshots(&store_dir).unwrap();
    assert_eq!(before.len(), 2);
    let s1 = before.last().unwrap().id(); // oldest = the 1-entry snapshot

    let report = revert_to_snapshot(
        &factory,
        keychain.as_ref(),
        RevertToSnapshotInput {
            vault: ctx.home.clone(),
            snapshot_id: s1,
            confirm_rollback: true,
        },
    )
    .await
    .unwrap();
    assert_eq!(report.entry_count, 1, "reverted to the 1-entry snapshot");
    assert_eq!(ctx.active_entry_count().await, 1, "the extra entry is gone");

    // ⑮-B: both originals still on disk + a NEW pre-restore snapshot (⑭).
    let after = store::list_snapshots(&store_dir).unwrap();
    assert!(
        after.len() >= 3,
        "S1 + S2 kept, a pre-restore added; got {}",
        after.len()
    );
    let pre = after
        .into_iter()
        .find(|s| s.manifest.reason == SnapshotReason::PreRestore)
        .expect("⑭ a pre-restore snapshot must exist");
    assert_eq!(
        pre.manifest.entry_count, 2,
        "the undo point captured the pre-revert state"
    );

    // ⑭: reverting to the pre-restore snapshot UNDOES the revert.
    revert_to_snapshot(
        &factory,
        keychain.as_ref(),
        RevertToSnapshotInput {
            vault: ctx.home.clone(),
            snapshot_id: pre.id(),
            confirm_rollback: true,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        ctx.active_entry_count().await,
        2,
        "⑭ the revert was undoable"
    );

    // The reverted home is coherent under the current KEK (every snapshot re-hashes + opens; #8).
    ctx.assert_coherent().await;
}

/// A revert to a NON-existent snapshot id fails with a dedicated variant (L3), not a
/// stringly error, and does not touch the vault.
#[tokio::test]
async fn revert_refuses_unknown_snapshot() {
    let (ctx, factory) = build_two_snapshot_vault().await;
    let err = revert_to_snapshot(
        &factory,
        &MemoryKeychainProvider::new(),
        RevertToSnapshotInput {
            vault: ctx.home.clone(),
            snapshot_id: "20990101T000000000Z".to_owned(),
            confirm_rollback: true,
        },
    )
    .await
    .unwrap_err();
    assert!(
        matches!(
            err,
            vedge_core::domain::vault::errors::VaultError::SnapshotNotFound(_)
        ),
        "expected SnapshotNotFound, got {err:?}"
    );
    assert_eq!(ctx.active_entry_count().await, 2, "the vault is untouched");
}

/// 🔴 **Finding H1, applied to the snapshot store.** 5.2.1 shipped this guard as `!=`,
/// which would orphan **every existing snapshot** the day `SNAPSHOT_FORMAT_VERSION`
/// becomes 2 — the same defect H1 names in the `.vbk` path, in code written one slice
/// earlier. The rule is the store's, not the container's: once it exists on a user's
/// disk, every future version must read it — forever.
///
/// Refuse NEWER; accept OLDER. Both halves pinned.
#[tokio::test]
async fn revert_accepts_an_older_snapshot_format_and_refuses_a_newer_one() {
    use vedge_core::domain::vault::errors::VaultError;
    use vedge_core::infrastructure::snapshot::manifest::SNAPSHOT_FORMAT_VERSION;

    let (ctx, factory) = build_two_snapshot_vault().await;
    let store_dir = ctx.home.join(SNAPSHOTS_DIR);
    let keychain = Arc::clone(&ctx.keychain);

    let snaps = store::list_snapshots(&store_dir).unwrap();
    let oldest = snaps.last().unwrap().clone(); // the 1-entry snapshot

    // --- a NEWER format is refused, and the vault is untouched ---
    let mut m = oldest.manifest.clone();
    m.format_version = SNAPSHOT_FORMAT_VERSION + 1;
    store::write_manifest(&oldest.dir, &m).unwrap();
    let err = revert_to_snapshot(
        &factory,
        keychain.as_ref(),
        RevertToSnapshotInput {
            vault: ctx.home.clone(),
            snapshot_id: oldest.id(),
            confirm_rollback: true,
        },
    )
    .await
    .unwrap_err();
    assert!(
        matches!(err, VaultError::SnapshotUnsupportedFormat(v) if v == SNAPSHOT_FORMAT_VERSION + 1),
        "a NEWER snapshot format → refuse, got {err:?}"
    );
    assert_eq!(ctx.active_entry_count().await, 2, "the vault is untouched");

    // --- an OLDER format still reverts (the half that matters) ---
    let mut m = oldest.manifest.clone();
    m.format_version = 0;
    store::write_manifest(&oldest.dir, &m).unwrap();
    let report = revert_to_snapshot(
        &factory,
        keychain.as_ref(),
        RevertToSnapshotInput {
            vault: ctx.home.clone(),
            snapshot_id: oldest.id(),
            confirm_rollback: true,
        },
    )
    .await
    .expect("an OLDER snapshot format_version must still revert — H1");
    assert_eq!(report.entry_count, 1);
    assert_eq!(ctx.active_entry_count().await, 1, "the revert took effect");
}
