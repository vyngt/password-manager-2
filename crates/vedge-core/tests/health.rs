//! Integration tests for the 4.3 password-health scan: the audit-silence
//! contract, the no-secret/no-digest invariant, the split weak/reuse taxonomy,
//! and the `secret_changed_at` write-path (create / carry-forward / bump).

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
use secrecy::SecretString;

use vedge_core::application::vault::ports::{CryptoProvider, VaultRepository};
use vedge_core::application::vault::session::VaultSession;
use vedge_core::domain::shared::{EntryId, Timestamp, now};
use vedge_core::domain::vault::aad::entry_aad;
use vedge_core::domain::vault::entities::EntryRow;
use vedge_core::domain::vault::payloads::{
    ApiKeyPayload, CardPayload, CommonMeta, EntryPayload, EntryType, LoginPayload,
};
use vedge_core::domain::vault::totp::TotpUpdate;
use vedge_core::{
    AgeConfidence, AuditAction, AuditQuery, CreateEntryInput, FindingKind, GetEntryInput,
    HealthScanInput, TotpParams, UnlockVaultInput, UpdateEntryInput, create_entry, get_entry,
    scan_health, set_favorite, soft_delete_entry, update_entry,
};

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

fn login(name: &str, username: &str, password: &str) -> EntryPayload {
    let mut meta = CommonMeta::new(name, EntryType::Login);
    meta.url = Some("https://example.com".into());
    EntryPayload::Login(LoginPayload {
        meta,
        username: username.to_owned(),
        password: SecretString::from(password),
        totp_secret: None,
        totp_params: TotpParams::default(),
        recovery_codes: vec![],
    })
}

fn card(name: &str, number: &str, cvv: &str, pin: Option<&str>) -> EntryPayload {
    EntryPayload::Card(CardPayload {
        meta: CommonMeta::new(name, EntryType::Card),
        cardholder_name: "A B".to_owned(),
        number: SecretString::from(number),
        expiry_month: 12,
        expiry_year: 2030,
        cvv: SecretString::from(cvv),
        pin: pin.map(SecretString::from),
    })
}

fn api_key(name: &str, key: &str) -> EntryPayload {
    EntryPayload::ApiKey(ApiKeyPayload {
        meta: CommonMeta::new(name, EntryType::ApiKey),
        key: SecretString::from(key),
        secret: None,
        endpoint: None,
        expiry: None,
        key_type: None,
    })
}

async fn create(session: &mut VaultSession, payload: EntryPayload) -> EntryId {
    create_entry(session, CreateEntryInput { payload })
        .await
        .unwrap()
        .entry_id
}

async fn count_action(h: &Harness, action: AuditAction) -> u64 {
    h.repo
        .query_audit(&AuditQuery {
            actions: vec![action],
            limit: 1000,
            ..Default::default()
        })
        .await
        .unwrap()
        .total
}

async fn stamp_of(session: &mut VaultSession, id: &EntryId) -> Option<Timestamp> {
    let payload = get_entry(
        session,
        GetEntryInput {
            entry_id: id.clone(),
        },
    )
    .await
    .unwrap();
    payload.meta().secret_changed_at
}

/// Encrypt+insert a Login directly (bypassing the use case) with a chosen
/// `created_at` and `secret_changed_at` — for age tests that need back-dating.
async fn seed_login_aged(
    h: &Harness,
    name: &str,
    password: &str,
    created_at: Timestamp,
    secret_changed_at: Option<Timestamp>,
) -> EntryId {
    let id = EntryId::new();
    let version: i64 = 1;
    let mut meta = CommonMeta::new(name, EntryType::Login);
    meta.secret_changed_at = secret_changed_at;
    let payload = EntryPayload::Login(LoginPayload {
        meta,
        username: "u".to_owned(),
        password: SecretString::from(password),
        totp_secret: None,
        totp_params: TotpParams::default(),
        recovery_codes: vec![],
    });
    let bytes = payload.to_encryptable_json().unwrap();
    let dek = h.crypto.generate_dek();
    let aad = entry_aad(&id, version).unwrap();
    let (nonce, ciphertext) = h.crypto.encrypt_entry(&dek, &bytes, &aad).unwrap();
    let dek_wrapped = h.crypto.wrap_dek(&dek, &h.kek).unwrap();
    let row = EntryRow {
        id: id.clone(),
        version,
        cipher_suite: 1,
        dek_wrapped,
        nonce,
        ciphertext,
        created_at,
        updated_at: created_at,
        accessed_at: None,
        is_trashed: false,
        trashed_at: None,
    };
    h.repo.insert_entry(&row).await.unwrap();
    id
}

