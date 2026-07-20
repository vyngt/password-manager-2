#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::needless_pass_by_value
)]

mod common;

use std::time::Duration;

use common::{Harness, build_unlock};
use secrecy::SecretString;

use vedge_core::TotpParams;
use vedge_core::application::vault::ports::VaultRepository;
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::{
    CopyEnvVarsInput, CopyFieldInput, CreateEntryInput, FieldSelector, GetEntryInput,
    RevealEnvVarsInput, RevealFieldInput, RevealRecoveryCodesInput, UnlockVaultInput,
    copy_env_vars, copy_field, create_entry, get_entry, move_entry, reveal_env_vars, reveal_field,
    reveal_recovery_codes, set_favorite, set_sort_order, set_tags,
};
use vedge_core::domain::shared::{EntryId, TagId};
use vedge_core::domain::vault::entities::AuditAction;
use vedge_core::domain::vault::env_export::EnvExportFormat;
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::domain::vault::payloads::{
    CommonMeta, EntryPayload, EntryType, EnvVar, EnvVarsPayload, FolderPayload, LoginPayload,
    NotePayload, SshKeyPayload,
};

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

fn login(name: &str, pw: &str, totp: Option<&str>) -> EntryPayload {
    EntryPayload::Login(LoginPayload {
        meta: CommonMeta::new(name, EntryType::Login),
        username: "alice".into(),
        password: SecretString::from(pw),
        totp_secret: totp.map(|s| SecretString::from(s.to_owned())),
        totp_params: TotpParams::default(),
        recovery_codes: vec![],
    })
}

#[tokio::test]
async fn copy_password_puts_value_on_clipboard() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("gh", "hunter2", None),
        },
    )
    .await
    .unwrap()
    .entry_id;

    let mut input = CopyFieldInput::new(id, FieldSelector::Password);
    input.clear_after_secs = 9999; // we don't want the clear to fire during the test
    copy_field(&mut session, input, 0).await.unwrap();

    assert_eq!(h.clipboard.peek().as_deref(), Some("hunter2"));
}

#[tokio::test]
async fn copy_wrong_field_for_type_errors() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: EntryPayload::Note(NotePayload {
                meta: CommonMeta::new("note", EntryType::Note),
                content: SecretString::from("body"),
            }),
        },
    )
    .await
    .unwrap()
    .entry_id;

    let mut input = CopyFieldInput::new(id, FieldSelector::Password);
    input.clear_after_secs = 9999;
    let err = copy_field(&mut session, input, 0).await.unwrap_err();
    assert!(matches!(err, VaultError::FieldNotApplicable));
}

#[tokio::test]
async fn copy_totp_produces_six_digits() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    // RFC 6238 test-vector secret ("12345678901234567890", 20 bytes / 160 bits)
    // encoded as base32 — totp-rs rejects secrets under 128 bits.
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("gh", "pw", Some("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ")),
        },
    )
    .await
    .unwrap()
    .entry_id;

    let mut input = CopyFieldInput::new(id, FieldSelector::TotpCode);
    input.clear_after_secs = 9999;
    copy_field(&mut session, input, 0).await.unwrap();

    let code = h.clipboard.peek().unwrap();
    assert_eq!(code.len(), 6);
    assert!(code.chars().all(|c| c.is_ascii_digit()));
}

#[tokio::test]
async fn copy_totp_without_secret_errors() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("gh", "pw", None),
        },
    )
    .await
    .unwrap()
    .entry_id;

    let mut input = CopyFieldInput::new(id, FieldSelector::TotpCode);
    input.clear_after_secs = 9999;
    let err = copy_field(&mut session, input, 0).await.unwrap_err();
    assert!(matches!(err, VaultError::FieldNotApplicable));
}

