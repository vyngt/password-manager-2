//! TOTP e2e scenario (slice 4.2): enrol a seed, reveal a rotating code in the
//! detail drawer, and assert a `TotpRevealed` audit row is written.
//!
//! `#[ignore]` by default; run via `mise e2e`.

mod common;

use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::{Value, json};
use thirtyfour::By;

use common::*;
use vedge_e2e::{Session, TestEnv, app_binary};

/// Slice 4.2: enrol a TOTP on a Login, then reveal it in the detail drawer and
/// assert a rotating code renders + a `TotpRevealed` audit row is written.
///
/// Enrolment is a setup mutation via `invoke` (like history/folder/theme): fetch
/// the door DTO with `get_entry`, splice a `Set` intent, and `update_entry`. The
/// seed never crosses the read path — the update carries it server-side. The
/// load-bearing assertions (the reveal, the display, the audit) run via the UI.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn totp_display_shows_and_rotates() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;

    add_login(&session, "GitHub", "octocat", "hunter2").await?;
    wait_row_count(&session, 1, Duration::from_secs(10)).await?;
    let entries = list_entries(&session, &vault).await?;
    let id = entry_id(&entries, "GitHub").context("find GitHub id")?;

    // Enrol a TOTP seed: fetch the door DTO, splice a `Set` intent, save.
    let mut payload = session
        .invoke("get_entry", json!({ "vault_path": vault, "entry_id": id }))
        .await?;
    payload["data"]["totp"] = json!({ "kind": "Set", "value": "JBSWY3DPEHPK3PXP" });
    session
        .invoke(
            "update_entry",
            json!({ "vault_path": vault, "entry_id": id, "payload": payload }),
        )
        .await
        .context("enrol TOTP via update_entry")?;

    // Open the detail drawer and reveal the code.
    open_entry(&session, &id).await?;
    session
        .click_testid("detail-show-totp")
        .await
        .context("Show one-time code")?;

    // The rotating code renders (6 digits, grouped "482 916"), role=timer.
    let el = session
        .wait_for(
            By::Css("[data-testid='detail-totp'] [role='timer']".to_string()),
            Duration::from_secs(10),
        )
        .await
        .context("TOTP code element should render")?;
    let text = el.text().await?;
    let digits = text.chars().filter(|c| c.is_ascii_digit()).count();
    assert_eq!(digits, 6, "expected a 6-digit code, got {text:?}");

    // A TotpRevealed audit row exists for this entry (locale-independent).
    let page = session
        .invoke(
            "list_audit",
            json!({
                "vault_path": vault,
                "query": { "entry_id": id, "actions": ["TotpRevealed"], "limit": 200, "offset": 0 }
            }),
        )
        .await?;
    let has_revealed = page
        .get("events")
        .and_then(Value::as_array)
        .is_some_and(|evs| !evs.is_empty());
    assert!(has_revealed, "a TotpRevealed audit event should exist");

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}