#[tokio::test]
async fn scan_health_is_audit_silent() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let a = create(&mut session, login("GitHub", "alice", "hunter2")).await;
    let b = create(
        &mut session,
        card("Visa", "4111111111111111", "123", Some("4321")),
    )
    .await;

    assert_eq!(count_action(&h, AuditAction::Viewed).await, 0);

    scan_health(&session, HealthScanInput::default())
        .await
        .unwrap();
    scan_health(&session, HealthScanInput::default())
        .await
        .unwrap();

    // Two scans → two HealthScanned rows, and ZERO Viewed rows.
    assert_eq!(count_action(&h, AuditAction::HealthScanned).await, 2);
    assert_eq!(count_action(&h, AuditAction::Viewed).await, 0);
    // accessed_at untouched (create leaves it None).
    assert_eq!(h.repo.get_entry(&a).await.unwrap().accessed_at, None);
    assert_eq!(h.repo.get_entry(&b).await.unwrap().accessed_at, None);
}

#[tokio::test]
async fn scan_health_detects_reuse_across_types() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    create(&mut session, login("GitHub", "alice", "SharedSecret_9x")).await;
    create(&mut session, api_key("AWS", "SharedSecret_9x")).await;

    let report = scan_health(&session, HealthScanInput::default())
        .await
        .unwrap();

    let reused: Vec<_> = report
        .findings
        .iter()
        .filter_map(|f| match f.kind {
            FindingKind::Reused { group, count } => Some((group, count)),
            _ => None,
        })
        .collect();
    assert_eq!(reused.len(), 2, "one reuse finding per sharing field");
    assert!(reused.iter().all(|(_, count)| *count == 2));
    assert_eq!(reused[0].0, reused[1].0, "same group id");
    assert_eq!(report.summary.reused, 2);
}

#[tokio::test]
async fn reuse_ignores_intra_entry_duplicate() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    // One entry whose password equals one of its own recovery codes: the same
    // value in two fields of ONE entry is not cross-entry reuse.
    let payload = EntryPayload::Login(LoginPayload {
        meta: CommonMeta::new("self", EntryType::Login),
        username: "u".to_owned(),
        password: SecretString::from("dup-secret-value-x9"),
        totp_secret: None,
        totp_params: TotpParams::default(),
        recovery_codes: vec![SecretString::from("dup-secret-value-x9")],
    });
    create(&mut session, payload).await;

    let report = scan_health(&session, HealthScanInput::default())
        .await
        .unwrap();
    assert_eq!(
        report.summary.reused, 0,
        "intra-entry duplicate is not reuse"
    );
}

#[tokio::test]
async fn reuse_never_emits_digest() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    create(&mut session, login("GitHub", "alice", "topsecretpassword")).await;
    create(&mut session, api_key("AWS", "topsecretpassword")).await;

    let report = scan_health(&session, HealthScanInput::default())
        .await
        .unwrap();
    let dbg = format!("{report:?}");
    assert!(
        !dbg.contains("topsecretpassword"),
        "no secret in the report Debug output"
    );
}

#[tokio::test]
async fn pin_and_cvv_exempt_from_weak_but_in_reuse() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    // A 4-digit PIN, and a Login whose password IS that same PIN.
    create(
        &mut session,
        card("Visa", "4111111111111111", "999", Some("1234")),
    )
    .await;
    create(&mut session, login("Weak", "bob", "1234")).await;

    let report = scan_health(&session, HealthScanInput::default())
        .await
        .unwrap();

    // No Weak finding for the exempt-by-design CardPin.
    let weak_on_pin = report.findings.iter().any(|f| {
        matches!(f.field, Some(vedge_core::SecretField::CardPin))
            && matches!(f.kind, FindingKind::Weak { .. })
    });
    assert!(!weak_on_pin, "CardPin must be exempt from weak-scoring");

    // But the Login password "1234" IS weak-scored.
    let weak_on_login = report.findings.iter().any(|f| {
        matches!(f.field, Some(vedge_core::SecretField::LoginPassword))
            && matches!(f.kind, FindingKind::Weak { .. })
    });
    assert!(weak_on_login, "LoginPassword '1234' should be weak");

    // And "1234" is reused across the pin and the password → a reuse group.
    assert_eq!(report.summary.reused, 2, "pin+password reuse group");
}

#[tokio::test]
async fn weak_uses_entry_name_as_user_input() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    create(&mut session, login("GitHub", "alice", "github2024")).await;

    let report = scan_health(&session, HealthScanInput::default())
        .await
        .unwrap();
    assert!(
        report.summary.weak >= 1,
        "github2024 is weak for a GitHub entry"
    );
}

