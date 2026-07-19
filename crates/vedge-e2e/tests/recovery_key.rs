//! Recovery Key UI e2e (slice 5.7; the enrol row moved to a Credentials tab in 5.8): enrol a
//! Recovery Key from Settings ▸ Credentials (the
//! `RK1-` key is shown once with the "keep it separate" copy), and recover a forgotten
//! master password from the launch screen (two documents → forced new password → in the
//! vault, recovery now off).
//!
//! `#[ignore]` by default; run via `mise e2e`.

mod common;

use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::{Value, json};
use thirtyfour::By;

use common::*;
use vedge_e2e::{MASTER_PASSWORD, Session, TestEnv, app_binary};

/// A strong new master password (≥ the onboarding strength floor), distinct from the harness
/// `MASTER_PASSWORD`. A leetspeak passphrase — a test fixture, not a real secret.
const NEW_PW: &str = "R3covered-Horse-Battery-Staple!";

/// Enrol recovery through Settings ▸ Credentials, returning the shown-once `RK1-` display.
///
/// Asserts the enroll dialog carries the "keep it separate from your Emergency Kit" copy, and
/// that phase 2 renders a genuine `RK1-` key.
async fn enrol_recovery(session: &Session) -> Result<String> {
    session
        .click_testid("nav-settings")
        .await
        .context("open settings")?;
    // The credential rows moved out of Security into a dedicated Credentials tab in slice 5.8.
    session
        .click_button_text("Credentials")
        .await
        .context("open the Credentials tab")?;
    session
        .click_testid("recovery-setup")
        .await
        .context("open the recovery enroll dialog")?;

    // 🔴 The dialog must tell the user to keep the Recovery Kit separate from the Emergency Kit.
    let intro = dialog_text(session).await?;
    assert!(
        intro.contains("SEPARATE") || intro.contains("separate"),
        "the enroll dialog must say to keep the two kits separate; got: {intro}"
    );

    session
        .fill_id("recovery-current-password", MASTER_PASSWORD)
        .await?;
    session
        .click_testid("recovery-setup-confirm")
        .await
        .context("confirm enroll with the current password")?;

    // Phase 2: the save-kit button + a genuine `RK1-` key.
    session
        .wait_for(
            By::Css("[data-testid='recovery-save-kit']"),
            Duration::from_secs(30),
        )
        .await
        .context("phase 2: the save-Recovery-Kit button")?;
    let rk = wait_for_map(Duration::from_secs(10), || async {
        let text = dialog_text(session).await?;
        Ok(text
            .split_whitespace()
            .find(|w| w.starts_with("RK1-"))
            .map(str::to_owned))
    })
    .await
    .context("phase 2: a RK1- Recovery Key rendered")?;

    // Close phase 2.
    session
        .click_testid("recovery-done")
        .await
        .context("close the enroll dialog")?;
    Ok(rk)
}

/// 5.7 — enrolling prints a separate Recovery Kit: the `RK1-` key is shown once, with the
/// keep-it-separate copy, and it is distinct from the vault's `A3-` Secret Key.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn enrolling_recovery_prints_a_separate_kit() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;

    let rk = enrol_recovery(&session).await?;
    assert!(
        rk.starts_with("RK1-"),
        "expected a RK1- Recovery Key; got {rk}"
    );

    // The Recovery Key must be a DIFFERENT document from the Emergency Kit's Secret Key.
    let kit = session
        .invoke("export_emergency_kit", json!({ "vault_path": vault }))
        .await
        .context("export_emergency_kit")?;
    let secret_key = kit
        .get("secret_key_display")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert_ne!(
        rk, secret_key,
        "the Recovery Key and the Secret Key must be distinct documents"
    );

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}

/// 5.7 — recover a forgotten master password through the UI: enrol → lock → on the launch
/// screen, "Forgot your password?" → the two documents (RK1- + A3-) → a forced new password →
/// in the vault. Recovery is then OFF (the forced change nulled the slot).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn recover_a_forgotten_password_via_ui() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;

    // The Secret Key (from the Emergency Kit) is the second document the recovery flow needs.
    let kit = session
        .invoke("export_emergency_kit", json!({ "vault_path": vault }))
        .await
        .context("export_emergency_kit")?;
    let secret_key = kit
        .get("secret_key_display")
        .and_then(Value::as_str)
        .context("secret_key_display")?
        .to_owned();

    // Enrol, capturing the shown-once Recovery Key.
    let rk = enrol_recovery(&session).await?;

    // Lock, then "forget" the password: recover from the launch screen.
    session.click_testid("vault-lock").await.context("lock")?;
    assert_unlocked(&session, &vault, false).await?;

    click_vault_option(&session).await?;
    session
        .wait_for(By::Id("master-password"), Duration::from_secs(10))
        .await
        .context("unlock panel after relock")?;
    session
        .click_testid("forgot-password")
        .await
        .context("open the recovery panel")?;
    session
        .wait_for(By::Id("recovery-key"), Duration::from_secs(10))
        .await
        .context("the recovery-key input")?;
    session.fill_id("recovery-key", &rk).await?;
    session.fill_id("recovery-secret-key", &secret_key).await?;
    session
        .click_testid("recovery-submit")
        .await
        .context("submit the two documents")?;

    // The forced new-password dialog appears; set the new password.
    session
        .wait_for(By::Id("recovery-new-password"), Duration::from_secs(30))
        .await
        .context("the forced set-new-password dialog")?;
    session.fill_id("recovery-new-password", NEW_PW).await?;
    session.fill_id("recovery-confirm-password", NEW_PW).await?;
    session
        .click_testid("recovery-reset-submit")
        .await
        .context("set the new master password")?;

    // We land inside the vault (recovery unlocked it, the new password is set).
    assert_unlocked(&session, &vault, true).await?;

    // Recovery is now OFF — the forced change nulled the slot (③). Enrolling again would be
    // required to turn it back on.
    let enrolled = session
        .invoke("recovery_key_enrolled", json!({ "vault_path": vault }))
        .await
        .context("recovery_key_enrolled after recovery")?;
    assert_eq!(
        enrolled,
        Value::Bool(false),
        "a recovery unlock ends with the slot deleted"
    );

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}
