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
use secrecy::{ExposeSecret, SecretString};
use zeroize::Zeroizing;

use vedge_core::application::vault::ports::{
    BiometricAuthenticator, KeyDerivationProvider, KeychainProvider, VaultRepository,
};
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::{
    ChangePasswordInput, CreateEntryInput, FieldSelector, RevealHistoryFieldInput,
    UnlockVaultInput, UpdateEntryInput, change_password, change_password_after_recovery,
    create_entry, enroll_recovery_key, get_history_value, hard_delete_entry, list_history,
    lock_vault, restore_from_history, reveal_history_field, update_entry,
};
use vedge_core::domain::shared::EntryId;
use vedge_core::domain::vault::crypto_constants::SECRET_KEY_LEN;
use vedge_core::domain::vault::entities::AuditAction;
use vedge_core::domain::vault::payloads::{CommonMeta, EntryPayload, EntryType, LoginPayload};
use vedge_core::parse_recovery_key;

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

async fn create_login(session: &mut VaultSession, name: &str, pw: &str) -> EntryId {
    create_entry(
        session,
        CreateEntryInput {
            payload: login(name, pw),
        },
    )
    .await
    .unwrap()
    .entry_id
}

async fn update_login(session: &mut VaultSession, id: &EntryId, pw: &str) {
    // A complete payload with the new password → `full` derives Set-everything.
    update_entry(
        session,
        UpdateEntryInput::full(id.clone(), login("GitHub", pw)),
    )
    .await
    .unwrap();
}

/// Reveal a snapshot's Login password via the audited history path.
async fn history_password(session: &VaultSession, id: &EntryId, history_id: &str) -> String {
    let EntryPayload::Login(p) = get_history_value(session, id, history_id).await.unwrap() else {
        panic!("expected Login snapshot");
    };
    p.password.expose_secret().to_owned()
}

#[tokio::test]
async fn update_captures_previous_version() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, "correct horse battery staple").await;
    let id = create_login(&mut session, "GitHub", "p1").await;

    // Before any edit: only the current live version, no snapshots.
    let before = list_history(&session, &id).await.unwrap();
    assert_eq!(before.len(), 1);
    assert!(before[0].history_id.is_none());

    update_login(&mut session, &id, "p2").await;

    // After one edit: current + one snapshot; changed_fields flags "password".
    let after = list_history(&session, &id).await.unwrap();
    assert_eq!(after.len(), 2);
    assert!(after[0].history_id.is_none(), "current version first");
    assert!(after[1].history_id.is_some(), "snapshot second");
    assert!(
        after[0].changed_fields.iter().any(|f| f == "password"),
        "current shows the password changed: {:?}",
        after[0].changed_fields
    );

    // The snapshot decrypts to the pre-edit value.
    let hid = after[1].history_id.clone().unwrap();
    assert_eq!(history_password(&session, &id, &hid).await, "p1");

    // The captured history version decrypts under the live entry's DEK (invariant #2 — 5.8).
    h.assert_coherent().await;
}

#[tokio::test]
async fn list_history_is_newest_first() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, "correct horse battery staple").await;
    let id = create_login(&mut session, "GitHub", "p1").await;
    update_login(&mut session, &id, "p2").await;
    update_login(&mut session, &id, "p3").await;
    update_login(&mut session, &id, "p4").await;

    let rows = list_history(&session, &id).await.unwrap();
    assert_eq!(rows.len(), 4); // current + 3 snapshots
    assert!(rows[0].history_id.is_none());
    // Strictly descending versions, snapshots after the current row.
    for pair in rows.windows(2) {
        assert!(pair[0].version > pair[1].version, "versions descending");
    }
    assert!(rows[1..].iter().all(|r| r.history_id.is_some()));
}

#[tokio::test]
async fn get_history_value_reveals_and_audits() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, "correct horse battery staple").await;
    let id = create_login(&mut session, "GitHub", "secret1").await;
    update_login(&mut session, &id, "secret2").await;

    let rows = list_history(&session, &id).await.unwrap();
    let hid = rows[1].history_id.clone().unwrap();
    assert_eq!(history_password(&session, &id, &hid).await, "secret1");

    // The reveal is audited as `Viewed`.
    let events = h.repo.recent_audit(50).await.unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e.action, AuditAction::Viewed) && e.entry_id.as_ref() == Some(&id))
    );
}

