//! Audit-page e2e scenario (slice 4.1): the audit page surfaces the trail with
//! resolved entry names (zero decrypt).
//!
//! `#[ignore]` by default; run via `mise e2e`.

mod common;

use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::{Value, json};

use common::*;
use vedge_e2e::{Session, TestEnv, app_binary};

/// 4.1 — the audit page surfaces the trail: after an audited reveal, a `Viewed`
/// event for the entry shows on `/v/audit` with the entry's **resolved name**
/// (zero decrypt). The name is user data → locale-independent, so the DOM
/// assertion needs no per-row testid.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn audit_page_shows_copy_event() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;

    add_login(&session, "GitHub", "octocat", "hunter2").await?;
    wait_row_count(&session, 1, Duration::from_secs(10)).await?;
    let entries = list_entries(&session, &vault).await?;
    let github_id = entry_id(&entries, "GitHub").context("find GitHub id")?;

    // Reveal the entry — the audited reveal path writes an `AuditAction::Viewed`
    // row (the copy button routes through the same audit). Terminal
    // read-that-audits via invoke; the audit page re-fetches on navigation.
    get_entry(&session, &vault, &github_id).await?;

    // Navigate to the audit page; the resolved entry name must render in the table.
    session
        .click_testid("nav-audit")
        .await
        .context("open audit page")?;
    wait_until(Duration::from_secs(10), || async {
        Ok(audit_table_text(&session).await?.contains("GitHub"))
    })
    .await
    .context("audit table should show the GitHub entry's resolved name")?;

    // Backend assertion (locale-independent): a Viewed event for this entry exists.
    let page = session
        .invoke(
            "list_audit",
            json!({
                "vault_path": vault,
                "query": { "entry_id": github_id, "actions": ["Viewed"], "limit": 200, "offset": 0 }
            }),
        )
        .await?;
    let has_viewed = page
        .get("events")
        .and_then(Value::as_array)
        .is_some_and(|evs| {
            evs.iter().any(|e| {
                e.get("action").and_then(Value::as_str) == Some("Viewed")
                    && e.get("entry_id").and_then(Value::as_str) == Some(github_id.as_str())
            })
        });
    assert!(
        has_viewed,
        "a Viewed audit event for the entry should exist"
    );

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}
