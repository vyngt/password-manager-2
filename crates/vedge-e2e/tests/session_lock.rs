//! Session-lock e2e scenarios (slices 2.6/3.8, 4.5a, 4.5b): lock-on-blur honors
//! the in-dialog guard, and both a hard session TTL and an OS screen-lock lock the
//! vault AND wipe decrypted material from WASM (the load-bearing teardown).
//!
//! `#[ignore]` by default; run via `mise e2e`.

mod common;

use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::json;
use thirtyfour::By;

use common::*;
use vedge_e2e::{Session, TestEnv, app_binary};

/// 2.6 / 3.8 — lock-on-blur honors the in-dialog guard: a real window blur locks,
/// but a blur while a (native) dialog is open does not. The pref is off by
/// default, so we enable it and restart so the layout's listener boots with it on;
/// the "dialog open" state is faked via the debug-only `__vedge_test_dialog` hook.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn lock_on_blur_respects_dialog_guard() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    let mut session = Session::launch(&env, &app).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;

    // Enable lock-on-blur (default off) by persisting the pref, then restart so
    // the /v layout's auto-lock listener boots with it enabled.
    session
        .invoke(
            "set_app_setting",
            json!({
                "key": "security.prefs",
                "value": { "auto_lock_minutes": 15, "lock_on_blur": true, "clipboard_clear_seconds": 30 }
            }),
        )
        .await
        .context("persist lock_on_blur=true")?;
    session.close().await;
    session = Session::launch(&env, &app).await.context("relaunch")?;
    unlock_ui(&session, &vault).await?;

    // Negative: with a "dialog in progress", a blur must NOT lock.
    session.set_dialog_in_progress(true).await?;
    session.dispatch_window_blur().await?;
    tokio::time::sleep(Duration::from_millis(800)).await;
    let still = session
        .invoke("is_unlocked", json!({ "vault_path": vault }))
        .await?
        .as_bool()
        .unwrap_or(false);
    assert!(still, "blur while a dialog is open must NOT lock the vault");
    session.set_dialog_in_progress(false).await?;

    // Positive: a real blur (no dialog) locks.
    session.dispatch_window_blur().await?;
    assert_unlocked(&session, &vault, false)
        .await
        .context("a plain blur should lock the vault")?;

    session.close().await;
    Ok(())
}

/// Slice 4.5a: a backend-enforced hard session TTL locks the vault on a timer,
/// and — the load-bearing part — the frontend teardown wipes decrypted material
/// from WASM. Launched with a short **per-scenario** `VEDGE_E2E_SESSION_TTL_SECS`
/// (global would lock every other scenario mid-run). We seed the generator
/// session history (`Zeroizing` plaintext), wait out the TTL, then assert BOTH
/// that `is_unlocked` flips AND that the DOM is back at the launch screen — and
/// that on re-unlock the history is empty. Asserting only `is_unlocked` would
/// pass while plaintext still sat in the renderer's heap — the exact bug this
/// slice must not ship.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn ttl_expiry_wipes_wasm_plaintext() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    // A short hard TTL — generous enough to finish setup before it fires, short
    // enough to keep the scenario quick.
    let session =
        Session::launch_with_env(&env, &app, &[("VEDGE_E2E_SESSION_TTL_SECS", "20")]).await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;

    // Seed the generator session history with a Zeroizing plaintext password.
    open_generator_panel(&session).await?;
    session
        .click_testid("gen-regenerate")
        .await
        .context("regenerate (seeds history)")?;
    session
        .click_testid("gen-history-toggle")
        .await
        .context("expand Recent")?;
    wait_until(Duration::from_secs(5), || async {
        Ok(session.count_css("[data-testid='gen-history-row']").await? >= 1)
    })
    .await
    .context("Recent should populate after regenerate")?;
    session.close_dialog().await?;

    // Wait out the hard TTL. The backend reaper zeroizes the KEK; the frontend
    // 1 Hz poll notices `is_unlocked == false` and drives the same teardown as a
    // manual lock (active.path Some→None → wipe generator history + health).
    assert_unlocked(&session, &vault, false)
        .await
        .context("the hard session TTL should lock the vault")?;
    // DOM assertion (the load-bearing part): teardown navigated back to `/` — the
    // vault picker (a `[role='option']` per recent vault) is only rendered there.
    session
        .wait_for(
            By::Css("[role='option']".to_string()),
            Duration::from_secs(10),
        )
        .await
        .context("frontend teardown should return to the launch screen (vault picker)")?;

    // Re-unlock and confirm the seeded history was wiped on the backend lock.
    unlock_ui(&session, &vault).await?;
    open_generator_panel(&session).await?;
    session
        .click_testid("gen-history-toggle")
        .await
        .context("expand Recent (post-unlock)")?;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let n = session.count_css("[data-testid='gen-history-row']").await?;
    assert_eq!(
        n, 0,
        "generator history must be wiped by the backend-initiated lock (found {n})"
    );

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}

