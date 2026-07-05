# Smoke tests

Manual, human-driven click-paths that exercise a shipped vertical slice **end-to-end through the real app** — the thing the automated suites can't cover. The `cargo test` suites (`vedge-core`, `vedge-tauri`, `vedge-ipc`, `vedge-app`) verify each layer in isolation; a smoke verifies the whole path from a click to SQLite and back, in a running Tauri window.

Run a smoke before merging a UI-facing slice, and after any change to the shell ↔ frontend boundary.

## Running the app (dev)

Prerequisites (one-time):

- Rust toolchain + the wasm target: `rustup target add wasm32-unknown-unknown`
- [`trunk`](https://trunkrs.dev/) (bundles the Leptos/WASM frontend) — `cargo install trunk`
- Tauri CLI (already vendored as `cargo tauri`, v2.x)
- `tailwindcss` is fetched automatically by trunk (see `crates/vedge-app/Trunk.toml`)

Then, from the Tauri crate (where `tauri.conf.json` lives):

```bash
cd crates/vedge-tauri
cargo tauri dev
```

This runs `trunk serve` for the frontend (localhost:1420) and launches the **VEdge** window. **The first build is slow** (compiles Tauri + the WASM frontend); subsequent runs are incremental.

Notes:
- The window uses a **custom titlebar** (`decorations: false`) — window controls are in-app.
- **`withGlobalTauri: true`**, so you can drive commands directly from devtools (`window.__TAURI__.core.invoke(...)`) — handy for command-level smokes before the UI exists.
- Dev app-level state (`app.db`, recent vaults, settings, themes) lives in **`<workspace>/local/`** (debug builds only; gitignored). Delete that folder to reset app state.
- Dev builds use the **real OS keychain** (`OsKeychainProvider`, service `"vedge"`) — creating a vault writes the Secret Key to your OS credential store.

## Index

| Smoke | Covers | Slices |
|---|---|---|
| [Phase 1 — Create Vault](phase-1-create-vault.md) | Create → add → list/detail/copy/delete → lock → relaunch → unlock | 1.1–1.6 |

## Doc conventions

Each smoke doc has: **Scope** (what's in/out), **Prerequisites**, **Steps** (numbered click-path with expected results), **Edge cases**, **Data & cleanup**, and a pointer to the **automated coverage** that backs it.
