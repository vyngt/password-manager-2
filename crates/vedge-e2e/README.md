# vedge-e2e — end-to-end WebDriver harness

Drives the **real** VEdge desktop binary (frontend → Tauri commands → `vedge-core`
→ SQLite) through [`tauri-driver`] + [`thirtyfour`], automating the Phase-2
"daily loop" the app previously only smoked by hand. Slice 2.9.2.

- Harness plumbing: [`src/lib.rs`](src/lib.rs) — `TestEnv`, `Session` (spawns
  `tauri-driver`, connects the WebDriver, kills the process tree on drop),
  `invoke()` for read-only IPC assertions, and the console-clean guard.
- Scenario: [`tests/daily_loop.rs`](tests/daily_loop.rs) — one serial
  `#[ignore]` test walking create → add → search → edit/history → favorite →
  folder → lock → **restart** → unlock → theme-persists, with a standing
  console-clean assertion.

## Run it

```sh
mise e2e
```

That task does three things (see `mise.toml`):

1. `trunk build --config ../vedge-app/Trunk.toml` — build the frontend as
   **debug** wasm (NO `--release`). The Leptos "outside a reactive tracking
   context" warning is `#[cfg(debug_assertions)]`; a release wasm bundle would
   silence the console-clean guard.
2. `cargo build -p vedge-tauri --features custom-protocol` — embed `dist/` into
   `target/debug/vedge-tauri.exe`. **`--features custom-protocol` is required**:
   a plain `cargo build` runs Tauri in *dev* mode (loads `devUrl`), producing a
   blank window with no embedded assets.
3. `cargo test -p vedge-e2e -- --ignored --test-threads=1` — the harness spawns
   `tauri-driver` itself; run **serial** (single-instance + one WebDriver port).

The default `cargo test` / `mise test` are unaffected — this crate is
`#[ignore]`-gated *and* `--exclude`d from the workspace gate.

## Prerequisites

Full setup: the Tauri [WebDriver manual-setup guide][tauri-manual]. In short:

- **`tauri-driver`**: `cargo install tauri-driver --locked` (also in `mise setup`).
- **Platform WebDriver**, version-matched to the webview:
  - **Windows**: `msedgedriver.exe` matching the installed **WebView2** runtime.
    Fetch a version-matched one with Tauri's helper:
    ```powershell
    cargo install --git https://github.com/chippers/msedgedriver-tool
    msedgedriver-tool          # downloads msedgedriver.exe matching your Edge
    ```
    Then put it **on `PATH`** — e.g. copy it next to `tauri-driver` in
    `~/.cargo/bin`. `tauri-driver`'s working directory is the *crate* dir, not
    the repo root, so a driver dropped at the repo root is **not** found; use
    `PATH` or point at it explicitly with `VEDGE_E2E_NATIVE_DRIVER`. WebView2
    auto-updates, so a mismatched driver is the #1 flake — re-run
    `msedgedriver-tool` if session creation hangs/errors on a version.
  - **Linux**: `WebKitWebDriver` (`webkit2gtk-driver` / `libwebkit2gtk-4.1`), run
    under `xvfb-run` for a headless display.
- A **display** (real or virtual). This is a GUI e2e; it is not headless-safe.

### Env overrides

| Var | Default | Purpose |
|---|---|---|
| `VEDGE_E2E_APP` | `target/debug/vedge-tauri[.exe]` | App binary tauri-driver launches |
| `VEDGE_E2E_NATIVE_DRIVER` | (auto) | Path to msedgedriver / WebKitWebDriver |
| `VEDGE_DATA_DIR` | (set by the harness) | Per-run temp `app.db` dir — see below |

## Determinism seams

- **`app.db` isolation** — the harness sets `VEDGE_DATA_DIR` to a per-run temp
  dir and passes it to the `tauri-driver` child; the launched app inherits it and
  `resolve_app_dir` honors it first. This also disables the single-instance lock
  (so the restart test can relaunch), and isolates recents/settings/themes.
