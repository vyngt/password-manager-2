//! End-to-end WebDriver harness for the VEdge desktop app (slice 2.9.2).
//!
//! Drives the **real** built binary (frontend → Tauri commands → `vedge-core` →
//! SQLite) through `tauri-driver` + a platform WebDriver (WebView2/msedgedriver
//! on Windows, WebKitWebDriver on Linux), using the async [`thirtyfour`] client.
//!
//! This module is the reusable plumbing; the actual scenario lives in
//! `tests/daily_loop.rs` (`#[ignore]`-gated so `cargo test` stays fast/headless).
//!
//! ## Determinism seams (see the slice spec)
//! - **app.db isolation** — [`TestEnv`] sets `VEDGE_DATA_DIR` to a per-run temp
//!   dir; the harness passes it to the `tauri-driver` child, which the launched
//!   app inherits (`resolve_app_dir` honors it first). This also disables the
//!   single-instance lock (so the restart test can relaunch the app).
//! - **vault dir** — a second temp dir holds the `.vdb`; the create wizard is
//!   pointed at it via the `#vault-path` input.
//! - **keychain** — the real OS keychain is used (Windows-first): the create
//!   wizard writes the Secret Key, UI-unlock reads it back non-interactively.
//!   The credential is keyed by the temp vault path, so runs don't collide; it
//!   is harmless residue (see the README for cleanup).
//!
//! ## Design rule
//! Mutations are driven through the **UI** so the Leptos state and the backend
//! session stay in sync; `invoke()` is used only for **read-only assertions**
//! (`is_unlocked`, `list_entries`, `list_history`, `get_active_theme`). An IPC
//! mutation would not update the Leptos view, so the DOM assertions would drift.
//!
//! ## Locale
//! Text/`aria-label` selectors assume the app renders the **default English**
//! locale. A machine defaulting to a non-`en` browser locale would need the
//! locale forced to `en` first (documented limitation; the `data-testid` sweep
//! is the future hardening).

use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Value, json};
use tempfile::TempDir;
use thirtyfour::prelude::*;

/// Strong master password used for the e2e vault (passes the strength gate).
pub const MASTER_PASSWORD: &str = "Corr3ct-Horse-Battery-Staple-42!";

const DRIVER_HOST: &str = "127.0.0.1";
const DRIVER_PORT: u16 = 4444;

fn driver_url() -> String {
    format!("http://{DRIVER_HOST}:{DRIVER_PORT}")
}

// ---------------------------------------------------------------------------
// Per-run temp environment
// ---------------------------------------------------------------------------

/// Owns the per-run temp directories. Outlives a [`Session`] so a "restart"
/// test can drop one session and launch another against the *same* data +
/// vault dirs (that is how theme/entry persistence across a relaunch is
/// asserted).
pub struct TestEnv {
    /// `VEDGE_DATA_DIR` → isolates `app.db` (recents, settings, themes).
    pub data_dir: TempDir,
    /// Holds the `.vdb` file and its sibling blob dir.
    pub vault_dir: TempDir,
}

impl TestEnv {
    pub fn new() -> Result<Self> {
        Ok(Self {
            data_dir: tempfile::Builder::new()
                .prefix("vedge-e2e-data-")
                .tempdir()
                .context("create temp data dir")?,
            vault_dir: tempfile::Builder::new()
                .prefix("vedge-e2e-vault-")
                .tempdir()
                .context("create temp vault dir")?,
        })
    }

    /// Absolute path to the vault file the create wizard will write.
    pub fn vault_path(&self) -> PathBuf {
        self.vault_dir.path().join("e2e.vdb")
    }

    /// The vault path as a string (typed into `#vault-path` / passed to invoke).
    pub fn vault_path_str(&self) -> String {
        self.vault_path().to_string_lossy().into_owned()
    }

    pub fn data_dir_path(&self) -> &Path {
        self.data_dir.path()
    }
}

/// Resolve the app binary tauri-driver should launch.
///
/// Override with `VEDGE_E2E_APP`. Default: the plain-cargo debug binary
/// (`target/debug/vedge-tauri[.exe]`), which is what `mise e2e` builds with
/// `--features custom-protocol` so the *debug* wasm is embedded (required for
/// the console-clean guard to fire). Note: NOT `VEDGE.exe` — that rename only
/// happens via `cargo tauri build`, whose release wasm silences the guard.
pub fn app_binary() -> Result<PathBuf> {
    if let Some(p) = std::env::var_os("VEDGE_E2E_APP") {
        let p = PathBuf::from(p);
        if !p.exists() {
            bail!("VEDGE_E2E_APP points at a missing file: {}", p.display());
        }
        return Ok(p);
    }
    // This crate is `crates/vedge-e2e`; the workspace target dir is `../../target`.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest
        .ancestors()
        .nth(2)
        .ok_or_else(|| anyhow!("cannot locate workspace root from {}", manifest.display()))?;
    let name = if cfg!(windows) {
        "vedge-tauri.exe"
    } else {
        "vedge-tauri"
    };
    let bin = workspace.join("target").join("debug").join(name);
    if !bin.exists() {
        bail!(
            "app binary not found at {} — build it first:\n  \
             trunk build --config crates/vedge-app/Trunk.toml\n  \
             cargo build -p vedge-tauri --features custom-protocol\n\
             (or set VEDGE_E2E_APP)",
            bin.display()
        );
    }
    Ok(bin)
}