#[tokio::test]
async fn clipboard_clears_after_delay() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("gh", "hunter2", None),
        },
    )
    .await
    .unwrap()
    .entry_id;

    copy_field(
        &mut session,
        CopyFieldInput {
            entry_id: id,
            field: FieldSelector::Password,
            clear_after_secs: 1,
        },
        0,
    )
    .await
    .unwrap();
    assert_eq!(h.clipboard.peek().as_deref(), Some("hunter2"));

    // Wait long enough for the spawned task to fire. The test runtime is
    // unpaused; we sleep a bit beyond the configured clear delay.
    tokio::time::sleep(Duration::from_millis(1250)).await;
    assert_eq!(h.clipboard.peek(), None);
}

// ---- reveal_field (slice 5.4) — the audited twin of copy_field ----

#[tokio::test]
async fn reveal_password_returns_the_value() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("gh", "hunter2", None),
        },
    )
    .await
    .unwrap()
    .entry_id;

    let value = reveal_field(
        &mut session,
        RevealFieldInput::new(id, FieldSelector::Password),
        0,
    )
    .await
    .unwrap();
    assert_eq!(value.as_str(), "hunter2");
    // reveal RETURNS the value; it does not touch the clipboard (unlike copy_field).
    assert_eq!(h.clipboard.peek(), None);
}

#[tokio::test]
async fn reveal_wrong_field_for_type_errors() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: EntryPayload::Note(NotePayload {
                meta: CommonMeta::new("note", EntryType::Note),
                content: SecretString::from("body"),
            }),
        },
    )
    .await
    .unwrap()
    .entry_id;

    let err = reveal_field(
        &mut session,
        RevealFieldInput::new(id, FieldSelector::Password),
        0,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, VaultError::FieldNotApplicable));
}

#[tokio::test]
async fn reveal_field_audits_secret_revealed_every_call_no_dedup() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("gh", "hunter2", None),
        },
    )
    .await
    .unwrap()
    .entry_id;

    // Two reveals of the same field in one session → TWO audit rows. Unlike
    // TotpRevealed (per-session dedup), every field reveal is a deliberate act.
    reveal_field(
        &mut session,
        RevealFieldInput::new(id.clone(), FieldSelector::Password),
        0,
    )
    .await
    .unwrap();
    reveal_field(
        &mut session,
        RevealFieldInput::new(id.clone(), FieldSelector::Password),
        0,
    )
    .await
    .unwrap();

    let events = h.repo.recent_audit(50).await.unwrap();
    let revealed = events
        .iter()
        .filter(|e| e.action == AuditAction::SecretRevealed && e.entry_id.as_ref() == Some(&id))
        .count();
    assert_eq!(revealed, 2, "each reveal audits SecretRevealed, no dedup");
}

#[tokio::test]
async fn audit_distinguishes_browse_from_extraction() {
    // Slice 5.4 ③: get_entry (browse) → Viewed; copy_field / reveal_field
    // (extraction) → SecretRevealed. Before 5.4 both said Viewed.
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("gh", "hunter2", None),
        },
    )
    .await
    .unwrap()
    .entry_id;

    get_entry(
        &mut session,
        GetEntryInput {
            entry_id: id.clone(),
        },
    )
    .await
    .unwrap();
    let mut cp = CopyFieldInput::new(id.clone(), FieldSelector::Password);
    cp.clear_after_secs = 9999;
    copy_field(&mut session, cp, 0).await.unwrap();
    reveal_field(
        &mut session,
        RevealFieldInput::new(id.clone(), FieldSelector::Password),
        0,
    )
    .await
    .unwrap();

    let events = h.repo.recent_audit(50).await.unwrap();
    let count = |a: AuditAction| {
        events
            .iter()
            .filter(|e| e.action == a && e.entry_id.as_ref() == Some(&id))
            .count()
    };
    assert_eq!(count(AuditAction::Viewed), 1, "get_entry browses → Viewed");
    assert_eq!(
        count(AuditAction::SecretRevealed),
        2,
        "copy_field + reveal_field extract → SecretRevealed"
    );
}

