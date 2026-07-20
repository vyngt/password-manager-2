//! Screenshot capture for the PG.5b external user guide.
//!
//! NOT a correctness test — a *generator*. It drives the real app through the
//! same `tauri-driver` + `thirtyfour` harness the e2e suite uses, walks every
//! state the guide documents, and writes a PNG per figure into
//! `docs/guide/images/` (override with `VEDGE_SHOTS_DIR`). Re-running it *is* the
//! screenshot refresh process (PG.5b Decision ④) — no manual capture, no drift.
//!
//! Why this is faithful chrome: VEdge is frameless (`decorations: false`) with
//! its titlebar rendered *inside* the webview, so a WebDriver viewport capture is
//! the whole app surface as the user sees it (no OS frame to miss).
//!
//! Sample data is **curated but disposable** — a throwaway e2e temp vault, so any
//! Secret-Key / kit string shown on the create screen is a one-run throwaway,
//! never a real secret.
//!
//! Robustness: each section is best-effort. A missing control logs and the run
//! continues, so one drifted selector never costs the whole (expensive) capture.
//! Only the create-and-unlock prerequisite is fatal — nothing else can run
//! without an open vault.
//!
//! `#[ignore]` by default; run via `mise screenshots`.

mod common;

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use thirtyfour::By;

use common::*;
use vedge_e2e::{MASTER_PASSWORD, Session, TestEnv, app_binary};

/// Where the PNGs land: `$VEDGE_SHOTS_DIR`, else `<repo>/docs/guide/images`.
fn shots_dir() -> Result<PathBuf> {
    let dir = if let Some(d) = std::env::var_os("VEDGE_SHOTS_DIR") {
        PathBuf::from(d)
    } else {
        // This crate is `crates/vedge-e2e`; the repo root is two levels up.
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest
            .ancestors()
            .nth(2)
            .context("cannot locate repo root from CARGO_MANIFEST_DIR")?
            .join("docs")
            .join("guide")
            .join("images")
    };
    std::fs::create_dir_all(&dir).with_context(|| format!("create shots dir {}", dir.display()))?;
    Ok(dir)
}

