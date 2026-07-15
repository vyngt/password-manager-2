//! **Open a backup** e2e scenarios (slice 5.2.2) — the SPACE half of Decision ⑦.
//!
//! 🔴 **Fully UI-driven.** The `.vbk` is produced by an `invoke` (that is *setup*: the "Back up
//! now" button opens a native save dialog, which no WebDriver can automate), but every mutation
//! **under test** goes through the interface a person actually uses — the launch-screen CTA, the
//! typed archive path, the name + location fields, the preview, and the button.
//!
//! That is only possible because the archive path is a real `Input` with a Browse button beside
//! it (the same pattern 5.2.0 gave the create wizard's Location field). A Browse-only field would
//! have made this flow untestable — and *untestable* is how 5.2 shipped a restore button that no
//! user could reach on the one vault that needed it (finding H0).
//!
//! `#[ignore]` by default; run via `mise e2e`.

mod common;

use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::json;
use thirtyfour::prelude::*;

use common::*;
use vedge_e2e::{MASTER_PASSWORD, Session, TestEnv, app_binary};

/// Drive the Open-a-backup dialog: CTA → archive → name + location → preview.
/// Leaves the dialog at step 2 with the preview rendered.
async fn open_dialog_to_preview(session: &Session, archive: &str, dest_home: &str) -> Result<()> {
    let (location, name) = split_home(dest_home);
    session
        .click_testid("open-backup")
        .await
        .context("the launch screen's Open-a-backup CTA")?;
    session.fill_id("backup-archive", archive).await?;
    session.fill_id("backup-name", &name).await?;
    session.fill_id("backup-location", &location).await?;
    session
        .click_testid("backup-preview-continue")
        .await
        .context("preview")?;
    session
        .wait_for(
            By::Css("[data-testid='backup-preview']"),
            Duration::from_secs(20),
        )
        .await
        .context("the preview must render before anything can be opened")?;
    Ok(())
}

/// 🔴 **The acceptance test.** A `.vbk` becomes a working vault at a fresh path, entirely
/// through the UI — the flow a person on a new machine performs. Then it unlocks, and the entry
/// is there.
///
/// It also exercises ② for free: the source vault is still live, so this is a **duplicate** —
/// the copy must be given a fresh identity, and its Secret Key carried across so it opens on the
/// master password alone.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn open_backup_to_new_path_via_ui() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    let backup = format!("{vault}.vbk");
    let copy = format!("{vault}-copy.vedge");

    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;
    add_login(&session, "keeper", "u", "p").await?;

    // Setup only: produce the archive. (Its button is behind a native save dialog.)
    session
        .invoke(
            "backup_vault",
            json!({ "vault_path": vault, "dest_path": backup }),
        )
        .await
        .context("backup_vault")?;

    // Back to the launch screen with the original still present and healthy.
    session.click_testid("vault-lock").await.context("lock")?;
    assert_unlocked(&session, &vault, false).await?;

    // ---- everything from here is what a person clicks ----
    open_dialog_to_preview(&session, &backup, &copy).await?;

    // ② The preview must SAY this is a copy — the user is about to end up with two vaults, and
    // being told that afterwards is not the same as being told it beforehand.
    session
        .wait_for(
            By::Css("[data-testid='backup-duplicate']"),
            Duration::from_secs(10),
        )
        .await
        .context("a copy of a live vault must be announced as a copy")?;

    session
        .click_testid("open-backup-confirm")
        .await
        .context("open")?;

    // The copy appears in the picker and is openable.
    wait_until(Duration::from_secs(30), || async {
        let rows: Vec<serde_json::Value> = session
            .invoke("list_registered_vaults_with_status", json!({}))
            .await
            .ok()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        Ok(rows.iter().any(|r| {
            r.get("path").and_then(|p| p.as_str()) == Some(copy.as_str())
                && r.get("openable").and_then(serde_json::Value::as_bool) == Some(true)
        }))
    })
    .await
    .context("the opened backup must show up in the picker, openable")?;

    // ② The copy opens with the MASTER PASSWORD ALONE — its Secret Key was carried across to the
    // new identity. (Had it not been, this unlock would fall through to the Emergency Kit panel.)
    let rows = session
        .driver()
        .find_all(By::Css("[role='option']"))
        .await?;
    for row in rows {
        if row.text().await?.contains("-copy") {
            row.click().await?;
            break;
        }
    }
    session.fill_id("master-password", MASTER_PASSWORD).await?;
    session.click_testid("unlock-submit").await?;
    assert_unlocked(&session, &copy, true).await?;

    let entries = list_entries(&session, &copy).await?;
    assert!(
        entry_id(&entries, "keeper").is_some(),
        "the opened copy must hold the backed-up entry"
    );

    // 🔴 And the ORIGINAL is untouched — opening a backup never reaches over and changes anything
    // you already had.
    //
    // Asserted through `inspect_backup`, which is read-only and needs NO session: the original is
    // locked right now, so `list_entries` would simply error, and an assertion that tolerates an
    // error would pass even if the vault had been deleted outright. This reads the live vault's
    // own identity and entry count off disk.
    let preview = session
        .invoke(
            "inspect_backup",
            json!({ "archive_path": backup, "target_vault": vault, "dest_home": null }),
        )
        .await
        .context("inspect the original after opening a copy of it")?;
    assert_eq!(
        preview.get("target_state").and_then(|v| v.as_str()),
        Some("readable"),
        "the original vault must still be readable after a copy of it was opened"
    );
    assert_eq!(
        preview.get("target_entry_count").and_then(|v| v.as_u64()),
        Some(1),
        "the original must still hold its entry — opening a backup touches nothing you had"
    );

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}

/// 🔴 **Open backup cannot overwrite. Full stop.**
///
/// Point it at the live vault's own path. The preview must BLOCK — and the confirm button must be
/// unclickable, because there is no "do it anyway". The moment a "just this once" override
/// appears here, the boundary that deleted three of 5.2's four confirmation dialogs is gone.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn open_backup_refuses_occupied_destination() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    let backup = format!("{vault}.vbk");

    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;
    add_login(&session, "keeper", "u", "p").await?;
    session
        .invoke(
            "backup_vault",
            json!({ "vault_path": vault, "dest_path": backup }),
        )
        .await
        .context("backup_vault")?;
    session.click_testid("vault-lock").await.context("lock")?;
    assert_unlocked(&session, &vault, false).await?;

    // Aim the destination squarely at the existing vault.
    open_dialog_to_preview(&session, &backup, &vault).await?;

    session
        .wait_for(
            By::Css("[data-testid='backup-blocked']"),
            Duration::from_secs(10),
        )
        .await
        .context("an occupied destination must be shown as BLOCKED, not merely warned about")?;

    // The action is refused, not "confirmable": the button exists but is disabled.
    let confirm = session
        .driver()
        .find(By::Css("[data-testid='open-backup-confirm']"))
        .await?;
    assert!(
        !confirm.is_enabled().await?,
        "the Open button must be DISABLED over an occupied path — there is no override"
    );

    // Close, unlock the original, and prove it is entirely intact.
    session.close_dialog().await?;
    unlock_ui(&session, &vault).await?;
    let entries = list_entries(&session, &vault).await?;
    assert!(
        entry_id(&entries, "keeper").is_some(),
        "the existing vault must be untouched by a refused open"
    );

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}
