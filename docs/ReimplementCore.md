# Reimplementing `vedge-core`

**Status:** Planning · **Scope:** Database layer first (app + vault) · **Owner:** core

---

## Why we are doing this

The current `vedge-core` mixes Clean Architecture concerns under a `business/` folder, puts errors at the crate root (`src/errors.rs`), bundles SQLCipher we will not use, and the schemas (`vault_item`, `theme`) do not match the canonical specs in `docs/Design/Vedge/Core/`. Rather than incrementally refactor, we drop the current implementation and rebuild against the specs.

This plan covers **only the database layer** for two reasons:

1. The schema is the contract every other module (vault crypto, sync, audit, etc.) reads from. Locking it in first prevents churn later.
2. Crypto, key derivation, and payload layers are large enough to deserve their own plans — see *Out of scope* below.

## Authoritative references

Read these before touching code. The plan below assumes their content.

| Doc | What it pins down |
|---|---|
| `docs/Design/Vedge/Core/vedge-core …md` | Layer boundaries, feature flags, dependency rules |
| `docs/Design/Vedge/Core/Database Design …md` | Two-database split (`app.db` vs `.vdb`) |
| `docs/Design/Vedge/Core/App Database Schema — app db …md` | All `app.db` tables |
| `docs/Design/Vedge/Core/Database Schema — Vault Storage Design …md` | All `.vdb` tables (`vault_config`, `tags`, `entries`, `audit_log`) |
| `docs/Design/Vedge/Core/Key Hierarchy …md` | Required crypto-related columns and their meaning |
| `docs/Design/Vedge/Core/vault …md` | Domain types, ports, errors (target shape for later phases) |

## SeaORM target version and conventions

Verified against upstream on 2026-04-18:

- **`sea-orm = "1.1.20"`, `sea-orm-migration = "1.1.20"`** — latest stable 1.x (MSRV 1.81, compatible with our Rust 2024 crate). `2.0.0-rc.*` is still in release-candidate and is **out of scope** until it ships stable and the 2.0 migration guide is complete.
- **Features used** (SQLite-only build):
  - `sea-orm`: `default-features = false`, features = `["sqlx-sqlite", "runtime-tokio", "macros", "sqlite-use-returning-for-3_35"]`. `runtime-tokio-rustls` is not needed — SQLite has no TLS. `sqlite-use-returning-for-3_35` enables `RETURNING` with SQLite ≥ 3.35 (already bundled); it becomes default in 2.0.
  - `sea-orm-migration`: `default-features = false`, features = `["runtime-tokio", "sqlx-sqlite", "sqlite-use-returning-for-3_35", "with-chrono"]`. **No `cli` feature** — we run migrations programmatically from the app, not via `sea-orm-cli`.
- **Bundled SQLite.** Depend on `libsqlite3-sys = { version = "*", features = ["bundled"] }` (no `bundled-sqlcipher*`) to guarantee a recent SQLite regardless of the host system library.
- **Migration trait shape** (1.1.x verified):
  ```rust
  use sea_orm_migration::prelude::*;

  #[derive(DeriveMigrationName)]          // auto-derives MigrationName from the module path
  pub struct Migration;

  #[async_trait::async_trait]
  impl MigrationTrait for Migration {
      async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> { /* … */ }
      async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> { /* … */ }
  }
  ```
  Prefer `#[derive(DeriveMigrationName)]` over manual `impl MigrationName` — the current codebase hand-rolls the trait, which we are dropping.
- **Migration file naming:** `mYYYYMMDD_HHMMSS_snake_case_name.rs`. The `HHMMSS` component is a real timestamp, not a sequence number — this is what `DeriveMigrationName` reads to order migrations and what `sea-orm-cli migrate status` displays. Example: `m20260418_093000_create_vault_config.rs`.
- **`Migrator` registration:**
  ```rust
  // infrastructure/sqlite/app/migrations.rs
  pub struct Migrator;

  impl MigratorTrait for Migrator {
      fn migrations() -> Vec<Box<dyn MigrationTrait>> {
          vec![
              Box::new(m20260418_000001_create_recent_vaults::Migration),
              Box::new(m20260418_000002_create_app_settings::Migration),
              // …in chronological order
          ]
      }
  }
  ```
  Each `app/` and `vault/` adapter has its own `Migrator` — they are separate schemas, tracked in separate `seaql_migrations` tables (different database files).
