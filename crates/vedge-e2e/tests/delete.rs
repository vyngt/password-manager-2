//! Delete-vault e2e (slice 5.2.4) — driven entirely through the UI.
//!
//! The launch-screen `⋯` → vault-details dialog → type-to-confirm delete dialog is a pure
//! `data-testid` chain (no native dialog), so the destructive path is UI-driven end to end. The
//! one `invoke()` is a **read-only negative assertion**: the backend refuses a delete while the
//! vault is unlocked (it mutates nothing — the refusal is the point).
//!
//! `#[ignore]` by default; run via `mise e2e`.

mod common;

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::{Value, json};
use thirtyfour::By;

use common::*;
use vedge_e2e::{Session, TestEnv, app_binary};

/// 🔴🔴 Create a vault, snapshot it, lock it, then DELETE it from the picker: `⋯` → Delete vault…
/// → type the vault name → confirm. The row leaves the registry and the folder leaves the disk.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn delete_vault_via_ui() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();

    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;
    add_login(&session, "keeper", "u", "p").await?;

    // A snapshot, so the confirm dialog has snapshots to name.
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

    // 🔴 The backend refuses a delete while the vault is UNLOCKED (the guard behind the UI, which
    // locks first). A read-only negative `invoke` — a refused delete mutates nothing.
    let refused = session
        .invoke(
            "delete_vault",
            json!({ "vault_path": vault, "registry_id": Value::Null }),
        )
        .await;
    assert!(
        refused.is_err(),
        "delete_vault must refuse an unlocked vault"
    );

    // Lock → the launch screen.
    session
        .click_testid("vault-lock")
        .await
        .context("lock the vault")?;
    assert_unlocked(&session, &vault, false).await?;

    // The registry row's display name — what the confirm gate makes the user type.
    let rows = session
        .invoke("list_registered_vaults_with_status", json!({}))
        .await?;
    let name = rows
        .as_array()
        .and_then(|a| a.first())
        .and_then(|r| r.get("display_name"))
        .and_then(Value::as_str)
        .context("registry row display name")?
        .to_owned();

    // ⋯ → details dialog → Delete vault → confirm dialog → type the name → confirm.
    session
        .wait_for(
            By::Css("[data-testid='vault-menu']".to_string()),
            Duration::from_secs(20),
        )
        .await
        .context("the row's ⋯ menu button")?;
    session
        .click_testid("vault-menu")
        .await
        .context("open the vault menu")?;
    session
        .wait_for(
            By::Css("[data-testid='vault-details']".to_string()),
            Duration::from_secs(5),
        )
        .await
        .context("vault details dialog")?;
    session
        .click_testid("vault-delete-open")
        .await
        .context("Delete vault… in the details dialog")?;
    session
        .wait_for(
            By::Css("[data-testid='vault-delete-confirm']".to_string()),
            Duration::from_secs(5),
        )
        .await
        .context("delete confirm dialog")?;
    session
        .fill_id("vault-delete-input", &name)
        .await
        .context("type the vault name to confirm")?;
    session
        .click_testid("vault-delete")
        .await
        .context("confirm the delete")?;

    // The row leaves the registry (it was the only vault → the registry is now empty).
    wait_until(Duration::from_secs(30), || async {
        let rows = session
            .invoke("list_registered_vaults_with_status", json!({}))
            .await
            .unwrap_or(Value::Null);
        Ok(rows.as_array().is_some_and(|a| a.is_empty()))
    })
    .await
    .context("the deleted vault must leave the registry")?;

    // And the vault home is gone from disk.
    assert!(
        !Path::new(&vault).exists(),
        "the vault folder must be deleted from disk"
    );

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}
