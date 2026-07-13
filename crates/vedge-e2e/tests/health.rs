//! Password-health e2e scenarios (slices 4.3b / 4.4): the scan surfaces reuse
//! findings, and — with the opt-in enabled — a known-breached password.
//!
//! `#[ignore]` by default; run via `mise e2e`.

mod common;

use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::{Value, json};

use common::*;
use vedge_e2e::{Session, TestEnv, app_binary};

/// Slice 4.3b: run the password-health scan from `/v/health` and assert findings
/// render + a `HealthScanned` audit row is written. Two logins share a password
/// (seeding a cross-entry reuse group); the scan is triggered via the UI button,
/// the findings table is asserted in the DOM (resolved entry names,
/// locale-independent), and the finding shape + the audit row are asserted via
/// `invoke` (backend, locale-independent).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn scan_health_shows_findings() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;

    // Two logins share one password → a cross-entry reuse group.
    add_login(&session, "GitHub", "octocat", "reused-pw-42").await?;
    add_login(&session, "GitLab", "tanuki", "reused-pw-42").await?;
    wait_row_count(&session, 2, Duration::from_secs(10)).await?;

    // Run the scan from the health page (the slowest op; explicit button).
    session
        .click_testid("nav-health")
        .await
        .context("open health page")?;
    session
        .click_testid("health-run-scan")
        .await
        .context("run health scan")?;

    // The findings table renders with a resolved entry name (zero decrypt).
    wait_until(Duration::from_secs(20), || async {
        Ok(health_table_text(&session).await?.contains("GitHub"))
    })
    .await
    .context("health table should show a finding for the GitHub entry")?;

    // Backend assertion 1: the UI run wrote EXACTLY ONE HealthScanned row. Asserted
    // BEFORE the finding-shape invoke below — that invoke writes a second row and
    // would confound this count, so it must come first (proves the *UI* path audits).
    let page = session
        .invoke(
            "list_audit",
            json!({
                "vault_path": vault,
                "query": { "actions": ["HealthScanned"], "limit": 200, "offset": 0 }
            }),
        )
        .await?;
    let scan_rows = page
        .get("events")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    assert_eq!(
        scan_rows, 1,
        "the UI scan should write exactly one HealthScanned audit row"
    );

    // Backend assertion 2 (locale-independent): a scan returns a Reused finding for
    // the shared password (adjacently-tagged `{{"kind":"Reused",…}}`).
    let report = session
        .invoke("scan_health", json!({ "vault_path": vault, "input": {} }))
        .await?;
    let has_reuse = report
        .get("findings")
        .and_then(Value::as_array)
        .is_some_and(|fs| {
            fs.iter().any(|f| {
                f.get("kind")
                    .and_then(|k| k.get("kind"))
                    .and_then(Value::as_str)
                    == Some("Reused")
            })
        });
    assert!(
        has_reuse,
        "the scan should detect the shared password as reuse"
    );

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}

/// Slice 4.4: with the opt-in breach check enabled, a health scan flags a
/// known-breached password. The env-gated offline `MemoryBreachChecker` reports
/// the seeded password as breached, so no network is touched. Enabling the check
/// is a setup mutation via `invoke` (the TOTP-enrolment precedent) — the **backend**
/// reads `security.prefs.breach_check_enabled` on every scan, so this drives the
/// exact consent gate a user's Settings toggle would. The Breached row is asserted
/// in the DOM (resolved entry name, locale-independent) and the finding shape +
/// summary via `invoke`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn scan_health_shows_breached() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;

    // Seed a login whose password the offline breach double reports as breached.
    // ⚠ Keep in sync with `resolve_breach` in vedge-tauri/src/setup/services.rs.
    add_login(&session, "GitHub", "octocat", "e2e-breached-pass-123").await?;
    wait_row_count(&session, 1, Duration::from_secs(10)).await?;

    // Opt in. Backend-verified consent: the scan command reads `security.prefs`
    // fresh, so setting the flag is enough to drive the breach phase.
    session
        .invoke(
            "set_app_setting",
            json!({ "key": "security.prefs", "value": { "breach_check_enabled": true } }),
        )
        .await
        .context("enable the breach-check opt-in")?;

    // Run the scan from the health page (the slowest op; explicit button).
    session
        .click_testid("nav-health")
        .await
        .context("open health page")?;
    session
        .click_testid("health-run-scan")
        .await
        .context("run health scan")?;

    // The findings table renders a row for the seeded entry (zero decrypt).
    wait_until(Duration::from_secs(20), || async {
        Ok(health_table_text(&session).await?.contains("GitHub"))
    })
    .await
    .context("health table should show a finding for the seeded entry")?;

    // Backend assertion (locale-independent): the scan returns a Breached finding
    // and the summary counts exactly one breached secret.
    let report = session
        .invoke("scan_health", json!({ "vault_path": vault, "input": {} }))
        .await?;
    let has_breached = report
        .get("findings")
        .and_then(Value::as_array)
        .is_some_and(|fs| {
            fs.iter().any(|f| {
                f.get("kind")
                    .and_then(|k| k.get("kind"))
                    .and_then(Value::as_str)
                    == Some("Breached")
            })
        });
    assert!(
        has_breached,
        "the scan should flag the seeded password as breached"
    );
    let breached_count = report
        .get("summary")
        .and_then(|s| s.get("breached"))
        .and_then(Value::as_u64);
    assert_eq!(
        breached_count,
        Some(1),
        "exactly one breached secret in the summary"
    );

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}
