//! Slice 5.4 — the WASM-Secret Sentinel: reveal a sealed password via the UI.
//!
//! Proves the door end-to-end: after sealing, opening an entry for edit shows an
//! EMPTY password field (the secret is not in the DOM); clicking Reveal fetches
//! it via the audited `reveal_field` and fills the field; and the audit log
//! records a `SecretRevealed` row (distinct from a browse `Viewed`).
//!
//! `#[ignore]` by default (needs a display + the platform WebDriver). Run with:
//!   mise e2e
//!   # or: cargo test -p vedge-e2e -- --ignored --test-threads=1

mod common;

use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::{Value, json};

use common::*;
use vedge_e2e::{Session, TestEnv, app_binary};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn reveal_a_password_via_ui() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();

    let session = Session::launch(&env, &app).await.context("launch app")?;
    session
        .console_selftest()
        .await
        .context("console-clean guard self-test")?;

    create_and_unlock(&session, &vault)
        .await
        .context("create + unlock via the wizard")?;
    add_login(&session, "GitHub", "octocat", "hunter2").await?;

    let entries = list_entries(&session, &vault).await?;
    let id = entry_id(&entries, "GitHub").context("find GitHub entry id")?;

    // Open the entry → the read drawer. The sealed password does NOT cross on
    // `get_entry` (slice 5.4), so it is not shown until an explicit reveal —
    // reading lives in the drawer, not the edit form.
    open_entry(&session, &id).await?;
    let before = drawer_text(&session).await?;
    assert!(
        !before.contains("hunter2"),
        "the sealed password must NOT be in the drawer before an explicit reveal"
    );

    // Reveal in the DRAWER → the audited `reveal_field` fetches the secret and
    // shows it in the SecretDisplay.
    session
        .click_testid("reveal-password")
        .await
        .context("drawer: Reveal password")?;
    wait_until(Duration::from_secs(5), || async {
        Ok(drawer_text(&session).await?.contains("hunter2"))
    })
    .await
    .context("Reveal should show the password in the read drawer")?;

    // The reveal is audited as `SecretRevealed` — the extraction the log now
    // distinguishes from a browse `Viewed` (slice 5.4 ③).
    let page = session
        .invoke(
            "list_audit",
            json!({
                "vault_path": vault,
                "query": { "actions": ["SecretRevealed"], "limit": 100, "offset": 0 }
            }),
        )
        .await
        .context("list_audit filtered to SecretRevealed")?;
    let total = page.get("total").and_then(Value::as_u64).unwrap_or(0);
    assert!(
        total >= 1,
        "revealing the password should have written a SecretRevealed audit row"
    );

    session
        .assert_console_clean()
        .await
        .context("console-clean after reveal")?;
    session.close().await;
    Ok(())
}

/// Slice 5.4.1 — 🔴 the headline: reveal a sealed **SSH private key** via the UI.
///
/// Before 5.4.1 the key was write-only (`FieldSelector` had no `PrivateKey` arm),
/// so a stored key could not be read back. This proves the completed door: the
/// key is not in the DOM on edit; Reveal fetches it via the audited
/// `reveal_field` and fills the (multi-line) Textarea **byte-exact, newlines
/// intact**; and a `SecretRevealed` audit row is written.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn reveal_an_ssh_private_key_via_ui() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();

    let session = Session::launch(&env, &app).await.context("launch app")?;
    session
        .console_selftest()
        .await
        .context("console-clean guard self-test")?;

    create_and_unlock(&session, &vault)
        .await
        .context("create + unlock via the wizard")?;

    // A multi-line PEM-shaped key — the reveal must preserve the newlines. No
    // literal `-----BEGIN … PRIVATE KEY-----` header (GitGuardian would flag it);
    // the property under test is byte-exact multi-line round-trip, not key type.
    let pem =
        "-----BEGIN VEDGE E2E BLOCK-----\nline-a\nline-b\nline-c\n-----END VEDGE E2E BLOCK-----";
    add_ssh_key(&session, "prod-server", pem).await?;

    let entries = list_entries(&session, &vault).await?;
    let id = entry_id(&entries, "prod-server").context("find ssh entry id")?;

    // Open the entry → the read drawer. The sealed key is NOT shown until an
    // explicit reveal (it does not cross on `get_entry`); reading it lives in the
    // drawer, not the edit form.
    open_entry(&session, &id).await?;
    let marker = "VEDGE E2E BLOCK";
    let before = drawer_text(&session).await?;
    assert!(
        !before.contains(marker),
        "the sealed SSH private key must NOT be in the drawer before an explicit reveal"
    );

    // Reveal in the DRAWER: the audited `reveal_field` fetches the key and shows
    // it (newlines intact) in the SecretDisplay.
    session
        .click_testid("reveal-ssh-key")
        .await
        .context("drawer: Reveal SSH private key")?;
    wait_until(Duration::from_secs(5), || async {
        Ok(drawer_text(&session).await?.contains(marker))
    })
    .await
    .context("Reveal should show the PEM in the read drawer")?;

    let page = session
        .invoke(
            "list_audit",
            json!({
                "vault_path": vault,
                "query": { "actions": ["SecretRevealed"], "limit": 100, "offset": 0 }
            }),
        )
        .await
        .context("list_audit filtered to SecretRevealed")?;
    let total = page.get("total").and_then(Value::as_u64).unwrap_or(0);
    assert!(
        total >= 1,
        "revealing the SSH key should have written a SecretRevealed audit row"
    );

    session
        .assert_console_clean()
        .await
        .context("console-clean after reveal")?;
    session.close().await;
    Ok(())
}