- **Running migrations programmatically** (no CLI): `Migrator::up(&db, None).await?` inside `AppDbConnection::open` / `VaultDbConnection::open`. Available helpers on `MigratorTrait`: `up`, `down`, `fresh`, `refresh`, `reset`, `status`, `get_pending_migrations`. All accept `&DatabaseConnection`.
- **Schema helpers:** `sea_orm_migration::schema::*` provides `timestamp`, `timestamp_null`, `binary`, `binary_len`, `string`, `string_null`, `integer`, `boolean`, `pk_uuid`, `table_auto`, etc. Use these where they match our columns; fall back to raw `ColumnDef::new(...)` for BLOB types with fixed length checks, JSON TEXT, and SQLite-specific constraints. The helpers are a shorthand, not a requirement.
- **BLOB columns** (`vault_salt`, `verify_hash`, `dek_wrapped`, `nonce`, `ciphertext`, `public_key`, `session_key`): use `ColumnDef::new(Col::Name).blob().not_null()`. In 1.1.x `binary()` and `blob()` both map to `BLOB` on SQLite; prefer `blob()` for readability. Enforce exact byte lengths in the **mapper layer**, not as a SQL constraint (SQLite does not enforce column types strictly).
- **Transactions in migrations:** SeaORM does not wrap each migration in an implicit transaction. For multi-statement migrations that must be atomic (e.g., create table + seed rows), use `manager.get_connection().begin().await?` explicitly and commit at the end, or split into two migrations if seeding is independent from schema.
- **Seed data** (e.g., built-in themes): do it in the migration using `manager.exec_stmt(Query::insert()…)` with `INSERT OR IGNORE` so re-runs are idempotent. Keep seed values in a `const` block at the top of the migration file — never read from external files.
- **`sqlite-use-returning-for-3_35`:** when enabled, `Model::insert(...).exec_with_returning(...)` works on SQLite. We rely on this in the repository layer for `upsert` paths that need the generated timestamp row back without a second query.

## Architectural commitments (binding)

- **Three layers, dependency arrows point inward only:** `domain → application → infrastructure`. A `shell/` (Tauri/CLI/Server) lives in sibling crates and wires infrastructure into use cases.
- **`business/` is gone.** Folders are exactly `domain/`, `application/`, `infrastructure/`.
- **Rust 2024 edition, no `mod.rs`.** The crate is on `edition = "2024"` (see `crates/vedge-core/Cargo.toml`). Modules use the sibling-file pattern: a `foo.rs` next to a `foo/` directory holding its submodules. Never create `mod.rs`. This matches the existing convention (`business.rs` + `business/`, `infra.rs` + `infra/`).
- **Errors live in `domain/`.** `VaultError`, `AppDbError`, etc. are domain types. Infrastructure converts driver errors (`sea_orm::DbErr`, `std::io::Error`) into domain errors at the boundary; it never leaks them upward.
- **No SQLCipher.** Drop `bundled-sqlcipher` and `bundled-sqlcipher-vendored-openssl`. `app.db` is plaintext by design; `.vdb` confidentiality comes from per-row XChaCha20-Poly1305, not from file-level encryption.
- **SeaORM lives only inside `infrastructure/sqlite/`.** Domain and application import nothing from SeaORM. Swapping to raw `sqlx` later must be a one-layer change.
- **Two databases, two repositories.** `AppDbRepository` and `VaultRepository` are separate ports with separate connection types. No shared "DataSource" abstraction over both.
- **Async only at the application/infrastructure boundary.** Domain is pure synchronous Rust (no `async`, no `tokio`).

## Out of scope for this plan

The following are deliberately deferred. Each will get its own plan once the database layer is solid.

- Cryptography: `CryptoProvider`, `KeyDerivationProvider`, `KeychainProvider`.
- Decrypted payload model (`EntryPayload`, `CommonMeta`, `LoginPayload`, etc.).
- High-level vault use cases (`UnlockVault`, `CreateEntry`, `CopyField`, `ChangePassword`).
- `VaultSession`, `VaultIndex`, in-memory state.
- Blob store (`BlobStore`) for documents.
- Tauri / CLI / Server wiring beyond what compiles.

This plan stops at: **the two SQLite schemas exist, are migrated, and can be read/written through repository ports that hand back raw row structs.**

---

## Phase 0 — Cleanup and crate skeleton

**Goal:** empty slate, dependencies trimmed, new directory layout in place.

