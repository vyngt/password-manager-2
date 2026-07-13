//! Backup & restore e2e scenarios (slice 5.2c).
//!
//! The `backup_vault` / `restore_vault` commands are driven through `invoke`, not the
//! Settings ▸ Backup UI: those buttons open **native** file dialogs (`window.__TAURI__
//! .dialog`) that a WebDriver can't automate. Create / unlock / lock / assertions stay
//! UI-driven so the Leptos state and backend session stay in sync (restore lands the
//! vault locked, and the subsequent UI unlock reloads the view from the restored DB).
//! The clickable Backup tab is covered by the manual QA smoke.
//!
//! `#[ignore]` by default; run via `mise e2e`.

mod common;

use anyhow::{Context, Result};
use serde_json::json;

use common::*;
use vedge_e2e::{Session, TestEnv, app_binary};

/// 5.2c — back up a vault, move it forward, then restore the older backup over it:
/// the restore reverts the vault to the backed-up state (a confirmed rollback).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn backup_and_restore() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    let backup = format!("{vault}.vbk");

    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;

    // Seed an entry, then snapshot the vault to a `.vbk` (read-only op via invoke).
    add_login(&session, "keeper", "u", "p").await?;
    session
        .invoke(
            "backup_vault",
            json!({ "vault_path": vault, "dest_path": backup }),
        )
        .await
        .context("backup_vault")?;

    // Move the live vault FORWARD of the backup (so restoring it is a rollback).
    add_login(&session, "extra", "u", "p").await?;

    // Lock, then restore the older backup (confirmed rollback).
    session.click_testid("vault-lock").await.context("lock")?;
    assert_unlocked(&session, &vault, false).await?;
    session
        .invoke(
            "restore_vault",
            json!({ "vault_path": vault, "archive_path": backup, "confirm_rollback": true }),
        )
        .await
        .context("restore_vault")?;

    // Re-unlock through the UI and confirm the vault is back at the backed-up state:
    // "keeper" is present, "extra" (added after the backup) is gone.
    unlock_ui(&session, &vault).await?;
    let entries = list_entries(&session, &vault).await?;
    assert!(
        entry_id(&entries, "keeper").is_some(),
        "the backed-up entry must survive the restore"
    );
    assert!(
        entry_id(&entries, "extra").is_none(),
        "an entry added after the backup must be gone after restoring it"
    );

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}

/// 5.2c — restore refuses an UNLOCKED target (Decision ⑦): the command errors and the
/// live vault is left untouched.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn restore_refuses_unlocked_vault() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    let backup = format!("{vault}.vbk");

    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;
    add_login(&session, "keeper", "u", "p").await?;
    session
        .invoke(
            "backup_vault",
            json!({ "vault_path": vault, "dest_path": backup }),
        )
        .await
        .context("backup_vault")?;

    // Restore WITHOUT locking first → the command must refuse.
    let refused = session
        .invoke(
            "restore_vault",
            json!({ "vault_path": vault, "archive_path": backup, "confirm_rollback": true }),
        )
        .await;
    assert!(
        refused.is_err(),
        "restoring an unlocked vault must be refused, got {refused:?}"
    );

    // The vault stays unlocked and intact.
    assert_unlocked(&session, &vault, true).await?;
    let entries = list_entries(&session, &vault).await?;
    assert!(
        entry_id(&entries, "keeper").is_some(),
        "a refused restore must leave the live vault untouched"
    );

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}