- **Vault dir** — a temp dir holds the `.vdb`; the create wizard is pointed at it
  via the `#vault-path` input (no native "Choose…" dialog).
- **Keychain** — Windows-first: the **real** OS keychain is used. The create
  wizard writes the Secret Key; UI-unlock reads it back non-interactively; the
  restart test relies on it persisting across the two app processes.

## The manual boundary (not automatable)

WebDriver cannot drive OS-native surfaces, so these stay **human smokes**:

- **Real biometric prompt** (Windows Hello / macOS Touch ID). The daily loop uses
  password unlock; a biometric e2e (via the `Memory` authenticator stub) is a
  future follow-up.
- **Native file dialogs** — Emergency-Kit save, Document attach/export *dialog*.
  The path-based document commands are drivable; the dialogs are not.
- The e2e uses the **real** Windows keychain rather than mocking it.

### Keychain residue

Each run writes a keychain credential keyed by the (unique temp) vault path —
service `vedge`, account `vault:<temp-path>`. It's harmless junk keyed by a
now-deleted path, but to clear it:

```powershell
cmdkey /list | Select-String vedge      # find them
cmdkey /delete:<target>                  # remove one
```

## CI (documented, not yet enabled)

An opt-in e2e job would need: the built binary, `tauri-driver`, the platform
WebDriver, and a display. On Linux, wrap the run in `xvfb-run` and install
`libwebkit2gtk-4.1` + the secret-service (for the keyring). On Windows, ensure
WebView2 + a matching `msedgedriver`. Keep it a separate, opt-in job — the
default CI stays fast and headless.

## Notes / limitations

- **English locale assumed.** Button/`aria-label` selectors match the default
  English strings (`crates/vedge-app/locales/en`). A machine defaulting to a
  non-`en` browser locale would need the locale forced to `en`. Hardening this
  with `data-testid` attributes across the ~30 daily-loop call sites is a
  follow-up.
- **Mutations via UI, assertions via IPC.** Actions go through the UI so the
  Leptos view and backend session stay in sync; `invoke()` is used only for
  read-only assertions. A few terminal mutations (history restore, folder move,
  theme set) use `invoke()` directly and are noted inline.

## References

**Testing (this repo)**

- [`docs/testing.md`](../../docs/testing.md) — VEdge's four testing layers; this
  harness is **level 4** (E2E). Levels 1–3: host `cargo test`, `wasm-pack test
  --node` (IPC wire codec), and the manual `cargo tauri dev` smoke.

**WebDriver + Tauri**

- Tauri — [WebDriver testing overview][tauri-wd] · [manual setup][tauri-manual]
  (install `tauri-driver` + the platform driver) · [the WebDriverIO/Selenium
  examples][tauri-example] (JS reference; we use `thirtyfour` in Rust instead).
- [`tauri-driver`][tauri-driver-crate] — the cross-platform WebDriver proxy.
- [`thirtyfour`] — the async Rust WebDriver client this harness drives.
- [`msedgedriver-tool`][msedgedriver-tool] — fetches a version-matched Edge driver
  (Windows).
- [Microsoft Edge WebDriver][edge-driver] · [WebKitWebDriver][webkit-driver]
  (Linux) — the platform drivers `tauri-driver` proxies to.

[`tauri-driver`]: https://v2.tauri.app/develop/tests/webdriver/
[`thirtyfour`]: https://docs.rs/thirtyfour
[tauri-wd]: https://v2.tauri.app/develop/tests/webdriver/
[tauri-manual]: https://v2.tauri.app/develop/tests/webdriver/manual-setup/
[tauri-example]: https://v2.tauri.app/develop/tests/webdriver/example/
[tauri-driver-crate]: https://crates.io/crates/tauri-driver
[msedgedriver-tool]: https://github.com/chippers/msedgedriver-tool
[edge-driver]: https://developer.microsoft.com/microsoft-edge/tools/webdriver/
[webkit-driver]: https://github.com/tauri-apps/wry/wiki/Webdriver
