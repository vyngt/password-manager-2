//! Generator e2e scenarios (slices 3.3 / 3.5): the inline generate wand fills the
//! Login password field, and session-only generator history is wiped on lock.
//!
//! `#[ignore]` by default; run via `mise e2e`.

mod common;

use std::time::Duration;

use anyhow::{Context, Result};

use common::*;
use vedge_e2e::{Session, TestEnv, app_binary};

/// 3.3 — the inline generate **wand** on the Login password field fills the
/// field from the last-used preset (zero IPC).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn generate_into_login_form() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    let session = Session::launch(&env, &app).await.context("launch")?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;

    session
        .click_testid("vault-new-entry")
        .await
        .context("+ New (Login)")?;
    session
        .by_id("ef-password")
        .await
        .context("Login form up")?;
    session
        .click_testid("login-generate-wand")
        .await
        .context("click generate wand")?;
    let pw = session
        .by_id("ef-password")
        .await?
        .prop("value")
        .await?
        .unwrap_or_default();
    assert!(!pw.is_empty(), "generate wand should populate ef-password");

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}

/// 3.5 — session-only generator history is **wiped on lock** (never persisted).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn generator_history_wipes_on_lock() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;

    // Generate a secret in the panel (Regenerate pushes it to session history),
    // then confirm the Recent list shows a row.
    open_generator_panel(&session).await?;
    session
        .click_testid("gen-regenerate")
        .await
        .context("regenerate (pushes to history)")?;
    session
        .click_testid("gen-history-toggle")
        .await
        .context("expand Recent")?;
    wait_until(Duration::from_secs(5), || async {
        Ok(session.count_css("[data-testid='gen-history-row']").await? >= 1)
    })
    .await
    .context("Recent should populate after regenerate")?;
    session.close_dialog().await?;

    // Lock → unlock → reopen the panel: Recent must be empty (wiped on lock).
    session.click_testid("vault-lock").await.context("lock")?;
    assert_unlocked(&session, &vault, false).await?;
    unlock_ui(&session, &vault).await?;

    open_generator_panel(&session).await?;
    session
        .click_testid("gen-history-toggle")
        .await
        .context("expand Recent (post-unlock)")?;
    // Give the list a beat to render, then assert it is empty.
    tokio::time::sleep(Duration::from_millis(300)).await;
    let n = session.count_css("[data-testid='gen-history-row']").await?;
    assert_eq!(
        n, 0,
        "session history must be wiped on lock (found {n} rows)"
    );

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}
