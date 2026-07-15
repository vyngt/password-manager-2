//! Biometric-unlock use cases against a mock authenticator (`MemoryBiometricAuthenticator`
//! on the harness). The real Windows Hello / Touch ID path can't run headless — this
//! suite proves everything the platform layer sits on top of: enroll → `unlock_with_kek`
//! roundtrip, wrong-KEK rejection, the wrong-password enroll gate, the load-bearing
//! change-password KEK refresh, and disable.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::too_many_lines,
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
    enroll_biometric, lock_vault,
};
use vedge_core::domain::vault::crypto_constants::KEK_LEN;
use vedge_core::domain::vault::entities::AuditAction;
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::domain::vault::payloads::{CommonMeta, EntryPayload, EntryType, LoginPayload};

const PW: &str = "correct horse battery staple";

async fn unlock_pw(h: &Harness, pw: &str) -> VaultSession {
    build_unlock(h)
        .execute(UnlockVaultInput {
            vault_path: h.home.clone(),
            master_password: Zeroizing::new(pw.to_owned()),
            secret_key: None,
        })
        .await
        .unwrap()
}

fn login(name: &str, pw: &str) -> EntryPayload {
    EntryPayload::Login(LoginPayload {
        meta: CommonMeta::new(name, EntryType::Login),
        username: "alice".into(),
        password: SecretString::from(pw),
        totp_secret: None,
        totp_params: vedge_core::TotpParams::default(),
        recovery_codes: vec![],
    })
}

async fn seed_entry(session: &mut VaultSession, name: &str, pw: &str) {
    create_entry(
        session,
        CreateEntryInput {
            payload: login(name, pw),
        },
    )
    .await
    .unwrap();
}

async fn enroll(h: &Harness, session: &VaultSession, pw: &str) -> Result<(), VaultError> {
    enroll_biometric(
        session,
        Arc::clone(&h.kdf) as Arc<dyn KeyDerivationProvider>,
        Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>,
        Arc::clone(&h.biometric) as Arc<dyn BiometricAuthenticator>,
        Zeroizing::new(pw.to_owned()),
    )
    .await
}

#[tokio::test]
async fn enroll_then_unlock_with_kek_roundtrip() {
    let h = Harness::fresh().await;
    let mut session = unlock_pw(&h, PW).await;
    seed_entry(&mut session, "GitHub", "p1").await;

    // Enroll with the correct master password stores the live KEK behind the gate.
    enroll(&h, &session, PW).await.unwrap();
    assert!(h.biometric.is_enrolled(&h.vault_uuid).unwrap());
    lock_vault(session).await.unwrap();

    // Biometric unlock: gate releases the KEK → `unlock_with_kek` rebuilds the session
    // with no password.
    let kek = h.biometric.retrieve(&h.vault_uuid).unwrap();
    let session2 = build_unlock(&h)
        .unlock_with_kek(h.home.clone(), kek)
        .await
        .unwrap();
    assert_eq!(session2.index().all_active().len(), 1);

    // The unlock is audited distinctly as `BiometricUnlocked`.
    let events = h.repo.recent_audit(50).await.unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e.action, AuditAction::BiometricUnlocked)),
        "biometric unlock is audited"
    );
    lock_vault(session2).await.unwrap();
}

/// 🔴 Slice 5.2.3 — B1: biometric enrollment is keyed on the vault's intrinsic `vault_uuid`,
/// not its file path, so a moved or renamed home keeps its Hello enrollment. The exact mirror
/// of 5.2.0's `a_moved_home_keeps_its_keychain_entry` — which passed while this one could not
/// even be written, because the port took a `&VaultId` (path). The physical-move proof is the
/// on-device human smoke; here the point is the credential lookup no longer involves any path.
#[tokio::test]
async fn a_moved_home_keeps_its_biometric_enrollment() {
    let h = Harness::fresh().await;
    let session = unlock_pw(&h, PW).await;
    enroll(&h, &session, PW).await.unwrap();
    lock_vault(session).await.unwrap();

    // The uuid is invariant under a move/rename (unlike the pre-5.2.3 `SHA-256(path)` key, which
    // a move would change and thereby silently un-enroll the user). The port takes ONLY the
    // uuid, so a change of the home path cannot reach or invalidate the stored credential.
    assert!(
        h.biometric.is_enrolled(&h.vault_uuid).unwrap(),
        "the vault is enrolled under its uuid"
    );
    let kek = h.biometric.retrieve(&h.vault_uuid).unwrap();
    assert_eq!(
        *kek, h.kek,
        "the exact KEK is released, keyed on the move-invariant uuid"
    );
}

