//! **Replace** e2e scenarios (slices 5.2c / 5.2.2) — the destructive escape hatch.
//!
//! 🔴 The everyday verb, *Open backup*, has its own suite in `tests/open_backup.rs`, and it is
//! fully UI-driven. This file covers the one that overwrites.
//!
//! `backup_vault` is driven through `invoke` here purely as **setup** (its Settings button opens
//! a native save dialog, which a WebDriver cannot automate). Every mutation *under test* — the
//! lock, the replace, the re-unlock — is either UI-driven or the command being asserted.
//!
//! `#[ignore]` by default; run via `mise e2e`.

mod common;

use anyhow::{Context, Result};
use serde_json::json;

use common::*;
use vedge_e2e::{Session, TestEnv, app_binary};

/// Back up a vault, move it forward, then REPLACE it with the older backup: the vault reverts to
/// the backed-up state (a confirmed rollback), and — ⑭ — an undo point is left behind.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn backup_and_replace() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    let backup = format!("{vault}.vbk");

    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;

    // Seed an entry, then snapshot the vault to a `.vbk` (read-only op via invoke).
    add_login(&session, "keeper", "u", "p").await?;
    session
        .invoke(
            "backup_vault",
            json!({ "vault_path": vault, "dest_path": backup }),
        )
        .await
        .context("backup_vault")?;

    // Move the live vault FORWARD of the backup (so restoring it is a rollback).
    add_login(&session, "extra", "u", "p").await?;

    // Lock, then replace with the older backup (confirmed rollback; same credentials, and the
    // target reads fine, so the other two acknowledgements are not required).
    session.click_testid("vault-lock").await.context("lock")?;
    assert_unlocked(&session, &vault, false).await?;
    let report = session
        .invoke(
            "replace_vault_from_backup",
            json!({
                "vault_path": vault,
                "archive_path": backup,
                "confirm_rollback": true,
                "confirm_credential_change": false,
                "confirm_unverified_target": false,
            }),
        )
        .await
        .context("replace_vault_from_backup")?;

    // ⑭ Even the escape hatch is undoable: a `pre-restore` snapshot of what we just destroyed.
    assert!(
        report.get("undo_snapshot_id").is_some_and(|v| !v.is_null()),
        "a replace must leave an undo point behind, got {report:?}"
    );

    // Re-unlock through the UI and confirm the vault is back at the backed-up state:
    // "keeper" is present, "extra" (added after the backup) is gone.
    unlock_ui(&session, &vault).await?;
    let entries = list_entries(&session, &vault).await?;
    assert!(
        entry_id(&entries, "keeper").is_some(),
        "the backed-up entry must survive the replace"
    );
    assert!(
        entry_id(&entries, "extra").is_none(),
        "an entry added after the backup must be gone after replacing with it"
    );

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}

/// Replace refuses an UNLOCKED target: the command errors and the live vault is untouched.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn replace_refuses_unlocked_vault() -> Result<()> {
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

    // Replace WITHOUT locking first → the command must refuse.
    let refused = session
        .invoke(
            "replace_vault_from_backup",
            json!({
                "vault_path": vault,
                "archive_path": backup,
                "confirm_rollback": true,
                "confirm_credential_change": true,
                "confirm_unverified_target": true,
            }),
        )
        .await;
    assert!(
        refused.is_err(),
        "replacing an unlocked vault must be refused, got {refused:?}"
    );

    // The vault stays unlocked and intact.
    assert_unlocked(&session, &vault, true).await?;
    let entries = list_entries(&session, &vault).await?;
    assert!(
        entry_id(&entries, "keeper").is_some(),
        "a refused replace must leave the live vault untouched"
    );

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}