// ---------------------------------------------------------------------------
// Session — one running app instance
// ---------------------------------------------------------------------------

/// A single launched app instance: the `tauri-driver` child plus a connected
/// [`WebDriver`]. Kill the whole process tree on drop so a panicking test never
/// leaves an orphaned driver/app (which would also block the next launch on
/// port 4444).
pub struct Session {
    driver: Option<WebDriver>,
    tauri_driver: Child,
}

impl Session {
    /// Launch `tauri-driver` (inheriting `VEDGE_DATA_DIR` from `env`), connect a
    /// WebDriver to the app binary, and wait until the app's IPC is reachable.
    pub async fn launch(env: &TestEnv, app: &Path) -> Result<Self> {
        let app = app
            .canonicalize()
            .with_context(|| format!("canonicalize app path {}", app.display()))?;

        let mut cmd = Command::new("tauri-driver");
        // The app process (spawned by tauri-driver → native driver → app)
        // inherits this env, so `resolve_app_dir` isolates app.db.
        cmd.env("VEDGE_DATA_DIR", env.data_dir_path());
        if let Some(native) = std::env::var_os("VEDGE_E2E_NATIVE_DRIVER") {
            cmd.arg("--native-driver").arg(native);
        }
        let child = cmd.spawn().context(
            "failed to spawn `tauri-driver` — install it with `cargo install tauri-driver \
             --locked` and ensure the platform WebDriver (msedgedriver / WebKitWebDriver) \
             is available (see crates/vedge-e2e/README.md)",
        )?;

        // Wait for the WebDriver port to open.
        wait_for_port(DRIVER_PORT, Duration::from_secs(20))
            .context("tauri-driver did not open its WebDriver port")?;

        // Connect, passing the app path via the `tauri:options` capability.
        let mut caps = Capabilities::new();
        caps.insert(
            "tauri:options".to_string(),
            json!({ "application": app.to_string_lossy() }),
        );
        let driver = WebDriver::new(driver_url(), caps).await.context(
            "WebDriver::new failed — is the platform driver (e.g. msedgedriver) version-matched \
             to the installed WebView2 runtime?",
        )?;

        let session = Self {
            driver: Some(driver),
            tauri_driver: child,
        };

        // Wait until the embedded frontend has booted and IPC is reachable.
        // This is also the positive "app actually loaded" assertion that keeps
        // the console-clean guard from vacuously passing on a blank window.
        session.wait_ready(Duration::from_secs(30)).await.context(
            "app IPC never became reachable (did the binary embed the frontend? build \
                      it with --features custom-protocol)",
        )?;
        Ok(session)
    }

    pub fn driver(&self) -> &WebDriver {
        self.driver
            .as_ref()
            .expect("driver present until close()/drop")
    }

    /// Gracefully quit the WebDriver (closes the app), then hard-kill the driver
    /// process tree and wait for it to exit. Call this before launching the next
    /// session so port 4444 is free and the old app is fully dead.
    pub async fn close(mut self) {
        if let Some(d) = self.driver.take() {
            let _ = d.quit().await;
        }
        let pid = self.tauri_driver.id();
        kill_tree(pid);
        let _ = self.tauri_driver.wait();
        // Give the OS a beat to release the port before the next launch binds it.
        std::thread::sleep(Duration::from_millis(400));
    }

    // --- IPC (read-only assertions) ---------------------------------------

    /// Call a Tauri command via `window.__TAURI__.core.invoke` and return its
    /// resolved value. Used for **read-only** assertions only (see module docs).
    pub async fn invoke(&self, cmd: &str, args: Value) -> Result<Value> {
        // WebDriver async script: the injected resolve callback is the LAST arg,
        // so `arguments` is `[cmd, args, done]`.
        let script = r#"
            const done = arguments[arguments.length - 1];
            const cmd = arguments[0];
            const args = arguments[1];
            try {
                const t = window.__TAURI__;
                if (!t || !t.core) { done({ err: "window.__TAURI__.core missing" }); return; }
                t.core.invoke(cmd, args)
                    .then(r => done({ ok: r === undefined ? null : r }))
                    .catch(e => done({ err: String(e) }));
            } catch (e) { done({ err: String(e) }); }
        "#;
        let ret = self
            .driver()
            .execute_async(script, vec![json!(cmd), args])
            .await
            .with_context(|| format!("execute_async invoke({cmd})"))?;
        let v = ret.json().clone();
        if let Some(err) = v.get("err") {
            bail!("invoke({cmd}) rejected: {err}");
        }
        Ok(v.get("ok").cloned().unwrap_or(Value::Null))
    }

