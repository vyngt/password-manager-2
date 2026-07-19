//! *Open backup* host suite (slice 5.2.2) — the SPACE half of Decision ⑦.
//!
//! **The `.vbk` is not a time machine. It is a vault in a suitcase.** This is the verb that
//! gets a vault onto a new machine, and it is the one users will actually reach for.
//!
//! Two properties carry the whole design, and both are tested here as *behaviour*, not as
//! fields:
//!
//! 1. 🔴 **It cannot overwrite anything.** An occupied destination is refused outright, and
//!    what was there is byte-identical afterwards. Three of 5.2's four confirmation dialogs
//!    existed only because an import could be pointed at an existing file; this one constraint
//!    is what deleted them.
//! 2. 🔴 **② A copy beside a live original gets a fresh identity** — and the proof is not that
//!    a uuid field changed, it is that **the original stops crying wolf**. Two divergent files
//!    sharing one uuid fight over the keychain rollback baseline (`counter:{uuid}`), and the
//!    loser is the user, who gets a false rollback warning on every unlock until they learn to
//!    ignore their own security alarm.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::needless_pass_by_value
)]

mod common;

use std::path::Path;

use zeroize::Zeroizing;

use common::{Harness, build_unlock};

use vedge_core::application::vault::ports::{KeychainProvider, VaultRepository};
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::{
    BackupVaultInput, CreateEntryInput, ImportDocumentInput, OpenBackupInput, UnlockVaultInput,
    backup_vault, create_entry, export_document, import_document, open_backup,
};
use vedge_core::domain::shared::{BLOBS_DIR, SNAPSHOTS_DIR, VAULT_FILE};
use vedge_core::domain::vault::entities::AuditAction;
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::domain::vault::payloads::{CommonMeta, EntryPayload, EntryType, LoginPayload};
use vedge_core::infrastructure::keychain::MemoryKeychainProvider;
use vedge_core::infrastructure::sqlite::vault::{SqliteVaultRepository, VaultDbConnection};

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

