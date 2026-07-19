//! Slice 5.7 — Recovery Key.
//!
//! An opt-in `RK1-` Recovery Key, bound to `2SKD(Recovery Key, Secret Key)`, reconstructs
//! the vault KEK when the master password is forgotten. This file's CG3 tests are the two
//! silent-brick guards, written to pass BEFORE the recovery use cases exist:
//!
//! - ③ a KEK change DELETES the slot (the slot wraps the old KEK; leaving it = a brick at
//!   the recovery moment). Also the C1 regression: the null only persists because
//!   `RecoverySlot` is in `rewrap_all_deks`'s `update_columns`.
//! - ④ a snapshot carries NO live slot (a revert reopens with the current KEK, so a live
//!   slot would re-arm a possibly-revoked Recovery Key).
//!
//! CG3 sets the slot with the repo primitive (`set_recovery_slot`) — the full enroll/recover
//! round-trip is CG4. `change_password` never unwraps the slot, only nulls it, so a dummy
//! 40-byte slot exercises ③/④ faithfully.

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
use secrecy::SecretString;
use zeroize::Zeroizing;

use vedge_core::application::vault::ports::{
    BiometricAuthenticator, KeyDerivationProvider, KeychainProvider, VaultRepository,
};
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::{
    ChangePasswordInput, CreateEntryInput, RevertToSnapshotInput, SeamlessRevertOutcome,
    UnlockVaultInput, change_password, change_password_after_recovery, create_entry,
    create_snapshot, enroll_recovery_key, lock_vault, revert_to_snapshot_in_session,
    revoke_recovery_key,
};
use vedge_core::domain::shared::SNAPSHOTS_DIR;
use vedge_core::domain::vault::crypto_constants::SECRET_KEY_LEN;
use vedge_core::domain::vault::entities::{AuditAction, AuditQuery};
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::domain::vault::payloads::{CommonMeta, EntryPayload, EntryType, LoginPayload};
use vedge_core::domain::vault::secret_key::parse_recovery_key;
use vedge_core::infrastructure::snapshot::manifest::SnapshotReason;
use vedge_core::infrastructure::snapshot::store;
use vedge_core::infrastructure::sqlite::vault::{
    SqliteVaultRepository, SqliteVaultRepositoryFactory, VaultDbConnection,
};

const PW: &str = "correct horse battery staple";
const NEW_PW: &str = "a-brand-new-passphrase-9";
const DUMMY_SLOT: [u8; 40] = [0x42; 40];

fn kdf(h: &Harness) -> Arc<dyn KeyDerivationProvider> {
    Arc::clone(&h.kdf) as Arc<dyn KeyDerivationProvider>
}
fn keychain(h: &Harness) -> Arc<dyn KeychainProvider> {
    Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>
}
fn biometric(h: &Harness) -> Arc<dyn BiometricAuthenticator> {
    Arc::clone(&h.biometric) as Arc<dyn BiometricAuthenticator>
}

async fn unlock(h: &Harness, pw: &str) -> Result<VaultSession, VaultError> {
    build_unlock(h)
        .execute(UnlockVaultInput {
            vault_path: h.home.clone(),
            master_password: Zeroizing::new(pw.to_owned()),
            secret_key: None,
        })
        .await
}

async fn change_pw(session: &mut VaultSession, h: &Harness, new_pw: &str) {
    change_password(
        session,
        Arc::clone(&h.kdf) as Arc<dyn KeyDerivationProvider>,
        Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>,
        Arc::clone(&h.biometric) as Arc<dyn BiometricAuthenticator>,
        ChangePasswordInput {
            new_password: Zeroizing::new(new_pw.to_owned()),
            new_secret_key: None,
            secret_key_rotated: false,
        },
    )
    .await
    .unwrap();
}

fn login(name: &str) -> EntryPayload {
    EntryPayload::Login(LoginPayload {
        meta: CommonMeta::new(name, EntryType::Login),
        username: "alice".into(),
        password: SecretString::from("pw"),
        totp_secret: None,
        totp_params: vedge_core::TotpParams::default(),
        recovery_codes: vec![],
    })
}

async fn seed_one_entry(session: &mut VaultSession) {
    create_entry(
        session,
        CreateEntryInput {
            payload: login("e1"),
        },
    )
    .await
    .unwrap();
}

