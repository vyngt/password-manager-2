//! Snapshot e2e scenarios (slice 5.2.1) — driven entirely through the UI.
//!
//! 🔴 Unlike the `.vbk` `backup.rs` scenarios (which drive `restore_vault` via `invoke`
//! because it opens a NATIVE file dialog a WebDriver can't automate), snapshot revert has NO
//! native dialog — the `/v/snapshots` page and the launch-screen Restore action are pure
//! `data-testid` controls. So these are UI-driven end to end, including the H0 acceptance
//! test: restoring a deliberately CORRUPTED vault from the picker, with NO `invoke()` for the
//! mutation. (5.2c filed "e2e drives restore via invoke" as a drift note; this fixes it.)
//!
//! `#[ignore]` by default; run via `mise e2e`.

mod common;

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::Value;
use thirtyfour::By;

use common::*;
use vedge_e2e::{Session, TestEnv, app_binary};

/// How many active entries carry this name (the tweezers recovers a fresh copy).
fn count_named(entries: &[Value], name: &str) -> usize {
    entries
        .iter()
        .filter(|e| e.get("name").and_then(Value::as_str) == Some(name))
        .count()
}

/// Overwrite a vault home's `vault.vdb` header so it no longer opens as a database — the
/// filesystem seam the H0 disaster test needs (no `invoke`, just the harness touching disk).
/// Retries briefly: on Windows a just-locked `.vdb` handle can linger for a moment.
fn corrupt_vault_db(home: &str) -> Result<()> {
    let vdb = Path::new(home).join("vault.vdb");
    let mut last = None;
    for _ in 0..20 {
        match std::fs::write(
            &vdb,
            b"NOT A SQLITE DATABASE -- corrupted by the e2e harness",
        ) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last = Some(e);
                std::thread::sleep(Duration::from_millis(150));
            }
        }
    }
    Err(last.map_or_else(
        || anyhow::anyhow!("corrupt vault.vdb: unknown"),
        |e| anyhow::anyhow!("corrupt vault.vdb: {e}"),
    ))
}

/// The happy path: take a snapshot, move the vault forward, then revert to the snapshot —
/// all through the `/v/snapshots` page. Since slice 5.2.3 (Decision ⑰) the revert is SEAMLESS:
/// the user STAYS in the vault (no eject to the launch screen, no re-unlock), and the entry
/// added after the snapshot is gone.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn snapshot_and_revert_via_ui() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();

    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;
    add_login(&session, "keeper", "u", "p").await?;

    // Take a snapshot on /v/snapshots.
    session
        .click_testid("nav-snapshots")
        .await
        .context("nav snapshots")?;
    session
        .wait_for(
            By::Css("[data-testid='snapshots-page']".to_string()),
            Duration::from_secs(5),
        )
        .await
        .context("snapshots page")?;
    session
        .click_testid("snapshot-take")
        .await
        .context("take snapshot")?;
    session
        .wait_for(
            By::Css("[data-testid='snapshot-row']".to_string()),
            Duration::from_secs(10),
        )
        .await
        .context("snapshot row appears")?;

    // Move the vault FORWARD of the snapshot.
    session
        .click_testid("nav-vault")
        .await
        .context("nav vault")?;
    add_login(&session, "extra", "u", "p").await?;

    // Revert to the snapshot (inline confirm).
    session
        .click_testid("nav-snapshots")
        .await
        .context("nav snapshots again")?;
    session
        .wait_for(
            By::Css("[data-testid='snapshot-revert']".to_string()),
            Duration::from_secs(5),
        )
        .await
        .context("revert button")?;
    session
        .click_testid("snapshot-revert")
        .await
        .context("arm revert confirm")?;
    session
        .wait_for(
            By::Css("[data-testid='snapshot-revert-confirm']".to_string()),
            Duration::from_secs(5),
        )
        .await
        .context("revert confirm button")?;
    session
        .click_testid("snapshot-revert-confirm")
        .await
        .context("confirm revert")?;

    // 🔴 Seamless (5.2.3): the revert reflects the snapshot WITHOUT ejecting to the launch
    // screen. Poll the entries — `list_entries` blocks on the session guard while the swap runs,
    // so it returns only once the reverted session is in place: "keeper" back, "extra" gone.
    wait_until(Duration::from_secs(30), || async {
        let entries = list_entries(&session, &vault).await.unwrap_or_default();
        Ok(entry_id(&entries, "keeper").is_some() && entry_id(&entries, "extra").is_none())
    })
    .await
    .context("the seamless revert must reflect the snapshot without a re-unlock")?;
    // And the vault stayed unlocked the whole time — no password was ever re-entered.
    assert_unlocked(&session, &vault, true).await?;

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}

