//! Shared helpers for the VEdge e2e scenarios (slice 2.9.2+).
//!
//! Each `tests/*.rs` scenario file pulls these in with `mod common; use common::*;`.
//! The reusable **driver plumbing** (`Session`, `TestEnv`, `app_binary`) lives in
//! the `vedge_e2e` library crate; this module is the higher-level **UI flows**,
//! **read-only invoke assertions**, and **polling** shared across scenarios.
//!
//! ## Design rule
//! Mutations are driven through the **UI** so the Leptos state and the backend
//! session stay in sync; `invoke()` is used only for **read-only assertions**
//! (`is_unlocked`, `list_entries`, `list_history`, `get_active_theme`). An IPC
//! mutation would not update the Leptos view, so the DOM assertions would drift.
//! A few genuinely fragile terminal mutations (history restore, folder move,
//! theme set, TOTP enrol) use `invoke()` directly and are asserted via `invoke()`
//! — noted inline in the scenarios. Controls are located by **`data-testid`**
//! (locale-independent); inputs keep their DOM `id`s.
//!
//! `#![allow(dead_code)]`: each scenario binary compiles its own copy of this
//! module and uses only the helpers it needs, so unused-helper warnings are
//! expected and harmless (this crate opts out of the workspace `-D warnings`).
#![allow(dead_code)]

use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use thirtyfour::By;

use vedge_e2e::{MASTER_PASSWORD, Session};

// ---------------------------------------------------------------------------
// UI flows
// ---------------------------------------------------------------------------

/// Unlock the (single) vault from the launch picker with the master password.
pub async fn unlock_ui(s: &Session, vault: &str) -> Result<()> {
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
pub async fn open_generator_panel(s: &Session) -> Result<()> {
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

/// Split a vault **home** path (`…/<name>.vedge`) into its parent location and
/// its `.vedge`-stripped name, so the redesigned Step-1 name+location fields
/// recompose the identical home (slice 5.2.0).
fn split_home(home: &str) -> (String, String) {
    let home = home.trim_end_matches(['/', '\\']);
    let (parent, last) = match home.rfind(['/', '\\']) {
        Some(i) => (&home[..i], &home[i + 1..]),
        None => ("", home),
    };
    let name = last.strip_suffix(".vedge").unwrap_or(last);
    (parent.to_owned(), name.to_owned())
}

/// Drive the create-vault wizard from the launch screen to a freshly unlocked
/// vault (create → Secret-Key ack → Finish). Shared by the daily loop and the
/// per-feature scenarios.
pub async fn create_and_unlock(s: &Session, vault: &str) -> Result<()> {
    create_and_unlock_named(s, vault, None).await
}

/// Like [`create_and_unlock`] but types a custom **display name** (the recents
/// label) into the wizard. `vault` is the full `.vedge` home path; it is split
/// into the name + location fields the redesigned Step 1 exposes (slice 5.2.0).
pub async fn create_and_unlock_named(
    s: &Session,
    vault: &str,
    display: Option<&str>,
) -> Result<()> {
    let (location, name) = split_home(vault);
    s.click_testid("launch-new-vault")
        .await
        .context("launch screen: New vault")?;
    s.fill_id("vault-name", &name).await?;
    s.fill_id("vault-location", &location).await?;
    if let Some(d) = display {
        s.fill_id("vault-display-name", d).await?;
    }
    finish_create_wizard(s, vault).await
}

/// The create-wizard tail shared by [`create_and_unlock_named`] and the
/// onboarding scenario: Next → master password → Create → Secret-Key ack →
/// Finish → assert unlocked.
pub async fn finish_create_wizard(s: &Session, vault: &str) -> Result<()> {
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

pub async fn add_login(s: &Session, name: &str, user: &str, pass: &str) -> Result<()> {
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

pub async fn add_note(s: &Session, name: &str, content: &str) -> Result<()> {
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
pub async fn open_entry(s: &Session, id: &str) -> Result<()> {
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
pub async fn click_vault_option(s: &Session) -> Result<()> {
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
pub async fn pick_non_active_theme(s: &Session) -> Result<String> {
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

pub async fn assert_unlocked(s: &Session, vault: &str, expected: bool) -> Result<()> {
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

pub async fn list_entries(s: &Session, vault: &str) -> Result<Vec<Value>> {
    Ok(s.invoke("list_entries", json!({ "vault_path": vault }))
        .await?
        .as_array()
        .cloned()
        .unwrap_or_default())
}

pub async fn list_history(s: &Session, vault: &str, entry_id: &str) -> Result<Vec<Value>> {
    Ok(s.invoke(
        "list_history",
        json!({ "vault_path": vault, "entry_id": entry_id }),
    )
    .await?
    .as_array()
    .cloned()
    .unwrap_or_default())
}

pub async fn get_entry(s: &Session, vault: &str, entry_id: &str) -> Result<Value> {
    s.invoke(
        "get_entry",
        json!({ "vault_path": vault, "entry_id": entry_id }),
    )
    .await
}

pub async fn get_active_theme(s: &Session) -> Result<Value> {
    s.invoke("get_active_theme", json!({})).await
}

/// Visible text of the audit table (empty string if it isn't mounted yet).
pub async fn audit_table_text(s: &Session) -> Result<String> {
    match s
        .driver()
        .find(By::Css("[data-testid='audit-table']"))
        .await
    {
        Ok(el) => Ok(el.text().await.unwrap_or_default()),
        Err(_) => Ok(String::new()),
    }
}

/// Visible text of the health findings table (empty string if not mounted yet).
pub async fn health_table_text(s: &Session) -> Result<String> {
    match s
        .driver()
        .find(By::Css("[data-testid='health-table']"))
        .await
    {
        Ok(el) => Ok(el.text().await.unwrap_or_default()),
        Err(_) => Ok(String::new()),
    }
}

pub fn entry_id(entries: &[Value], name: &str) -> Option<String> {
    entries.iter().find_map(|e| {
        (e.get("name").and_then(Value::as_str) == Some(name))
            .then(|| e.get("id").and_then(Value::as_str).map(str::to_owned))
            .flatten()
    })
}

// ---------------------------------------------------------------------------
// Polling helpers
// ---------------------------------------------------------------------------

pub async fn wait_row_count(s: &Session, n: usize, timeout: Duration) -> Result<()> {
    wait_until(timeout, || async { Ok(s.entry_row_count().await? == n) })
        .await
        .with_context(|| format!("expected {n} entry rows"))
}

pub async fn wait_until<F, Fut>(timeout: Duration, mut cond: F) -> Result<()>
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

pub async fn wait_for_map<F, Fut>(timeout: Duration, mut f: F) -> Result<String>
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
