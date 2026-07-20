# Testing layers

VEdge's tests sit at four levels. Each catches a different class of bug; none subsumes the others.

## 1. Backend / host tests — `cargo test --workspace`

Pure Rust, host target, real SQLite + real crypto. This is where the **dangerous** bugs are caught — crypto, KDF, persistence, session lifecycle, "does an entry survive lock → re-unlock". Also the frontend's **pure logic** (extracted from components): `matches()`, `display_name_from_path`, `to_login_payload`, `ApiError::from_envelope`, dialog-option serde.

```
cargo test --workspace --exclude vedge-e2e --lib --bins --tests   # the workspace host suite (mise test)
```

(`--exclude vedge-e2e` keeps the WebDriver deps off the default gate — see level 4; `--lib --bins
--tests` skips a pre-existing broken `vedge-ui` doctest.)

**Limitation to remember:** these run with `serde_json`. The frontend talks to Tauri through **`serde_wasm_bindgen`** — a *different* codec. A DTO that round-trips in a `serde_json` host test can still fail in the app (notably `#[serde(flatten)]`, integer coercion, camelCase). Host DTO tests verify *shape*, not the *wire*.

## 2. Wasm wire-codec tests — `wasm-pack test --node`

Browserless. Exercises the **real** frontend IPC decode under **node** (no browser, no chromedriver): shell serializes with `serde_json` → the frontend receives a JS object (`JSON.parse`) → decodes with `serde_wasm_bindgen::from_value`. This is the only automated level that catches `serde_wasm_bindgen`-specific bugs.

```
wasm-pack test --node crates/vedge-ipc
```

Lives in `crates/vedge-ipc/tests/wire_wasm.rs` (cfg-gated to `wasm32`, so host `cargo test` skips it). It round-trips the IPC DTOs — including `RecentVaultStatusDto` (`#[serde(flatten)]`), `IndexEntryDto` (`i32` + options), the tagged `FieldSelectorDto`/`PayloadDto` unions, and `ErrorEnvelope`. **Add a case here whenever a DTO gains a `flatten`, an integer field, an untagged/adjacently-tagged enum, or a rename** — those are the shapes `serde_wasm_bindgen` can trip on.

> Prereqs: `wasm-pack` + `node` (no browser). First run compiles the wasm target and fetches the matching `wasm-bindgen` CLI.

## 3. Manual GUI smoke — `cargo tauri dev`