/// Unlock a vault at an arbitrary path with the harness's credentials, bypassing the keychain.
async fn unlock_at(h: &Harness, path: &Path) -> VaultSession {
    build_unlock(h)
        .execute(UnlockVaultInput {
            vault_path: path.to_path_buf(),
            master_password: h.master_password.clone(),
            secret_key: Some(Zeroizing::new(h.secret_key)),
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

/// Stamp a specific `vault_uuid` onto a harness vault + re-seed the keychain under it (the
/// keychain is uuid-keyed since 5.2.0).
async fn set_uuid(h: &Harness, uuid: &str) {
    let mut cfg = h.config.clone();
    cfg.vault_uuid = Some(uuid.to_owned());
    h.repo.save_config(&cfg).await.unwrap();
    h.keychain.store_secret_key(uuid, &h.secret_key).unwrap();
}

/// The plaintext `vault_uuid` of a vault on disk.
async fn uuid_of(home: &Path) -> Option<String> {
    let db = VaultDbConnection::open(&home.join(VAULT_FILE))
        .await
        .unwrap();
    let repo = SqliteVaultRepository::new(db.handle());
    let uuid = repo.load_config().await.unwrap().vault_uuid;
    drop(repo);
    db.close().await.unwrap();
    uuid
}

// ---- the round trip -------------------------------------------------------------

/// The whole-verb proof: back up a live vault (3 logins + a document), OPEN it at a fresh
/// path, unlock it there, and decrypt the document. Every layer round-trips through the tar.
///
/// Also test **#3** (the successor to the spec's now-dead "stem trap"): 5.2.0 deleted
/// `derive_blob_root`, so blobs no longer key off a file stem — but the concern survives in a
/// new form. The blobs must land in the **destination's** `blobs/`, under a name the
/// destination chose, and they must still DECRYPT — which they only can if the AAD (ULID +
/// version) travelled intact.
#[tokio::test]
async fn open_round_trips_entries_and_a_document_into_a_fresh_home() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    add(&mut session, "a").await;
    add(&mut session, "b").await;
    add(&mut session, "c").await;
    let doc_id = import_document(
        &mut session,
        ImportDocumentInput {
            filename: "notes.txt".into(),
            mime_type: "text/plain".into(),
            content: b"the quick brown fox".to_vec(),
            meta: CommonMeta::new("notes", EntryType::Document),
        },
    )
    .await
    .unwrap();

    let out = tempfile::tempdir().unwrap();
    let vbk = out.path().join("work.vbk");
    backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: vbk.clone(),
        },
    )
    .await
    .unwrap();
    drop(session);

    // Open it under a DIFFERENT name than the source — the normal way to make a copy.
    let dest_dir = tempfile::tempdir().unwrap();
    let dest = dest_dir.path().join("personal.vedge");
    let report = open_backup(
        &MemoryKeychainProvider::new(),
        OpenBackupInput {
            archive_path: vbk,
            dest_home: dest.clone(),
            known_vaults: vec![],
        },
    )
    .await
    .unwrap();
    assert_eq!(report.entry_count, 4);
    assert_eq!(report.blob_count, 1);

    // The home is COMPLETE: a fail-closed blob store refuses a home with no `blobs/`, and a
    // revert needs `snapshots/` — both must exist even though the archive carried neither.
    assert!(dest.join(VAULT_FILE).is_file());
    assert!(
        dest.join(BLOBS_DIR).is_dir(),
        "the DEST's blobs/, not the source's"
    );
    assert!(dest.join(SNAPSHOTS_DIR).is_dir());
    assert_eq!(
        std::fs::read_dir(dest.join(BLOBS_DIR)).unwrap().count(),
        1,
        "the document's blob landed in the destination's own blobs/"
    );

    // And it decrypts — the real assertion. A blob in the right folder that cannot be read is
    // not a restored document.
    let opened = unlock_at(&h, &dest).await;
    let (name, bytes) = export_document(&opened, &doc_id).await.unwrap();
    assert_eq!(name, "notes.txt");
    assert_eq!(bytes.to_vec(), b"the quick brown fox".to_vec());
    drop(opened);

    // The opened home is a coherent vault: every entry + the document blob decrypt (os = None —
    // the open used a throwaway keychain, so the OS-state checks don't apply here).
    common::coherence::assert_vault_coherent(
        &dest,
        &common::coherence::Creds {
            master_password: &h.master_password,
            secret_key: &h.secret_key,
            recovery_key: None,
        },
        None,
    )
    .await;

    // Opening is not restoring: no `BackupRestored` row is invented for a vault that was
    // never overwritten.
    let db = VaultDbConnection::open(&dest.join(VAULT_FILE))
        .await
        .unwrap();
    let repo = SqliteVaultRepository::new(db.handle());
    assert_eq!(repo.all_entries().await.unwrap().len(), 4);
    let restored_rows = repo
        .recent_audit(200)
        .await
        .unwrap()
        .iter()
        .filter(|e| e.action == AuditAction::BackupRestored)
        .count();
    assert_eq!(restored_rows, 0, "an open is not a restore");
}

// ---- ① it cannot overwrite ------------------------------------------------------

