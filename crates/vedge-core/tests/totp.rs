//! Integration tests for the 4.2 TOTP reveal + door (`reveal_totp`, the
//! `TotpUpdate` merge in `update_entry`, and the shared copy/reveal engine).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::needless_pass_by_value
)]

mod common;

use common::{Harness, build_unlock};
use secrecy::{ExposeSecret, SecretString};

use vedge_core::application::vault::ports::VaultRepository;
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::{
    CopyFieldInput, CreateEntryInput, FieldSelector, GetEntryInput, UnlockVaultInput,
    UpdateEntryInput, copy_field, create_entry, get_entry, reveal_totp, update_entry,
};
use vedge_core::domain::shared::EntryId;
use vedge_core::domain::vault::payloads::{CommonMeta, EntryPayload, EntryType, LoginPayload};
use vedge_core::{AuditAction, AuditQuery, TotpParams, TotpUpdate};

/// RFC 4648 Base32 test secret ("Hello!\xDE\xAD\xBE\xEF").
const SECRET: &str = "JBSWY3DPEHPK3PXP";

async fn unlock(h: &Harness) -> VaultSession {
    build_unlock(h)
        .execute(UnlockVaultInput {
            vault_path: h.vdb_path.clone(),
            master_password: h.master_password.clone(),
            secret_key: None,
        })
        .await
        .unwrap()
}

fn login(name: &str, totp: Option<&str>) -> EntryPayload {
    EntryPayload::Login(LoginPayload {
        meta: CommonMeta::new(name, EntryType::Login),
        username: "alice".into(),
        password: SecretString::from("pw"),
        totp_secret: totp.map(|s| SecretString::from(s.to_owned())),
        totp_params: TotpParams::default(),
        recovery_codes: vec![],
    })
}

async fn create(session: &mut VaultSession, payload: EntryPayload) -> EntryId {
    create_entry(session, CreateEntryInput { payload })
        .await
        .unwrap()
        .entry_id
}

async fn stored_totp(session: &mut VaultSession, id: &EntryId) -> Option<String> {
    let payload = get_entry(
        session,
        GetEntryInput {
            entry_id: id.clone(),
        },
    )
    .await
    .unwrap();
    let EntryPayload::Login(l) = payload else {
        panic!("expected Login");
    };
    l.totp_secret.map(|s| s.expose_secret().to_owned())
}

async fn count_totp_revealed(h: &Harness) -> u64 {
    h.repo
        .query_audit(&AuditQuery {
            actions: vec![AuditAction::TotpRevealed],
            limit: 100,
            ..Default::default()
        })
        .await
        .unwrap()
        .total
}

#[tokio::test]
async fn reveal_totp_audits_once_per_session() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create(&mut session, login("gh", Some(SECRET))).await;

    // Two reveals in one session → exactly one TotpRevealed row.
    reveal_totp(&mut session, &id, 1000).await.unwrap();
    reveal_totp(&mut session, &id, 1030).await.unwrap();
    assert_eq!(count_totp_revealed(&h).await, 1);

    // Lock (drop) + unlock + reveal → the set reset, so a second row.
    drop(session);
    let mut session2 = unlock(&h).await;
    reveal_totp(&mut session2, &id, 1060).await.unwrap();
    assert_eq!(count_totp_revealed(&h).await, 2);
}

#[tokio::test]
async fn reveal_totp_re_audits_after_update() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create(&mut session, login("gh", Some(SECRET))).await;

    reveal_totp(&mut session, &id, 1000).await.unwrap();
    assert_eq!(count_totp_revealed(&h).await, 1);

    // A content edit re-arms the audit within the same session (Unchanged keeps
    // the seed). The next reveal audits again.
    update_entry(
        &mut session,
        UpdateEntryInput {
            entry_id: id.clone(),
            payload: login("gh-renamed", None),
            totp: TotpUpdate::Unchanged,
        },
    )
    .await
    .unwrap();

    reveal_totp(&mut session, &id, 1030).await.unwrap();
    assert_eq!(count_totp_revealed(&h).await, 2);
}

#[tokio::test]
async fn reveal_totp_does_not_bump_accessed_at_on_refresh() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create(&mut session, login("gh", Some(SECRET))).await;

    let before = h.repo.get_entry(&id).await.unwrap().accessed_at;
    reveal_totp(&mut session, &id, 1000).await.unwrap();
    reveal_totp(&mut session, &id, 1030).await.unwrap();
    let after = h.repo.get_entry(&id).await.unwrap().accessed_at;

    // A code refresh is not an access — copy_field is the audited access.
    assert_eq!(before, after);
}

#[tokio::test]
async fn copy_and_reveal_share_engine() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create(&mut session, login("gh", Some(SECRET))).await;

    let now: u64 = 1_700_000_000;
    let revealed = reveal_totp(&mut session, &id, now).await.unwrap();

    let mut input = CopyFieldInput::new(id.clone(), FieldSelector::TotpCode);
    input.clear_after_secs = 9999;
    copy_field(&mut session, input, now).await.unwrap();
    let copied = h.clipboard.peek().unwrap();

    // Same `now` → identical code from both paths (one engine).
    assert_eq!(revealed.code.to_string(), copied);
    assert_eq!(copied.len(), 6);
}

#[tokio::test]
async fn reveal_totp_without_secret_errors() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create(&mut session, login("gh", None)).await;

    // `TotpCode` is intentionally not `Debug`, so match the `Result` directly.
    let result = reveal_totp(&mut session, &id, 1000).await;
    assert!(matches!(
        result,
        Err(vedge_core::domain::vault::errors::VaultError::FieldNotApplicable)
    ));
    // No audit row for a no-op reveal.
    assert_eq!(count_totp_revealed(&h).await, 0);
}

#[tokio::test]
async fn update_entry_unchanged_preserves_totp() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create(&mut session, login("gh", Some(SECRET))).await;

    // Simulate an edit-form save: the DTO carries NO seed → payload has None +
    // the intent is Unchanged. The door must preserve the stored seed.
    update_entry(
        &mut session,
        UpdateEntryInput {
            entry_id: id.clone(),
            payload: login("gh-renamed", None),
            totp: TotpUpdate::Unchanged,
        },
    )
    .await
    .unwrap();

    assert_eq!(
        stored_totp(&mut session, &id).await.as_deref(),
        Some(SECRET)
    );
}

#[tokio::test]
async fn update_entry_set_and_clear_totp() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create(&mut session, login("gh", None)).await;
    assert_eq!(stored_totp(&mut session, &id).await, None);

    // Set enrols.
    update_entry(
        &mut session,
        UpdateEntryInput {
            entry_id: id.clone(),
            payload: login("gh", None),
            totp: TotpUpdate::Set(SecretString::from(SECRET)),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        stored_totp(&mut session, &id).await.as_deref(),
        Some(SECRET)
    );

    // Clear removes.
    update_entry(
        &mut session,
        UpdateEntryInput {
            entry_id: id.clone(),
            payload: login("gh", None),
            totp: TotpUpdate::Clear,
        },
    )
    .await
    .unwrap();
    assert_eq!(stored_totp(&mut session, &id).await, None);
}
