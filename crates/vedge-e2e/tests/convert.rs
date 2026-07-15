//! Legacy-layout convert e2e (slice 5.2.3, B4) — driven through the UI.
//!
//! `register_vault` refuses a bare `.vdb` path, so an old-layout vault can't be seeded through
//! the normal API. A debug-only, `VEDGE_E2E_SEED`-gated seam (`e2e_downgrade_to_legacy`, inert in
//! release) lays a real `.vedge/` home back out as a legacy `<stem>.vdb` + `<stem>.vedge_blobs/`
//! and re-points its recents row — so the picker renders it as convertible. Only the SETUP uses
//! `invoke`; the mutation under test (Convert) and the unlock are UI-driven.
//!
//! `#[ignore]` by default; run via `mise e2e`.

mod common;

use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::json;
use thirtyfour::By;

use common::*;
use vedge_e2e::{Session, TestEnv, app_binary};

/// Create a real vault, downgrade it to the old layout, and Convert it back through the picker —
/// then unlock it and assert its document decrypts. The round trip exercises the real
/// `migrate_vault_layout` (copy → atomic commit → uuid → biometric purge → reap) + unlock.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn open_old_layout_vault_and_convert() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();

    // The seed seam is per-scenario (debug + this env var only), never global.
    let session = Session::launch_with_env(&env, &app, &[("VEDGE_E2E_SEED", "1")]).await?;
    session.console_selftest().await?;

    // A genuine, unlockable vault with a document, then lock it.
    create_and_unlock(&session, &vault).await?;
    add_login(&session, "keeper", "u", "p").await?;
    session
        .click_testid("vault-lock")
        .await
        .context("lock the vault")?;
    assert_unlocked(&session, &vault, false).await?;

    // SETUP (invoke): lay the home back out as a legacy `.vdb`, then reload so the picker re-reads
    // recents and renders the convertible row.
    session
        .invoke("e2e_downgrade_to_legacy", json!({ "home_path": vault }))
        .await
        .context("downgrade the home to a legacy .vdb layout")?;
    session.driver().refresh().await.context("reload the app")?;

    // The picker offers Convert on the legacy row.
    session
        .wait_for(
            By::Css("[data-testid='vault-convert']".to_string()),
            Duration::from_secs(25),
        )
        .await
        .context("a legacy .vdb vault must show a Convert action")?;
    session
        .click_testid("vault-convert")
        .await
        .context("click Convert")?;

    // Convert rebuilds the `.vedge/` home and re-points the recents row; the Convert action
    // disappears as the row becomes a healthy, openable home.
    wait_until(Duration::from_secs(30), || async {
        Ok(session
            .driver()
            .find(By::Css("[data-testid='vault-convert']"))
            .await
            .is_err())
    })
    .await
    .context("the vault must become a converted .vedge home")?;

    // Unlock the converted vault (same home path) and confirm the document decrypts.
    unlock_ui(&session, &vault).await?;
    let entries = list_entries(&session, &vault).await?;
    assert!(
        entry_id(&entries, "keeper").is_some(),
        "the document must decrypt after the convert"
    );

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}