1. Delete everything under `crates/vedge-core/src/` except `lib.rs`.
2. Delete everything under `crates/vedge-core/tests/`.
3. Rewrite `crates/vedge-core/Cargo.toml`:
   - Remove `libsqlite3-sys` with `bundled-sqlcipher*` features; replace with `libsqlite3-sys = { version = "*", features = ["bundled"] }` to guarantee a modern SQLite without SQLCipher.
   - Remove `aes-gcm`, `argon2`, `validator`, `dotenvy`, `anyhow` (not needed at this layer).
   - Keep / add:
     - `serde = { version = "1", features = ["derive"] }`
     - `serde_json = "1"`
     - `thiserror = "2"`
     - `chrono = { version = "0.4", features = ["serde"] }`
     - `ulid = "1"` (replaces the existing `uuid` dependency — the spec uses ULIDs)
     - `async-trait = "0.1"`
     - `sea-orm = { version = "1.1.20", default-features = false, features = ["sqlx-sqlite", "runtime-tokio", "macros", "sqlite-use-returning-for-3_35"] }`
     - `sea-orm-migration = { version = "1.1.20", default-features = false, features = ["runtime-tokio", "sqlx-sqlite", "sqlite-use-returning-for-3_35", "with-chrono"] }`
   - `tokio` becomes a `[dev-dependencies]` entry with `features = ["rt-multi-thread", "macros"]` (domain/application are sync; infrastructure is `async` via `async-trait` but the runtime is provided by the shell, not the core).
   - Drop the `vedge-codegen` path dependency unless a concrete use case reintroduces it.
4. Create the new module tree using the Rust 2024 sibling-file pattern (no `mod.rs` anywhere). Each module is a `name.rs` file next to an optional `name/` directory for submodules. Empty files / re-exports only at this stage:
   ```
   crates/vedge-core/src/
   ├── lib.rs
   ├── domain.rs                       // pub mod shared; pub mod app; pub mod vault;
   ├── domain/
   │   ├── shared.rs                   // pub mod ids; pub mod timestamps; pub mod errors;
   │   ├── shared/
   │   │   ├── ids.rs
   │   │   ├── timestamps.rs
   │   │   └── errors.rs               // StorageError (shared by AppDbError + VaultError)
   │   ├── app.rs                      // pub mod entities; pub mod errors;
   │   ├── app/
   │   │   ├── entities.rs             // pub mod recent_vault; pub mod app_setting; …
   │   │   ├── entities/
   │   │   │   ├── recent_vault.rs
   │   │   │   ├── app_setting.rs
   │   │   │   ├── theme.rs
   │   │   │   ├── known_device.rs
   │   │   │   └── extension_session.rs
   │   │   └── errors.rs               // AppDbError
   │   ├── vault.rs                    // pub mod entities; pub mod errors; pub mod kdf_params;
   │   └── vault/
   │       ├── entities.rs             // pub mod vault_config; pub mod entry_row; …
   │       ├── entities/
   │       │   ├── vault_config.rs
   │       │   ├── entry_row.rs
   │       │   ├── tag_row.rs
   │       │   └── audit_event.rs
   │       ├── errors.rs               // VaultError
   │       └── kdf_params.rs
   ├── application.rs                  // pub mod app; pub mod vault;
   ├── application/
   │   ├── app.rs                      // pub mod ports;
   │   ├── app/
   │   │   └── ports.rs                // RecentVaultRepository, AppSettingRepository, …
   │   ├── vault.rs                    // pub mod ports;
   │   └── vault/
   │       └── ports.rs                // VaultRepository
   ├── infrastructure.rs               // pub mod sqlite;
   └── infrastructure/
       └── sqlite.rs                   // pub mod app; pub mod vault;
       └── sqlite/
           ├── app.rs                  // pub mod connection; pub mod entities; pub mod mappers; pub mod migrations; pub mod repositories;
           ├── app/
           │   ├── connection.rs
           │   ├── entities.rs
           │   ├── entities/
           │   │   ├── recent_vault.rs
           │   │   ├── app_setting.rs
           │   │   ├── theme.rs
           │   │   ├── known_device.rs
           │   │   ├── extension_session.rs
           │   │   └── schema_migration.rs
           │   ├── mappers.rs
           │   ├── mappers/
           │   │   ├── recent_vault.rs
           │   │   ├── app_setting.rs
           │   │   ├── theme.rs
           │   │   ├── known_device.rs
           │   │   └── extension_session.rs
           │   ├── migrations.rs       // SeaORM Migrator + module list
           │   ├── migrations/
           │   │   ├── m20260418_000001_create_recent_vaults.rs
           │   │   ├── m20260418_000002_create_app_settings.rs
           │   │   ├── m20260418_000003_create_themes.rs
           │   │   ├── m20260418_000004_create_known_devices.rs
           │   │   ├── m20260418_000005_create_extension_sessions.rs
           │   │   └── m20260418_000006_create_indexes.rs
           │   ├── repositories.rs
           │   └── repositories/
           │       ├── recent_vault.rs
           │       ├── app_setting.rs
           │       ├── theme.rs
           │       ├── known_device.rs
           │       └── extension_session.rs
           ├── vault.rs                // pub mod connection; pub mod entities; pub mod mappers; pub mod migrations; pub mod repository;
           └── vault/
               ├── connection.rs
               ├── entities.rs
               ├── entities/
               │   ├── vault_config.rs
               │   ├── tag.rs
               │   ├── entry.rs
               │   ├── audit_log.rs
               │   └── schema_migration.rs
               ├── mappers.rs
               ├── mappers/
               │   ├── vault_config.rs
               │   ├── tag.rs
               │   ├── entry.rs
               │   └── audit_log.rs
               ├── migrations.rs
               ├── migrations/
               │   ├── m20260418_000001_create_vault_config.rs
               │   ├── m20260418_000002_create_tags.rs
               │   ├── m20260418_000003_create_entries.rs
               │   ├── m20260418_000004_create_audit_log.rs
               │   └── m20260418_000005_create_indexes.rs
               └── repository.rs       // SqliteVaultRepository (single struct implements VaultRepository)
   ```
   Note the duplicated `└──` for `sqlite.rs` + `sqlite/` is intentional in the diagram — both exist side by side, and that is exactly the pattern.