async fn count_action(h: &Harness, action: AuditAction) -> u64 {
    h.repo
        .query_audit(&AuditQuery {
            actions: vec![action],
            limit: 100,
            ..Default::default()
        })
        .await
        .unwrap()
        .total
}

/// 🔴 ③ + C1 — a password change DELETES the recovery slot, atomically with the rewrap.
///
/// Written to fail against the pre-③ code (the slot would survive, wrapping a dead KEK → a
/// brick at recovery). Also the C1 regression guard: if `RecoverySlot` were dropped from
/// `rewrap_all_deks`'s `update_columns`, the null would not persist and this fails.
#[tokio::test]
async fn a_password_change_deletes_the_recovery_slot() {
    let h = Harness::fresh().await;
    // "Enroll" via the repo primitive (CG4 does the real derivation). change_password never
    // unwraps the slot, so a dummy value exercises ③ faithfully.
    h.repo.set_recovery_slot(&DUMMY_SLOT).await.unwrap();

    // Re-unlock so the session's in-memory config carries Some(slot) — otherwise
    // change_password would clone a stale None and the null would trivially pass.
    let mut session = unlock(&h, PW).await.unwrap();
    seed_one_entry(&mut session).await;
    change_pw(&mut session, &h, "new-hunter2").await;
    lock_vault(session).await.unwrap();

    assert!(
        h.repo.load_config().await.unwrap().recovery_slot.is_none(),
        "a password change must delete the recovery slot (③, C1)"
    );

    // Refuse-not-brick: the vault still opens with the NEW password and the entry decrypts.
    let s = unlock(&h, "new-hunter2")
        .await
        .expect("the new password must open the vault");
    assert_eq!(s.index().all_active().len(), 1);
    lock_vault(s).await.unwrap();
}

/// 🔴 ④ Fix A — a snapshot is born slot-less, even though the live vault is enrolled; and a
/// later password change (which rewraps the snapshot) leaves it slot-less (Fix B).
#[tokio::test]
async fn a_snapshot_carries_no_live_recovery_slot() {
    let h = Harness::fresh().await;
    h.repo.set_recovery_slot(&DUMMY_SLOT).await.unwrap();

    let mut session = unlock(&h, PW).await.unwrap();
    seed_one_entry(&mut session).await;
    create_snapshot(&session, SnapshotReason::Manual)
        .await
        .unwrap();

    let snap_dir = store::list_snapshots(&h.home.join(SNAPSHOTS_DIR)).unwrap()[0]
        .dir
        .clone();
    assert!(
        snapshot_slot(&snap_dir).await.is_none(),
        "a snapshot must be born slot-less (④ Fix A) even though the live vault is enrolled"
    );
    // The live vault keeps its slot — capture nulled only the copy.
    assert!(h.repo.load_config().await.unwrap().recovery_slot.is_some());

    // A password change rewraps the snapshot; it must stay slot-less (④ Fix B), never a
    // resurrected slot wrapping the pre-rewrap KEK.
    change_pw(&mut session, &h, "new-hunter2").await;
    lock_vault(session).await.unwrap();
    assert!(
        snapshot_slot(&snap_dir).await.is_none(),
        "a rewrapped snapshot must stay slot-less (④ Fix B)"
    );
}

/// 🔴 #1 — THE FEATURE, end to end. Enroll → lock → recover with the `RK1-` key + the Secret
/// Key → forced new password → the vault opens on the new password and the entry decrypts;
/// the OLD password no longer works, and recovery is now off (the forced change nulled it).
#[tokio::test]
async fn recover_a_forgotten_password_end_to_end() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, PW).await.unwrap();
    seed_one_entry(&mut session).await;

    let display = enroll_recovery_key(&mut session, kdf(&h), keychain(&h))
        .await
        .unwrap()
        .recovery_key_display;
    assert!(display.starts_with("RK1-"), "got {display}");
    lock_vault(session).await.unwrap();

    // The password is forgotten. Recover with the two documents.
    let recovery_key = parse_recovery_key(&display).unwrap();
    let unlocker = build_unlock(&h);
    let mut recovered = unlocker
        .unlock_with_recovery_key(h.home.clone(), recovery_key, Zeroizing::new(h.secret_key))
        .await
        .expect("recovery unlock succeeds with the right kit + secret key");

    // The forced new master password (no old-password reauth).
    change_password_after_recovery(
        &mut recovered,
        kdf(&h),
        keychain(&h),
        biometric(&h),
        Zeroizing::new(NEW_PW.to_owned()),
    )
    .await
    .unwrap();
    lock_vault(recovered).await.unwrap();

    // The NEW password opens the vault and the entry survived.
    let s = unlock(&h, NEW_PW)
        .await
        .expect("the new password opens the recovered vault");
    assert_eq!(s.index().all_active().len(), 1);
    lock_vault(s).await.unwrap();

    // The OLD password no longer works, and recovery is off (re-enrol to turn it back on).
    assert!(matches!(
        unlock(&h, PW).await.unwrap_err(),
        VaultError::WrongCredentials
    ));
    assert!(h.repo.load_config().await.unwrap().recovery_slot.is_none());

    // The audit timeline records the enroll, the recovery unlock, and the forced change.
    assert_eq!(count_action(&h, AuditAction::RecoveryKeyEnabled).await, 1);
    assert_eq!(count_action(&h, AuditAction::RecoveryUsed).await, 1);
    assert_eq!(count_action(&h, AuditAction::PasswordChanged).await, 1);
}

