//! The Phase-2 "daily loop" end-to-end scenario (slice 2.9.2).
//!
//! Drives the real app through `tauri-driver`, automating the human smokes each
//! Phase-2 slice left behind: create → add → search → edit/history → favorite →
//! folder → lock → **restart** → unlock → theme-persists, plus a standing
//! console-clean guard for the Leptos reactive-context warning.
//!
//! `#[ignore]` by default (needs a display + the platform WebDriver). Run with:
//!   mise e2e
//!   # or: cargo test -p vedge-e2e -- --ignored --test-threads=1
//!
//! Mutations go through the **UI** (so Leptos state stays in sync); `invoke()`
//! is used only for read-only assertions. A few genuinely fragile terminal
//! mutations (history restore, folder move, theme set) use `invoke()` directly
//! and are asserted via `invoke()` — noted inline. Controls are located by
//! **`data-testid`** (locale-independent); inputs keep their DOM `id`s.

use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use thirtyfour::By;

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

// ---------------------------------------------------------------------------
// Phase-3 feature scenarios (each a separate #[ignore] test so a failure names
// the feature it broke — not one mega-test).
// ---------------------------------------------------------------------------

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

/// 2.6 / 3.8 — lock-on-blur honors the in-dialog guard: a real window blur locks,
/// but a blur while a (native) dialog is open does not. The pref is off by
/// default, so we enable it and restart so the layout's listener boots with it on;
/// the "dialog open" state is faked via the debug-only `__vedge_test_dialog` hook.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn lock_on_blur_respects_dialog_guard() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    let mut session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;

    // Enable lock-on-blur (default off) by persisting the pref, then restart so
    // the /v layout's auto-lock listener boots with it enabled.
    session
        .invoke(
            "set_app_setting",
            json!({
                "key": "security.prefs",
                "value": { "auto_lock_minutes": 15, "lock_on_blur": true, "clipboard_clear_seconds": 30 }
            }),
        )
        .await
        .context("persist lock_on_blur=true")?;
    session.close().await;
    session = Session::launch(&env, &app).await.context("relaunch")?;
    unlock_ui(&session, &vault).await?;

    // Negative: with a "dialog in progress", a blur must NOT lock.
    session.set_dialog_in_progress(true).await?;
    session.dispatch_window_blur().await?;
    tokio::time::sleep(Duration::from_millis(800)).await;
    let still = session
        .invoke("is_unlocked", json!({ "vault_path": vault }))
        .await?
        .as_bool()
        .unwrap_or(false);
    assert!(still, "blur while a dialog is open must NOT lock the vault");
    session.set_dialog_in_progress(false).await?;

    // Positive: a real blur (no dialog) locks.
    session.dispatch_window_blur().await?;
    assert_unlocked(&session, &vault, false)
        .await
        .context("a plain blur should lock the vault")?;

    session.close().await;
    Ok(())
}

// ---------------------------------------------------------------------------
// UI flows
// ---------------------------------------------------------------------------

/// Unlock the (single) vault from the launch picker with the master password.
async fn unlock_ui(s: &Session, vault: &str) -> Result<()> {
    click_vault_option(s).await?;
    s.fill_id("master-password", MASTER_PASSWORD).await?;
    s.click_testid("unlock-submit")
        .await
        .context("unlock panel: Unlock")?;
    assert_unlocked(s, vault, true).await?;
    Ok(())
}

/// Open the generator panel via the inline Login-password caret (the standalone
/// route needs nav we don't drive; the caret hosts the same panel in a dialog).
async fn open_generator_panel(s: &Session) -> Result<()> {
    s.click_testid("vault-new-entry")
        .await
        .context("+ New (Login)")?;
    s.by_id("ef-password").await.context("Login form up")?;
    s.click_testid("login-generate-tune")
        .await
        .context("open generator panel")?;
    s.wait_for(
        By::Css("div[role='dialog']".to_string()),
        Duration::from_secs(5),
    )
    .await
    .context("generator panel dialog")?;
    Ok(())
}

/// Drive the create-vault wizard from the launch screen to a freshly unlocked
/// vault (create → Secret-Key ack → Finish). Shared by the daily loop and the
/// per-feature scenarios below.
async fn create_and_unlock(s: &Session, vault: &str) -> Result<()> {
    s.click_testid("launch-new-vault")
        .await
        .context("launch screen: New vault")?;
    s.fill_id("vault-path", vault).await?;
    s.click_testid("onboarding-next")
        .await
        .context("wizard: Next")?;
    s.fill_id("master-password", MASTER_PASSWORD).await?;
    s.fill_id("master-password-confirm", MASTER_PASSWORD)
        .await?;
    s.click_testid("onboarding-create")
        .await
        .context("wizard: Create vault (Argon2 — fast-KDF seam keeps it quick)")?;
    // Step 3 shows the Secret Key; its ack checkbox is sr-only, so js-click it.
    s.wait_for(
        By::Css("[data-testid='onboarding-ack']".to_string()),
        Duration::from_secs(45),
    )
    .await
    .context("wizard: Secret Key screen")?;
    s.js_click_testid("onboarding-ack").await?;
    s.click_testid("onboarding-finish")
        .await
        .context("wizard: Finish")?;
    assert_unlocked(s, vault, true).await?;
    Ok(())
}

async fn add_login(s: &Session, name: &str, user: &str, pass: &str) -> Result<()> {
    s.click_testid("vault-new-entry")
        .await
        .with_context(|| format!("open add form for {name}"))?;
    s.fill_id("ef-name", name).await?;
    s.fill_id("ef-username", user).await?;
    s.fill_id("ef-password", pass).await?;
    s.click_testid("entry-save")
        .await
        .with_context(|| format!("save login {name}"))?;
    Ok(())
}