5. `lib.rs` re-exports the three layers: `pub mod domain; pub mod application; pub mod infrastructure;`.
6. Audit `vedge-tauri` and `vedge-tools` for `vedge_core::*` imports. Mark them broken — they will be rewired in a later, separate plan. Add `#[allow(dead_code)]` shims only if needed to keep the workspace compiling for adjacent crates; otherwise let them break and fix in their own PR.

**Done when:** `cargo build -p vedge-core` succeeds with an empty crate; the workspace state of other crates is documented (broken-and-tracked vs. still-building).

---

## Phase 1 — Domain: shared primitives

**Goal:** value types and identifiers used by both `app/` and `vault/` domains.

**Files:** `domain/shared.rs`, `domain/shared/{ids.rs, timestamps.rs, errors.rs}`.

- `EntryId(String)`, `TagId(String)`, `VaultId(PathBuf)`, `DeviceId(String)`, `SessionId(String)`, `ThemeId(String)` — newtypes around ULID strings (or `PathBuf` for `VaultId`). Provide `new()` for ULID-backed ones using the `ulid` crate.
- `Timestamp` = type alias for `chrono::DateTime<chrono::Utc>` plus a `now()` helper. Keep the rest of the codebase off `chrono::Utc::now()` so we have one swap point for time.
- No `serde` derives required at this layer yet — add them only when application/use-cases need them.

**Done when:** `cargo test -p vedge-core` (empty test set is fine) compiles; module layout matches the doc.

---

## Phase 2 — Domain: `app.db` entities and errors

**Goal:** every `app.db` table modelled as a pure-Rust struct, with a domain `AppDbError` enum.

**Files:** `domain/app.rs`, `domain/app/entities.rs`, `domain/app/entities/{recent_vault.rs, app_setting.rs, theme.rs, known_device.rs, extension_session.rs}`, `domain/app/errors.rs`.

Entity structs (one per table; field names match the spec exactly, types are domain types not DB types):

- `RecentVault { id: ThemeId-style ULID, path: PathBuf, display_name: String, last_opened: Option<Timestamp>, sort_order: i32 }`
- `AppSetting { key: String, value: serde_json::Value, updated_at: Timestamp }` — value is `serde_json::Value` so the domain can stay schema-agnostic; typed accessors live in application/use cases.
- `Theme { id: ThemeId, name: String, is_built_in: bool, root_background: String, root_foreground: String, root_primary: String, danger_base: Option<String>, warning_base: Option<String>, success_base: Option<String>, created_at: Timestamp, updated_at: Timestamp }`
- `KnownDevice { device_id: DeviceId, display_name: String, public_key: [u8; 32], first_seen: Timestamp, last_seen: Option<Timestamp> }`
- `ExtensionSession { session_id: SessionId, browser: String, profile_name: Option<String>, session_key: [u8; 32], created_at: Timestamp, last_active_at: Option<Timestamp> }`

