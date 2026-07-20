//! PG.1 — the `assert_vault_coherent` helper's own acceptance: a coherent, realistically-exercised
//! vault asserts CLEAN (no false positives). This is what makes the helper trustworthy enough to
//! wire into every other operation test. The HISTORICAL acceptance — re-introduce each of the three
//! Phase-5 bugs and watch the helper fail — is run on a scratch branch and recorded in the tracking
//! note (a helper that asserts nothing also passes).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use common::{Harness, build_unlock};
use secrecy::SecretString;

use vedge_core::application::vault::use_cases::{
    CreateEntryInput, ImportDocumentInput, UnlockVaultInput, UpdateEntryInput, create_entry,
    create_snapshot, import_document, soft_delete_entry, update_entry,
};
use vedge_core::domain::vault::payloads::{CommonMeta, EntryPayload, EntryType, LoginPayload};
use vedge_core::infrastructure::snapshot::manifest::SnapshotReason;

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

/// A freshly created, seeded, and exercised vault — entries with real version history, a tagged
/// entry, a document with an external blob, a trashed entry, and a snapshot — asserts coherent.
/// No false positives across all 13 invariants.
#[tokio::test]
async fn a_coherent_exercised_vault_passes_every_invariant() {
    let h = Harness::fresh().await;
    // A DEK-sealed tag the entry below will reference (invariants #5 + #10).
    let tag_id = h.seed_tag("work").await;
    // An unrecognized entry type → decodes to `EntryPayload::Unknown` (invariant #1 must still
    // decrypt + parse it, and #10 must still police its bare meta).
    h.seed_unknown("mystery", "FutureType").await;

    let unlock = build_unlock(&h);
    let mut session = unlock
        .execute(UnlockVaultInput {
            vault_path: h.home.clone(),
            master_password: h.master_password.clone(),
            secret_key: None,
        })
        .await
        .unwrap();

    // A login with a tag reference + real version history (v1 → v2 → v3).
    let mut meta = CommonMeta::new("github", EntryType::Login);
    meta.tag_ids = vec![tag_id.clone()];
    let login_id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: EntryPayload::Login(LoginPayload {
                meta,
                username: "alice".into(),
                password: SecretString::from("pw-v1"),
                totp_secret: None,
                totp_params: vedge_core::TotpParams::default(),
                recovery_codes: vec![],
            }),
        },
    )
    .await
    .unwrap()
    .entry_id;
    update_entry(
        &mut session,
        UpdateEntryInput::full(login_id.clone(), login("github", "pw-v2")),
    )
    .await
    .unwrap();
    update_entry(
        &mut session,
        UpdateEntryInput::full(login_id.clone(), login("github", "pw-v3")),
    )
    .await
    .unwrap();

    // A document with a real external blob (invariants #3 + #4).
    let doc_bytes: Vec<u8> = (0..2_000u32).flat_map(u32::to_le_bytes).collect();
    import_document(
        &mut session,
        ImportDocumentInput {
            filename: "passport.pdf".into(),
            mime_type: "application/pdf".into(),
            content: doc_bytes,
            meta: CommonMeta::new("passport", EntryType::Document),
        },
    )
    .await
    .unwrap();

    // A trashed entry — `all_entries()` includes it, so it must still decrypt (invariant #1).
    let temp_id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("temp", "throwaway"),
        },
    )
    .await
    .unwrap()
    .entry_id;
    soft_delete_entry(&mut session, &temp_id).await.unwrap();

    // A snapshot (invariants #8 + #9).
    create_snapshot(&session, SnapshotReason::Manual)
        .await
        .unwrap();

    // 🔴 The whole point: a fresh open of this exercised vault is internally consistent.
    h.assert_coherent().await;
}