The whole path from a click to SQLite and back, in a real Tauri window. Catches **composition/navigation/reactive** bugs that only appear when components are mounted and driven (e.g. Leptos's "accessed outside a reactive tracking context" warning). Human-run; see [`docs/technical/smokes/`](smokes/README.md).

```
cd crates/vedge-tauri && cargo tauri dev
```

## 4. End-to-end — `mise e2e` (slice 2.9.2)

The full path from a WebDriver click to SQLite and back, in the **real built binary**, driven from Rust. `tauri-driver` + [`thirtyfour`] against the platform WebDriver (WebView2/msedgedriver on Windows, WebKitWebDriver on Linux) automate the Phase-2 daily loop — create → add → search → edit/history → favorite → folder → lock → **restart** → unlock → theme-persists — and stand a **console-clean guard** that fails on any Leptos "outside a reactive tracking context" warning (the recurring `spawn_local`/i18n footgun; the guard proves itself live with a self-test).

```
mise e2e                      # trunk build (debug wasm) → cargo build --features custom-protocol → cargo test -p vedge-e2e -- --ignored
cargo test --workspace        # unaffected: vedge-e2e is #[ignore]-gated AND --exclude'd from the gate
```

Lives in `crates/vedge-e2e/` (harness in `src/lib.rs`, scenario in `tests/daily_loop.rs`). **Opt-in / `#[ignore]`-gated** — it needs a display + the platform driver, so it never runs in the default `mise test`/`mise ci`. See [`crates/vedge-e2e/README.md`](../../crates/vedge-e2e/README.md) for prerequisites, the deterministic seams (`VEDGE_DATA_DIR`, temp vault dir, real keychain), and the manual boundary (real biometric prompt, native OS file dialogs).

> Prereqs: `cargo install tauri-driver --locked` + a version-matched platform WebDriver (msedgedriver ↔ WebView2). The build must embed **debug** wasm (`trunk build` without `--release`) or the console-clean warning never compiles in.

## Supply-chain gate — `mise audit` (slice 2.10.2)

Tests catch *our* bugs; the audit gate catches bugs (and licensing/provenance problems) in the many
crates we depend on. For a password manager this is a first-class security gate. It's separate from the
code gates because it needs network (the RustSec advisory DB) and two extra tools.

```
mise audit                      # cargo deny check  (advisories + bans + licenses + sources)
```

- **`cargo deny check`** (config: [`/deny.toml`](../../deny.toml)) is **the gate** — four checks:
  **advisories** ([RustSec DB](https://rustsec.org), plus unmaintained/yanked), **bans**
  (duplicate-version surfacing; the crypto version spread is documented-structural, so it runs at
  `warn`), **licenses** (a permissive SPDX allowlist — no GPL/AGPL in the tree), and **sources**
  (crates.io only). It is **build-graph / feature / target aware**, so it flags advisories only for
  crates we actually compile.
- **`cargo audit`** (installed by `mise setup`) is a supplementary **manual** tool, not the gate: it
  scans the raw `Cargo.lock` feature-union and therefore over-reports crates that aren't in our build
  (e.g. `rsa`, `quinn-proto` — feature-gated-unused, confirmed by an empty `cargo tree -i`). Gating on
  those would mean misleading `ignore`s for CVEs in crypto crates we don't compile. Run `cargo audit`
  by hand for a raw-lockfile view.

Establishing the baseline (2026-07-10) meant real triage: three transitive advisories were cleared by a
targeted `cargo update` (anyhow, crossbeam-epoch, rustls-webpki), the direct `aes` dep was moved off a
yanked `0.9.0` → `0.9.1`, and the `printpdf 0.7` PDF-export cluster (lopdf / quick-xml) is
**dated-`ignore`d with a reason** in `deny.toml` (we *generate* PDFs, never parse untrusted ones). Every
ignore carries a RUSTSEC id + reason — no silent ignores. Prereq: `cargo install cargo-audit cargo-deny
--locked` (in `mise setup`). Like `e2e`, it is **not** in the default `mise ci`; run it pre-release and
in an opt-in CI job.

## What's deliberately *not* here (yet)

- **A lint for the reactive/`spawn_local` footgun** — reading a signal (or `t!`/`t_string!`) inside a `spawn_local` future runs outside a reactive owner and warns at runtime. A grep/`cargo-dylint` check would catch it *at build time*; until then the e2e **console-clean guard** (level 4) nets it at runtime, and the convention holds: read signals in the handler body, move plain values into the async block.
- **Component/DOM unit tests** — the e2e level (4) now covers mounted-and-driven flows end-to-end; a lighter component-test tier is still absent (highest-fidelity coverage lives in the e2e daily loop).

## Continuous integration (slice 3.8)

Two CI substrates, so a fast dev pace never exhausts GitHub-hosted runner minutes:

- **GitHub Actions — the authoritative PR gate.** `.github/workflows/ci.yml` runs the core gate
  (`mise ci` + `mise audit`) on **`pull_request` into `develop`/`main` + manual dispatch only — no
  `push` trigger**, so feature-branch commits fire nothing (≈ one run per PR). `.github/workflows/e2e.yml`
  runs `mise e2e` on **`pull_request` into `develop`/`main` + a nightly `schedule`**, on a
  **self-hosted Windows runner** (your machine already has WebView2 + msedgedriver) — so e2e is part of the
  PR gate (PG.2a). Honest caveat: an offline runner makes the PR check *queue* rather than fail; the nightly
  is the backstop. The e2e job enables the fast-KDF + in-memory-biometric seams via the harness env.
- **Local Jenkins in Docker — off GitHub entirely.** `Jenkinsfile` + `ci/jenkins/` bring up a Dockerised
  Jenkins controller (toolchain baked in) that runs the same core gate on demand. See
  [`ci/jenkins/README.md`](../../ci/jenkins/README.md). Core gate only — e2e stays on the `mise e2e` /
  GitHub-e2e path (needs a display + WebDriver).

The e2e suite is **locale-independent** as of 3.8: controls are located by `data-testid`, not English text.
It also runs a **fast-KDF seam** (`VEDGE_E2E_FAST_KDF`, debug-only + release-impossible) so create/unlock no
longer dominate the run.

## Rule of thumb

| Bug class | Level that catches it |
|---|---|
| Crypto / persistence / data-loss | 1 (backend) |
| Frontend pure logic | 1 (host component tests) |
| IPC wire-format (`serde_wasm_bindgen`, flatten, tags) | **2 (wasm-node)** |
| Reactive-context / render / navigation / composition | 3 (manual smoke) + **4 (e2e daily loop + console-clean guard)** |
| Full user flow regression (create→edit→restart→unlock) | **4 (e2e — `mise e2e`)** |
| Dependency vulnerability / license / provenance | **Supply-chain gate (`mise audit`)** |
