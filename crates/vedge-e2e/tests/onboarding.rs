//! Onboarding e2e scenario (slice 5.2.0 polish): the redesigned create-vault
//! Step 1 — separate vault name / display name / location, plus the live
//! `<location>/<name>.vedge` final-path preview.
//!
//! `#[ignore]` by default; run via `mise e2e`.

mod common;

use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::{Value, json};

use common::*;
use vedge_e2e::{Session, TestEnv, app_binary};

/// 5.2.0 — Step 1 shows a live preview of the composed home and carries a custom
/// display name through to the recents list.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn create_previews_and_names_vault() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    // A `work.vedge` home under the temp vault dir — the location field gets the
    // parent, the name field gets `work`, and the UI recomposes this exact path.
    let location = env.vault_dir.path().to_string_lossy().into_owned();
    let home = env
        .vault_dir
        .path()
        .join("work.vedge")
        .to_string_lossy()
        .into_owned();
    let display = "My Work Vault";

    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;

    // Step 1: type a name + location, then assert the live preview reflects the
    // composed home before advancing.
    session
        .click_testid("launch-new-vault")
        .await
        .context("launch screen: New vault")?;
    session.fill_id("vault-name", "work").await?;
    session.fill_id("vault-location", &location).await?;
    wait_for_map(Duration::from_secs(5), || async {
        let text = session.by_testid("vault-preview").await?.text().await?;
        Ok(text.contains("work.vedge").then_some(text))
    })
    .await
    .context("live preview should reflect the composed home")?;
    session.fill_id("vault-display-name", display).await?;

    // Finish the wizard (Next → password → create → ack → Finish → unlocked).
    finish_create_wizard(&session, &home).await?;

    // The recents row carries the custom display name (read-only invoke).
    wait_until(Duration::from_secs(10), || async {
        let rows = session.invoke("list_recent_vaults", json!({})).await?;
        Ok(rows
            .as_array()
            .map(|arr| {
                arr.iter()
                    .any(|r| r.get("display_name").and_then(Value::as_str) == Some(display))
            })
            .unwrap_or(false))
    })
    .await
    .context("recents row should show the custom display name")?;

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}
