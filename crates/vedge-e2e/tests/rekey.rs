//! Re-key UI e2e (slice 5.8): re-key the vault from Settings ▸ Credentials → the vault LOCKS → a
//! seeded entry survives and opens under the fresh DEKs after unlocking with the NEW password.
//!
//! DRIFT (documented): the deep completeness — a document + version history re-encrypted under new
//! DEKs, and old DEKs opening nothing — is proven exhaustively at the host level
//! (`vedge-core/tests/rekey_vault.rs`), and snapshot RETIREMENT at host + journal level
//! (`rekey_retires_the_snapshot_store` + the journal `retiring_swap_*` crash matrix). The UI has no
//! simple document/history/snapshot seeding path, so this scenario proves a seeded *entry* survives
//! re-key end-to-end through the UI plus the lock → re-unlock-with-the-new-password flow.
//! `rekey_retires_snapshots_via_ui` is DEFERRED for the same reason — the property is already
//! exhaustively covered and a UI revert-attempt would add fragile steps for no new signal.
//!
//! `#[ignore]` by default; run via `mise e2e`.

mod common;

use std::time::Duration;

use anyhow::{Context, Result};
use thirtyfour::By;

use common::*;
use vedge_e2e::{MASTER_PASSWORD, Session, TestEnv, app_binary};

/// A strong new master password (≥ the onboarding strength floor), distinct from the harness
/// `MASTER_PASSWORD`. A test fixture, not a real secret.
const NEW_PW: &str = "Rek3yed-Horse-Battery-Staple!";

/// 5.8 — re-key through the UI: a seeded entry survives the fresh-DEK re-encrypt, the vault LOCKS
/// on success, and it then opens with the NEW password (data intact under the re-keyed DEKs).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn rekey_vault_via_ui() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;

    // Seed an entry so we can prove data survives the O(total) fresh-DEK re-encrypt end-to-end.
    add_login(&session, "GitHub", "alice", "login-value-1").await?;
    wait_row_count(&session, 1, Duration::from_secs(10)).await?;

    // Settings ▸ Credentials tab (the credential ops moved out of Security in 5.8) → Re-key vault.
    session
        .click_testid("nav-settings")
        .await
        .context("open settings")?;
    session
        .driver()
        .find(By::Css("[id$='-trigger-credentials']"))
        .await
        .context("find the Credentials tab")?
        .click()
        .await
        .context("switch to the Credentials tab")?;
    session
        .click_testid("rekey-vault")
        .await
        .context("open the re-key dialog")?;

    // Fill the current + new + confirm passwords, then re-key.
    session
        .fill_id("rekey-current-password", MASTER_PASSWORD)
        .await?;
    session.fill_id("rekey-new-password", NEW_PW).await?;
    session.fill_id("rekey-confirm-password", NEW_PW).await?;
    session
        .click_testid("rekey-submit")
        .await
        .context("submit the re-key")?;

    // On success the backend LOCKS the vault and the UI navigates to the launch screen; poll for
    // the locked state (assert_unlocked waits up to 25 s — plenty for Argon2 + re-encrypt + swap).
    assert_unlocked(&session, &vault, false)
        .await
        .context("re-key must LOCK the vault on success")?;

    // Unlock with the NEW password.
    click_vault_option(&session).await?;
    session
        .wait_for(By::Id("master-password"), Duration::from_secs(10))
        .await
        .context("unlock panel after re-key")?;
    session.fill_id("master-password", NEW_PW).await?;
    session
        .click_testid("unlock-submit")
        .await
        .context("unlock with the new password")?;
    assert_unlocked(&session, &vault, true).await?;

    // The seeded entry survived the re-key (it opens under the fresh DEKs).
    let entries = list_entries(&session, &vault).await?;
    assert!(
        entry_id(&entries, "GitHub").is_some(),
        "the seeded entry must survive the re-key"
    );

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}

/// 5.8 — re-key WITH Secret-Key rotation: the show-once new key must appear on the LAUNCH screen
/// (re-key locks the vault, so it can't be shown on the settings dialog — the auto-lock watcher
/// would eject it within ~1 s). This is the regression guard for that bug: the new-key dialog must
/// be present AFTER the re-key navigates to launch.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn rekey_rotate_sk_shows_new_key_on_launch_via_ui() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;
    add_login(&session, "GitHub", "alice", "login-value-1").await?;
    wait_row_count(&session, 1, Duration::from_secs(10)).await?;

    // Settings ▸ Credentials tab → Re-key vault.
    session
        .click_testid("nav-settings")
        .await
        .context("open settings")?;
    session
        .driver()
        .find(By::Css("[id$='-trigger-credentials']"))
        .await
        .context("find the Credentials tab")?
        .click()
        .await
        .context("switch to the Credentials tab")?;
    session
        .click_testid("rekey-vault")
        .await
        .context("open the re-key dialog")?;

    session
        .fill_id("rekey-current-password", MASTER_PASSWORD)
        .await?;
    session.fill_id("rekey-new-password", NEW_PW).await?;
    session.fill_id("rekey-confirm-password", NEW_PW).await?;
    // Toggle "also rotate my Secret Key" — the dialog's only role=switch.
    session
        .driver()
        .find(By::Css("div[role='dialog'] [role='switch']"))
        .await
        .context("find the rotate-Secret-Key toggle")?
        .click()
        .await
        .context("toggle rotate-Secret-Key")?;
    session
        .click_testid("rekey-submit")
        .await
        .context("submit the re-key")?;

    // 🔴 THE FIX: the show-once new key appears on the LAUNCH screen (the vault locked + navigated),
    // NOT ejected by the auto-lock. Waiting for its dismiss button proves it is shown; before the
    // fix this timed out (the dialog was cut off by the auto-lock's ~1 s poll).
    session
        .wait_for(
            By::Css("[data-testid='rekey-new-kit-done']"),
            Duration::from_secs(30),
        )
        .await
        .context("the new Secret Key must be shown on the launch screen after a rotating re-key")?;
    session
        .click_testid("rekey-new-kit-done")
        .await
        .context("dismiss the new-key panel")?;

    // Unlock with the NEW password (the rotated Secret Key is read from the keychain).
    click_vault_option(&session).await?;
    session
        .wait_for(By::Id("master-password"), Duration::from_secs(10))
        .await
        .context("unlock panel after re-key")?;
    session.fill_id("master-password", NEW_PW).await?;
    session
        .click_testid("unlock-submit")
        .await
        .context("unlock with the new password")?;
    assert_unlocked(&session, &vault, true).await?;

    let entries = list_entries(&session, &vault).await?;
    assert!(
        entry_id(&entries, "GitHub").is_some(),
        "the seeded entry must survive the rotating re-key"
    );

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}