/// Capture the full viewport to `<dir>/<name>.png`, after a short settle so any
/// open/close animation has finished.
///
/// Parks the pointer in the top-left corner first so a hover tooltip from the
/// control we just clicked (e.g. a sidebar nav icon) doesn't bleed into the shot.
async fn shot(s: &Session, dir: &Path, name: &str) -> Result<()> {
    let _ = s.driver().action_chain().move_to(2, 2).perform().await;
    tokio::time::sleep(Duration::from_millis(450)).await;
    let path = dir.join(format!("{name}.png"));
    s.driver()
        .screenshot(&path)
        .await
        .with_context(|| format!("capture {name}"))?;
    eprintln!("  ✓ {}", path.display());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: captures the user-guide screenshots into docs/guide/images; run via `mise screenshots`"]
async fn capture_user_guide_screenshots() -> Result<()> {
    let dir = shots_dir()?;
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    let session = Session::launch(&env, &app).await?;

    // Best-effort: a taller window so scrollable pages (settings, health) capture
    // more than the default 600px. If tauri-driver ignores it, the viewport shots
    // are still correct, just shorter.
    let _ = session.driver().set_window_rect(0, 0, 1180, 1000).await;

    let mut failures: Vec<String> = Vec::new();
    macro_rules! section {
        ($label:literal, $body:expr) => {
            if let Err(e) = $body.await {
                eprintln!("  ✗ {}: {e:#}", $label);
                failures.push(format!("{}: {e:#}", $label));
            }
        };
    }

    // 1. Pre-unlock launch surface (only reachable before a vault exists).
    section!("launch", capture_launch(&session, &dir));

    // 2. Create + unlock — FATAL prerequisite; also captures the wizard + kit.
    onboarding_and_unlock(&session, &dir, &vault)
        .await
        .context("create-and-unlock prerequisite failed — no later state is reachable")?;

    // 3. Seed a small, realistic vault for the list / drawer / search shots.
    section!("seed", seed_entries(&session));

    // 4. Vault page states.
    section!("vault", capture_vault(&session, &dir, &vault));
    // 5. Generator.
    section!("generator", capture_generator(&session, &dir));
    // 6. Health + audit pages.
    section!("pages", capture_pages(&session, &dir));
    // 7. Snapshots.
    section!("snapshots", capture_snapshots(&session, &dir));
    // 8. The six settings tabs.
    section!("settings", capture_settings(&session, &dir));
    // 9. Credential-operation dialogs (incl. the corrected rotate copy).
    section!("credentials", capture_credentials(&session, &dir));

    eprintln!("\nScreenshots written to {}", dir.display());
    if failures.is_empty() {
        eprintln!("All sections captured.");
    } else {
        eprintln!(
            "{} section(s) had issues (other shots still captured):",
            failures.len()
        );
        for f in &failures {
            eprintln!("  - {f}");
        }
    }

    session.close().await;
    Ok(())
}

/// Welcome (no-vaults) + the pre-unlock Help dialog.
async fn capture_launch(s: &Session, dir: &Path) -> Result<()> {
    // A generous wait — this is the very first DOM after boot, which can lag the
    // IPC-ready signal under load; 10s occasionally lost the race.
    s.wait_for(By::Css(".empty-state__title"), Duration::from_secs(25))
        .await
        .context("first-run welcome")?;
    shot(s, dir, "01-welcome").await?;

    // The launch Help door (answers the four recovery questions pre-unlock).
    s.click_testid("launch-help").await.context("open Help")?;
    s.wait_for(
        By::Css("[data-testid='launch-help-dialog']"),
        Duration::from_secs(10),
    )
    .await
    .context("launch Help dialog")?;
    shot(s, dir, "02-launch-help").await?;
    s.close_dialog().await.ok();
    Ok(())
}

/// Drive the create wizard, capturing step 1, the master-password step, and the
/// Secret-Key (Emergency Kit) screen, then finish → unlocked.
async fn onboarding_and_unlock(s: &Session, dir: &Path, vault: &str) -> Result<()> {
    let (location, name) = split_home(vault);
    s.click_testid("launch-new-vault")
        .await
        .context("New vault")?;
    s.fill_id("vault-name", &name).await?;
    s.fill_id("vault-location", &location).await?;
    shot(s, dir, "03-create-vault").await.ok();

    s.click_testid("onboarding-next").await.context("Next")?;
    s.fill_id("master-password", MASTER_PASSWORD).await?;
    s.fill_id("master-password-confirm", MASTER_PASSWORD)
        .await?;
    shot(s, dir, "04-master-password").await.ok();

    s.click_testid("onboarding-create")
        .await
        .context("Create vault")?;
    s.wait_for(
        By::Css("[data-testid='onboarding-ack']"),
        Duration::from_secs(45),
    )
    .await
    .context("Secret Key screen")?;
    shot(s, dir, "05-secret-key").await.ok();

    s.js_click_testid("onboarding-ack").await?;
    s.click_testid("onboarding-finish")
        .await
        .context("Finish")?;
    assert_unlocked(s, vault, true).await?;
    Ok(())
}

/// A handful of varied, realistic-looking entries so the list/drawer look real.
async fn seed_entries(s: &Session) -> Result<()> {
    add_login(s, "GitHub", "octocat", "gT7!qm2Zx9$Lw0pR").await?;
    add_login(s, "Gmail", "you@gmail.com", "Vb4#nK8sQ1!eR6tY").await?;
    add_login(s, "AWS Console", "admin", "Pz9@wD3fH7!kL2mN").await?;
    add_ssh_key(
        s,
        "prod-server",
        "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAA (sample — throwaway)\n-----END OPENSSH PRIVATE KEY-----",
    )
    .await?;
    add_note(
        s,
        "Wi-Fi & recovery notes",
        "Home network: VEdge-Guest\nRouter admin: 192.168.1.1",
    )
    .await?;
    Ok(())
}

/// Vault list, a detail drawer, the new-entry form, and a filtered search.
async fn capture_vault(s: &Session, dir: &Path, vault: &str) -> Result<()> {
    s.close_dialog().await.ok();
    s.click_testid("nav-vault").await.ok();
    wait_row_count(s, 5, Duration::from_secs(10)).await.ok();
    shot(s, dir, "10-vault-list").await?;

    // Detail drawer on the first entry.
    let entries = list_entries(s, vault).await.unwrap_or_default();
    if let Some(id) = entries
        .first()
        .and_then(|e| e.get("id"))
        .and_then(|v| v.as_str())
    {
        open_entry(s, id).await.context("open drawer")?;
        s.wait_for(By::Css("[data-detail-drawer]"), Duration::from_secs(5))
            .await
            .ok();
        shot(s, dir, "11-entry-detail").await?;
        s.click_testid("nav-vault").await.ok(); // dismiss the drawer
    }

    // The new-entry form (Login type), lightly filled so it reads as real.
    s.click_testid("vault-new-entry").await.context("+ New")?;
    s.by_id("ef-name").await.context("entry form")?;
    s.fill_id("ef-name", "Cloudflare").await.ok();
    s.fill_id("ef-username", "you@example.com").await.ok();
    shot(s, dir, "12-entry-form").await?;
    s.close_dialog().await.ok();

    // Filtered search.
    s.fill_id("vault-search", "git").await.ok();
    shot(s, dir, "13-search").await?;
    s.fill_id("vault-search", "").await.ok();
    Ok(())
}

/// The password generator panel (opened via the inline Login-password caret).
async fn capture_generator(s: &Session, dir: &Path) -> Result<()> {
    s.close_dialog().await.ok();
    open_generator_panel(s).await.context("open generator")?;
    shot(s, dir, "20-generator").await?;
    s.close_dialog().await.ok(); // generator dialog
    s.close_dialog().await.ok(); // the new-entry form beneath it
    Ok(())
}

/// Health + audit pages.
async fn capture_pages(s: &Session, dir: &Path) -> Result<()> {
    s.close_dialog().await.ok();
    s.click_testid("nav-health").await.context("nav health")?;
    s.wait_for(
        By::Css("[data-testid='health-table']"),
        Duration::from_secs(10),
    )
    .await
    .ok();
    shot(s, dir, "30-health").await?;

    s.click_testid("nav-audit").await.context("nav audit")?;
    s.wait_for(
        By::Css("[data-testid='audit-table']"),
        Duration::from_secs(10),
    )
    .await
    .ok();
    shot(s, dir, "31-audit").await?;
    Ok(())
}

/// The snapshots page (take one so a row is present).
async fn capture_snapshots(s: &Session, dir: &Path) -> Result<()> {
    s.close_dialog().await.ok();
    s.click_testid("nav-snapshots")
        .await
        .context("nav snapshots")?;
    s.wait_for(
        By::Css("[data-testid='snapshots-page']"),
        Duration::from_secs(10),
    )
    .await
    .context("snapshots page")?;
    s.click_testid("snapshot-take").await.ok();
    s.wait_for(
        By::Css("[data-testid='snapshot-row']"),
        Duration::from_secs(15),
    )
    .await
    .ok();
    shot(s, dir, "40-snapshots").await?;
    Ok(())
}

/// All six settings tabs.
async fn capture_settings(s: &Session, dir: &Path) -> Result<()> {
    s.close_dialog().await.ok();
    s.click_testid("nav-settings")
        .await
        .context("nav settings")?;
    // Security is the default active tab.
    tokio::time::sleep(Duration::from_millis(300)).await;
    shot(s, dir, "50-settings-security").await?;

    for (label, name) in [
        ("Credentials", "51-settings-credentials"),
        ("Appearance", "52-settings-appearance"),
        ("Maintenance", "53-settings-maintenance"),
        ("Backup", "54-settings-backup"),
        ("About", "55-settings-about"),
    ] {
        s.click_button_text(label)
            .await
            .with_context(|| format!("open {label} tab"))?;
        if label == "About" {
            s.wait_for(
                By::Css("[data-testid='about-version']"),
                Duration::from_secs(10),
            )
            .await
            .ok();
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
        shot(s, dir, name).await?;
    }
    Ok(())
}

/// The credential-operation dialogs on the Credentials tab — including the rotate
/// dialog, which carries the corrected keep-your-old-kit copy (PG.5b Part A).
async fn capture_credentials(s: &Session, dir: &Path) -> Result<()> {
    s.close_dialog().await.ok();
    s.click_testid("nav-settings")
        .await
        .context("nav settings")?;
    s.click_button_text("Credentials")
        .await
        .context("Credentials tab")?;

    for (open, name) in [
        ("change-master-password", "60-change-password"),
        ("rotate-secret-key", "61-rotate-secret-key"),
        ("rekey-vault", "62-rekey"),
    ] {
        s.click_testid(open)
            .await
            .with_context(|| format!("open {open} dialog"))?;
        s.wait_for(By::Css("div[role='dialog']"), Duration::from_secs(5))
            .await
            .with_context(|| format!("{open} dialog"))?;
        shot(s, dir, name).await?;
        s.close_dialog().await.ok();
    }
    Ok(())
}
