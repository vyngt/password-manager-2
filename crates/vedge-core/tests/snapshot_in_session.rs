//! Seamless in-place revert (slice 5.2.3, Decision ⑰): reverting from `/v/snapshots` keeps the
//! user IN the vault instead of ejecting them to the launch screen. Proves the core
//! `revert_to_snapshot_in_session` orchestration end-to-end:
//!
//! - a HEALTHY revert stays unlocked, reflects the reverted state, and forges no `BiometricUnlocked` row;
//! - a STALE snapshot (its rewrap failed, so it opens only under the OLD KEK) becomes `NeedsUnlock`, not a revert failure — the swap committed (L1);
//! - a pre-commit failure (an unknown snapshot id, or an unconfirmed rollback) returns `Err` with the live session UNTOUCHED — the ejection-bug regression guard.
//!
//! Like the product (which reverts a live vault), the swap needs no OTHER open handle on
//! `vault.vdb`: each test drops the harness's own repo/blob handles, leaving only the live
//! SESSION — whose DB the use case closes itself before the swap.

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
use zeroize::Zeroizing;

use vedge_core::application::vault::ports::{
    BiometricAuthenticator, KeyDerivationProvider, KeychainProvider,
};
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::{
    ChangePasswordInput, CreateEntryInput, RevertToSnapshotInput, SeamlessRevertOutcome,
    UnlockVault, UnlockVaultInput, change_password, create_entry, create_snapshot, list_audit,
    revert_to_snapshot_in_session,
};
use vedge_core::domain::shared::{SNAPSHOTS_DIR, VAULT_FILE};
use vedge_core::domain::vault::entities::{AuditAction, AuditQuery};
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::domain::vault::payloads::{CommonMeta, EntryPayload, EntryType, LoginPayload};
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

/// Everything a seamless revert needs, with the harness's OWN `vault.vdb` handle already
/// dropped so only the live session holds it.
struct Live {
    _tempdir: tempfile::TempDir,
    session: VaultSession,
    unlock: UnlockVault,
    keychain: Arc<dyn KeychainProvider>,
    kdf: Arc<dyn KeyDerivationProvider>,
    biometric: Arc<dyn BiometricAuthenticator>,
    home: std::path::PathBuf,
    factory: SqliteVaultRepositoryFactory,
}

/// Build a vault, unlock it (a LIVE session), add "keeper", snapshot it, add "extra" — then
/// drop the harness's own repo/blob so only the session holds `vault.vdb`. Returns the live
/// session (2 entries) and the id of the 1-entry snapshot.
async fn build_live_with_snapshot() -> (Live, String) {
    let h = Harness::fresh().await;
    let keychain: Arc<dyn KeychainProvider> = Arc::clone(&h.keychain) as _;
    let kdf: Arc<dyn KeyDerivationProvider> = Arc::clone(&h.kdf) as _;
    let biometric: Arc<dyn BiometricAuthenticator> = Arc::clone(&h.biometric) as _;
    let unlock = build_unlock(&h);
    let home = h.home.clone();

    let mut session = unlock
        .execute(UnlockVaultInput {
            vault_path: home.clone(),
            master_password: h.master_password.clone(),
            secret_key: None,
        })
        .await
        .unwrap();
    add(&mut session, "keeper").await;
    let snap = create_snapshot(&session, SnapshotReason::Manual)
        .await
        .unwrap();
    add(&mut session, "extra").await;
    assert_eq!(session.index().all_active().len(), 2);

    // Drop the harness's own handles; keep the tempdir so the vault files survive.
    let Harness {
        tempdir,
        repo,
        blob,
        ..
    } = h;
    drop(repo);
    drop(blob);

    (
        Live {
            _tempdir: tempdir,
            session,
            unlock,
            keychain,
            kdf,
            biometric,
            home,
            factory: SqliteVaultRepositoryFactory::new(),
        },
        snap.id,
    )
}

