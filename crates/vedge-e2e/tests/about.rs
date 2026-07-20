//! About-tab e2e (slice PG.3): the Settings ▸ About tab renders the app version,
//! the MIT licence, and the third-party notices list — under the production CSP,
//! with the console clean of any CSP violation.
//!
//! 🔴 That last part is the standing proof PG.3's ① held: the update check is
//! Rust-side, so nothing on this surface makes a frontend network request that
//! `connect-src 'self'` would refuse. The manual "Check for updates" button is
//! deliberately NOT clicked (it needs the network); the auto-check is off by
//! default, so opening the tab issues no request. The version arriving as
//! "1.0.0" also proves the `app_version` command end-to-end, and the notices
//! list proves the committed attribution JSON parses and renders natively.
//!
//! `#[ignore]` by default; run via `mise e2e`.

mod common;

use std::time::Duration;

use anyhow::{Context, Result};
use thirtyfour::By;

use common::*;
use vedge_e2e::{Session, TestEnv, app_binary};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn about_tab_renders_version_licence_and_notices() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;

    // Settings ▸ About.
    session
        .click_testid("nav-settings")
        .await
        .context("open settings")?;
    session
        .click_button_text("About")
        .await
        .context("open the About tab")?;
    session
        .wait_for(
            By::Css("[data-testid='about-version']"),
            Duration::from_secs(10),
        )
        .await
        .context("About tab renders")?;

    // Version — `app.package_info().version` via the Rust `app_version` command,
    // filled in async on mount.
    wait_until(Duration::from_secs(10), || async {
        Ok(session
            .by_testid("about-version")
            .await?
            .text()
            .await?
            .trim()
            == "1.0.0")
    })
    .await
    .context("About shows the bundled version 1.0.0")?;

    // Licence — MIT.
    let licence = session.by_testid("about-licence").await?.text().await?;
    assert!(
        licence.contains("MIT"),
        "About shows the MIT licence, got: {licence:?}"
    );

    // Third-party notices — the attribution list, rendered natively from the
    // committed JSON (no inner_html). MIT is the dominant licence, so it must
    // appear.
    let notices = session.by_testid("about-third-party").await?.text().await?;
    assert!(
        notices.contains("MIT"),
        "About renders the third-party notices list ({}-char block)",
        notices.len()
    );

    // 🔴 The standing proof ① held: no CSP violation was logged rendering this
    // surface (a frontend fetch to GitHub would trip `connect-src 'self'` here).
    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}