/// 🔴 **The constraint the whole design rests on.** An occupied destination is refused — no
/// flag, no confirmation, no override — and what was there is **byte-identical** afterwards.
///
/// Tested three ways, because "occupied" does not mean "is a vault": a live vault, a bare
/// file, and someone's unrelated folder must all be equally untouchable. The moment this
/// answers "well, if it's not a vault…", the boundary is gone and 5.2's four dialogs come back.
#[tokio::test]
async fn open_refuses_an_occupied_destination_and_changes_nothing() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    add(&mut session, "keeper").await;
    let out = tempfile::tempdir().unwrap();
    let vbk = out.path().join("o.vbk");
    backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: vbk.clone(),
        },
    )
    .await
    .unwrap();
    drop(session);

    let keychain = MemoryKeychainProvider::new();
    let dest_dir = tempfile::tempdir().unwrap();

    // (a) A LIVE vault. The catastrophic case: this is someone's real data.
    let live = dest_dir.path().join("live.vedge");
    open_backup(
        &keychain,
        OpenBackupInput {
            archive_path: vbk.clone(),
            dest_home: live.clone(),
            known_vaults: vec![],
        },
    )
    .await
    .unwrap();
    let before = std::fs::read(live.join(VAULT_FILE)).unwrap();

    let err = open_backup(
        &keychain,
        OpenBackupInput {
            archive_path: vbk.clone(),
            dest_home: live.clone(),
            known_vaults: vec![],
        },
    )
    .await
    .unwrap_err();
    assert!(
        matches!(err, VaultError::DestinationOccupied(_)),
        "a live vault must be untouchable, got {err:?}"
    );
    assert_eq!(
        std::fs::read(live.join(VAULT_FILE)).unwrap(),
        before,
        "the existing vault must be BYTE-IDENTICAL after a refusal"
    );

    // (b) A plain file that is not a vault at all.
    let a_file = dest_dir.path().join("notes.vedge");
    std::fs::write(&a_file, b"someone's file").unwrap();
    let err = open_backup(
        &keychain,
        OpenBackupInput {
            archive_path: vbk.clone(),
            dest_home: a_file.clone(),
            known_vaults: vec![],
        },
    )
    .await
    .unwrap_err();
    assert!(
        matches!(err, VaultError::DestinationOccupied(_)),
        "got {err:?}"
    );
    assert_eq!(std::fs::read(&a_file).unwrap(), b"someone's file");

    // (c) An unrelated, non-empty folder. "It isn't a vault" is not a licence to clobber it.
    let a_dir = dest_dir.path().join("photos.vedge");
    std::fs::create_dir_all(&a_dir).unwrap();
    std::fs::write(a_dir.join("holiday.jpg"), b"not a vault").unwrap();
    let err = open_backup(
        &keychain,
        OpenBackupInput {
            archive_path: vbk,
            dest_home: a_dir.clone(),
            known_vaults: vec![],
        },
    )
    .await
    .unwrap_err();
    assert!(
        matches!(err, VaultError::DestinationOccupied(_)),
        "got {err:?}"
    );
    assert_eq!(
        std::fs::read(a_dir.join("holiday.jpg")).unwrap(),
        b"not a vault"
    );
    assert!(
        !a_dir.join(VAULT_FILE).exists(),
        "a refused open must not leave a vault file in someone's folder"
    );
}

