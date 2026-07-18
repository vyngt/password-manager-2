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
    ChangePasswordInput, CreateEntryInput, UnlockVaultInput, change_password, create_entry,
    create_snapshot, lock_vault,
};
use vedge_core::domain::shared::SNAPSHOTS_DIR;
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::domain::vault::payloads::{CommonMeta, EntryPayload, EntryType, LoginPayload};
use vedge_core::infrastructure::snapshot::manifest::SnapshotReason;
use vedge_core::infrastructure::snapshot::store;
use vedge_core::infrastructure::sqlite::vault::{SqliteVaultRepository, VaultDbConnection};

const PW: &str = "correct horse battery staple";
const DUMMY_SLOT: [u8; 40] = [0x42; 40];

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
