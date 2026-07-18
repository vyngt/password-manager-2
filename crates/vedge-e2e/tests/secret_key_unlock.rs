//! Emergency-Kit recovery e2e scenario (slice 5.1): a user with only their master
//! password + the printed `A3-…` Secret Key can unlock from the launch screen,
//! plus the negative — a mangled key is rejected and stays locked.
//!
//! `#[ignore]` by default; run via `mise e2e`.

mod common;

use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::{Value, json};

use common::*;
use vedge_e2e::{MASTER_PASSWORD, Session, TestEnv, app_binary};

/// 5.1 — Emergency Kit recovery: a user with only their master password + the
/// printed `A3-…` Secret Key can unlock from the launch screen. (Keychain is
/// intact here; recovery is valid regardless — it parses, unlocks, and rewrites
/// the keychain.) Plus the negative: a mangled key is rejected and stays locked.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn secret_key_unlock() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    let session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;

    // Capture the vault's Secret Key display string (read-only; needs unlocked).
    let kit = session
        .invoke("export_emergency_kit", json!({ "vault_path": vault }))
        .await
        .context("export_emergency_kit")?;
    let secret_key = kit
        .get("secret_key_display")
        .and_then(Value::as_str)
        .context("secret_key_display in kit DTO")?
        .to_owned();

    // Lock, then recover from the picker via the Emergency Kit panel.
    session.click_testid("vault-lock").await.context("lock")?;
    assert_unlocked(&session, &vault, false).await?;

    // ---- Positive: master password + the real Secret Key → unlocked ----
    click_vault_option(&session).await?;
    session
        .click_testid("use-kit")
        .await
        .context("reveal Emergency Kit panel")?;
    session.fill_id("master-password", MASTER_PASSWORD).await?;
    session.fill_id("secret-key", &secret_key).await?;
    session
        .click_testid("secret-key-submit")
        .await
        .context("Recover & Unlock")?;
    assert_unlocked(&session, &vault, true).await?;

    // ---- Negative: mangle one data char → checksum fails, stays locked ----
    session.click_testid("vault-lock").await.context("relock")?;
    assert_unlocked(&session, &vault, false).await?;

    // Flip the last base32 data char to a non-confusable one (2↔3); the core
    // parser owns 0/O and 1/I/L, so those would normalize away — 2/3 do not.
    let mut chars: Vec<char> = secret_key.chars().collect();
    let pos = chars
        .iter()
        .rposition(char::is_ascii_alphanumeric)
        .context("no data char to mangle")?;
    chars[pos] = if chars[pos] == '2' { '3' } else { '2' };
    let mangled: String = chars.into_iter().collect();

    click_vault_option(&session).await?;
    session
        .click_testid("use-kit")
        .await
        .context("reveal Emergency Kit panel (negative)")?;
    session.fill_id("master-password", MASTER_PASSWORD).await?;
    session.fill_id("secret-key", &mangled).await?;
    session
        .click_testid("secret-key-submit")
        .await
        .context("Recover & Unlock (mangled)")?;
    // The checksum fails before the KDF, so this returns fast; give a hypothetical
    // (broken) unlock ample time to complete, then confirm we are still locked.
    tokio::time::sleep(Duration::from_secs(3)).await;
    let still = session
        .invoke("is_unlocked", json!({ "vault_path": vault }))
        .await?;
    assert_eq!(
        still.as_bool(),
        Some(false),
        "a mangled Secret Key must NOT unlock the vault"
    );

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}