#[tokio::test]
async fn unlock_with_kek_rejects_wrong_kek() {
    let h = Harness::fresh().await;
    let mut session = unlock_pw(&h, PW).await;
    seed_entry(&mut session, "GitHub", "p1").await;
    lock_vault(session).await.unwrap();

    // A bogus KEK can't decrypt the seeded entry → surfaces as WrongCredentials so the
    // UI falls back to the password screen.
    let wrong = Zeroizing::new([0u8; KEK_LEN]);
    let err = build_unlock(&h)
        .unlock_with_kek(h.home.clone(), wrong)
        .await
        .unwrap_err();
    assert!(matches!(err, VaultError::WrongCredentials), "got {err:?}");
}

#[tokio::test]
async fn enroll_rejects_wrong_password() {
    let h = Harness::fresh().await;
    let session = unlock_pw(&h, PW).await;

    let err = enroll(&h, &session, "not-the-password").await.unwrap_err();
    assert!(matches!(err, VaultError::WrongCredentials), "got {err:?}");
    assert!(
        !h.biometric.is_enrolled(&h.vault_uuid).unwrap(),
        "nothing stored on a wrong-password enroll"
    );
}

#[tokio::test]
async fn change_password_restores_stored_kek() {
    // The load-bearing lifecycle test: change_password derives a *new* KEK and re-wraps
    // every DEK, so the enrolled KEK goes stale. The change_password hook must re-store
    // the new KEK, or biometric unlock breaks silently.
    let h = Harness::fresh().await;
    let mut session = unlock_pw(&h, PW).await;
    seed_entry(&mut session, "GitHub", "p1").await;
    enroll(&h, &session, PW).await.unwrap();

    change_password(
        &mut session,
        Arc::clone(&h.kdf) as Arc<dyn KeyDerivationProvider>,
        Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>,
        Arc::clone(&h.biometric) as Arc<dyn BiometricAuthenticator>,
        ChangePasswordInput {
            new_password: Zeroizing::new("new-master".into()),
            new_secret_key: None,
        },
    )
    .await
    .unwrap();
    lock_vault(session).await.unwrap();

    // The gate now holds the refreshed KEK; biometric unlock still decrypts the entry
    // (which was re-wrapped under the new KEK).
    let kek = h.biometric.retrieve(&h.vault_uuid).unwrap();
    let session2 = build_unlock(&h)
        .unlock_with_kek(h.home.clone(), kek)
        .await
        .unwrap();
    assert_eq!(session2.index().all_active().len(), 1);
    lock_vault(session2).await.unwrap();
}

#[tokio::test]
async fn disable_removes_key() {
    let h = Harness::fresh().await;
    let session = unlock_pw(&h, PW).await;
    enroll(&h, &session, PW).await.unwrap();
    assert!(h.biometric.is_enrolled(&h.vault_uuid).unwrap());

    h.biometric.disable(&h.vault_uuid).unwrap();
    assert!(!h.biometric.is_enrolled(&h.vault_uuid).unwrap());
    assert!(matches!(
        h.biometric.retrieve(&h.vault_uuid).unwrap_err(),
        VaultError::BiometricNotEnrolled
    ));

    // Disable is idempotent — a second call is not an error.
    h.biometric.disable(&h.vault_uuid).unwrap();
    lock_vault(session).await.unwrap();
}

/// Slice 4.6a: `unlock_with_kek` (the biometric path) must ALSO backfill a missing
/// `vault_uuid` — it's the easy-to-miss second unlock path. The harness vault starts
/// pre-4.6 (uuid `None`).
#[tokio::test]
async fn vault_uuid_backfilled_on_biometric_unlock() {
    let h = Harness::fresh().await;
    h.seed_login("gh", "alice", "pw").await;
    // The harness now seeds a uuid-bearing vault (slice 5.2.0); recreate the pre-4.6
    // precondition (uuid `None`) this backfill test exercises.
    let mut cfg = h.repo.load_config().await.unwrap();
    cfg.vault_uuid = None;
    h.repo.save_config(&cfg).await.unwrap();
    assert!(
        h.repo.load_config().await.unwrap().vault_uuid.is_none(),
        "precondition: a pre-4.6 vault has no uuid"
    );

    let uv = build_unlock(&h);
    let session = uv
        .unlock_with_kek(h.home.clone(), Zeroizing::new(h.kek))
        .await
        .unwrap();
    assert_eq!(session.vault_id(), &h.vault_id);
    lock_vault(session).await.unwrap();

    assert!(
        h.repo.load_config().await.unwrap().vault_uuid.is_some(),
        "biometric unlock must backfill vault_uuid"
    );
}
