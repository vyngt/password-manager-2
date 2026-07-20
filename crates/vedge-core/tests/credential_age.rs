//! Slice 5.9 ③ — credential-age tracking + the `SecretKeyRotated` audit split.
//!
//! Two properties under test:
//!   1. The right credential path stamps the right `vault_config` timestamp, persisted to disk.
//!   2. A Secret-Key rotation is DISTINCT from a password change in the audit log, and a recovery
//!      reset (which re-stores the SAME key) is NOT recorded as a rotation — the edge that makes
//!      `new_secret_key.is_some()` an invalid signal.

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

use chrono::Duration;
use common::{Harness, build_unlock};
use zeroize::Zeroizing;

use vedge_core::application::vault::ports::{
    BiometricAuthenticator, KeyDerivationProvider, KeychainProvider, VaultRepository,
};
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::{
    ChangePasswordInput, UnlockVaultInput, change_password, change_password_after_recovery,
    enroll_recovery_key, lock_vault,
};
use vedge_core::domain::shared::now;
use vedge_core::domain::vault::crypto_constants::SECRET_KEY_LEN;
use vedge_core::domain::vault::entities::{AuditAction, AuditQuery};
use vedge_core::domain::vault::secret_key::parse_recovery_key;

const PW: &str = "correct horse battery staple";

async fn unlock(h: &Harness, pw: &str) -> VaultSession {
    build_unlock(h)
        .execute(UnlockVaultInput {
            vault_path: h.home.clone(),
            master_password: Zeroizing::new(pw.to_owned()),
            secret_key: None,
        })
        .await
        .unwrap()
}

fn kdf(h: &Harness) -> Arc<dyn KeyDerivationProvider> {
    Arc::clone(&h.kdf) as _
}
fn keychain(h: &Harness) -> Arc<dyn KeychainProvider> {
    Arc::clone(&h.keychain) as _
}
fn biometric(h: &Harness) -> Arc<dyn BiometricAuthenticator> {
    Arc::clone(&h.biometric) as _
}

/// The most-recent CREDENTIAL audit action (newest first), filtered SQL-side so the `Locked` /
/// `Unlocked` / `RecoveryUsed` rows that bracket the operation can't shadow it.
async fn latest_credential_action(h: &Harness) -> AuditAction {
    h.repo
        .query_audit(&AuditQuery {
            actions: vec![AuditAction::PasswordChanged, AuditAction::SecretKeyRotated],
            limit: 1,
            ..Default::default()
        })
        .await
        .unwrap()
        .events
        .first()
        .expect("a credential audit row")
        .action
        .clone()
}

/// Assert the vault is coherent from a fresh open under the post-change credentials.
async fn assert_coherent_after(h: &Harness, new_pw: &str, sk: &[u8; SECRET_KEY_LEN]) {
    common::coherence::assert_vault_coherent(
        &h.home,
        &common::coherence::Creds {
            master_password: new_pw,
            secret_key: sk,
            recovery_key: None,
        },
        Some(&common::coherence::OsState {
            vault_uuid: &h.vault_uuid,
            keychain: h.keychain.as_ref(),
        }),
    )
    .await;
}

#[tokio::test]
async fn plain_change_stamps_password_only() {
    let h = Harness::fresh().await;
    assert!(
        h.repo
            .load_config()
            .await
            .unwrap()
            .last_password_change_at
            .is_none(),
        "a fresh vault has no credential-age stamp"
    );

    let mut session = unlock(&h, PW).await;
    change_password(
        &mut session,
        kdf(&h),
        keychain(&h),
        biometric(&h),
        ChangePasswordInput {
            new_password: Zeroizing::new("new-pw-one".into()),
            new_secret_key: None,
            secret_key_rotated: false,
        },
    )
    .await
    .unwrap();
    lock_vault(session).await.unwrap();

    let cfg = h.repo.load_config().await.unwrap();
    assert!(cfg.last_password_change_at.is_some(), "password stamped");
    assert!(
        cfg.last_secret_key_rotation_at.is_none(),
        "a plain change must NOT stamp a Secret-Key rotation"
    );
    assert_eq!(
        latest_credential_action(&h).await,
        AuditAction::PasswordChanged
    );

    assert_coherent_after(&h, "new-pw-one", &h.secret_key).await;
}