Domain error:

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppDbError {
    #[error("recent vault not found: {0}")]
    RecentVaultNotFound(String),
    #[error("setting not found: {0}")]
    SettingNotFound(String),
    #[error("theme not found: {0}")]
    ThemeNotFound(ThemeId),
    #[error("built-in theme cannot be modified or deleted")]
    BuiltInThemeImmutable,
    #[error("device not found: {0}")]
    DeviceNotFound(DeviceId),
    #[error("extension session not found: {0}")]
    ExtensionSessionNotFound(SessionId),
    #[error("invalid setting value for key {key}: {reason}")]
    InvalidSettingValue { key: String, reason: String },
    #[error("storage failure")]
    Storage(#[source] StorageError),
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(String),                     // wraps stringified sea_orm::DbErr at infra boundary
    #[error("io error: {0}")]
    Io(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("migration failed: {0}")]
    Migration(String),
}
```

`StorageError` is shared with `vault/errors.rs` (both wrap it via a `Storage` variant). Place it in `domain/shared/errors.rs` and re-export.

**Done when:** the entities + errors compile, with no SeaORM, no `async`, no `tokio` imports.

---

## Phase 3 — Application: `app.db` ports

**Goal:** repository traits the use-case layer (and Tauri/CLI shells) will depend on.

**Files:** `application/app.rs`, `application/app/ports.rs`.

Define one port per table family. Keep them narrow — no "god repository":

```rust
#[async_trait]
pub trait RecentVaultRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<RecentVault>, AppDbError>;
    async fn get(&self, id: &str) -> Result<RecentVault, AppDbError>;
    async fn upsert(&self, vault: &RecentVault) -> Result<(), AppDbError>;
    async fn delete(&self, id: &str) -> Result<(), AppDbError>;
    async fn touch_last_opened(&self, id: &str, when: Timestamp) -> Result<(), AppDbError>;
}

#[async_trait]
pub trait AppSettingRepository: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<AppSetting>, AppDbError>;
    async fn set(&self, key: &str, value: serde_json::Value, when: Timestamp) -> Result<(), AppDbError>;
    async fn delete(&self, key: &str) -> Result<(), AppDbError>;
    async fn list_prefix(&self, prefix: &str) -> Result<Vec<AppSetting>, AppDbError>;
}

#[async_trait]
pub trait ThemeRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<Theme>, AppDbError>;
    async fn get(&self, id: &ThemeId) -> Result<Theme, AppDbError>;
    async fn upsert(&self, theme: &Theme) -> Result<(), AppDbError>;
    async fn delete(&self, id: &ThemeId) -> Result<(), AppDbError>;     // rejects built-ins via BuiltInThemeImmutable
}

#[async_trait]
pub trait KnownDeviceRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<KnownDevice>, AppDbError>;
    async fn upsert(&self, device: &KnownDevice) -> Result<(), AppDbError>;
    async fn touch_last_seen(&self, id: &DeviceId, when: Timestamp) -> Result<(), AppDbError>;
    async fn delete(&self, id: &DeviceId) -> Result<(), AppDbError>;
}

