# Smoke — Create Vault (Phase 1)

Verifies the create-vault vertical slice end-to-end: **UI wizard → `create_vault` command → `vedge-core` → SQLite**, landing in an unlocked vault. Backs slices **1.1** (core), **1.2** (shell), **1.3** (onboarding UI).

## Scope

**In:** create a brand-new vault from the UI (location dialog → master password → Emergency Kit → unlocked at `/v/vault`), plus the error/edge cases (already-exists, password gate, kit acknowledgement, keychain warning).

**Out (not built yet):** unlock an existing vault from the UI, add/list/edit/delete entries (slices 1.4/1.5), import/recover (Phase 3). After "Finish" you land on the vault shell but the entry list is not wired.

## Prerequisites

- App running via `cd crates/vedge-tauri && cargo tauri dev` (see [README](README.md)).
- A writable folder to hold the test vault, e.g. `C:\Users\<you>\vedge-smoke\` — **create this folder first** (the create flow makes the `.vdb` file, not its parent directories).
- Use a **fresh `.vdb` filename** each run, or clean up between runs (see Data & cleanup) — re-using a path triggers the "already exists" case (which is itself worth testing once).

## Steps (happy path)

| # | Action | Expected |
|---|---|---|
| 1 | App launches | Lands on the **unlock screen** (password field + "Create a new vault" button). |
| 2 | Click **"Create a new vault"** | Navigates to `/onboarding`; wizard shows step **Location** with the 3-step indicator (Location · Password · Emergency Kit). |
| 3 | Click **"Choose…"** | A **native save dialog** opens, filtered to `*.vdb`. Pick `…\vedge-smoke\test.vdb`. The path appears in the field. (You can also type/paste a path instead.) |
| 4 | Click **Next** | Enabled only when the path is non-empty. Advances to **Password**. |
| 5 | Type a master password, e.g. `correct horse battery staple` | The **strength meter** fills (this passphrase reads ~"Good"). |
| 6 | Type the **same** password in **Confirm** | **"Create vault"** becomes enabled (needs strength ≥ *Fair* **and** matching confirm). |
| 7 | Click **"Create vault"** | Runs `create_vault`. Advances to **Emergency Kit**. |
| 8 | Observe the **Secret Key** (`A3-XXXXX-…`) | Rendered in a monospace block. Click the **copy** icon → it copies (paste elsewhere to confirm). |
| 9 | Click **"Download Emergency Kit (PDF)"** | A **native save dialog** opens (`*.pdf`). Choose a path → "Emergency Kit saved." appears. **Open the PDF** → it renders the kit (vault name, Secret Key, KDF summary). |
| 10 | Tick **"I have saved my Secret Key / Emergency Kit."** | **"Finish"** becomes enabled (it is disabled until acknowledged). |
| 11 | Click **Finish** | Navigates to **`/v/vault`** — the unlocked vault shell (sidebar visible). Entry list is empty / not wired yet (slice 1.5). |

**On disk after step 7:** `test.vdb` exists at the chosen path, with a sibling `test.vedge_blobs/` directory. The Secret Key is in the OS keychain (service `"vedge"`).

## Edge cases

- **Vault already exists** — re-run the wizard, choose a path that already has a `.vdb`, reach step Password, click **"Create vault"** → an inline error **"A vault already exists at this location."** appears and you stay on the Password step (nothing is overwritten). This confirms `VaultError::VaultAlreadyExists` → `CommandError::AlreadyExists` → `ApiError::AlreadyExists` is surfaced.
- **Password strength gate** — a weak password (e.g. `abc`) leaves the meter low and **"Create vault" disabled**. A confirm that doesn't match also keeps it disabled.
- **Kit acknowledgement is non-skippable** — **"Finish" stays disabled** until the checkbox is ticked.
- **Keychain unavailable** — if the OS keychain write fails (`keychain_stored == false`), step Emergency Kit shows a stronger warning that the kit is the only way back in. Hard to force on a healthy Windows machine; primarily a code-path note.

## Add a login entry (slice 1.4)

Once you land at `/v/vault` (the `ActiveVault` path is set by the create flow), the "new item" form persists through the backend. The list itself is **not** wired until slice 1.5, so confirm the write out-of-band.

| # | Action | Expected |
|---|---|---|
| 1 | Click **"New item"** | The add-entry form appears (Title / Identifier / Password / URL). |
| 2 | Fill **Title** = `GitHub`, Identifier = `alice`, Password = `s3cret`, URL = `https://github.com` | Save is enabled once Title is non-empty. |
| 3 | Click **Save** | The form closes (no error). Under the hood: `create_entry` → an encrypted row in `entries`. |
| 4 | In **devtools**: `await window.__TAURI__.core.invoke('list_entries', { vault_path: '<your path>' })` | Returns an array with the new `IndexEntryDto` (`name: "GitHub"`, `entry_type: "Login"`, `url`, timestamps). |
| 5 | (Optional) Inspect the `entries` table in the `.vdb` | The row's payload is **ciphertext**, not plaintext `s3cret`. |