#[tokio::test]
async fn rotate_stamps_both_and_audits_secret_key_rotated() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, PW).await;

    let new_sk: [u8; SECRET_KEY_LEN] = [0x5A; SECRET_KEY_LEN];
    change_password(
        &mut session,
        kdf(&h),
        keychain(&h),
        biometric(&h),
        ChangePasswordInput {
            new_password: Zeroizing::new(PW.into()),
            new_secret_key: Some(Zeroizing::new(new_sk)),
            secret_key_rotated: true,
        },
    )
    .await
    .unwrap();
    lock_vault(session).await.unwrap();

    let cfg = h.repo.load_config().await.unwrap();
    assert!(
        cfg.last_password_change_at.is_some(),
        "rotation refreshes pw KEK"
    );
    assert!(
        cfg.last_secret_key_rotation_at.is_some(),
        "a rotation stamps BOTH timestamps"
    );
    assert_eq!(
        latest_credential_action(&h).await,
        AuditAction::SecretKeyRotated,
        "a rotation is distinguishable from a password change in the audit log"
    );

    assert_coherent_after(&h, PW, &new_sk).await;
}

/// 🔴 The fail-first guard for the same-key edge (slice 5.9 ③). A recovery reset re-stores the
/// SAME Secret Key (`new_secret_key: Some(same_key)`), so a naive `is_some()` check would record a
/// rotation that never happened. It must stamp the password but NOT the Secret-Key rotation, and
/// audit `PasswordChanged`, not `SecretKeyRotated`.
#[tokio::test]
async fn recovery_reset_stamps_password_not_rotation() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, PW).await;
    let display = enroll_recovery_key(&mut session, kdf(&h), keychain(&h))
        .await
        .unwrap()
        .recovery_key_display;
    lock_vault(session).await.unwrap();

    let recovery_key = parse_recovery_key(&display).unwrap();
    let mut recovered = build_unlock(&h)
        .unlock_with_recovery_key(h.home.clone(), recovery_key, Zeroizing::new(h.secret_key))
        .await
        .unwrap();
    change_password_after_recovery(
        &mut recovered,
        kdf(&h),
        keychain(&h),
        biometric(&h),
        Zeroizing::new("new-after-recovery".into()),
    )
    .await
    .unwrap();
    lock_vault(recovered).await.unwrap();

    let cfg = h.repo.load_config().await.unwrap();
    assert!(
        cfg.last_password_change_at.is_some(),
        "recovery reset refreshes the password"
    );
    assert!(
        cfg.last_secret_key_rotation_at.is_none(),
        "recovery reset re-stores the SAME key — it must NOT record a rotation"
    );
    assert_eq!(
        latest_credential_action(&h).await,
        AuditAction::PasswordChanged,
        "recovery reset audits PasswordChanged, never SecretKeyRotated"
    );

    // The recovery reset also nulled the slot (#7 = None, no RK needed).
    assert_coherent_after(&h, "new-after-recovery", &h.secret_key).await;
}

/// The credential age must live in `vault_config`, NOT be derived from the retention-pruned audit
/// log — otherwise it would silently read "never" once the last `PasswordChanged` row is pruned
/// (the inverted failure mode). Prove it survives wiping the entire log.
#[tokio::test]
async fn age_stamp_survives_audit_prune() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, PW).await;
    change_password(
        &mut session,
        kdf(&h),
        keychain(&h),
        biometric(&h),
        ChangePasswordInput {
            new_password: Zeroizing::new("pruned-pw".into()),
            new_secret_key: None,
            secret_key_rotated: false,
        },
    )
    .await
    .unwrap();
    lock_vault(session).await.unwrap();

    assert!(
        h.repo
            .load_config()
            .await
            .unwrap()
            .last_password_change_at
            .is_some()
    );

    // Retention pushes the cutoff past every row → the whole log is pruned.
    let deleted = h
        .repo
        .delete_audit_before(now() + Duration::days(3650))
        .await
        .unwrap();
    assert!(deleted > 0, "the change wrote audit rows to prune");
    assert!(
        h.repo
            .query_audit(&AuditQuery {
                limit: 100,
                ..Default::default()
            })
            .await
            .unwrap()
            .events
            .is_empty(),
        "the audit log is now empty"
    );

    // The credential age is untouched — it is a config column, not a log projection.
    assert!(
        h.repo
            .load_config()
            .await
            .unwrap()
            .last_password_change_at
            .is_some(),
        "credential age survives an audit-log prune"
    );

    // Coherent even with an EMPTY audit log (#11 handles zero rows; #13 reads the config stamp).
    assert_coherent_after(&h, "pruned-pw", &h.secret_key).await;
}
