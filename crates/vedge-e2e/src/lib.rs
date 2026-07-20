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
//! - **vault dir** — a second temp dir holds the `<name>.vedge/` home; the create
//!   wizard's fields are pointed at it (slice 5.2.0 split the old single
//!   `#vault-path` field into `#vault-name` + `#vault-location`).
//! - **fast KDF** — [`Session::launch`] sets `VEDGE_E2E_FAST_KDF=1` so
//!   `create_vault` uses a cheap Argon2 profile (m 8 / t 1 / p 1) instead of the
//!   256 MiB production KDF that otherwise dominates the run. **Test-only,
//!   release-impossible**: gated behind `cfg(debug_assertions)` *and* the env var
//!   (see `vedge-tauri` `resolve_kdf_seam`), so it can't exist in a shipped bundle.
//! - **in-memory biometric** — `VEDGE_E2E_BIOMETRIC_MEMORY=1` swaps the platform
//!   authenticator for the deterministic `MemoryBiometricAuthenticator` (reports
//!   available, never prompts) so the biometric scenario runs headlessly. Same
//!   two-factor gating (`resolve_biometric`).
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
//! ## Selectors — `data-testid` (locale-independent)
//! Interactive controls are located by **stable `data-testid`** attributes
//! (`by_testid` / `click_testid` / `js_click_testid`), so a copy edit or a locale
//! switch never breaks the suite. Form **inputs** keep their existing DOM `id`s
//! (`vault-name`, `vault-location`, `master-password`, `ef-*`, `vault-search`, `folder-new`,
//! `gen-bulk-count`) used via `fill_id`. A few structural hooks are reused
//! directly: `tr[data-testid=<entry-id>]` (entry rows, since 5.3.1b's `DataTable`
//! migration — the `row_testid` hook carries the id), `[role='option']`
//! (vault picker + Select options, plus `[data-value=…]` for a specific option),
//! `[role='combobox']` (a Select trigger), `div[role='dialog']`. The testid naming
//! convention is documented in the crate README.

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
    /// Holds the `<name>.vedge/` vault home (slice 5.2.0).
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

    /// Absolute path to the vault **home** the create wizard will write (slice 5.2.0).
    /// A `.vedge` home so the wizard's `ensure_vedge_home` normalization is a no-op and
    /// every `invoke(vault_path=…)` targets the same home the UI created.
    pub fn vault_path(&self) -> PathBuf {
        self.vault_dir.path().join("e2e.vedge")
    }

    /// The vault path as a string (split into `#vault-name` + `#vault-location`
    /// by the create helper / passed to invoke).
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
        Self::launch_with_env(env, app, &[]).await
    }

    /// Like [`launch`](Self::launch) but sets `extra` env vars on the app
    /// process. Used for **per-scenario** seams that must NOT be global — e.g.
    /// `VEDGE_E2E_SESSION_TTL_SECS`, which forces a short hard TTL and would lock
    /// every other scenario mid-run if set for the whole suite (slice 4.5a), or
    /// `VEDGE_E2E_SCREEN_LOCK_MEMORY` (a sentinel-file path), the screen-lock
    /// double's deterministic lever (slice 4.5b).
    pub async fn launch_with_env(
        env: &TestEnv,
        app: &Path,
        extra: &[(&str, &str)],
    ) -> Result<Self> {
        let app = app
            .canonicalize()
            .with_context(|| format!("canonicalize app path {}", app.display()))?;

        let mut cmd = Command::new("tauri-driver");
        // The app process (spawned by tauri-driver → native driver → app)
        // inherits this env, so `resolve_app_dir` isolates app.db.
        cmd.env("VEDGE_DATA_DIR", env.data_dir_path());
        // Test-only seams (each also gated behind `cfg(debug_assertions)`, so
        // they are absent from a release bundle): a cheap Argon2 profile so
        // create+unlock don't dominate the run, and the in-memory biometric stub
        // so the biometric scenario runs without a real Hello/Touch-ID prompt.
        cmd.env("VEDGE_E2E_FAST_KDF", "1");
        cmd.env("VEDGE_E2E_BIOMETRIC_MEMORY", "1");
        // Offline breach double (slice 4.4): reports one fixed password as
        // breached so `scan_health_shows_breached` runs without hitting HIBP.
        // Inert unless a scenario enables the Settings breach toggle.
        cmd.env("VEDGE_E2E_BREACH_MEMORY", "1");
        // 🔴 Keep the tests OUT of the real credential store's namespace.
        //
        // Unlike every other seam here, the keychain is NOT faked — the OS one is used for real,
        // because that is the only way to test it. So each run leaves real entries behind: a
        // Secret Key (`secret:{uuid}`) and a rollback baseline (`counter:{uuid}`) per vault, plus
        // another pair for every duplicate that slice 5.2.2's ② opens. Under the production
        // `vedge` service they would be indistinguishable from the developer's OWN vault keys —
        // orphans nobody dares delete, because deleting the wrong one destroys a real vault.
        //
        // This puts them all under `vedge-e2e-test`, so they are obvious in Credential Manager
        // and safe to purge: `mise e2e-clean`.
        cmd.env("VEDGE_E2E_KEYCHAIN_TEST", "1");
        for (k, v) in extra {
            cmd.env(k, v);
        }
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
            if self
                .invoke("list_registered_vaults", json!({}))
                .await
                .is_ok()
            {
                return Ok(());
            }
            if start.elapsed() > timeout {
                bail!("`list_registered_vaults` never succeeded within {timeout:?}");
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

    /// Fail if the app logged anything indicating a runtime fault:
    /// - a Leptos "outside a reactive tracking context" warning (the recurring
    ///   i18n-in-`spawn_local` footgun), or
    /// - an **uncaught exception** — a wasm panic surfacing as
    ///   `Uncaught RuntimeError: unreachable`, or a wasm-bindgen glue fault as an
    ///   `Uncaught TypeError` (both captured as `uncaught:` by `index.html`), or
    /// - an **unhandled promise rejection** (`rejection:`).
    ///
    /// The last two are what let 5.3.1d's `Popover`-disposed and keyless-keydown
    /// panics reach the manual smoke: they never touch `console.*`, so a guard
    /// that only greps the reactive-context warning was blind to them. `unreachable`
    /// / `panicked` are also matched directly, in case a panic ever arrives via
    /// `console.error` instead of the `error` event.
    ///
    /// - a **Content-Security-Policy violation** (PG.2). A throwing block (dropping
    ///   `'wasm-unsafe-eval'` → `WebAssembly.instantiate` faults) already surfaces
    ///   as `uncaught:`, but a NON-throwing one — an `img-src`/`connect-src` beacon
    ///   block — only emits a `console.error` ("Refused to … Content Security Policy
    ///   directive …"), which every match above would miss. Matching the phrase
    ///   turns every e2e scenario into a standing CSP regression test.
    pub async fn assert_console_clean(&self) -> Result<()> {
        let bad: Vec<String> = self
            .console_messages()
            .await?
            .into_iter()
            .filter(|m| {
                m.contains("outside a reactive tracking context")
                    || m.starts_with("uncaught:")
                    || m.starts_with("rejection:")
                    || m.contains("unreachable")
                    || m.contains("panicked")
                    || m.contains("Content Security Policy")
            })
            .collect();
        if !bad.is_empty() {
            bail!("console fault(s) detected:\n{}", bad.join("\n"));
        }
        Ok(())
    }

    /// Prove the guard is live (not vacuous): inject BOTH a `console.warn` and a
    /// synthetic uncaught `error` event, confirm each was captured, then clear the
    /// buffer. The error path is the one that catches a wasm panic / glue fault
    /// (which never touches `console.*`), so a broken listener there must fail
    /// loudly rather than pass silently.
    pub async fn console_selftest(&self) -> Result<()> {
        const MARK: &str = "__vedge_e2e_selftest__";
        self.driver()
            .execute(
                &format!(
                    "console.warn('{MARK}'); \
                     window.dispatchEvent(new ErrorEvent('error', {{ message: '{MARK}-uncaught' }}));"
                ),
                vec![],
            )
            .await
            .context("inject self-test warning + error event")?;
        let msgs = self.console_messages().await?;
        let warn_seen = msgs
            .iter()
            .any(|m| m.starts_with("warn:") && m.contains(MARK));
        let uncaught_seen = msgs
            .iter()
            .any(|m| m.starts_with("uncaught:") && m.contains(MARK));
        if !warn_seen || !uncaught_seen {
            bail!(
                "console guard captured incompletely (warn={warn_seen}, uncaught={uncaught_seen}) — \
                 is the console buffer in crates/vedge-app/index.html present + broadened in this build?"
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

    /// Wait for an element by `data-testid` (the primary, locale-independent hook).
    pub async fn by_testid(&self, id: &str) -> Result<WebElement> {
        self.wait_for(
            By::Css(format!("[data-testid='{id}']")),
            Duration::from_secs(10),
        )
        .await
        .with_context(|| format!("element [data-testid={id:?}]"))
    }

    /// Click an element by `data-testid`.
    pub async fn click_testid(&self, id: &str) -> Result<()> {
        let el = self.by_testid(id).await?;
        el.click()
            .await
            .with_context(|| format!("click [data-testid={id:?}]"))?;
        Ok(())
    }

    /// Scripted `.click()` on a `data-testid` element — for visually-hidden
    /// controls (e.g. the sr-only "I saved my Secret Key" acknowledgement) that
    /// WebDriver refuses to click as "not interactable".
    pub async fn js_click_testid(&self, id: &str) -> Result<()> {
        self.by_testid(id).await?;
        // If the testid is on a wrapper around an sr-only control (e.g. the ack
        // checkbox), click the inner input/label/button — clicking the wrapper
        // itself wouldn't toggle it. For a directly-tagged control there is no
        // such descendant, so we fall back to clicking the host.
        let script = r#"
            const id = arguments[0];
            const host = document.querySelector('[data-testid="' + id + '"]');
            if (!host) { return "not-found"; }
            const target = host.querySelector('input, label, button') || host;
            target.click();
            return "ok";
        "#;
        let ret = self
            .driver()
            .execute(script, vec![json!(id)])
            .await
            .with_context(|| format!("js-click [data-testid={id:?}]"))?;
        if ret.json().as_str() == Some("not-found") {
            bail!("no element [data-testid={id:?}] for js-click");
        }
        Ok(())
    }

    /// Count elements matching a CSS selector (0 if none). Used for "history is
    /// empty" / row-count style assertions.
    pub async fn count_css(&self, css: &str) -> Result<usize> {
        Ok(self
            .driver()
            .find_all(By::Css(css.to_string()))
            .await
            .unwrap_or_default()
            .len())
    }

    /// Dispatch a synthetic window `blur` (for the lock-on-blur assertions —
    /// WebDriver can't truly defocus the OS window, but the app listens on the
    /// DOM `blur` event).
    pub async fn dispatch_window_blur(&self) -> Result<()> {
        self.driver()
            .execute("window.dispatchEvent(new Event('blur'));", vec![])
            .await
            .context("dispatch window blur")?;
        Ok(())
    }

    /// Toggle the debug-only in-dialog flag via the `window.__vedge_test_dialog`
    /// hook installed by `app.rs` (debug builds only). Lets the negative
    /// lock-on-blur assertion simulate an open native dialog.
    pub async fn set_dialog_in_progress(&self, on: bool) -> Result<()> {
        self.driver()
            .execute(
                "if (window.__vedge_test_dialog) { window.__vedge_test_dialog(arguments[0]); } \
                 else { throw new Error('__vedge_test_dialog missing (release build?)'); }",
                vec![json!(on)],
            )
            .await
            .context("call window.__vedge_test_dialog")?;
        Ok(())
    }

    /// Click a `<button>` whose trimmed visible text equals `text`
    /// (English-locale coupled — prefer `click_testid`; kept for the rare control
    /// without a testid).
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

    /// Number of entry rows currently rendered in the vault table. Since slice
    /// 5.3.1b the table is a `DataTable`; each body row carries `data-testid`
    /// (the entry id) via the `row_testid` hook — the header row does not, so
    /// `tr[data-testid]` selects exactly the entry rows.
    pub async fn entry_row_count(&self) -> Result<usize> {
        Ok(self
            .driver()
            .find_all(By::Css("tr[data-testid]"))
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
