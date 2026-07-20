# vedge-e2e — end-to-end WebDriver harness

Drives the **real** VEdge desktop binary (frontend → Tauri commands → `vedge-core`
→ SQLite) through [`tauri-driver`] + [`thirtyfour`], automating the human smokes
each slice previously left behind. Started as the Phase-2 "daily loop" (slice 2.9.2).

- Harness plumbing: [`src/lib.rs`](src/lib.rs) — `TestEnv`, `Session` (spawns
  `tauri-driver`, connects the WebDriver, kills the process tree on drop),
  `invoke()` for read-only IPC assertions, and the console-clean guard.
- Shared scenario helpers: [`tests/common/mod.rs`](tests/common/mod.rs) — the
  UI flows (`create_and_unlock`, `add_login`, …), read-only invoke assertions,
  and polling, pulled into each scenario file with `mod common; use common::*;`.
- Scenarios: one topical `#[ignore]` test file per feature area, each fully
  self-contained (its own `Session`) so a failure names the feature it broke:
  [`daily_loop`](tests/daily_loop.rs) (create → add → search → edit/history →
  favorite → folder → lock → **restart** → unlock → theme-persists) is the broadest;
  the rest cover one feature area each — generator, trash, audit, totp, health,
  session-lock, secret-key unlock, snapshots, backup, credential-lifecycle, re-key,
  recovery-key, reveal, delete, onboarding, about, help, and the docs-screenshot
  capture. Each carries a standing console-clean assertion. `mise e2e` runs **all** of them
  (it is not pinned to a single file); cargo runs the binaries serially.

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
- **Vault dir** — a temp dir holds the `.vedge` vault home; the create wizard is
  pointed at it via the `#vault-name` + `#vault-location` inputs (no native "Choose…" dialog).
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

## CI (opt-in, wired in 3.8)

The e2e job is defined in **`.github/workflows/e2e.yml`** and runs on **manual
dispatch only** (Windows runners bill at 2×), with a `runner` input — point it at
a **self-hosted Windows runner** (your dev machine already has WebView2 +
msedgedriver) for zero GitHub-hosted minutes. A nightly `schedule` is left
commented-out. The core gate (`mise ci` + `mise audit`) runs on PRs
(`.github/workflows/ci.yml`) and, off GitHub, in the local Dockerised Jenkins
(`ci/jenkins/`). An opt-in Linux e2e job would additionally need `xvfb-run` +
`libwebkit2gtk-4.1` + the secret-service (keyring).

## Selector convention — `data-testid`

Controls are located by **stable `data-testid`** attributes (kebab-case, semantic:
`onboarding-create`, `entry-save`, `sidebar-trash`, `row-restore`,
`login-generate-wand`, …) via `by_testid` / `click_testid` / `js_click_testid`.
This keeps the suite **locale-independent** — a copy edit or a locale switch never
breaks it. Form **inputs** keep their DOM `id`s (`ef-*`, `vault-search`,
`vault-name`, `vault-location`, `master-password`, `folder-new`, `gen-bulk-count`), used via
`fill_id`. A few structural hooks are reused directly: `tr[data-testid=<entry-id>]`
(entry rows — since 5.3.1b the vault table is a `DataTable` whose `row_testid` hook
carries the id), `[role='option']` (+ `[data-value=…]` for a specific
Select option), `[role='combobox']` (a Select trigger), `div[role='dialog']`.
When adding a scenario, tag the new control with `data-testid` at its call site
(`attr:data-testid="…"` spreads onto `vedge-ui` `Button`/`IconButton`/`SidebarItem`)
rather than selecting by text.

## Notes / limitations

- **Mutations via UI, assertions via IPC.** Actions go through the UI so the
  Leptos view and backend session stay in sync; `invoke()` is used only for
  read-only assertions. A few terminal mutations (history restore, folder move,
  theme set, and per-scenario setup like `soft_delete_entry` / persisting a pref)
  use `invoke()` directly and are noted inline.
- **Deterministic seams (3.8).** `Session::launch` sets `VEDGE_E2E_FAST_KDF=1`
  (cheap Argon2) and `VEDGE_E2E_BIOMETRIC_MEMORY=1` (in-memory biometric stub).
  Both are **debug-only + env-gated**, so they cannot exist in a release bundle.

## 🔑 The keychain is REAL — and the entries it leaves behind

Every other external dependency here is faked. The **OS keychain is not**, because using the real
credential store is the only way to actually test it. So each run writes real entries: a Secret
Key (`secret:{uuid}`) and a rollback baseline (`counter:{uuid}`) **per vault** — plus another
pair for every duplicate that slice 5.2.2's ② opens. They become orphans the moment the run's
temp dirs are cleaned up.

Under the production service (`vedge`) those orphans would be **indistinguishable from your own
vault keys**: they pile up forever, and deleting the wrong one destroys a real vault's Secret
Key. So the harness sets `VEDGE_E2E_KEYCHAIN_TEST=1`, and the app files every test entry under a
separate service:

```
vedge-e2e-test.secret:{uuid}     ← test entries, safe to delete
vedge-e2e-test.counter:{uuid}
vedge.secret:{uuid}              ← YOUR vaults. never written by a test.
```

**To purge them:** `mise e2e-clean` — it matches only `vedge-e2e-test*`, so it cannot touch a
real vault even if it misfires.

Like the other seams, the redirect is **debug-only + env-gated** (`resolve_keychain` in
`vedge-tauri/src/setup/services.rs`): a release bundle compiles the branch out entirely and can
only ever reach the real `vedge` service, so no environment variable can talk a shipped app into
storing secrets somewhere else.

## References

**Testing (this repo)**

- [`docs/technical/testing.md`](../../docs/technical/testing.md) — VEdge's four testing layers; this
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