// ---- 5.4.1: complete the reveal door — new arms + set copy/reveal ----

fn login_with_codes(name: &str, pw: &str, codes: &[&str]) -> EntryPayload {
    EntryPayload::Login(LoginPayload {
        meta: CommonMeta::new(name, EntryType::Login),
        username: "alice".into(),
        password: SecretString::from(pw),
        totp_secret: None,
        totp_params: TotpParams::default(),
        recovery_codes: codes.iter().map(|c| SecretString::from(*c)).collect(),
    })
}

fn envvars(name: &str, pairs: &[(&str, &str)]) -> EntryPayload {
    EntryPayload::EnvVars(EnvVarsPayload {
        meta: CommonMeta::new(name, EntryType::EnvVars),
        vars: pairs
            .iter()
            .map(|(k, v)| EnvVar {
                key: (*k).to_owned(),
                value: SecretString::from(*v),
            })
            .collect(),
    })
}

async fn seed(session: &mut VaultSession, payload: EntryPayload) -> EntryId {
    create_entry(session, CreateEntryInput { payload })
        .await
        .unwrap()
        .entry_id
}

fn count_revealed(
    events: &[vedge_core::domain::vault::entities::AuditEvent],
    id: &EntryId,
) -> usize {
    events
        .iter()
        .filter(|e| e.action == AuditAction::SecretRevealed && e.entry_id.as_ref() == Some(id))
        .count()
}

/// 🔴 ② — the whole recovery-code list crosses in one call, audited as ONE row.
#[tokio::test]
async fn reveal_recovery_codes_returns_all_and_audits_one_row() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = seed(
        &mut session,
        login_with_codes("gh", "hunter2", &["rc-0", "rc-1", "rc-2"]),
    )
    .await;

    let codes = reveal_recovery_codes(&mut session, RevealRecoveryCodesInput::new(id.clone()))
        .await
        .unwrap();
    let plain: Vec<&str> = codes.iter().map(|c| c.as_str()).collect();
    assert_eq!(plain, vec!["rc-0", "rc-1", "rc-2"]);

    let events = h.repo.recent_audit(50).await.unwrap();
    assert_eq!(
        count_revealed(&events, &id),
        1,
        "revealing the whole list audits exactly ONE row, not N"
    );
}

#[tokio::test]
async fn reveal_recovery_codes_on_non_login_errors() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = seed(
        &mut session,
        EntryPayload::Note(NotePayload {
            meta: CommonMeta::new("n", EntryType::Note),
            content: SecretString::from("body"),
        }),
    )
    .await;
    let err = reveal_recovery_codes(&mut session, RevealRecoveryCodesInput::new(id))
        .await
        .unwrap_err();
    assert!(matches!(err, VaultError::FieldNotApplicable));
}

/// 🔴 The headline through a session: a sealed SSH private key is revealable,
/// byte-exact (newlines intact), and audited.
#[tokio::test]
async fn reveal_ssh_private_key_returns_pem_and_audits() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let pem = "-----BEGIN VEDGE TEST BLOCK-----\nl1\nl2\n-----END VEDGE TEST BLOCK-----";
    let id = seed(
        &mut session,
        EntryPayload::SshKey(SshKeyPayload {
            meta: CommonMeta::new("box", EntryType::SshKey),
            private_key_pem: SecretString::from(pem),
            passphrase: None,
            public_key: "pk".into(),
            fingerprint: "fp".into(),
            key_type: "ed25519".into(),
        }),
    )
    .await;

    let value = reveal_field(
        &mut session,
        RevealFieldInput::new(id.clone(), FieldSelector::PrivateKey),
        0,
    )
    .await
    .unwrap();
    assert_eq!(
        value.as_str(),
        pem,
        "PEM crosses byte-exact, newlines intact"
    );

    let events = h.repo.recent_audit(50).await.unwrap();
    assert_eq!(count_revealed(&events, &id), 1);
}