/// Test 5 — a healthy revert keeps the user IN the vault and forges no unlock audit row.
#[tokio::test]
async fn seamless_revert_stays_unlocked_and_reflects_the_snapshot() {
    let (ctx, snap_id) = build_live_with_snapshot().await;

    let outcome = revert_to_snapshot_in_session(
        &ctx.session,
        &ctx.unlock,
        &ctx.factory,
        ctx.keychain.as_ref(),
        RevertToSnapshotInput {
            vault: ctx.home.clone(),
            snapshot_id: snap_id,
            confirm_rollback: true,
        },
    )
    .await
    .unwrap();
    drop(ctx.session); // the old (now-closed) session

    let session = match outcome {
        SeamlessRevertOutcome::Reverted { session, report } => {
            assert_eq!(report.entry_count, 1, "reverted to the 1-entry snapshot");
            session
        }
        _ => panic!("expected Reverted, got a different outcome"),
    };

    // The user STAYED in the vault — a live session over the reverted state, no re-unlock.
    let active = session.index().all_active();
    assert_eq!(active.len(), 1, "the seamless session reflects the revert");

    // 🔴 The re-open must NOT forge a `BiometricUnlocked` row (the honest event is
    // `BackupRestored`, written by the revert). This vault was never biometric-unlocked.
    let page = list_audit(
        &session,
        AuditQuery {
            limit: 200,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(
        !page
            .events
            .iter()
            .any(|e| matches!(e.action, AuditAction::BiometricUnlocked)),
        "the seamless re-open forged a BiometricUnlocked row"
    );
    assert!(
        page.events
            .iter()
            .any(|e| matches!(e.action, AuditAction::BackupRestored)),
        "the revert recorded BackupRestored (the honest event)"
    );
}

/// Test 6 — a STALE snapshot (rewrap failed) falls back to `NeedsUnlock`, never a revert
/// failure. The swap has committed; a re-open failure must not present as "revert failed" (L1).
///
/// Construction: snapshot S1 under KEK₁ (self-consistent), then change the password to KEK₂
/// (which re-wraps S1). Restore S1's ORIGINAL `vault.vdb` + `manifest.json` — so S1 is exactly a
/// snapshot whose rewrap "did not happen": self-verifying, but decryptable only under KEK₁, while
/// the live session now holds KEK₂.
#[tokio::test]
async fn a_stale_snapshot_falls_back_to_needs_unlock_not_a_failure() {
    let h = Harness::fresh().await;
    let keychain: Arc<dyn KeychainProvider> = Arc::clone(&h.keychain) as _;
    let kdf: Arc<dyn KeyDerivationProvider> = Arc::clone(&h.kdf) as _;
    let biometric: Arc<dyn BiometricAuthenticator> = Arc::clone(&h.biometric) as _;
    let unlock = build_unlock(&h);
    let home = h.home.clone();
    let store_dir = home.join(SNAPSHOTS_DIR);

    let mut session = unlock
        .execute(UnlockVaultInput {
            vault_path: home.clone(),
            master_password: h.master_password.clone(),
            secret_key: None,
        })
        .await
        .unwrap();
    add(&mut session, "keeper").await;
    let snap = create_snapshot(&session, SnapshotReason::Manual)
        .await
        .unwrap();

    // Save S1's KEK₁ vault.vdb + manifest aside BEFORE the password change re-wraps them.
    let s1_dir = store::resolve_snapshot_dir(&store_dir, &snap.id).unwrap();
    let aside = tempfile::tempdir().unwrap();
    std::fs::copy(s1_dir.join(VAULT_FILE), aside.path().join("vault.vdb")).unwrap();
    std::fs::copy(
        s1_dir.join("manifest.json"),
        aside.path().join("manifest.json"),
    )
    .unwrap();

    // Change the password → KEK₂. The hook re-wraps S1 (best-effort). The live session is KEK₂.
    change_password(
        &mut session,
        Arc::clone(&kdf),
        Arc::clone(&keychain),
        Arc::clone(&biometric),
        ChangePasswordInput {
            new_password: Zeroizing::new("a different master".into()),
            new_secret_key: None,
            secret_key_rotated: false,
        },
    )
    .await
    .unwrap();

    // Restore S1 exactly as it was under KEK₁ — a snapshot whose rewrap "never happened".
    std::fs::copy(aside.path().join("vault.vdb"), s1_dir.join(VAULT_FILE)).unwrap();
    std::fs::copy(
        aside.path().join("manifest.json"),
        s1_dir.join("manifest.json"),
    )
    .unwrap();

    // Drop the harness's own handles so only the KEK₂ session holds vault.vdb.
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
        &session,
        &unlock,
        &factory,
        keychain.as_ref(),
        RevertToSnapshotInput {
            vault: home.clone(),
            snapshot_id: snap.id,
            confirm_rollback: true,
        },
    )
    .await
    .expect("a stale snapshot must NOT surface as an Err — the swap already committed");
    drop(session);

    match outcome {
        SeamlessRevertOutcome::NeedsUnlock { .. } => {
            // The swap committed: the home now holds the KEK₁ vault, unreadable under KEK₂.
            // The user re-unlocks with that snapshot's credentials — not a revert failure.
        }
        SeamlessRevertOutcome::Reverted { .. } => {
            panic!("the KEK₁ snapshot must not re-open under the KEK₂ session")
        }
        SeamlessRevertOutcome::CommitFailed { error } => {
            panic!("the swap must have committed, but it failed: {error:?}")
        }
    }
    let _ = tempdir;
}

/// 🔴 Test 7 — the HEALTHY counterpart to test 6, and the "change master password, then revert"
/// path: after a password change re-wraps the snapshot to KEK₂, reverting to it OPENS cleanly
/// under the new session KEK (`Reverted`, not `NeedsUnlock`) — not corrupt. Guards the
/// `change_password` → `rewrap_snapshots` → revert chain (incl. slice 5.7's slot-nulling in the
/// snapshot copy).
#[tokio::test]
async fn revert_after_change_password_opens_the_rewrapped_snapshot() {
    let (mut ctx, snap_id) = build_live_with_snapshot().await; // KEK₁; live=2 (keeper,extra), snap=1

    change_password(
        &mut ctx.session,
        Arc::clone(&ctx.kdf),
        Arc::clone(&ctx.keychain),
        Arc::clone(&ctx.biometric),
        ChangePasswordInput {
            new_password: Zeroizing::new("a-different-master-9".into()),
            new_secret_key: None,
            secret_key_rotated: false,
        },
    )
    .await
    .unwrap();

    let outcome = revert_to_snapshot_in_session(
        &ctx.session,
        &ctx.unlock,
        &ctx.factory,
        ctx.keychain.as_ref(),
        RevertToSnapshotInput {
            vault: ctx.home.clone(),
            snapshot_id: snap_id,
            confirm_rollback: true,
        },
    )
    .await
    .unwrap();
    drop(ctx.session);

    match outcome {
        SeamlessRevertOutcome::Reverted { session, report } => {
            assert_eq!(report.entry_count, 1, "reverted to the 1-entry snapshot");
            assert_eq!(
                session.index().all_active().len(),
                1,
                "the re-wrapped snapshot opens cleanly under the new KEK — not corrupt"
            );
        }
        SeamlessRevertOutcome::NeedsUnlock { .. } => {
            panic!(
                "a re-wrapped snapshot must Revert under the new KEK, not fall back to NeedsUnlock"
            )
        }
        SeamlessRevertOutcome::CommitFailed { error } => panic!("revert commit failed: {error:?}"),
    }
    let _ = (ctx.kdf, ctx.biometric);
}

/// 🔴 Test 7b — the "rotate the Secret Key, then revert" path. A rotation is `change_password`
/// with a fresh Secret Key (same master password), which re-derives KEK₂ and re-wraps the
/// snapshot; reverting to it must open cleanly under the new session KEK.
#[tokio::test]
async fn revert_after_secret_key_rotation_opens_the_rewrapped_snapshot() {
    let (mut ctx, snap_id) = build_live_with_snapshot().await;

    // Rotate: keep the master password, hand in a fresh Secret Key (what rotate_secret_key does).
    change_password(
        &mut ctx.session,
        Arc::clone(&ctx.kdf),
        Arc::clone(&ctx.keychain),
        Arc::clone(&ctx.biometric),
        ChangePasswordInput {
            // Keep the master password (the harness's), rotate only the Secret Key.
            new_password: Zeroizing::new("correct horse battery staple".into()),
            new_secret_key: Some(Zeroizing::new([0xAB; 16])),
            secret_key_rotated: true,
        },
    )
    .await
    .unwrap();

    let outcome = revert_to_snapshot_in_session(
        &ctx.session,
        &ctx.unlock,
        &ctx.factory,
        ctx.keychain.as_ref(),
        RevertToSnapshotInput {
            vault: ctx.home.clone(),
            snapshot_id: snap_id,
            confirm_rollback: true,
        },
    )
    .await
    .unwrap();
    drop(ctx.session);

    match outcome {
        SeamlessRevertOutcome::Reverted { session, report } => {
            assert_eq!(report.entry_count, 1);
            assert_eq!(
                session.index().all_active().len(),
                1,
                "the re-wrapped snapshot opens cleanly after a Secret-Key rotation"
            );
        }
        SeamlessRevertOutcome::NeedsUnlock { .. } => {
            panic!("a re-wrapped snapshot must Revert after a rotation, not NeedsUnlock")
        }
        SeamlessRevertOutcome::CommitFailed { error } => panic!("revert commit failed: {error:?}"),
    }
    let _ = (ctx.kdf, ctx.biometric);
}

/// Test 6b — a pre-commit failure (an unknown snapshot id) returns `Err` with the live session
/// UNTOUCHED: the preflight runs BEFORE any teardown, so the user stays unlocked. This is the
/// ejection-bug regression guard.
#[tokio::test]
async fn a_preflight_failure_leaves_the_session_untouched() {
    let (mut ctx, _snap_id) = build_live_with_snapshot().await;

    let outcome = revert_to_snapshot_in_session(
        &ctx.session,
        &ctx.unlock,
        &ctx.factory,
        ctx.keychain.as_ref(),
        RevertToSnapshotInput {
            vault: ctx.home.clone(),
            snapshot_id: "20990101T000000000Z".to_owned(),
            confirm_rollback: true,
        },
    )
    .await;
    assert!(
        matches!(outcome, Err(VaultError::SnapshotNotFound(_))),
        "an unknown snapshot must fail the preflight before any teardown"
    );

    // The session's DB was NEVER closed — a write still succeeds, and the state is intact.
    add(&mut ctx.session, "after").await;
    assert_eq!(
        ctx.session.index().all_active().len(),
        3,
        "the live session is untouched after a preflight failure"
    );
    let _ = (ctx.kdf, ctx.biometric);
}

/// Test 6c — an UNCONFIRMED rollback is caught in the read-only preflight (the live vault is
/// ahead of the snapshot), so it too returns `Err` with the session untouched — never tearing it
/// down inside the post-teardown revert. (Found in the pre-commit `/code-review`.)
#[tokio::test]
async fn an_unconfirmed_rollback_is_caught_in_preflight_and_keeps_the_session() {
    let (mut ctx, snap_id) = build_live_with_snapshot().await;
    // The live vault (2 entries) is AHEAD of the 1-entry snapshot, so reverting is a rollback.
    let outcome = revert_to_snapshot_in_session(
        &ctx.session,
        &ctx.unlock,
        &ctx.factory,
        ctx.keychain.as_ref(),
        RevertToSnapshotInput {
            vault: ctx.home.clone(),
            snapshot_id: snap_id,
            confirm_rollback: false,
        },
    )
    .await;
    assert!(
        matches!(outcome, Err(VaultError::RollbackNotConfirmed)),
        "an unconfirmed rollback must fail the preflight, not tear down the session"
    );

    add(&mut ctx.session, "after").await;
    assert_eq!(
        ctx.session.index().all_active().len(),
        3,
        "the live session survives an unconfirmed-rollback refusal"
    );
    let _ = (ctx.kdf, ctx.biometric);
}