/// A refused open must not leave its staging scrap behind either — a `.foo.vedge.opening-tmp`
/// left lying around is a half-vault waiting to confuse the next run.
#[tokio::test]
async fn a_refused_open_leaves_no_staging_scrap() {
    let dir = tempfile::tempdir().unwrap();
    let not_an_archive = dir.path().join("bad.vbk");
    std::fs::write(&not_an_archive, b"definitely not a tar").unwrap();

    let dest_dir = tempfile::tempdir().unwrap();
    let dest = dest_dir.path().join("x.vedge");
    let err = open_backup(
        &MemoryKeychainProvider::new(),
        OpenBackupInput {
            archive_path: not_an_archive,
            dest_home: dest,
            known_vaults: vec![],
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(err, VaultError::Storage(_)), "got {err:?}");
    assert_eq!(
        std::fs::read_dir(dest_dir.path()).unwrap().count(),
        0,
        "nothing at all — not the vault, not a staging dir"
    );
}

// ---- ② duplicate detection ------------------------------------------------------

/// 🔴🔴 **② tested by its SYMPTOM, not its field.**
///
/// Open a backup of vault A while A is still live. The copy must get a fresh `vault_uuid` —
/// and the way we prove that matters is by showing **A stops crying wolf**.
///
/// The mechanism: the keychain rollback baseline is keyed `counter:{uuid}` (5.2c). If the copy
/// kept A's uuid, then opening the copy advances the shared baseline, and A's next unlock sees
/// `file.commit_counter < baseline` → a rollback warning. Forever. On every unlock. Until the
/// user learns that their security alarm lies, which is the worst outcome a security alarm can
/// have.
///
/// So this test asserts the uuid differs AND that A's baseline is undisturbed — the actual
/// user-visible harm.
#[tokio::test]
async fn a_copy_beside_a_live_original_gets_a_fresh_identity_and_the_original_stops_crying_wolf() {
    const A_UUID: &str = "01ORIGINALORIGINALORIGINA";
    let h = Harness::fresh().await;
    set_uuid(&h, A_UUID).await;
    let mut session = unlock(&h).await;
    add(&mut session, "a").await;
    add(&mut session, "b").await;

    let out = tempfile::tempdir().unwrap();
    let vbk = out.path().join("a.vbk");
    backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: vbk.clone(),
        },
    )
    .await
    .unwrap();

    // A keeps working AFTER the backup — so the backup is now BEHIND the live vault. This is
    // the setup that makes a shared baseline produce a false alarm.
    add(&mut session, "c").await;
    let a_counter = h.repo.load_config().await.unwrap().commit_counter;
    drop(session);

    // A's baseline, as the last unlock of A left it.
    let keychain = MemoryKeychainProvider::new();
    keychain.store_secret_key(A_UUID, &h.secret_key).unwrap();
    keychain.store_commit_baseline(A_UUID, a_counter).unwrap();

    // Open the backup beside the still-live A. `known_vaults` carries A — this is a DUPLICATE.
    let dest_dir = tempfile::tempdir().unwrap();
    let copy = dest_dir.path().join("copy.vedge");
    let report = open_backup(
        &keychain,
        OpenBackupInput {
            archive_path: vbk,
            dest_home: copy.clone(),
            known_vaults: vec![h.home.clone()],
        },
    )
    .await
    .unwrap();

    assert_eq!(
        report.duplicate_of.as_deref(),
        Some(h.home.as_path()),
        "the live original must be recognised"
    );
    assert!(
        report.fresh_uuid,
        "② a duplicate must be given a new identity"
    );

    let copy_uuid = uuid_of(&copy).await.expect("the copy has a uuid");
    assert_ne!(copy_uuid, A_UUID, "② the copy must NOT share A's identity");
    assert_eq!(
        uuid_of(&h.home).await.as_deref(),
        Some(A_UUID),
        "and the ORIGINAL's identity must be left exactly alone"
    );

    // 🔴 The symptom. A's baseline is untouched, so A's next unlock is quiet. Had the copy kept
    // A's uuid, the two would now be sharing `counter:{A_UUID}` and A would warn forever.
    assert_eq!(
        keychain.read_commit_baseline(A_UUID).unwrap(),
        Some(a_counter),
        "the copy must not have touched the ORIGINAL's rollback baseline"
    );
    assert_eq!(
        keychain.read_commit_baseline(&copy_uuid).unwrap(),
        None,
        "the copy starts with no baseline — its first unlock establishes one, quietly"
    );

    // ② the Secret Key travelled to the new identity, so the copy opens on the master password
    // alone. (Same salt, same key material — only the uuid it is filed under changed.)
    assert!(report.secret_key_copied);
    assert_eq!(
        keychain.read_secret_key(&copy_uuid).unwrap().as_ref(),
        &h.secret_key,
        "the copy must be openable with the master password alone"
    );
}