/// 🔴 ⑥ — copy the whole env set to the clipboard (`.env`), audited as ONE row.
#[tokio::test]
async fn copy_env_vars_dotenv_hits_clipboard_and_audits_one_row() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = seed(
        &mut session,
        envvars("env", &[("A", "one"), ("B", "t w o")]),
    )
    .await;

    copy_env_vars(
        &mut session,
        CopyEnvVarsInput {
            entry_id: id.clone(),
            format: EnvExportFormat::DotEnv,
            clear_after_secs: 9999,
        },
    )
    .await
    .unwrap();
    assert_eq!(h.clipboard.peek().as_deref(), Some("A='one'\nB='t w o'"));

    let events = h.repo.recent_audit(50).await.unwrap();
    assert_eq!(
        count_revealed(&events, &id),
        1,
        "copying the whole set audits exactly ONE row, not N"
    );
}

/// ⑥ — reveal returns the blob (JSON lossless); a `.env` newline errors, never
/// truncates; and reveal never touches the clipboard.
#[tokio::test]
async fn reveal_env_vars_json_returns_blob_and_dotenv_newline_errors() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = seed(&mut session, envvars("env", &[("CERT", "l1\nl2")])).await;

    let json = reveal_env_vars(
        &mut session,
        RevealEnvVarsInput {
            entry_id: id.clone(),
            format: EnvExportFormat::Json,
        },
    )
    .await
    .unwrap();
    assert_eq!(json.as_str(), r#"{"CERT":"l1\nl2"}"#);
    assert_eq!(
        h.clipboard.peek(),
        None,
        "reveal returns the blob; it does not touch the clipboard"
    );

    let err = reveal_env_vars(
        &mut session,
        RevealEnvVarsInput {
            entry_id: id,
            format: EnvExportFormat::DotEnv,
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(err, VaultError::EnvValueNotDotEnvSafe { .. }));
}

#[tokio::test]
async fn move_entry_updates_folder_and_bumps_version() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let folder_id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: EntryPayload::Folder(FolderPayload {
                meta: CommonMeta::new("Work", EntryType::Folder),
            }),
        },
    )
    .await
    .unwrap()
    .entry_id;

    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("gh", "pw", None),
        },
    )
    .await
    .unwrap()
    .entry_id;

    let v1 = h.repo.get_entry(&id).await.unwrap();
    assert_eq!(v1.version, 1);

    move_entry(&mut session, &id, Some(&folder_id))
        .await
        .unwrap();

    let v2 = h.repo.get_entry(&id).await.unwrap();
    assert_eq!(v2.version, 2);
    assert_ne!(v1.nonce, v2.nonce);
    assert_eq!(
        session.index().entries.get(&id).unwrap().folder_id.as_ref(),
        Some(&folder_id)
    );

    // The moved entry's folder_id resolves to a live Folder entry (invariant #10).
    h.assert_coherent().await;
}

#[tokio::test]
async fn move_entry_rejects_missing_folder() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("gh", "pw", None),
        },
    )
    .await
    .unwrap()
    .entry_id;

    let ghost = EntryId::new();
    let err = move_entry(&mut session, &id, Some(&ghost))
        .await
        .unwrap_err();
    assert!(matches!(err, VaultError::FolderNotFound(_)));
}

#[tokio::test]
async fn move_entry_to_root_clears_folder() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let folder_id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: EntryPayload::Folder(FolderPayload {
                meta: CommonMeta::new("Work", EntryType::Folder),
            }),
        },
    )
    .await
    .unwrap()
    .entry_id;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("gh", "pw", None),
        },
    )
    .await
    .unwrap()
    .entry_id;
    move_entry(&mut session, &id, Some(&folder_id))
        .await
        .unwrap();
    move_entry(&mut session, &id, None).await.unwrap();
    assert!(
        session
            .index()
            .entries
            .get(&id)
            .unwrap()
            .folder_id
            .is_none()
    );
}