/// Slice 4.5b: an OS screen-lock locks the vault, and — the load-bearing part —
/// the frontend teardown wipes decrypted material from WASM. WebDriver can't lock
/// a real workstation, so the memory `ScreenLockWatcher` uses a **sentinel file**
/// as its lever (`VEDGE_E2E_SCREEN_LOCK_MEMORY` = the path): it reports "locked"
/// while the file exists. We finish all setup and seed the generator history
/// (`Zeroizing` plaintext), *then* create the file — a deterministic `false→true`
/// edge on a live session (no timing race). The scheduler's screen-lock sweep
/// reaps the session; the frontend `is_unlocked` poll drives the same teardown as
/// a manual lock. We assert BOTH `is_unlocked` flips AND the DOM is back at the
/// launch screen AND the history is empty on re-unlock — asserting only
/// `is_unlocked` would pass while plaintext still sat in the renderer's heap.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "e2e: needs tauri-driver + a platform WebDriver + a display; run via `mise e2e`"]
async fn screen_lock_wipes_wasm_plaintext() -> Result<()> {
    let env = TestEnv::new()?;
    let app = app_binary()?;
    let vault = env.vault_path_str();
    // The sentinel the memory watcher reports "locked" while it exists. Created
    // only after setup, so the lock edge lands on a live, unlocked session.
    let sentinel = env.data_dir_path().join("e2e-screen-locked.flag");
    let sentinel_str = sentinel.to_string_lossy().into_owned();
    let session = Session::launch_with_env(
        &env,
        &app,
        &[("VEDGE_E2E_SCREEN_LOCK_MEMORY", &sentinel_str)],
    )
    .await?;
    session.console_selftest().await?;
    create_and_unlock(&session, &vault).await?;

    // Seed the generator session history with a Zeroizing plaintext password.
    open_generator_panel(&session).await?;
    session
        .click_testid("gen-regenerate")
        .await
        .context("regenerate (seeds history)")?;
    session
        .click_testid("gen-history-toggle")
        .await
        .context("expand Recent")?;
    wait_until(Duration::from_secs(5), || async {
        Ok(session.count_css("[data-testid='gen-history-row']").await? >= 1)
    })
    .await
    .context("Recent should populate after regenerate")?;
    session.close_dialog().await?;

    // "Lock the screen": create the sentinel. The scheduler's screen-lock sweep
    // sees the false→true edge (≤5 s), reaps the session; the frontend 1 Hz poll
    // notices `is_unlocked == false` and drives the same teardown as a manual
    // lock (active.path Some→None → wipe generator history + health).
    std::fs::write(&sentinel, b"1").context("create screen-lock sentinel")?;

    assert_unlocked(&session, &vault, false)
        .await
        .context("an OS screen-lock should lock the vault")?;
    // DOM assertion (the load-bearing part): teardown navigated back to `/` — the
    // vault picker (a `[role='option']` per recent vault) is only rendered there.
    session
        .wait_for(
            By::Css("[role='option']".to_string()),
            Duration::from_secs(10),
        )
        .await
        .context("frontend teardown should return to the launch screen (vault picker)")?;

    // "Unlock the screen" before re-unlocking the vault (models real use: the
    // workstation unlocks, then the user unlocks the vault). Removing the sentinel
    // also clears the watcher's locked level so the re-unlock isn't re-reaped.
    std::fs::remove_file(&sentinel).ok();

    // Re-unlock and confirm the seeded history was wiped on the screen-lock.
    unlock_ui(&session, &vault).await?;
    open_generator_panel(&session).await?;
    session
        .click_testid("gen-history-toggle")
        .await
        .context("expand Recent (post-unlock)")?;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let n = session.count_css("[data-testid='gen-history-row']").await?;
    assert_eq!(
        n, 0,
        "generator history must be wiped by the screen-lock-initiated lock (found {n})"
    );

    session.assert_console_clean().await?;
    session.close().await;
    Ok(())
}