/// #9 — the two new audit actions land (and the `ACTION_NAMES` compile-tie stays green).
#[tokio::test]
async fn enroll_and_revoke_write_audit_rows() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, PW).await.unwrap();
    enroll_recovery_key(&mut session, kdf(&h), keychain(&h))
        .await
        .unwrap();
    revoke_recovery_key(&mut session).await.unwrap();
    lock_vault(session).await.unwrap();

    assert_eq!(count_action(&h, AuditAction::RecoveryKeyEnabled).await, 1);
    assert_eq!(count_action(&h, AuditAction::RecoveryKeyRevoked).await, 1);
}

/// 🔴 #2 — the 2SKD binding: neither document alone opens the vault. A right Recovery Key with
/// a WRONG Secret Key fails; a WRONG Recovery Key with the right Secret Key fails; both right
/// succeeds. This is what makes the Recovery Key not a bearer credential.
#[tokio::test]
async fn recovery_needs_both_documents() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, PW).await.unwrap();
    let display = enroll_recovery_key(&mut session, kdf(&h), keychain(&h))
        .await
        .unwrap()
        .recovery_key_display;
    lock_vault(session).await.unwrap();

    let recovery_key = parse_recovery_key(&display).unwrap();
    let unlocker = build_unlock(&h);

    // Right Recovery Key, WRONG Secret Key → the AES-KW unwrap fails.
    let e1 = unlocker
        .unlock_with_recovery_key(
            h.home.clone(),
            recovery_key.clone(),
            Zeroizing::new([0x11u8; SECRET_KEY_LEN]),
        )
        .await
        .unwrap_err();
    assert!(matches!(e1, VaultError::WrongCredentials), "got {e1:?}");

    // WRONG Recovery Key, right Secret Key → fails too.
    let e2 = unlocker
        .unlock_with_recovery_key(
            h.home.clone(),
            Zeroizing::new([0x99u8; 32]),
            Zeroizing::new(h.secret_key),
        )
        .await
        .unwrap_err();
    assert!(matches!(e2, VaultError::WrongCredentials), "got {e2:?}");

    // Both documents → opens.
    let s = unlocker
        .unlock_with_recovery_key(h.home.clone(), recovery_key, Zeroizing::new(h.secret_key))
        .await
        .expect("both documents open the vault");
    lock_vault(s).await.unwrap();
}

/// 🔴 C2 — the no-reauth forced change is gated to a session freshly opened by recovery. A
/// NORMAL unlock has no pending reset, so `change_password_after_recovery` is refused — a
/// walk-up attacker at an unlocked vault cannot change the password without the old one.
#[tokio::test]
async fn forced_change_is_refused_on_a_normal_session() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, PW).await.unwrap();
    let e = change_password_after_recovery(
        &mut session,
        kdf(&h),
        keychain(&h),
        biometric(&h),
        Zeroizing::new(NEW_PW.to_owned()),
    )
    .await
    .unwrap_err();
    assert!(
        matches!(e, VaultError::NoRecoveryResetPending),
        "a normal session must not allow a no-reauth password change; got {e:?}"
    );
    lock_vault(session).await.unwrap();
}