/// 🔴 **Regression guard: opening a DUPLICATE must survive the Windows rename-after-close race.**
///
/// A duplicate is the one path that opens the staged `vault.vdb` before committing (to mint its
/// fresh uuid — ②). `sqlx`'s pool close is not synchronous, so for a moment afterwards Windows
/// still holds the `.vdb`/`-wal`/`-shm` handles and **refuses to rename the directory containing
/// them** (`ERROR_ACCESS_DENIED` 5). A bare `std::fs::rename` on the async thread fails — and
/// worse, retrying *on that thread* starves the very pool-close task it is waiting for.
///
/// This shipped broken and only the e2e caught it: **4 of 6 opens failed with os error 5** in the
/// real app. The host suite passed anyway, because its runtime had threads to spare. So: hammer
/// the duplicate path repeatedly, which is the shape that actually exposed it.
///
/// (Same class as the 5.2.1 field bug. *An async close is not a close.*)
#[tokio::test]
async fn opening_a_duplicate_repeatedly_never_loses_the_rename_race() {
    const UUID: &str = "01RACERACERACERACERACERACE";
    let h = Harness::fresh().await;
    set_uuid(&h, UUID).await;
    let mut session = unlock(&h).await;
    add(&mut session, "keeper").await;
    let out = tempfile::tempdir().unwrap();
    let vbk = out.path().join("race.vbk");
    backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: vbk.clone(),
        },
    )
    .await
    .unwrap();

    // The original stays LIVE and open — so every one of these is a duplicate, and every one
    // therefore opens + closes the staged DB before renaming it into place.
    let keychain = MemoryKeychainProvider::new();
    keychain.store_secret_key(UUID, &h.secret_key).unwrap();
    let dest_dir = tempfile::tempdir().unwrap();

    for i in 0..6 {
        let dest = dest_dir.path().join(format!("copy{i}.vedge"));
        let report = open_backup(
            &keychain,
            OpenBackupInput {
                archive_path: vbk.clone(),
                dest_home: dest.clone(),
                known_vaults: vec![h.home.clone()],
            },
        )
        .await
        .unwrap_or_else(|e| panic!("open #{i} lost the rename race: {e}"));

        assert!(report.fresh_uuid, "#{i} must be seen as a duplicate");
        assert!(dest.join(VAULT_FILE).is_file(), "#{i} did not commit");
        // And no staging scrap is left lying around.
        assert!(
            !dest_dir
                .path()
                .join(format!(".copy{i}.vedge.opening-tmp"))
                .exists(),
            "#{i} left its staging dir behind"
        );
    }
}

/// ② The other half, and the one that breaks the **new-machine** flow if you get it wrong: when
/// nothing else on this machine carries the uuid, the vault is the SAME vault, relocated — so
/// it KEEPS its identity. Minting a fresh uuid here would orphan it from its own keychain entry
/// and its own baseline on every device it was ever restored to.
#[tokio::test]
async fn a_vault_opened_where_the_original_is_absent_keeps_its_uuid() {
    const UUID: &str = "01RELOCATERELOCATERELOCAT";
    let h = Harness::fresh().await;
    set_uuid(&h, UUID).await;
    let session = unlock(&h).await;
    let out = tempfile::tempdir().unwrap();
    let vbk = out.path().join("r.vbk");
    backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: vbk.clone(),
        },
    )
    .await
    .unwrap();
    drop(session);

    let keychain = MemoryKeychainProvider::new();
    let dest_dir = tempfile::tempdir().unwrap();

    // (a) The new-machine case: this machine knows of no vaults at all.
    let fresh = dest_dir.path().join("newmachine.vedge");
    let report = open_backup(
        &keychain,
        OpenBackupInput {
            archive_path: vbk.clone(),
            dest_home: fresh.clone(),
            known_vaults: vec![],
        },
    )
    .await
    .unwrap();
    assert!(!report.fresh_uuid);
    assert!(report.duplicate_of.is_none());
    assert_eq!(uuid_of(&fresh).await.as_deref(), Some(UUID));

    // (b) A CORRUPT vault carrying the same uuid does not count as "already registered" — you
    // are recovering FROM it, not duplicating it, so the recovered copy keeps the identity.
    let corrupt = dest_dir.path().join("corrupt.vedge");
    std::fs::create_dir_all(corrupt.join(BLOBS_DIR)).unwrap();
    std::fs::write(corrupt.join(VAULT_FILE), b"unreadable garbage").unwrap();

    let recovered = dest_dir.path().join("recovered.vedge");
    let report = open_backup(
        &keychain,
        OpenBackupInput {
            archive_path: vbk,
            dest_home: recovered.clone(),
            known_vaults: vec![corrupt],
        },
    )
    .await
    .unwrap();
    assert!(
        !report.fresh_uuid,
        "a corrupt vault is not an openable duplicate — this is a recovery, not a copy"
    );
    assert_eq!(uuid_of(&recovered).await.as_deref(), Some(UUID));
}