#[async_trait]
pub trait ExtensionSessionRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<ExtensionSession>, AppDbError>;
    async fn upsert(&self, session: &ExtensionSession) -> Result<(), AppDbError>;
    async fn touch_last_active(&self, id: &SessionId, when: Timestamp) -> Result<(), AppDbError>;
    async fn delete(&self, id: &SessionId) -> Result<(), AppDbError>;
}
```

No use cases yet — those land in a follow-up plan once shells need them. Ports being defined unblocks the infrastructure phase.

**Done when:** ports compile and have no SeaORM types in their signatures.

---

## Phase 4 — Infrastructure: `app.db` SeaORM adapter

**Goal:** concrete SQLite repositories backed by SeaORM, plus the migration set.

**Files:** `infrastructure/sqlite/app.rs`, then under `infrastructure/sqlite/app/`: `connection.rs`, `entities.rs` + `entities/*.rs`, `mappers.rs` + `mappers/*.rs`, `migrations.rs` + `migrations/m*.rs`, `repositories.rs` + `repositories/*.rs`. (See Phase 0 tree for the full file list.)

1. **Connection** — `AppDbConnection::open(path: &Path) -> Result<Self, StorageError>`. Builds a `sea_orm::DatabaseConnection` against `sqlite://{path}?mode=rwc`. No SQLCipher pragmas. Runs pending migrations on open.
2. **SeaORM entities** — one per table, in `entities/`: `recent_vault.rs`, `app_setting.rs`, `theme.rs`, `known_device.rs`, `extension_session.rs`, `schema_migration.rs`. These are SeaORM-flavoured structs only; never escape this module.
3. **Migrations** — versioned in `migrations/`, registered in a single `Migrator` per the SeaORM 1.1.x convention. Each file carries `#[derive(DeriveMigrationName)] pub struct Migration;` plus an `#[async_trait::async_trait] impl MigrationTrait for Migration` with real `up` and `down` bodies (no "We Don't Do That Here" placeholders — `down` drops what `up` created). Filenames use the canonical `mYYYYMMDD_HHMMSS_name.rs` format; pick real clock values at creation time rather than `_000001_`-style counters.
   Initial migration set:
   - `m20260418_090001_create_recent_vaults`
   - `m20260418_090002_create_app_settings`
   - `m20260418_090003_create_themes` (seeds the two built-in rows: `builtin-light`, `builtin-dark`, `is_built_in = 1` — use `INSERT OR IGNORE` so re-runs are idempotent)
   - `m20260418_090004_create_known_devices`
   - `m20260418_090005_create_extension_sessions`
   - `m20260418_090006_create_indexes` (the three `CREATE INDEX` statements from the spec)
   Migrations are not auto-transactional; for the themes seeding step, wrap the create-table + seed inserts in a single `manager.get_connection().begin().await?` transaction so a partial seed never leaves the DB in a mid-migration state.
4. **Mappers** — pure functions converting SeaORM models ↔ domain entities. One file per entity. Path conversions, `serde_json` parsing for `app_setting.value`, `[u8; 32]` length checks for keys all happen here. Length / parse failures map to `AppDbError::InvalidSettingValue` or `StorageError::Serialization`.
5. **Repositories** — one struct per port, each holding `Arc<DatabaseConnection>`. Implement the port traits from Phase 3. The only place `sea_orm::DbErr` exists is inside these files; convert to `StorageError::Database(err.to_string())` at every call.
6. **Built-in theme seeding** is *idempotent*: the migration uses `INSERT OR IGNORE`. Re-running migrations on an existing DB never overwrites user edits to those rows (consistent with the spec's "unknown keys preserved").

**Done when:** `cargo test -p vedge-core` runs an integration test that creates a temp `app.db`, runs migrations, performs CRUD across all five tables via the repositories, and asserts round-trips match.

---

## Phase 5 — Domain: vault (`.vdb`) entities and errors

**Goal:** the four `.vdb` tables modelled as raw row structs (no decryption logic — that lives in the future crypto layer).

**Files:** `domain/vault.rs`, `domain/vault/entities.rs`, `domain/vault/entities/{vault_config.rs, entry_row.rs, tag_row.rs, audit_event.rs}`, `domain/vault/errors.rs`, `domain/vault/kdf_params.rs`.

Entity structs:

- `VaultConfig { id: String /* "default" */, magic: String, schema_version: i32, vault_salt: [u8; 32], kdf_params: KdfParams, verify_hash: [u8; 32], preferred_cipher_suite: i32, trash_retention_days: i32, audit_retention_days: i32, created_at: Timestamp, last_unlocked_at: Option<Timestamp> }`
- `KdfParams { alg: String, m: u32, t: u32, p: u32, version: u32 }` with `Serialize`/`Deserialize` — the SQLite column is JSON.
- `EntryRow { id: EntryId, version: i64, cipher_suite: i32, dek_wrapped: [u8; 40], nonce: [u8; 24], ciphertext: Vec<u8>, created_at: Timestamp, updated_at: Timestamp, accessed_at: Option<Timestamp>, is_trashed: bool, trashed_at: Option<Timestamp> }` — exactly the spec's `entries` columns. This is *not* a decrypted entry; that type belongs to a later crypto phase and will live in `domain/vault/payload/`.
- `TagRow { id: TagId, nonce: [u8; 24], ciphertext: Vec<u8>, created_at: Timestamp, updated_at: Timestamp }`
- `AuditEvent { id: String, entry_id: Option<EntryId>, action: AuditAction, occurred_at: Timestamp, device_id: Option<DeviceId> }`
- `AuditAction` enum — every variant from the spec (`Unlocked`, `Locked`, `Created`, `Viewed`, `Updated`, `Deleted`, `Restored`, `PermanentlyDeleted`, `Exported`, `PasswordChanged`, `TagCreated`, `TagRenamed`, `TagDeleted`).

Domain error:

```rust
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("vault config not found")]
    ConfigMissing,
    #[error("vault file is not a Vedge vault (magic mismatch)")]
    BadMagic,
    #[error("unsupported schema version: {0}")]
    UnsupportedSchemaVersion(i32),
    #[error("entry not found: {0:?}")]
    EntryNotFound(EntryId),
    #[error("tag not found: {0:?}")]
    TagNotFound(TagId),
    #[error("audit event not found: {0}")]
    AuditNotFound(String),
    #[error("invalid kdf params: {0}")]
    InvalidKdfParams(String),
    #[error("invalid blob length for {field} (expected {expected}, got {actual})")]
    InvalidBlobLength { field: &'static str, expected: usize, actual: usize },
    #[error("storage failure")]
    Storage(#[source] StorageError),
}
```

`StorageError` is reused from `domain/shared/errors.rs`.

Crypto-related variants (`DecryptionFailed`, `WrongCredentials`, `MlockFailed`, …) are deliberately deferred to the crypto plan — the database layer cannot raise them.

**Done when:** entity + error files compile; no SeaORM, no async, no crypto deps.

---

## Phase 6 — Application: vault ports

**Goal:** the `VaultRepository` trait, scoped to raw rows only.

**Files:** `application/vault.rs`, `application/vault/ports.rs`.

```rust
#[async_trait]
pub trait VaultRepository: Send + Sync {
    // vault_config (exactly one row, id = "default")
    async fn load_config(&self) -> Result<VaultConfig, VaultError>;
    async fn save_config(&self, config: &VaultConfig) -> Result<(), VaultError>;

    // entries — raw encrypted rows
    async fn insert_entry(&self, row: &EntryRow) -> Result<(), VaultError>;
    async fn update_entry(&self, row: &EntryRow) -> Result<(), VaultError>;
    async fn get_entry(&self, id: &EntryId) -> Result<EntryRow, VaultError>;
    async fn all_entries(&self) -> Result<Vec<EntryRow>, VaultError>;
    async fn soft_delete_entry(&self, id: &EntryId, when: Timestamp) -> Result<(), VaultError>;
    async fn restore_entry(&self, id: &EntryId, when: Timestamp) -> Result<(), VaultError>;
    async fn hard_delete_entry(&self, id: &EntryId) -> Result<(), VaultError>;
    async fn hard_delete_trashed_before(&self, cutoff: Timestamp) -> Result<u64, VaultError>;
    async fn update_accessed_at(&self, id: &EntryId, when: Timestamp) -> Result<(), VaultError>;

    // tags — raw encrypted rows
    async fn insert_tag(&self, row: &TagRow) -> Result<(), VaultError>;
    async fn update_tag(&self, row: &TagRow) -> Result<(), VaultError>;
    async fn get_tag(&self, id: &TagId) -> Result<TagRow, VaultError>;
    async fn all_tags(&self) -> Result<Vec<TagRow>, VaultError>;
    async fn delete_tag(&self, id: &TagId) -> Result<(), VaultError>;

    // audit_log
    async fn append_audit(&self, event: &AuditEvent) -> Result<(), VaultError>;
    async fn recent_audit(&self, limit: u32) -> Result<Vec<AuditEvent>, VaultError>;
    async fn audit_by_entry(&self, entry_id: &EntryId, limit: u32) -> Result<Vec<AuditEvent>, VaultError>;
    async fn delete_audit_before(&self, cutoff: Timestamp) -> Result<u64, VaultError>;
}
```

The trait deliberately mirrors `vault …md` § Ports verbatim so the crypto-layer plan can adopt it without renames.

**Done when:** trait compiles, all parameter and return types are domain types only.

---

## Phase 7 — Infrastructure: vault SeaORM adapter

**Goal:** SeaORM-backed `SqliteVaultRepository` implementing the Phase 6 port.

**Files:** `infrastructure/sqlite/vault.rs`, then under `infrastructure/sqlite/vault/`: `connection.rs`, `entities.rs` + `entities/*.rs`, `mappers.rs` + `mappers/*.rs`, `migrations.rs` + `migrations/m*.rs`, `repository.rs`. (See Phase 0 tree for the full file list.)

1. **Connection** — `VaultDbConnection::open(path: &Path) -> Result<Self, StorageError>`. Plain SQLite, **no SQLCipher**, `mode=rwc`. Migrations run on open.
2. **SeaORM entities** — `vault_config.rs`, `tag.rs`, `entry.rs`, `audit_log.rs`, `schema_migration.rs`. BLOB columns are `Vec<u8>`; mappers enforce length invariants.
3. **Migrations** — each in its own file under `migrations/`, all using `#[derive(DeriveMigrationName)]` + `MigrationTrait`:
   - `m20260418_100001_create_vault_config` (defines the table; inserting the single `id = 'default'` row is the **application's** job at vault creation time, not a migration — leave the table empty).
   - `m20260418_100002_create_tags`
   - `m20260418_100003_create_entries`
   - `m20260418_100004_create_audit_log`
   - `m20260418_100005_create_indexes` — exactly the four indexes from the spec (`idx_entries_trashed`, `idx_entries_updated`, `idx_entries_accessed`, `idx_audit_entry`).
4. **Mappers** — convert between SeaORM models and `domain/vault/entities`. KDF params (TEXT JSON) parsed via `serde_json`; parse failure → `VaultError::InvalidKdfParams`. BLOB length checks for `vault_salt`, `verify_hash`, `dek_wrapped`, `nonce` map to `VaultError::InvalidBlobLength`.
5. **Repository** — `SqliteVaultRepository { conn: Arc<DatabaseConnection> }` implements `VaultRepository`. All methods are short — fetch/insert/update/delete via SeaORM, convert errors. `hard_delete_trashed_before` and `delete_audit_before` use a single `DELETE … WHERE` and return the affected row count.

**Done when:** an integration test creates a temp `.vdb`, runs migrations, exercises every `VaultRepository` method with synthetic encrypted blobs (random bytes — we are not asserting decryption, only round-trip storage), and asserts row counts and field equality.

---

## Phase 8 — Wiring + verification

**Goal:** the rest of the workspace compiles against the new core (or the breakage is explicitly tracked).

1. Re-export the public surface from `lib.rs`:
   ```rust
   pub mod domain;
   pub mod application;
   pub mod infrastructure;

   pub use domain::shared::*;          // ids, timestamps, StorageError
   pub use domain::app::{entities::*, errors::AppDbError};
   pub use domain::vault::{entities::*, errors::VaultError};
   pub use application::app::ports::*;
   pub use application::vault::ports::*;
   pub use infrastructure::sqlite::app::{AppDbConnection, repositories::*};
   pub use infrastructure::sqlite::vault::{VaultDbConnection, SqliteVaultRepository};
   ```
2. `vedge-tauri` and `vedge-tools` will not compile against this surface — their old types (`VaultItem`, `Theme` use cases, `UnlockVaultUseCase`, etc.) are gone. Decide per consumer:
   - If the consumer is needed for the next product milestone: open a follow-up task to rewire it against the new repositories. This is *not* in scope for this plan.
   - If the consumer can stay broken for now: gate it behind a workspace member exclusion in `Cargo.toml` and document in `MEMORY.md`/PR description.
3. Run the verification matrix:
   - `cargo build -p vedge-core` — no warnings.
   - `cargo test -p vedge-core` — phase 4 + phase 7 integration tests pass.
   - `cargo build --workspace` — either green, or red with a documented and tracked set of consumer crates.
4. Smoke-test on Windows (primary dev platform): ensure `app.db` and a sample `.vdb` can be created at `%APPDATA%\vedge\` paths used by Tauri.

**Done when:** the verification matrix passes and all non-core crate breakage is captured in follow-up issues.

---

## What comes after this plan

Sketched here so phasing decisions above stay aligned with the longer arc — these are *not* commitments inside this plan.

1. **Crypto plan** — `CryptoProvider`, `KeyDerivationProvider`, `KeychainProvider` ports + XChaCha20-Poly1305 / Argon2id / OS keychain implementations. Adds the `Secret`/`Zeroizing` types and the crypto-flavoured variants to `VaultError`.
2. **Payload plan** — `CommonMeta` + per-type `*Payload` structs, `EntryPayload` enum, `payload_schema` versioning, JSON serialization rules.
3. **Use-case plan** — `UnlockVault`, `CreateEntry`, `UpdateEntry`, `CopyField`, `ChangePassword`, `RunMaintenance`, `Create/Rename/DeleteTag`. Introduces `VaultSession` and `VaultIndex`.
4. **Blob store plan** — `BlobStore` port + filesystem implementation for `.vedge_blobs/`.
5. **Shell rewire plan** — repoint `vedge-tauri` / `vedge-cli` / `vedge-server` at the new use cases and ports.

---

## Cross-cutting rules (apply to every phase)

- **No `unwrap()` / `expect()` outside test code.** Every fallible path returns a domain error.
- **No `anyhow` in `vedge-core`.** Use `thiserror`-derived domain errors only. `anyhow` is acceptable in `vedge-tools` binaries.
- **Time goes through `domain::shared::Timestamp::now()`.** No direct `chrono::Utc::now()` calls outside that helper.
- **IDs are generated through the newtypes** (`EntryId::new()`, `TagId::new()`, etc.). No raw `ulid::Ulid::new().to_string()` at call sites.
- **Migrations are append-only.** Once committed, never edit a migration file; add a new one. The `schema_migrations` table is the source of truth.
- **Tests live next to the layer they test.** Domain tests in `#[cfg(test)] mod tests` blocks; infrastructure integration tests in `crates/vedge-core/tests/sqlite_*.rs`.
