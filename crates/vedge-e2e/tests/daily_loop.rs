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
//! and are asserted via `invoke()` — noted inline. Text/aria selectors assume
//! the default **English** locale.

use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use vedge_e2e::{MASTER_PASSWORD, Session, TestEnv, app_binary};

/// English UI strings (i18n keys in comments) — see `crates/vedge-app/locales/en`.
mod ui {
    pub const NEW_VAULT: &str = "New"; // unlock.new_vault
    pub const NEXT: &str = "Next"; // onboarding.next
    pub const CREATE_VAULT: &str = "Create vault"; // onboarding.create
    pub const FINISH: &str = "Finish"; // onboarding.finish
    pub const ACK_ARIA: &str = "I have saved my Secret Key"; // onboarding.ack_aria
    pub const UNLOCK: &str = "Unlock"; // unlock.unlock
    pub const NEW_ITEM: &str = "+ New"; // vault.new_item
    pub const SAVE: &str = "Save"; // vault.save
    pub const EDIT: &str = "Edit"; // vault.edit
    pub const TYPE_ARIA: &str = "Entry type"; // vault.type_picker_aria
    pub const TYPE_NOTE: &str = "Note"; // vault.type_note
    pub const VERSION_HISTORY: &str = "Version history"; // vault.version_history (aria)
    pub const FAVORITE: &str = "Add to favorites"; // vault.favorite (aria)
    pub const FOLDER_NEW: &str = "New folder"; // vault.folder_new (aria)
    pub const LOCK: &str = "Lock"; // unlock.lock (aria)
}

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
    session
        .click_button_text(ui::NEW_VAULT)
        .await
        .context("launch screen: New vault")?;
    session.fill_id("vault-path", &vault).await?;
    session
        .click_button_text(ui::NEXT)
        .await
        .context("wizard: Next")?;
    session.fill_id("master-password", MASTER_PASSWORD).await?;
    session
        .fill_id("master-password-confirm", MASTER_PASSWORD)
        .await?;
    session
        .click_button_text(ui::CREATE_VAULT)
        .await
        .context("wizard: Create vault (runs Argon2 — may take a few seconds)")?;
    // Step 3 of the wizard shows the Secret Key; acknowledge + finish.
    session
        .wait_for(
            thirtyfour::By::XPath(format!("//*[@aria-label='{}']", ui::ACK_ARIA)),
            Duration::from_secs(45),
        )
        .await
        .context("wizard: Secret Key screen (acknowledge checkbox)")?;
    // The ack is a custom checkbox: its native <input> is sr-only, so a plain
    // WebDriver click is "not interactable". Scripted click toggles it + fires
    // on:change, which enables Finish.
    session.js_click_aria(ui::ACK_ARIA).await?;
    session
        .click_button_text(ui::FINISH)
        .await
        .context("wizard: Finish")?;

    assert_unlocked(&session, &vault, true).await?;
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
        .click_button_text(ui::EDIT)
        .await
        .context("drawer: Edit")?;
    session.fill_id("ef-username", "octocat-renamed").await?;
    session
        .click_button_text(ui::SAVE)
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
        .click_aria(ui::VERSION_HISTORY)
        .await
        .context("open version history")?;
    session
        .wait_for(
            thirtyfour::By::Css("div[role='dialog']"),
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
        .click_aria(ui::FAVORITE)
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
        .click_aria(ui::FOLDER_NEW)
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
        .click_aria(ui::LOCK)
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
        .click_button_text(ui::UNLOCK)
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
// UI flows
// ---------------------------------------------------------------------------

async fn add_login(s: &Session, name: &str, user: &str, pass: &str) -> Result<()> {
    s.click_button_text(ui::NEW_ITEM)
        .await
        .with_context(|| format!("open add form for {name}"))?;
    s.fill_id("ef-name", name).await?;
    s.fill_id("ef-username", user).await?;
    s.fill_id("ef-password", pass).await?;
    s.click_button_text(ui::SAVE)
        .await
        .with_context(|| format!("save login {name}"))?;
    Ok(())
}

async fn add_note(s: &Session, name: &str, content: &str) -> Result<()> {
    s.click_button_text(ui::NEW_ITEM)
        .await
        .context("open add form for note")?;
    // Switch the type picker (custom Select) from the default to Note.
    s.click_aria(ui::TYPE_ARIA)
        .await
        .context("open entry-type picker")?;
    let opt = thirtyfour::By::XPath(format!(
        "//*[@role='option'][normalize-space(.)='{}']",
        ui::TYPE_NOTE
    ));
    s.wait_for(opt, Duration::from_secs(5))
        .await
        .context("Note option")?
        .click()
        .await
        .context("select Note type")?;
    s.fill_id("ef-name", name).await?;
    s.fill_id("ef-note", content).await?;
    s.click_button_text(ui::SAVE).await.context("save note")?;
    Ok(())
}

/// Click an entry row (opens the detail drawer). Uses the `data-entry-id` hook.
async fn open_entry(s: &Session, id: &str) -> Result<()> {
    let by = thirtyfour::By::Css(format!("tr[data-entry-id='{id}']"));
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
        thirtyfour::By::Css("[role='option']"),
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
    let got = s
        .invoke("is_unlocked", json!({ "vault_path": vault }))
        .await?
        .as_bool()
        .context("is_unlocked returned non-bool")?;
    if got != expected {
        bail!("is_unlocked = {got}, expected {expected}");
    }
    Ok(())
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