#[tokio::test]
async fn reveal_history_field_returns_old_value_and_audits_secret_revealed() {
    // The history twin of reveal_field (slice 5.4): a single old field is returned
    // to the renderer and audited as an EXTRACTION (SecretRevealed), not a browse.
    let h = Harness::fresh().await;
    let mut session = unlock(&h, "correct horse battery staple").await;
    let id = create_login(&mut session, "GitHub", "secret1").await;
    update_login(&mut session, &id, "secret2").await;

    let rows = list_history(&session, &id).await.unwrap();
    let hid = rows[1].history_id.clone().unwrap();

    let value = reveal_history_field(
        &session,
        RevealHistoryFieldInput {
            entry_id: id.clone(),
            history_id: hid,
            field: FieldSelector::Password,
        },
        0,
    )
    .await
    .unwrap();
    assert_eq!(value.as_str(), "secret1");

    let events = h.repo.recent_audit(50).await.unwrap();
    assert!(
        events
            .iter()
            .any(|e| e.action == AuditAction::SecretRevealed && e.entry_id.as_ref() == Some(&id)),
        "reveal_history_field audits SecretRevealed"
    );
}

#[tokio::test]
async fn restore_sets_current_and_snapshots_replaced() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, "correct horse battery staple").await;
    let id = create_login(&mut session, "GitHub", "v1pass").await;
    update_login(&mut session, &id, "v2pass").await;

    // Restore the (only) snapshot — the v1 value.
    let rows = list_history(&session, &id).await.unwrap();
    let v1_hid = rows[1].history_id.clone().unwrap();
    restore_from_history(&mut session, &id, &v1_hid)
        .await
        .unwrap();

    // Current is back to v1pass.
    let EntryPayload::Login(cur) = vedge_core::get_entry(
        &mut session,
        vedge_core::GetEntryInput {
            entry_id: id.clone(),
        },
    )
    .await
    .unwrap() else {
        panic!("login");
    };
    assert_eq!(cur.password.expose_secret(), "v1pass");

    // The replaced value (v2pass) was snapshotted first, so it survives.
    let after = list_history(&session, &id).await.unwrap();
    let mut snap_pws = Vec::new();
    for row in after.iter().filter(|r| r.history_id.is_some()) {
        let hid = row.history_id.clone().unwrap();
        snap_pws.push(history_password(&session, &id, &hid).await);
    }
    assert!(
        snap_pws.contains(&"v2pass".to_owned()),
        "replaced value kept: {snap_pws:?}"
    );

    h.assert_coherent().await;
}

#[tokio::test]
async fn retention_prunes_oldest_beyond_cap() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, "correct horse battery staple").await;
    let id = create_login(&mut session, "GitHub", "v1").await;

    // 21 edits → 21 snapshots captured, pruned to the cap of 20.
    for i in 2..=22 {
        update_login(&mut session, &id, &format!("v{i}")).await;
    }

    let snaps = h.repo.list_history(&id).await.unwrap();
    assert_eq!(snaps.len(), 20, "capped at HISTORY_MAX_VERSIONS");
    // The very first version (v1, entry version 1) was pruned.
    assert!(snaps.iter().all(|s| s.version >= 2), "oldest dropped");
}

#[tokio::test]
async fn history_decryptable_after_password_change() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, "correct horse battery staple").await;
    let id = create_login(&mut session, "GitHub", "orig").await;
    update_login(&mut session, &id, "edited").await;

    let hid = list_history(&session, &id).await.unwrap()[1]
        .history_id
        .clone()
        .unwrap();

    // Rotate the master password, then re-unlock with the new one.
    change_password(
        &mut session,
        Arc::clone(&h.kdf) as Arc<dyn KeyDerivationProvider>,
        Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>,
        Arc::clone(&h.biometric) as Arc<dyn BiometricAuthenticator>,
        ChangePasswordInput {
            new_password: Zeroizing::new("new-master".into()),
            new_secret_key: None,
            secret_key_rotated: false,
        },
    )
    .await
    .unwrap();
    lock_vault(session).await.unwrap();

    let session2 = unlock(&h, "new-master").await;

    // The snapshot — encrypted under the entry's stable DEK, whose *wrapping*
    // rotated — still decrypts, because we unwrap from the live row.
    assert_eq!(history_password(&session2, &id, &hid).await, "orig");
    lock_vault(session2).await.unwrap();
}