**Edge cases:**
- **No active vault** — if you reach the form without an active vault (e.g. navigate directly), Save shows an inline **"Could not save the entry: …"** and keeps your input.
- **Empty title** — the Save button stays disabled.
- **Not in the list yet** — the row won't render in the on-screen table until slice 1.5 wires `list_entries`. This is expected for 1.4.

## Command-level smoke (optional, faster than the UI)

Because `withGlobalTauri` is enabled, you can exercise the command directly in **devtools** (Ctrl+Shift+I in the dev window) without the wizard:

```js
// Create (parent folder must exist; use forward slashes to avoid JS escaping)
await window.__TAURI__.core.invoke('create_vault', {
  input: { vault_path: 'C:/Users/<you>/vedge-smoke/test2.vdb',
           master_password: 'correct horse battery staple' }
})
// → { secret_key_display: "A3-…", keychain_stored: true }

// Confirm it ended unlocked
await window.__TAURI__.core.invoke('is_unlocked', { vault_path: 'C:/Users/<you>/vedge-smoke/test2.vdb' })
// → true

// Re-run the same path → the AlreadyExists error
await window.__TAURI__.core.invoke('create_vault', {
  input: { vault_path: 'C:/Users/<you>/vedge-smoke/test2.vdb', master_password: 'x' }
}).catch(e => e)
// → { kind: "AlreadyExists" }
```

## Data & cleanup

Created by a run:
- The vault file `…/test.vdb` **and** its sibling `…/test.vedge_blobs/` directory.
- An OS keychain entry under service **`vedge`** (Windows: *Credential Manager → Windows Credentials*, search "vedge").
- App-level rows in `<workspace>/local/app.db` (only if you wired recents — the create flow itself does not add a recent yet).

To reset:
1. Delete the `.vdb` file and the `.vedge_blobs/` sibling.
2. (Optional) Remove the `vedge` entry from the OS keychain.
3. (Optional) Delete `<workspace>/local/` to wipe all dev app-level state.

## Automated coverage backing this smoke

- **Core:** `cargo test -p vedge-core` — `create_then_unlock_roundtrip`, `create_refuses_existing_path`, `config_fields_are_correct`, `created_audit_event_appended`, `keychain_failure_returns_valid_vault`, …
- **Shell:** `cargo test -p vedge-tauri` — `create_vault_leaves_vault_unlocked`, `create_vault_on_existing_path_is_already_exists`.
- **IPC:** `cargo test -p vedge-ipc` — DTO serde round-trips.
- **App logic:** `cargo test -p vedge-app` — `ApiError` envelope decoding (incl. `AlreadyExists`), dialog-option serde, password-strength scoring.

These cover every layer independently; this smoke is what confirms they compose in the running app (the one path with no automated coverage).