/// 🟠 B2 focus (slice 5.2.3, Decision ⑰): a revert from `/v/snapshots` keeps the user IN the
/// vault — the snapshots page stays mounted and no launch-screen password field ever appears.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn revert_stays_in_the_vault() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();

    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;
    add_login(&session, "keeper", "u", "p").await?;

    // Snapshot on /v/snapshots.
    session
        .click_testid("nav-snapshots")
        .await
        .context("nav snapshots")?;
    session
        .wait_for(
            By::Css("[data-testid='snapshots-page']".to_string()),
            Duration::from_secs(5),
        )
        .await
        .context("snapshots page")?;
    session
        .click_testid("snapshot-take")
        .await
        .context("take snapshot")?;
    session
        .wait_for(
            By::Css("[data-testid='snapshot-row']".to_string()),
            Duration::from_secs(10),
        )
        .await
        .context("snapshot row appears")?;

    // Revert (inline confirm).
    session
        .click_testid("snapshot-revert")
        .await
        .context("arm revert confirm")?;
    session
        .wait_for(
            By::Css("[data-testid='snapshot-revert-confirm']".to_string()),
            Duration::from_secs(5),
        )
        .await
        .context("revert confirm button")?;
    session
        .click_testid("snapshot-revert-confirm")
        .await
        .context("confirm revert")?;

    // The vault stays unlocked and the snapshots page stays mounted — no eject. Give the swap
    // time to finish, then assert both.
    assert_unlocked(&session, &vault, true).await?;
    session
        .wait_for(
            By::Css("[data-testid='snapshots-page']".to_string()),
            Duration::from_secs(10),
        )
        .await
        .context("the snapshots page must stay mounted — the revert did not eject to launch")?;
    // No launch-screen password field is present (the eject path would have shown one).
    assert!(
        session
            .driver()
            .find(By::Id("master-password"))
            .await
            .is_err(),
        "a seamless revert must not surface the launch-screen unlock prompt"
    );

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}

/// 🔴🔴 The acceptance test (H0): a CORRUPTED vault is restored from the picker, by clicking,
/// with no `invoke()`. Proves slice 5.2's DoD ("a corrupted `.vdb` is survivable") in the UI.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn revert_corrupt_vault_from_picker() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();

    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;
    add_login(&session, "keeper", "u", "p").await?;

    // Snapshot, then lock.
    session
        .click_testid("nav-snapshots")
        .await
        .context("nav snapshots")?;
    session
        .wait_for(
            By::Css("[data-testid='snapshots-page']".to_string()),
            Duration::from_secs(5),
        )
        .await
        .context("snapshots page")?;
    session
        .click_testid("snapshot-take")
        .await
        .context("take snapshot")?;
    session
        .wait_for(
            By::Css("[data-testid='snapshot-row']".to_string()),
            Duration::from_secs(10),
        )
        .await
        .context("snapshot row appears")?;
    session
        .click_testid("vault-lock")
        .await
        .context("lock the vault")?;
    assert_unlocked(&session, &vault, false).await?;

    // 🔴 Corrupt the vault's DB from the harness (NO invoke), then reload so the picker
    // re-probes it and marks it un-openable.
    corrupt_vault_db(&vault)?;
    session.driver().refresh().await.context("reload the app")?;

    // The picker offers Restore for the corrupt vault.
    session
        .wait_for(
            By::Css("[data-testid='vault-restore']".to_string()),
            Duration::from_secs(25),
        )
        .await
        .context("the corrupt vault must show a Restore action (H0)")?;
    session
        .click_testid("vault-restore")
        .await
        .context("click Restore on the picker")?;

    // Restore reverts to the newest snapshot; the vault becomes openable again (the Restore
    // action disappears). Then unlock and confirm the entry is recovered.
    wait_until(Duration::from_secs(30), || async {
        Ok(session
            .driver()
            .find(By::Css("[data-testid='vault-restore']"))
            .await
            .is_err())
    })
    .await
    .context("the vault must become openable after restore")?;
    unlock_ui(&session, &vault).await?;
    let entries = list_entries(&session, &vault).await?;
    assert!(
        entry_id(&entries, "keeper").is_some(),
        "the entry must be recovered from the snapshot"
    );

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}

/// 🟢 The tweezers (slice 5.3c): recover entries FROM a snapshot into the live vault,
/// UI-driven end to end — no native dialog. Add "keeper", snapshot it, then use the
/// snapshot's **Recover** action + preview table + Recover-selected to pull it back; the
/// recovered copy materializes in the live vault (a second active "keeper"). This is the
/// first import e2e — 5.3a/5.3b deferred theirs because their sources need native file
/// dialogs; the snapshot source has none.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn recover_one_entry_from_a_snapshot() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();

    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;
    add_login(&session, "keeper", "u", "p").await?;

    // Snapshot on /v/snapshots.
    session
        .click_testid("nav-snapshots")
        .await
        .context("nav snapshots")?;
    session
        .wait_for(
            By::Css("[data-testid='snapshots-page']".to_string()),
            Duration::from_secs(5),
        )
        .await
        .context("snapshots page")?;
    session
        .click_testid("snapshot-take")
        .await
        .context("take snapshot")?;
    session
        .wait_for(
            By::Css("[data-testid='snapshot-row']".to_string()),
            Duration::from_secs(10),
        )
        .await
        .context("snapshot row appears")?;

    // The tweezers: open the snapshot's entries, then Recover them into the live vault.
    session
        .click_testid("snapshot-recover")
        .await
        .context("open the recover preview")?;
    session
        .wait_for(
            By::Css("[data-testid='recover-preview-table']".to_string()),
            Duration::from_secs(10),
        )
        .await
        .context("recover preview table appears")?;
    session
        .click_testid("recover-commit")
        .await
        .context("recover selected")?;

    // The snapshot's "keeper" is recovered as a FRESH copy → two active "keeper" entries.
    wait_until(Duration::from_secs(30), || async {
        let entries = list_entries(&session, &vault).await.unwrap_or_default();
        Ok(count_named(&entries, "keeper") == 2)
    })
    .await
    .context("the recovered entry must appear in the live vault (a fresh copy)")?;

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}