/// History survives a **Secret-Key rotation** — the same rewrap as a password change (the entry
/// DEK is unchanged; only its KEK-wrapping rotates), here driven with a fresh Secret Key.
#[tokio::test]
async fn history_decryptable_after_secret_key_rotation() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, "correct horse battery staple").await;
    let id = create_login(&mut session, "GitHub", "orig").await;
    update_login(&mut session, &id, "edited").await;
    let hid = list_history(&session, &id).await.unwrap()[1]
        .history_id
        .clone()
        .unwrap();

    // Rotate the Secret Key, keeping the password: change_password with a fresh SK.
    let new_sk: [u8; SECRET_KEY_LEN] = [0xEE; SECRET_KEY_LEN];
    change_password(
        &mut session,
        Arc::clone(&h.kdf) as Arc<dyn KeyDerivationProvider>,
        Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>,
        Arc::clone(&h.biometric) as Arc<dyn BiometricAuthenticator>,
        ChangePasswordInput {
            new_password: Zeroizing::new("correct horse battery staple".into()),
            new_secret_key: Some(Zeroizing::new(new_sk)),
            secret_key_rotated: true,
        },
    )
    .await
    .unwrap();
    lock_vault(session).await.unwrap();

    // Re-unlock with the same password (the new SK comes from the keychain); history decrypts.
    let session2 = unlock(&h, "correct horse battery staple").await;
    assert_eq!(history_password(&session2, &id, &hid).await, "orig");
    lock_vault(session2).await.unwrap();
}

/// History survives a **recovery** (forgot-password) reset: recovery-unlock (Recovery Key + Secret
/// Key) → the forced new password (`change_password_after_recovery`, the same rewrap core) → the
/// pre-recovery snapshot still decrypts under the new credentials.
#[tokio::test]
async fn history_decryptable_after_recovery_reset() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, "correct horse battery staple").await;
    let id = create_login(&mut session, "GitHub", "orig").await;
    update_login(&mut session, &id, "edited").await;
    let hid = list_history(&session, &id).await.unwrap()[1]
        .history_id
        .clone()
        .unwrap();

    // Enrol a Recovery Key, then lock (the password is "forgotten").
    let display = enroll_recovery_key(
        &mut session,
        Arc::clone(&h.kdf) as Arc<dyn KeyDerivationProvider>,
        Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>,
    )
    .await
    .unwrap()
    .recovery_key_display;
    lock_vault(session).await.unwrap();

    // Recover with the two documents, then set the forced new password.
    let recovery_key = parse_recovery_key(&display).unwrap();
    let mut recovered = build_unlock(&h)
        .unlock_with_recovery_key(h.home.clone(), recovery_key, Zeroizing::new(h.secret_key))
        .await
        .unwrap();
    change_password_after_recovery(
        &mut recovered,
        Arc::clone(&h.kdf) as Arc<dyn KeyDerivationProvider>,
        Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>,
        Arc::clone(&h.biometric) as Arc<dyn BiometricAuthenticator>,
        Zeroizing::new("new-master".into()),
    )
    .await
    .unwrap();
    lock_vault(recovered).await.unwrap();

    // The new password opens the vault; the pre-recovery history still decrypts.
    let session2 = unlock(&h, "new-master").await;
    assert_eq!(history_password(&session2, &id, &hid).await, "orig");
    lock_vault(session2).await.unwrap();
}

#[tokio::test]
async fn hard_delete_purges_history() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, "correct horse battery staple").await;
    let id = create_login(&mut session, "GitHub", "p1").await;
    update_login(&mut session, &id, "p2").await;

    assert_eq!(h.repo.list_history(&id).await.unwrap().len(), 1);

    hard_delete_entry(&mut session, &id).await.unwrap();

    assert!(
        h.repo.list_history(&id).await.unwrap().is_empty(),
        "history cascaded on hard delete"
    );
}