async fn add_note(s: &Session, name: &str, content: &str) -> Result<()> {
    s.click_testid("vault-new-entry")
        .await
        .context("open add form for note")?;
    // Switch the entry-type picker (a `vedge-ui` Select) from Login to Note.
    // Scope by the create-form testid — the vault toolbar also has filter
    // comboboxes, so a bare `[role=combobox]` would open the wrong one. Each
    // option carries a locale-independent `data-value` = the type key ("Note").
    s.wait_for(
        By::Css("[data-testid='entry-type-select'] [role='combobox']".to_string()),
        Duration::from_secs(5),
    )
    .await
    .context("open entry-type picker")?
    .click()
    .await
    .context("click type picker")?;
    s.wait_for(
        By::Css("[data-testid='entry-type-select'] [role='option'][data-value='Note']".to_string()),
        Duration::from_secs(5),
    )
    .await
    .context("Note option")?
    .click()
    .await
    .context("select Note type")?;
    s.fill_id("ef-name", name).await?;
    s.fill_id("ef-note", content).await?;
    s.click_testid("entry-save").await.context("save note")?;
    Ok(())
}

/// Click an entry row (opens the detail drawer). Uses the `data-entry-id` hook.
async fn open_entry(s: &Session, id: &str) -> Result<()> {
    let by = By::Css(format!("tr[data-entry-id='{id}']"));
    s.wait_for(by, Duration::from_secs(5))
        .await
        .with_context(|| format!("entry row {id}"))?
        .click()
        .await
        .context("open detail drawer")?;
    Ok(())
}

/// Click the (single) vault row in the launch-screen picker.
async fn click_vault_option(s: &Session) -> Result<()> {
    s.wait_for(
        By::Css("[role='option']".to_string()),
        Duration::from_secs(10),
    )
    .await
    .context("vault picker option")?
    .click()
    .await
    .context("select the vault")?;
    Ok(())
}

/// Return a built-in theme id different from the currently active one.
async fn pick_non_active_theme(s: &Session) -> Result<String> {
    let active = get_active_theme(s).await?;
    let active_id = active.get("id").and_then(Value::as_str).unwrap_or_default();
    let themes = s.invoke("list_themes", json!({})).await?;
    themes
        .as_array()
        .and_then(|arr| {
            arr.iter()
                .filter_map(|t| t.get("id").and_then(Value::as_str))
                .find(|id| *id != active_id)
                .map(str::to_owned)
        })
        .context("no alternative theme to switch to")
}

// ---------------------------------------------------------------------------
// Read-only invoke assertions
// ---------------------------------------------------------------------------

async fn assert_unlocked(s: &Session, vault: &str, expected: bool) -> Result<()> {
    // Unlock runs Argon2 asynchronously (a second or two), so poll rather than
    // reading is_unlocked once. Transient invoke errors during the lock/unlock
    // transition count as "not yet".
    wait_until(Duration::from_secs(25), || async {
        Ok(s.invoke("is_unlocked", json!({ "vault_path": vault }))
            .await
            .ok()
            .and_then(|v| v.as_bool())
            == Some(expected))
    })
    .await
    .with_context(|| format!("is_unlocked should become {expected}"))
}

async fn list_entries(s: &Session, vault: &str) -> Result<Vec<Value>> {
    Ok(s.invoke("list_entries", json!({ "vault_path": vault }))
        .await?
        .as_array()
        .cloned()
        .unwrap_or_default())
}

async fn list_history(s: &Session, vault: &str, entry_id: &str) -> Result<Vec<Value>> {
    Ok(s.invoke(
        "list_history",
        json!({ "vault_path": vault, "entry_id": entry_id }),
    )
    .await?
    .as_array()
    .cloned()
    .unwrap_or_default())
}

async fn get_entry(s: &Session, vault: &str, entry_id: &str) -> Result<Value> {
    s.invoke(
        "get_entry",
        json!({ "vault_path": vault, "entry_id": entry_id }),
    )
    .await
}

async fn get_active_theme(s: &Session) -> Result<Value> {
    s.invoke("get_active_theme", json!({})).await
}

fn entry_id(entries: &[Value], name: &str) -> Option<String> {
    entries.iter().find_map(|e| {
        (e.get("name").and_then(Value::as_str) == Some(name))
            .then(|| e.get("id").and_then(Value::as_str).map(str::to_owned))
            .flatten()
    })
}

// ---------------------------------------------------------------------------
// Polling helpers
// ---------------------------------------------------------------------------

async fn wait_row_count(s: &Session, n: usize, timeout: Duration) -> Result<()> {
    wait_until(timeout, || async { Ok(s.entry_row_count().await? == n) })
        .await
        .with_context(|| format!("expected {n} entry rows"))
}

async fn wait_until<F, Fut>(timeout: Duration, mut cond: F) -> Result<()>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<bool>>,
{
    let start = Instant::now();
    loop {
        if cond().await? {
            return Ok(());
        }
        if start.elapsed() > timeout {
            bail!("condition not met within {timeout:?}");
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

async fn wait_for_map<F, Fut>(timeout: Duration, mut f: F) -> Result<String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<Option<String>>>,
{
    let start = Instant::now();
    loop {
        if let Some(v) = f().await? {
            return Ok(v);
        }
        if start.elapsed() > timeout {
            bail!("value not available within {timeout:?}");
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}
