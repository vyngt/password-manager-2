# Testing layers

VEdge's tests sit at three levels. Each catches a different class of bug; none subsumes the others.

## 1. Backend / host tests — `cargo test --workspace`

Pure Rust, host target, real SQLite + real crypto. This is where the **dangerous** bugs are caught — crypto, KDF, persistence, session lifecycle, "does an entry survive lock → re-unlock". Also the frontend's **pure logic** (extracted from components): `matches()`, `display_name_from_path`, `to_login_payload`, `ApiError::from_envelope`, dialog-option serde.

```
cargo test --workspace          # ~352 tests
```

**Limitation to remember:** these run with `serde_json`. The frontend talks to Tauri through **`serde_wasm_bindgen`** — a *different* codec. A DTO that round-trips in a `serde_json` host test can still fail in the app (notably `#[serde(flatten)]`, integer coercion, camelCase). Host DTO tests verify *shape*, not the *wire*.

## 2. Wasm wire-codec tests — `wasm-pack test --node`

Browserless. Exercises the **real** frontend IPC decode under **node** (no browser, no chromedriver): shell serializes with `serde_json` → the frontend receives a JS object (`JSON.parse`) → decodes with `serde_wasm_bindgen::from_value`. This is the only automated level that catches `serde_wasm_bindgen`-specific bugs.

```
wasm-pack test --node crates/vedge-ipc
```

Lives in `crates/vedge-ipc/tests/wire_wasm.rs` (cfg-gated to `wasm32`, so host `cargo test` skips it). It round-trips the IPC DTOs — including `RecentVaultStatusDto` (`#[serde(flatten)]`), `IndexEntryDto` (`i32` + options), the tagged `FieldSelectorDto`/`PayloadDto` unions, and `ErrorEnvelope`. **Add a case here whenever a DTO gains a `flatten`, an integer field, an untagged/adjacently-tagged enum, or a rename** — those are the shapes `serde_wasm_bindgen` can trip on.

> Prereqs: `wasm-pack` + `node` (no browser). First run compiles the wasm target and fetches the matching `wasm-bindgen` CLI.

## 3. Manual GUI smoke — `cargo tauri dev`

The whole path from a click to SQLite and back, in a real Tauri window. Catches **composition/navigation/reactive** bugs that only appear when components are mounted and driven (e.g. Leptos's "accessed outside a reactive tracking context" warning). Human-run; see [`docs/smokes/`](smokes/README.md).

```
cd crates/vedge-tauri && cargo tauri dev
```

## What's deliberately *not* here (yet)

- **A lint for the reactive/`spawn_local` footgun** — reading a signal (or `t!`/`t_string!`) inside a `spawn_local` future runs outside a reactive owner and warns at runtime. A grep/`cargo-dylint` check would catch it without a browser. (Convention for now: read signals in the handler body, move plain values into the async block.)
- **Component/DOM tests** and **full Tauri E2E** (WebDriver/`tauri-driver`) — highest fidelity but need a display/driver and are flaky-prone. The manual smoke covers this mile until a flow keeps regressing; when E2E lands, wire it to **fail on Leptos reactive-context console warnings** so it also nets that class.

## Rule of thumb

| Bug class | Level that catches it |
|---|---|
| Crypto / persistence / data-loss | 1 (backend) |
| Frontend pure logic | 1 (host component tests) |
| IPC wire-format (`serde_wasm_bindgen`, flatten, tags) | **2 (wasm-node)** |
| Reactive-context / render / navigation / composition | 3 (manual smoke) → later a lint + E2E |
