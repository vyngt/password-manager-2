//! The Phase-2 "daily loop" end-to-end scenario (slice 2.9.2).
//!
//! Drives the real app through `tauri-driver`, automating the human smokes each
//! Phase-2 slice left behind: create → add → search → edit/history → favorite →
//! folder → lock → **restart** → unlock → theme-persists, plus a standing
//! console-clean guard for the Leptos reactive-context warning.
//!
//! Shared UI flows / assertions / polling live in `common` (see `tests/common/mod.rs`);
//! the reusable driver plumbing lives in the `vedge_e2e` crate. `#[ignore]` by
//! default (needs a display + the platform WebDriver). Run with:
//!   mise e2e
//!   # or: cargo test -p vedge-e2e -- --ignored --test-threads=1

mod common;

use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::{Value, json};
use thirtyfour::By;

use common::*;
use vedge_e2e::{MASTER_PASSWORD, Session, TestEnv, app_binary};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn daily_loop() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();

    // ---- boot ----------------------------------------------------------------
    let mut session = Session::launch(&env, &app)
        .await
        .context("launch app session 1")?;

    // The console-clean guard must be live before we trust it (step 7).
    session
        .console_selftest()
        .await
        .context("console-clean guard self-test")?;

    // ---- step 1: create a vault (UI wizard) → land unlocked -------------------
    create_and_unlock(&session, &vault)
        .await
        .context("create + unlock via the wizard")?;
    session
        .assert_console_clean()
        .await
        .context("after create")?;

    // ---- step 2: add entries of several types → the list shows them ----------
    add_login(&session, "GitHub", "octocat", "hunter2-a").await?;
    add_login(&session, "GitLab", "octodog", "hunter2-b").await?;
    add_note(&session, "Deploy notes", "prod deploy at 5pm").await?;

    wait_row_count(&session, 3, Duration::from_secs(10))
        .await
        .context("3 entries visible after add")?;
    let entries = list_entries(&session, &vault).await?;
    assert_eq!(entries.len(), 3, "list_entries should report 3 entries");
    let github_id = entry_id(&entries, "GitHub").context("find GitHub entry id")?;

    // ---- step 3: search filters the list to the matching subset --------------
    session.fill_id("vault-search", "GitHub").await?;
    wait_row_count(&session, 1, Duration::from_secs(5))
        .await
        .context("search 'GitHub' → 1 row")?;
    session.fill_id("vault-search", "").await?; // clear
    wait_row_count(&session, 3, Duration::from_secs(5))
        .await
        .context("cleared search → 3 rows")?;

    // ---- step 4: edit → new version; history shows it → restore --------------
    open_entry(&session, &github_id).await?;
    session
        .click_testid("detail-edit")
        .await
        .context("drawer: Edit")?;
    session.fill_id("ef-username", "octocat-renamed").await?;
    session
        .click_testid("entry-save")
        .await
        .context("edit dialog: Save")?;

    // The snapshot-on-update is the crux of slice 2.7 — assert via the read API.
    let history = list_history(&session, &vault, &github_id).await?;
    let snapshot_id = history
        .iter()
        .find_map(|h| {
            h.get("history_id")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .context("a prior-version snapshot should exist after the edit")?;

    // Open the history panel from the (still-open) detail drawer and confirm the
    // prior version renders. Don't re-click the row — it would toggle the drawer.
    session
        .click_testid("detail-history")
        .await
        .context("open version history")?;
    session
        .wait_for(
            By::Css("div[role='dialog']".to_string()),
            Duration::from_secs(5),
        )
        .await
        .context("version-history dialog")?;

    // Restore is the most fragile UI step (row action + confirm banner share the
    // "Restore" label); drive it via the read/mutate API and assert the result.
    session
        .invoke(
            "restore_history",
            json!({ "vault_path": vault, "entry_id": github_id, "history_id": snapshot_id }),
        )
        .await
        .context("restore_history")?;
    let restored = get_entry(&session, &vault, &github_id).await?;
    let username = restored
        .get("data")
        .and_then(|d| d.get("username"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert_eq!(username, "octocat", "restore should revert the username");

    // Close the version-history modal so it doesn't cover the search box below.
    session.close_dialog().await?;

    // ---- step 5: favorite (UI) + folder create (UI) + move -------------------
    session.fill_id("vault-search", "GitHub").await?;
    wait_row_count(&session, 1, Duration::from_secs(5)).await?;
    session
        .click_testid("row-favorite")
        .await
        .context("favorite the GitHub row")?;
    session.fill_id("vault-search", "").await?;
    wait_until(Duration::from_secs(5), || async {
        let e = list_entries(&session, &vault).await?;
        Ok(e.iter().any(|x| {
            x.get("name").and_then(Value::as_str) == Some("GitHub")
                && x.get("is_favorite").and_then(Value::as_bool) == Some(true)
        }))
    })
    .await
    .context("GitHub should be favorited")?;

    // Create a folder via the sidebar, assert it exists (folders are entries).
    session
        .click_testid("folder-new-toggle")
        .await
        .context("sidebar: New folder")?;
    session.fill_id("folder-new", "Work").await?;
    session
        .by_id("folder-new")
        .await?
        .send_keys(thirtyfour::Key::Enter)
        .await
        .context("submit new folder")?;
    let folder_id = wait_for_map(Duration::from_secs(10), || async {
        let e = list_entries(&session, &vault).await?;
        Ok(e.iter().find_map(|x| {
            (x.get("name").and_then(Value::as_str) == Some("Work")
                && x.get("entry_type").and_then(Value::as_str) == Some("Folder"))
            .then(|| x.get("id").and_then(Value::as_str).map(str::to_owned))
            .flatten()
        }))
    })
    .await
    .context("the 'Work' folder should exist")?;

    // Move GitHub into the folder (fragile move dialog → use the mutate API).
    session
        .invoke(
            "move_entry",
            json!({ "vault_path": vault, "entry_id": github_id, "folder_id": folder_id }),
        )
        .await
        .context("move_entry into folder")?;

    // ---- step 6: theme set + lock → restart → unlock → theme persists --------
    let theme_id = pick_non_active_theme(&session).await?;
    session
        .invoke("set_active_theme", json!({ "id": theme_id }))
        .await
        .context("set_active_theme")?;

    session
        .click_testid("vault-lock")
        .await
        .context("Lock the vault")?;
    assert_unlocked(&session, &vault, false)
        .await
        .context("locked after Lock")?;
    session
        .assert_console_clean()
        .await
        .context("before restart")?;

    // Restart: drop session 1 (kills the app), launch a new one on the SAME
    // data + vault dirs. Theme (app.db) and entries (.vdb) must survive.
    session.close().await;
    session = Session::launch(&env, &app)
        .await
        .context("relaunch app session 2")?;

    let active = get_active_theme(&session).await?;
    assert_eq!(
        active.get("id").and_then(Value::as_str),
        Some(theme_id.as_str()),
        "the active theme should survive a restart"
    );

    // Unlock via the picker + panel (real UI unlock; reads the OS keychain).
    click_vault_option(&session).await?;
    session.fill_id("master-password", MASTER_PASSWORD).await?;
    session
        .click_testid("unlock-submit")
        .await
        .context("unlock panel: Unlock")?;
    assert_unlocked(&session, &vault, true)
        .await
        .context("unlocked after restart")?;

    let after = list_entries(&session, &vault).await?;
    assert_eq!(
        after.len(),
        4,
        "3 entries + 1 folder should persist across restart"
    );

    // ---- step 7: console-clean guard (final) ---------------------------------
    session
        .assert_console_clean()
        .await
        .context("final console-clean assertion")?;

    session.close().await;
    Ok(())
}
