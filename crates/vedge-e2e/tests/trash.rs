//! Trash e2e scenario (slice 3.6): the Trash sidebar folder's verbs — Restore,
//! Delete-permanently, Empty.
//!
//! `#[ignore]` by default; run via `mise e2e`.

mod common;

use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::{Value, json};

use common::*;
use vedge_e2e::{Session, TestEnv, app_binary};

/// 3.6 — the Trash sidebar folder's verbs: Restore, Delete-permanently, Empty.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn trash_restore_delete_empty() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;

    add_login(&session, "Alpha", "a", "pw-a").await?;
    add_login(&session, "Bravo", "b", "pw-b").await?;
    add_login(&session, "Charlie", "c", "pw-c").await?;
    wait_row_count(&session, 3, Duration::from_secs(10)).await?;

    // Trash all three (setup mutation via invoke; the Trash view re-fetches on
    // navigation, so the DOM reflects it when we open the folder).
    for e in list_entries(&session, &vault).await? {
        if let Some(id) = e.get("id").and_then(Value::as_str) {
            session
                .invoke(
                    "soft_delete_entry",
                    json!({ "vault_path": vault, "entry_id": id }),
                )
                .await
                .context("soft_delete_entry")?;
        }
    }

    session
        .click_testid("sidebar-trash")
        .await
        .context("open Trash folder")?;
    wait_row_count(&session, 3, Duration::from_secs(10))
        .await
        .context("3 trashed rows")?;

    // Restore one → two remain.
    session
        .click_testid("row-restore")
        .await
        .context("restore a trashed entry")?;
    wait_row_count(&session, 2, Duration::from_secs(10))
        .await
        .context("2 trashed after restore")?;

    // Permanently delete one (confirm) → one remains.
    session
        .click_testid("row-delete-permanent")
        .await
        .context("delete permanently")?;
    session
        .click_testid("confirm-delete-permanent")
        .await
        .context("confirm permanent delete")?;
    wait_row_count(&session, 1, Duration::from_secs(10))
        .await
        .context("1 trashed after permanent delete")?;

    // Empty trash (confirm) → zero.
    session
        .click_testid("trash-empty")
        .await
        .context("empty trash")?;
    session
        .click_testid("confirm-empty-trash")
        .await
        .context("confirm empty trash")?;
    wait_row_count(&session, 0, Duration::from_secs(10))
        .await
        .context("0 trashed after empty")?;

    // The restored entry is back in the active vault.
    session
        .click_testid("sidebar-all")
        .await
        .context("back to All items")?;
    wait_row_count(&session, 1, Duration::from_secs(10))
        .await
        .context("1 active (restored) entry")?;

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}
