//! Credential-lifecycle UI e2e (slice 5.6): change the master password from
//! Settings ▸ Security (wrong current password rejected → correct → relock →
//! unlock with the new password → data survives), and rotate the Secret Key
//! (the reversed "keep your old kit" copy → a genuinely new key is issued).
//!
//! DRIFT (documented): the "tagged vault survives" guarantee is proven at the
//! host level (`vedge-core/tests/tag_keys.rs`), not here — the UI has no simple
//! tag-creation path (tags are created via import or bulk-applied from an
//! existing set), and the DEK rewrap is identical with or without a tag. This
//! scenario proves a seeded *entry* survives the O(n) rewrap through the UI.
//!
//! `#[ignore]` by default; run via `mise e2e`.

mod common;

use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::{Value, json};
use thirtyfour::By;

use common::*;
use vedge_e2e::{MASTER_PASSWORD, Session, TestEnv, app_binary};

/// A strong new master password (≥ the onboarding strength floor), distinct from
/// the harness `MASTER_PASSWORD`. A leetspeak passphrase — a test fixture, not a
/// real secret.
const NEW_PW: &str = "Ch4nged-Horse-Battery-Staple!";

/// 5.6 ② — change the master password through the UI: a wrong current password
/// is rejected *before* the rewrap, the correct one succeeds and keeps the
/// session live, and the vault then opens with the NEW password (data intact).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn change_master_password_via_ui() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;

    // Seed an entry so we can prove data survives the O(n) DEK rewrap end-to-end.
    add_login(&session, "GitHub", "alice", "login-value-1").await?;
    wait_row_count(&session, 1, Duration::from_secs(10)).await?;

    // Settings ▸ Security (the default tab) → Change master password.
    session
        .click_testid("nav-settings")
        .await
        .context("open settings")?;
    session
        .click_testid("change-master-password")
        .await
        .context("open change-password dialog")?;

    // ---- Negative: a wrong CURRENT password is rejected before any rewrap ----
    session
        .fill_id("current-master-password", "not-the-password")
        .await?;
    session.fill_id("new-master-password", NEW_PW).await?;
    session
        .fill_id("new-master-password-confirm", NEW_PW)
        .await?;
    session
        .click_testid("change-password-submit")
        .await
        .context("submit (wrong current password)")?;
    wait_until(Duration::from_secs(20), || async {
        Ok(dialog_text(&session)
            .await?
            .contains("Wrong current password"))
    })
    .await
    .context("a wrong current password must be rejected in-dialog")?;

    // ---- Positive: the correct current password → the change succeeds ----
    // Only the current field is re-typed; the new/confirm fields persist.
    session
        .fill_id("current-master-password", MASTER_PASSWORD)
        .await?;
    session
        .click_testid("change-password-submit")
        .await
        .context("submit (correct current password)")?;
    // On success the dialog closes; poll for it to unmount.
    wait_until(Duration::from_secs(30), || async {
        Ok(session
            .driver()
            .find(By::Css("div[role='dialog']"))
            .await
            .is_err())
    })
    .await
    .context("the change-password dialog should close on success")?;

    // ---- Relock, then unlock with the NEW password ----
    session.click_testid("vault-lock").await.context("lock")?;
    assert_unlocked(&session, &vault, false).await?;

    click_vault_option(&session).await?;
    session
        .wait_for(By::Id("master-password"), Duration::from_secs(10))
        .await
        .context("unlock panel after relock")?;
    session.fill_id("master-password", NEW_PW).await?;
    session
        .click_testid("unlock-submit")
        .await
        .context("unlock with the new password")?;
    assert_unlocked(&session, &vault, true).await?;

    // The seeded entry survived the rewrap (the whole point of 5.6.0's safety).
    let entries = list_entries(&session, &vault).await?;
    assert!(
        entry_id(&entries, "GitHub").is_some(),
        "the seeded entry must survive the password change"
    );

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}

/// 5.6 ③ — rotate the Secret Key: the dialog reverses the "orphaned kit" copy
/// (tells the user to KEEP the old kit), and a rotation issues a genuinely NEW
/// Secret Key (≠ the old one) that becomes the one on file.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn rotate_secret_key_reissues_the_kit() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;

    // The ORIGINAL Secret Key (read-only; proves the rotation issues a new one).
    let kit = session
        .invoke("export_emergency_kit", json!({ "vault_path": vault }))
        .await
        .context("export_emergency_kit (before)")?;
    let old_key = kit
        .get("secret_key_display")
        .and_then(Value::as_str)
        .context("secret_key_display before rotation")?
        .to_owned();

    // Settings ▸ Security → Rotate Secret Key.
    session
        .click_testid("nav-settings")
        .await
        .context("open settings")?;
    session
        .click_testid("rotate-secret-key")
        .await
        .context("open rotate dialog")?;

    // 🔴 The dialog must tell the user to KEEP their old kit — never "orphaned".
    let confirm_text = dialog_text(&session).await?;
    assert!(
        confirm_text.contains("Keep your old Emergency Kit"),
        "the rotate dialog must say to keep the old kit; got: {confirm_text}"
    );

    // Confirm with the current password → phase 2 renders the new key.
    session
        .fill_id("rotate-master-password", MASTER_PASSWORD)
        .await?;
    session
        .click_testid("rotate-secret-key-confirm")
        .await
        .context("confirm the rotation")?;

    // Phase 2: the Save-kit button appears and a NEW A3- key is shown.
    session
        .wait_for(
            By::Css("[data-testid='rotate-save-kit']"),
            Duration::from_secs(30),
        )
        .await
        .context("phase 2: the re-issue-kit button")?;
    let new_key = wait_for_map(Duration::from_secs(10), || async {
        let text = dialog_text(&session).await?;
        Ok(text
            .split_whitespace()
            .find(|w| w.starts_with("A3-"))
            .map(str::to_owned))
    })
    .await
    .context("phase 2: a new A3- Secret Key rendered")?;
    assert_ne!(
        new_key, old_key,
        "the rotation must issue a NEW Secret Key, not re-show the old one"
    );

    // The re-issued key is the one now on file (the kit reads it back).
    let kit2 = session
        .invoke("export_emergency_kit", json!({ "vault_path": vault }))
        .await
        .context("export_emergency_kit (after)")?;
    let on_file = kit2
        .get("secret_key_display")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert_eq!(
        on_file, new_key,
        "the rotated Secret Key must be the one now stored"
    );

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}