    async fn wait_ready(&self, timeout: Duration) -> Result<()> {
        let start = Instant::now();
        loop {
            if self.invoke("list_recent_vaults", json!({})).await.is_ok() {
                return Ok(());
            }
            if start.elapsed() > timeout {
                bail!("`list_recent_vaults` never succeeded within {timeout:?}");
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    // --- Console-clean guard ----------------------------------------------

    /// Read the `window.__vedge_warnings` ring buffer (installed by index.html)
    /// as `"level: text"` lines.
    pub async fn console_messages(&self) -> Result<Vec<String>> {
        let ret = self
            .driver()
            .execute(
                "return (window.__vedge_warnings || []).map(function(w){return w.level+': '+w.text;});",
                vec![],
            )
            .await
            .context("read window.__vedge_warnings")?;
        Ok(ret
            .json()
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default())
    }

    /// Fail if the app logged any Leptos "outside a reactive tracking context"
    /// warning — the recurring i18n-in-`spawn_local` footgun.
    pub async fn assert_console_clean(&self) -> Result<()> {
        let bad: Vec<String> = self
            .console_messages()
            .await?
            .into_iter()
            .filter(|m| m.contains("outside a reactive tracking context"))
            .collect();
        if !bad.is_empty() {
            bail!("reactive-context warning(s) detected:\n{}", bad.join("\n"));
        }
        Ok(())
    }

    /// Prove the guard is live (not vacuous): inject a warning, confirm it was
    /// captured, then clear the buffer.
    pub async fn console_selftest(&self) -> Result<()> {
        const MARK: &str = "__vedge_e2e_selftest__";
        self.driver()
            .execute(&format!("console.warn('{MARK}');"), vec![])
            .await
            .context("inject self-test warning")?;
        let seen = self
            .console_messages()
            .await?
            .iter()
            .any(|m| m.contains(MARK));
        if !seen {
            bail!(
                "console-clean guard captured nothing — is the console buffer in \
                 crates/vedge-app/index.html present in this build?"
            );
        }
        self.clear_console().await
    }

    pub async fn clear_console(&self) -> Result<()> {
        self.driver()
            .execute(
                "if (window.__vedge_warnings) { window.__vedge_warnings.length = 0; }",
                vec![],
            )
            .await
            .context("clear console buffer")?;
        Ok(())
    }

    // --- DOM helpers -------------------------------------------------------

    /// Poll for an element matching `by`, up to `timeout`.
    pub async fn wait_for(&self, by: By, timeout: Duration) -> Result<WebElement> {
        let start = Instant::now();
        loop {
            match self.driver().find(by.clone()).await {
                Ok(el) => return Ok(el),
                Err(_) if start.elapsed() < timeout => {
                    tokio::time::sleep(Duration::from_millis(150)).await;
                }
                Err(e) => return Err(anyhow!("element {by:?} not found in {timeout:?}: {e}")),
            }
        }
    }

    /// Wait for an element by DOM `id`.
    pub async fn by_id(&self, id: &str) -> Result<WebElement> {
        self.wait_for(By::Id(id.to_string()), Duration::from_secs(10))
            .await
    }

    /// Clear a field and type into it.
    ///
    /// Clearing (`text == ""`) is done by setting the value and dispatching a
    /// real `input` event via JS: a bare WebDriver `clear()` doesn't reliably
    /// fire `input`, so a *controlled* Leptos input (`prop:value` +
    /// `on:input:target`) never sees the change and re-applies its old value on
    /// the next tick. Typing fires `input` per keystroke, so that path is fine.
    pub async fn fill_id(&self, id: &str, text: &str) -> Result<()> {
        let el = self.by_id(id).await?;
        el.clear().await.ok();
        if text.is_empty() {
            self.driver()
                .execute(
                    "const el = document.getElementById(arguments[0]); \
                     if (el) { el.value = ''; \
                     el.dispatchEvent(new Event('input', { bubbles: true })); \
                     el.dispatchEvent(new Event('change', { bubbles: true })); }",
                    vec![json!(id)],
                )
                .await
                .with_context(|| format!("clear #{id}"))?;
        } else {
            el.send_keys(text)
                .await
                .with_context(|| format!("type into #{id}"))?;
        }
        Ok(())
    }

    /// Click a `<button>` whose trimmed visible text equals `text`
    /// (English-locale coupled — see module docs).
    pub async fn click_button_text(&self, text: &str) -> Result<()> {
        let xpath = format!("//button[normalize-space(.)={}]", xpath_literal(text));
        let el = self
            .wait_for(By::XPath(xpath), Duration::from_secs(10))
            .await
            .with_context(|| format!("button with text {text:?}"))?;
        el.click()
            .await
            .with_context(|| format!("click {text:?}"))?;
        Ok(())
    }

    /// Click any element carrying `aria-label == label` (buttons/icon-buttons).
    pub async fn click_aria(&self, label: &str) -> Result<()> {
        let xpath = format!("//*[@aria-label={}]", xpath_literal(label));
        let el = self
            .wait_for(By::XPath(xpath), Duration::from_secs(10))
            .await
            .with_context(|| format!("element with aria-label {label:?}"))?;
        el.click()
            .await
            .with_context(|| format!("click aria {label:?}"))?;
        Ok(())
    }

    /// Click an element by `aria-label` using a **scripted** `.click()`. Needed
    /// for visually-hidden controls — e.g. a custom checkbox whose native
    /// `<input>` carries the label but is `sr-only`, which WebDriver refuses to
    /// click as "not interactable". The scripted click still toggles state and
    /// fires the `change` handler. (Waits for the element to appear first.)
    pub async fn js_click_aria(&self, label: &str) -> Result<()> {
        self.wait_for(
            By::XPath(format!("//*[@aria-label={}]", xpath_literal(label))),
            Duration::from_secs(10),
        )
        .await
        .with_context(|| format!("element with aria-label {label:?} (js-click)"))?;
        let script = r#"
            const label = arguments[0];
            const el = document.querySelector('[aria-label="' + label + '"]');
            if (!el) { return "not-found"; }
            el.click();
            return "ok";
        "#;
        let ret = self
            .driver()
            .execute(script, vec![json!(label)])
            .await
            .with_context(|| format!("js-click aria {label:?}"))?;
        if ret.json().as_str() == Some("not-found") {
            bail!("no element with aria-label {label:?} for js-click");
        }
        Ok(())
    }

    /// Dismiss the top-most modal Dialog by sending Escape to it (the Dialog
    /// closes on Escape). Best-effort: a no-op if no dialog is open. Call this
    /// after asserting a modal's content so it doesn't cover later interactions.
    pub async fn close_dialog(&self) -> Result<()> {
        if let Ok(el) = self.driver().find(By::Css("div[role='dialog']")).await {
            el.send_keys(Key::Escape).await.ok();
            // Give the exit a moment so the scrim is gone before the next click.
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        Ok(())
    }

    /// Number of entry rows currently rendered in the vault table.
    pub async fn entry_row_count(&self) -> Result<usize> {
        Ok(self
            .driver()
            .find_all(By::Css("tr[data-entry-row]"))
            .await
            .unwrap_or_default()
            .len())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // Backstop for a panicking test where `close()` never ran: force-kill
        // the whole tree (driver + native driver + app).
        kill_tree(self.tauri_driver.id());
        let _ = self.tauri_driver.kill();
        let _ = self.tauri_driver.wait();
    }
}

// ---------------------------------------------------------------------------
// Free helpers
// ---------------------------------------------------------------------------

fn wait_for_port(port: u16, timeout: Duration) -> Result<()> {
    let start = Instant::now();
    loop {
        if TcpStream::connect((DRIVER_HOST, port)).is_ok() {
            return Ok(());
        }
        if start.elapsed() > timeout {
            bail!("port {port} did not open within {timeout:?}");
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Kill a process and all of its descendants (the native driver + the app that
/// `tauri-driver` spawned). `Child::kill` alone would orphan them.
fn kill_tree(pid: u32) {
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .output();
    }
    #[cfg(not(windows))]
    {
        // Best-effort: kill children then the parent.
        let _ = Command::new("pkill")
            .args(["-P", &pid.to_string()])
            .output();
        let _ = Command::new("kill").args(["-9", &pid.to_string()]).output();
    }
}

/// Build an XPath string literal that is safe even if `s` contains quotes.
fn xpath_literal(s: &str) -> String {
    if !s.contains('\'') {
        format!("'{s}'")
    } else if !s.contains('"') {
        format!("\"{s}\"")
    } else {
        // Both quote kinds present: assemble via concat().
        let parts: Vec<String> = s.split('\'').map(|p| format!("'{p}'")).collect();
        format!("concat({}, '')", parts.join(", \"'\", "))
    }
}
