//! In-app help e2e (slice PG.5a): the pre-unlock launch Help surface, the
//! first-run welcome, and the launch-screen version.
//!
//! The launch screen is exactly where a locked-out user needs help, so this
//! drives it BEFORE any unlock: a fresh env has an empty registry, so the
//! redesigned first-run welcome renders; the passive version line shows `1.0.0`
//! (proving the offline `app_version` command reaches the pre-unlock screen with
//! no capability/network); and the Help door opens a Dialog answering the four
//! recovery questions. `assert_console_clean` is the standing proof the help
//! surface added no network request `connect-src 'self'` would refuse and no
//! owner-less reactive warning.
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
async fn launch_help_surface_shows_welcome_version_and_topics() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;

    // Fresh env → empty registry → the redesigned first-run welcome.
    let title = session
        .wait_for(By::Css(".empty-state__title"), Duration::from_secs(10))
        .await
        .context("the first-run welcome renders on the empty registry")?
        .text()
        .await?;
    assert!(
        title.contains("Welcome to VEdge"),
        "welcome title, got: {title:?}"
    );

    // The passive version line renders pre-unlock (offline `app_version`, no
    // capability/network) — the launch-screen version deferred from PG.3/PG.4.
    wait_until(Duration::from_secs(10), || async {
        Ok(session
            .by_testid("launch-version")
            .await?
            .text()
            .await?
            .trim()
            == "1.0.0")
    })
    .await
    .context("launch footer shows the bundled version 1.0.0 pre-unlock")?;

    // The Help door opens the four-topic recovery dialog.
    session
        .click_testid("launch-help")
        .await
        .context("open the launch Help dialog")?;
    let help = session
        .wait_for(
            By::Css("[data-testid='launch-help-dialog']"),
            Duration::from_secs(10),
        )
        .await
        .context("the launch Help dialog renders")?
        .text()
        .await?;
    // The four topics answer the recovery questions — both the Emergency Kit and
    // Recovery Kit questions must be present.
    assert!(
        help.contains("Emergency Kit") && help.contains("Recovery Kit"),
        "Help dialog answers the recovery questions, got: {help:?}"
    );

    // 🔴 The standing proof: no CSP violation / owner-less warning rendering the
    // pre-unlock help surface (help makes no network request).
    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}