#[tokio::test]
async fn scan_health_skips_unknown_payload() {
    let h = Harness::fresh().await;
    let uid = h.seed_unknown("mystery", "FutureType").await;
    let session = unlock(&h).await;

    let report = scan_health(&session, HealthScanInput::default())
        .await
        .unwrap();
    assert!(
        report
            .skipped
            .iter()
            .any(|s| s.entry_id == uid && s.reason == vedge_core::SkipReason::UnknownPayload),
        "unknown entry skipped-and-counted"
    );
    assert_eq!(report.entries_scanned, 0, "the scan still completes");
}

#[tokio::test]
async fn scan_health_skips_trashed() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create(&mut session, login("Weak", "bob", "1234")).await;
    soft_delete_entry(&mut session, &id).await.unwrap();

    let report = scan_health(&session, HealthScanInput::default())
        .await
        .unwrap();
    assert_eq!(report.entries_scanned, 0, "trashed entries are not scanned");
    assert_eq!(report.summary.weak, 0);
}

#[tokio::test]
async fn secret_changed_at_set_on_create() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create(&mut session, login("GitHub", "alice", "hunter2")).await;
    assert!(stamp_of(&mut session, &id).await.is_some());
}

#[tokio::test]
async fn secret_changed_at_carried_forward_on_metadata_edit() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create(&mut session, login("GitHub", "alice", "hunter2")).await;
    let t0 = stamp_of(&mut session, &id).await.unwrap();

    // Favorite toggle — a metadata mutation — must NOT reset the stamp.
    set_favorite(&mut session, &id, true).await.unwrap();
    assert_eq!(stamp_of(&mut session, &id).await, Some(t0));

    // A content edit that keeps every secret identical (rename only) also carries
    // the stamp forward.
    update_entry(
        &mut session,
        UpdateEntryInput {
            entry_id: id.clone(),
            payload: login("GitHub-renamed", "alice", "hunter2"),
            totp: TotpUpdate::Unchanged,
        },
    )
    .await
    .unwrap();
    assert_eq!(stamp_of(&mut session, &id).await, Some(t0));
}

#[tokio::test]
async fn secret_changed_at_bumped_when_secret_differs() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;

    // Login password change bumps.
    let login_id = create(&mut session, login("GitHub", "alice", "hunter2")).await;
    let t0 = stamp_of(&mut session, &login_id).await.unwrap();
    update_entry(
        &mut session,
        UpdateEntryInput {
            entry_id: login_id.clone(),
            payload: login("GitHub", "alice", "a-different-password"),
            totp: TotpUpdate::Unchanged,
        },
    )
    .await
    .unwrap();
    let t1 = stamp_of(&mut session, &login_id).await.unwrap();
    assert!(t1 > t0, "login secret change should bump the stamp");

    // A Card CVV change bumps too — proving the decrypt hoist covers non-Login.
    let card_id = create(&mut session, card("Visa", "4111111111111111", "111", None)).await;
    let c0 = stamp_of(&mut session, &card_id).await.unwrap();
    update_entry(
        &mut session,
        UpdateEntryInput {
            entry_id: card_id.clone(),
            payload: card("Visa", "4111111111111111", "222", None),
            totp: TotpUpdate::Unchanged,
        },
    )
    .await
    .unwrap();
    let c1 = stamp_of(&mut session, &card_id).await.unwrap();
    assert!(
        c1 > c0,
        "card secret change should bump the stamp (non-Login hoist)"
    );
}

#[tokio::test]
async fn age_exact_from_stamp_else_estimated_from_created_at() {
    let h = Harness::fresh().await;
    let old = now() - chrono::Duration::days(400);

    // (1) Stamped 400 days ago → Old with Exact confidence.
    let exact = seed_login_aged(&h, "OldStamped", "pw1", now(), Some(old)).await;
    // (2) No stamp, created 400 days ago, no history → Old with Estimated (created_at).
    let estimated = seed_login_aged(&h, "OldUnstamped", "pw2", old, None).await;

    let session = unlock(&h).await;
    let report = scan_health(&session, HealthScanInput::default())
        .await
        .unwrap();

    let conf = |id: &EntryId| {
        report
            .findings
            .iter()
            .find_map(|f| match (&f.kind, f.entry_id == *id) {
                (FindingKind::Old { confidence, .. }, true) => Some(*confidence),
                _ => None,
            })
    };
    assert_eq!(conf(&exact), Some(AgeConfidence::Exact));
    assert_eq!(conf(&estimated), Some(AgeConfidence::Estimated));
    assert_eq!(report.summary.old, 2);
}