/// Enroll → recovery-not-configured after revoke: `revoke_recovery_key` clears the slot, so a
/// later recovery unlock is refused (`RecoveryNotConfigured`), not a brick.
#[tokio::test]
async fn revoke_turns_recovery_off() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, PW).await.unwrap();
    let display = enroll_recovery_key(&mut session, kdf(&h), keychain(&h))
        .await
        .unwrap()
        .recovery_key_display;
    revoke_recovery_key(&mut session).await.unwrap();
    lock_vault(session).await.unwrap();

    assert!(h.repo.load_config().await.unwrap().recovery_slot.is_none());

    let recovery_key = parse_recovery_key(&display).unwrap();
    let e = build_unlock(&h)
        .unlock_with_recovery_key(h.home.clone(), recovery_key, Zeroizing::new(h.secret_key))
        .await
        .unwrap_err();
    assert!(matches!(e, VaultError::RecoveryNotConfigured), "got {e:?}");
}

/// 🔴 The recovery counterpart of the change-password/rotate → revert guards: after a
/// **recovery unlock + forced new password** rewraps a snapshot to the new KEK, reverting to
/// it opens **cleanly** (`Reverted`), not `SnapshotCorrupt` — the "recover, then revert" path
/// is not corrupt. (Regression guard for the `rewrap_one` `vault_blake3` re-stamp.)
#[tokio::test]
async fn revert_after_recovery_opens_the_rewrapped_snapshot() {
    let h = Harness::fresh().await;
    let unlocker = build_unlock(&h);
    let keychain_arc = keychain(&h); // captured before `h` is destructured below
    let home = h.home.clone();

    // Enrol recovery, snapshot the 1-entry "keeper" state, then diverge (add a 2nd entry).
    let mut session = unlock(&h, PW).await.unwrap();
    seed_one_entry(&mut session).await; // "e1" — the keeper
    let display = enroll_recovery_key(&mut session, kdf(&h), keychain(&h))
        .await
        .unwrap()
        .recovery_key_display;
    let snap = create_snapshot(&session, SnapshotReason::Manual)
        .await
        .unwrap();
    create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("e2"),
        },
    )
    .await
    .unwrap();
    lock_vault(session).await.unwrap();

    // Recover → forced new password (KEK₂) → rewrap_snapshots re-wraps the snapshot to KEK₂.
    let recovery_key = parse_recovery_key(&display).unwrap();
    let mut recovered = unlocker
        .unlock_with_recovery_key(home.clone(), recovery_key, Zeroizing::new(h.secret_key))
        .await
        .unwrap();
    change_password_after_recovery(
        &mut recovered,
        kdf(&h),
        keychain(&h),
        biometric(&h),
        Zeroizing::new(NEW_PW.to_owned()),
    )
    .await
    .unwrap();

    // Only the recovered session may hold `vault.vdb` for the swap — drop the harness handles.
    let Harness {
        tempdir,
        repo,
        blob,
        ..
    } = h;
    drop(repo);
    drop(blob);
    let factory = SqliteVaultRepositoryFactory::new();

    let outcome = revert_to_snapshot_in_session(
        &recovered,
        &unlocker,
        &factory,
        keychain_arc.as_ref(),
        RevertToSnapshotInput {
            vault: home,
            snapshot_id: snap.id,
            confirm_rollback: true,
        },
    )
    .await
    .unwrap();
    drop(recovered);

    match outcome {
        SeamlessRevertOutcome::Reverted { session, report } => {
            assert_eq!(report.entry_count, 1, "reverted to the 1-entry snapshot");
            assert_eq!(
                session.index().all_active().len(),
                1,
                "the re-wrapped snapshot opens cleanly after a recovery-forced password change"
            );
        }
        SeamlessRevertOutcome::NeedsUnlock { .. } => {
            panic!("a re-wrapped snapshot must Revert after recovery, not fall back to NeedsUnlock")
        }
        SeamlessRevertOutcome::CommitFailed { error } => panic!("revert commit failed: {error:?}"),
    }
    let _ = tempdir;
}

/// Read a snapshot's own `vault.vdb` `recovery_slot` directly.
async fn snapshot_slot(snap_dir: &std::path::Path) -> Option<[u8; 40]> {
    let db = VaultDbConnection::open(&snap_dir.join("vault.vdb"))
        .await
        .unwrap();
    let repo = SqliteVaultRepository::new(db.handle());
    let slot = repo.load_config().await.unwrap().recovery_slot;
    drop(repo);
    db.close().await.unwrap();
    slot
}
