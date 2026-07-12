# VEdge — Architecture

VEdge is a **local-first, offline** password manager built with **Rust + Tauri v2** (desktop shell) and
**Leptos 0.8 / WASM** (CSR frontend). There is no server and no cloud: a vault is a single encrypted
file on disk, unlocked with a master password (+ a device Secret Key), and every secret is encrypted
**at the application level, per entry** — the database file itself is plaintext SQLite.

This document describes the shipped (v3) architecture. It is kept in sync with the code; every crypto
claim below is drawn from `crates/vedge-core`. The canonical, deeper specs live in the design vault
(`03 Specs/10 Core/10.3 - Key Hierarchy`, `20.0/20.2 Database`).

## Workspace layout

A single Cargo workspace of eight crates:

| Crate | Role |
|---|---|
| **`vedge-core`** | All business logic — domain model, use-cases, crypto, and storage. Clean-architecture layers `application/` · `domain/` · `infrastructure/`. No async runtime or UI of its own. |
| **`vedge-tauri`** | The Tauri v2 desktop shell (the app binary). A thin adapter: `#[tauri::command]` handlers, `AppState` (session registry), DTO conversions, PDF/emergency-kit rendering. Zero business logic. |
| **`vedge-ipc`** | The serde wire types shared frontend ↔ shell (DTOs + `ErrorEnvelope` + b64/timestamp helpers). Compiles to `wasm32`; no `vedge-core`/`tauri` deps. |
| **`vedge-generator`** | Pure secret-generation engine — five modes (random / passphrase / PIN / pronounceable / pattern) dispatched through one `GenSpec` + `generate`/`entropy_bits`, each returning `(secret, entropy_bits)` from **one code path** so the meter can't disagree with the generator; **honest per-mode entropy** (process, not string length). `generate_many(&GenSpec, count)` (capped at `MAX_BATCH = 50`) fans the same dispatch into a batch. Embeds the EFF 7776 wordlist via `include_str!` (the repo's first embedded asset). No SQLite/OS/serde deps; compiles native **and** `wasm32` (browser RNG via `getrandom`'s `wasm_js` feature), so the frontend generates with **zero IPC** and native shells reuse it. |
| **`vedge-app`** | The Leptos **CSR** frontend application — pages, features, routing (`leptos_router`), i18n (`leptos_i18n`), and typed `api::*` wrappers over `invoke()`. |
| **`vedge-ui`** | The Leptos **CSR** design-system component library (`foundation`/`form`/`feedback`/`data_display`/`icon`) + theme engine. |
| **`vedge-codegen`** | Proc-macros (e.g. `#[derive(StringEnum)]`). |
| **`vedge-e2e`** | WebDriver end-to-end harness (`tauri-driver` + `thirtyfour`); `publish = false`, opt-in / `#[ignore]`-gated. |

```
┌──────────────────────────── Desktop window (Tauri v2) ─────────────────────────┐
│  vedge-app (Leptos CSR)  ──uses──▶  vedge-ui (design system)                    │
│        │                                                                        │
│        │  api::* wrappers  →  window.__TAURI__.invoke(cmd, args)                │
│        ▼                                                                        │
│  ══════════════════ IPC boundary: vedge-ipc DTOs (serde) ══════════════════     │
│        ▼                                                                        │
│  vedge-tauri  (#[tauri::command] handlers · AppState · DTO⇄domain conversions)  │
│        │                                                                        │
│        ▼                                                                        │
│  vedge-core   application/ (use-cases, VaultSession)                            │
│               domain/      (payloads, index, recovery, key hierarchy, AAD)      │
│               infrastructure/ (crypto · Sea-ORM SQLite · keychain · clipboard)  │
│        │                                                                        │
│        ▼                                                                        │
│  app.db  (settings/themes/recents — Sea-ORM)     vault.vdb  (encrypted entries) │
└─────────────────────────────────────────────────────────────────────────────────┘
```

## `vedge-core` — Clean Architecture

- **`domain/`** — pure types and rules, no I/O. Vault: the entry `payloads/` (login, card, note,
  api_key, ssh_key, env_vars, identity, document, folder, tag), the in-memory `index`, `recovery`
  (Emergency Kit / Secret Key encoding), `aad`, `kdf_params`, `crypto_constants`, and the SQLite
  `entities/` (`vault_config`, `entry_row`, `tag_row`, `entry_history_row`, `audit_event`).
- **`application/`** — use-cases orchestrating the domain over **ports** (traits): `UnlockVault`,
  `create_entry`/`update_entry`/`soft_delete`/`restore`/`hard_delete`, `copy_field`, `move_entry`, the
  tag ops, `import/export_document`, `change_password`, `run_maintenance`, plus `VaultSession` (the
  live, unlocked session). Ports abstract crypto, KDF, keychain, clipboard, blob store, and repos.
- **`infrastructure/`** — the adapters: RustCrypto (`crypto/`), Sea-ORM SQLite repos (`sqlite/`),
  OS keychain (`keyring`), clipboard (`arboard`), and the filesystem blob store.

## Security architecture

The whole point of VEdge: **secrets are encrypted at rest, per entry, and keys live only in locked RAM
during a session.** There is no full-database encryption (no SQLCipher) — the SQLite file is plaintext
and holds ciphertext columns.

### Key hierarchy — unlock

```
master password ─┐
                 ├─(1) HKDF-SHA256 "2SKD"──▶ 32-byte preprocessed input ─┐
16-byte Secret ──┘   (ikm=password, salt=Secret Key,                    │
      Key            info="vedge-v1-2skd")                               │
                                                                        (2) Argon2id
per-vault 32-byte salt ─────────────────────────────────────────────────┘  (v1.3; default
                                                                            m=256 MiB, t=3, p=4)
                                                                            → 32-byte master key
                             (3) HKDF-SHA256 expand (from master key):
                                   • info="vedge-v1-kek"    → KEK (Key Encryption Key)
                                   • info="vedge-v1-verify" → verify-hash
                                   • info="vedge-v1-sync-auth" → sync-auth subkey (reserved)
```

1. **2SKD** — the master password (IKM) and the 16-byte **Secret Key** (HKDF salt) are combined by
   HKDF-SHA256 into one 32-byte input. The Secret Key is a device-held random value (shown to the user
   in the Emergency Kit as `A3-XXXXX-…`), so a stolen vault file **plus** a guessed password is still
   not enough — the attacker also needs the Secret Key.
2. **Argon2id** (v1.3) runs over the 2SKD output + a per-vault 32-byte salt to produce the 32-byte
   **master key**. The Argon2 parameters (`m`, `t`, `p`) are stored **per vault**, so they can be
   upgraded over time. This step runs on a blocking thread.
3. **HKDF-SHA256** expands the master key into subkeys: the **KEK**, a **verify-hash**, and a reserved
   sync-auth subkey. On unlock the verify-hash is compared **in constant time** against the stored value
   **before any secret material is loaded** — a wrong password fails here, cheaply, with no oracle.

The KEK is held in a `SecretMem` region (see below) for the life of the session; the master key and
2SKD output are `Zeroizing` and dropped immediately.

### Per-entry encryption

- Each entry carries a **per-entry, per-write Data Encryption Key (DEK)** — a fresh 32-byte key is
  generated on every create/update (no key reuse across versions). The DEK is **wrapped with AES-KW
  (AES Key Wrap, RFC 3394)** under the session KEK, yielding the 40-byte `dek_wrapped` stored on the
  row.
- The entry payload (serialized JSON of the secret fields) is sealed with **XChaCha20-Poly1305** under
  the DEK: a 24-byte XNonce + a 16-byte Poly1305 tag. The **associated data (AAD)** binds the
  ciphertext to its identity: `AAD = 16-byte entry ULID ‖ u64-LE version`, so a row can't be swapped or
  rolled back to a different entry/version without failing decryption.
- **Tags** are sealed with XChaCha20-Poly1305 **directly under the KEK** (no DEK), with `AAD = 16-byte
  tag ULID`. **Document blobs** use a per-entry DEK with `AAD = entry ULID ‖ "blob"`.

At rest a row is `{ dek_wrapped, nonce, ciphertext, … }`. Unlock unwraps each DEK with the KEK, then
opens each payload; lock zeroizes the KEK and the decrypted in-memory index.

### Key-material handling

- **`SecretMem<T>`** — a page-aligned, heap-pinned region that is `mlock`ed (via the `region` crate) so
  the KEK never lands in swap / `pagefile.sys`, and is zeroized on `Drop`. The KEK lives here.
- **`secrecy::SecretString`** wraps the plaintext secret fields inside payloads (passwords, TOTP seeds,
  card numbers, API secrets, note bodies, SSH keys, …) so they don't linger in `String`s and never
  print in `Debug`.
- **`VaultSession`** owns the mlock'd KEK + the in-memory `VaultIndex`; it is `!Clone`, its `Drop`
  unconditionally zeroizes, and its `Debug` never reveals secrets. Locking drops the session.

### Recovery & biometric unlock

- **Emergency Kit / recovery:** the Secret Key is encoded as a checksummed Crockford-base32 string
  (`A3-XXXXX-…`) the user records; recovery re-supplies it to unlock without the keychain.
- **Biometric (Windows Hello / Touch ID):** an alternate `unlock_with_kek` path releases the KEK from
  the OS-protected store after a biometric check and validates it against ciphertext — skipping the KDF.
  The master password remains a fallback. (Windows Hello is shipped; macOS Touch ID is a follow-up.)

## Storage at rest

- **Plain, bundled SQLite** (`libsqlite3-sys` with the bundled SQLite compiled in) accessed through
  **Sea-ORM 1.1.20** (→ `sqlx-sqlite`). **No SQLCipher**, no file-level DB encryption — confidentiality
  comes entirely from the per-row application-level AEAD above.
- **Two databases, same Sea-ORM stack:**
  - **`app.db`** — non-secret app state: settings, themes, recent vaults, known devices. One shared
    connection, its own migrator.
  - **the vault `*.vdb`** — the encrypted entries, tags, per-entry history, `vault_config` (KDF params,
    salts, verify-hash), and the audit log. Its own connection + migrator, opened on unlock.

The in-memory `VaultIndex` is a decrypted projection (names, types, folders, tags, timestamps) rebuilt
from the payloads on unlock; it never persists plaintext.

## IPC boundary

The frontend never touches `vedge-core` directly. It calls typed `api::*` wrappers in `vedge-app` that
`invoke()` `#[tauri::command]`s in `vedge-tauri`; both sides serialize with the **`vedge-ipc`** DTOs.
Command errors cross as a `{kind, message}` `ErrorEnvelope` — each `CommandError` variant has a stable
`kind` string (the `envelope::kind::*` constants) the frontend matches structurally into `ApiError`,
never by parsing `message` (e.g. `DocumentTooLarge` is its own kind, not a `contains("too large")`).
Plaintext secrets stay backend-side wherever possible (e.g. copy-to-clipboard is a backend command with
auto-clear); the one sanctioned, audited exception is the explicit "reveal" (`get_entry`). **The TOTP
seed is held to a stricter rule (slice 4.2 — the "door"):** it flows WASM → core on enrolment only and
**never** core → WASM — `get_entry` omits it (`LoginPayloadDto` carries a non-invertible `has_totp` flag,
not the seed). The 6–8-digit **code** is a permitted derivative: short-lived, non-invertible to the
seed, worthless once expired, produced server-side by the audited `reveal_totp`. Generalizing that seed
rule to the other payload secrets (`password`, recovery codes, card `cvv`, SSH key) is a backlogged
follow-up that would actually clean the renderer.

The egress rule extends to the **network** (slice 4.4 — the app's only outbound call): the opt-in
HaveIBeenPwned breach check hashes a credential in **core**, sends **only** the uppercase 5-hex SHA-1
prefix (k-anonymity) to `api.pwnedpasswords.com` from `vedge-tauri` (`reqwest`, never the webview's HTTP
ACL), and matches the suffix locally — the password and full hash never leave the process. It is
**off by default and the backend verifies consent** before any egress. The rule, stated once: *only
non-invertible derivatives leave the process — to WASM or to the network — and network egress only with
explicit opt-in.*

Every clipboard write — `copy_field`, `copy_history_field`, and the generic `copy_text` used by the
renderer-side copies (the password generator and the Secret Key display) — funnels through one hardened
`ClipboardProvider::set`, which marks the payload with the platform **no-history / no-cloud exclusion
hints** (Windows monitor-processing exclusion, macOS `ConcealedType`, X11 password-manager hint) so it
skips OS clipboard history / cloud sync and well-behaved managers, alongside the timed auto-clear.

## Frontend

- **`vedge-app`** — Leptos CSR: pages under `/` (launch/unlock) and `/v` (the unlocked vault:
  list/detail/edit, search, command palette, folders — including a **Trash** sidebar folder for
  soft-deleted entries with restore / delete-permanently / empty-trash — settings). State is signals/stores + a few
  well-scoped contexts (`ActiveVault`, `VaultUiState`, `SecurityPrefsCtx`, `GeneratorPrefsCtx`,
  `GeneratedHistoryCtx`, `ThemeState`). App-global preferences (auto-lock/clipboard, the last-used
  generator **mode + per-mode preset**) live in the `app_settings` KV store under fixed keys
  (`security.prefs`, `generator.prefs`); `GeneratorPrefs` is the app-side serde mirror of the engine
  configs and stays `Copy` (the free-text pattern string is session-local, not persisted).
- **Session-secret hygiene** — the generator's **bulk results** and its **Recent history**
  (`GeneratedHistoryCtx`, an app-root `RwSignal<Vec<HistoryItem>>`) hold `Zeroizing<String>` secrets that
  are **never** serialized/persisted (`HistoryItem` is non-`Serialize`/`Debug`), are capped, and are
  **wiped on vault lock** — the app's first explicit session-secret wipe, a prev-guarded `Effect` in
  `app.rs` watching `ActiveVault.path` `Some→None` (the single choke point all three lock paths funnel through).
- **`vedge-ui`** — the reusable component library and the OKLCH theme engine (light/dark/custom themes,
  injected as CSS custom properties). CSR-only; styles live in `styles/*.css` under `@layer components`.

## Technology stack

- **Language / shell:** Rust 2024, Tauri v2, Trunk (frontend bundler), Tailwind-style utility CSS.
- **Frontend:** Leptos 0.8 (CSR), `leptos_router`, `leptos_i18n`, `reactive_stores`, `icondata`.
- **Crypto (RustCrypto):** `argon2` (Argon2id), `hkdf` + `sha2` (HKDF-SHA256), `chacha20poly1305`
  (XChaCha20-Poly1305), `aes` + `aes-kw` (AES Key Wrap), `hmac` (recovery checksum), `zeroize`,
  `secrecy`, `subtle`, `region` (mlock), `rand`.
- **Storage:** Sea-ORM 1.1.20 + `sqlx-sqlite` + bundled `libsqlite3-sys`.
- **Platform:** `keyring` (OS credential store), `arboard` (clipboard), `printpdf` (emergency-kit PDF),
  `totp-rs` (TOTP), `windows` (Windows Hello).

## Testing & gates

Four testing layers (host `cargo test` · wasm wire-codec · manual `cargo tauri dev` smoke · WebDriver
e2e) plus the CI gates (`fmt` / `leptosfmt` / `clippy -D warnings` whole-workspace / `wasm` check) and a
supply-chain gate (`cargo audit` + `cargo deny`). See [`docs/testing.md`](docs/testing.md) and
[`README.md`](README.md#development) for the commands (`mise ci`, `mise audit`, `mise e2e`).

CI runs on **two substrates**: GitHub Actions (`.github/workflows/ci.yml` — the authoritative PR gate,
triggered by PR-into-`develop`/`master` + manual dispatch only, deliberately **no `push`**; `e2e.yml`
dispatch-only) and a local **Jenkins-in-Docker** controller (`Jenkinsfile` + `ci/jenkins/`) that runs
the same core gate off GitHub — both pinned to the project's dev rust toolchain so CI clippy == local.
The WebDriver e2e drives a fast, locale-independent (`data-testid`) daily-loop; its Argon2 fast-KDF and
in-memory-biometric **test seams are release-impossible** — gated on **both** `cfg(debug_assertions)`
**and** an explicit `VEDGE_E2E_*` env var, so they compile out of any `cargo tauri build` (release) bundle.