#[tokio::test]
async fn set_favorite_roundtrips() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("gh", "pw", None),
        },
    )
    .await
    .unwrap()
    .entry_id;

    // Fresh entries default to not-favorite.
    assert!(!session.index().entries.get(&id).unwrap().is_favorite);

    set_favorite(&mut session, &id, true).await.unwrap();
    assert!(session.index().entries.get(&id).unwrap().is_favorite);

    // Idempotent flip back clears it.
    set_favorite(&mut session, &id, false).await.unwrap();
    assert!(!session.index().entries.get(&id).unwrap().is_favorite);

    h.assert_coherent().await;
}

#[tokio::test]
async fn set_favorite_bumps_version_and_nonce() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("gh", "pw", None),
        },
    )
    .await
    .unwrap()
    .entry_id;

    let v1 = h.repo.get_entry(&id).await.unwrap();
    assert_eq!(v1.version, 1);

    set_favorite(&mut session, &id, true).await.unwrap();

    let v2 = h.repo.get_entry(&id).await.unwrap();
    assert_eq!(v2.version, 2);
    assert_ne!(v1.nonce, v2.nonce);
}

#[tokio::test]
async fn set_sort_order_roundtrips_and_bumps_version() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("gh", "pw", None),
        },
    )
    .await
    .unwrap()
    .entry_id;

    // Fresh entries default to sort_order 0.
    assert_eq!(session.index().entries.get(&id).unwrap().sort_order, 0);
    let v1 = h.repo.get_entry(&id).await.unwrap();
    assert_eq!(v1.version, 1);

    set_sort_order(&mut session, &id, 3).await.unwrap();
    assert_eq!(session.index().entries.get(&id).unwrap().sort_order, 3);

    // Re-encrypt bumped the version + rotated the nonce.
    let v2 = h.repo.get_entry(&id).await.unwrap();
    assert_eq!(v2.version, 2);
    assert_ne!(v1.nonce, v2.nonce);

    h.assert_coherent().await;
}

#[tokio::test]
async fn set_favorite_rejects_unknown() {
    let h = Harness::fresh().await;
    // Seed before unlock so the rebuilt session index contains the entry —
    // the existence guard passes and we exercise the payload-level reject.
    let ghost = h.seed_unknown("mystery", "vault.futuretype").await;
    let mut session = unlock(&h).await;

    let err = set_favorite(&mut session, &ghost, true).await.unwrap_err();
    assert!(matches!(err, VaultError::UnsupportedEntryType(_)));
}

#[tokio::test]
async fn set_tags_roundtrips() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("gh", "pw", None),
        },
    )
    .await
    .unwrap()
    .entry_id;

    assert!(session.index().entries.get(&id).unwrap().tag_ids.is_empty());

    let t1 = TagId::new();
    let t2 = TagId::new();
    set_tags(&mut session, &id, vec![t1.clone(), t2.clone()])
        .await
        .unwrap();
    assert_eq!(
        session.index().entries.get(&id).unwrap().tag_ids,
        vec![t1, t2]
    );

    // Clearing works too.
    set_tags(&mut session, &id, vec![]).await.unwrap();
    assert!(session.index().entries.get(&id).unwrap().tag_ids.is_empty());

    // Asserted only after the CLEAR: the intermediate [t1, t2] were synthetic ids with no tag
    // rows (core set_tags does not validate existence), which invariant #10 would flag. The final
    // state has no tag refs, so it is coherent — and still exercises the set_tags use case.
    h.assert_coherent().await;
}

#[tokio::test]
async fn set_tags_rejects_unknown() {
    let h = Harness::fresh().await;
    let ghost = h.seed_unknown("mystery", "vault.futuretype").await;
    let mut session = unlock(&h).await;

    let err = set_tags(&mut session, &ghost, vec![TagId::new()])
        .await
        .unwrap_err();
    assert!(matches!(err, VaultError::UnsupportedEntryType(_)));
}
