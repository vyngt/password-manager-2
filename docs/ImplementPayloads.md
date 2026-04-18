# Implementing `vedge-core` payload layer

**Status:** Planning · **Scope:** `CommonMeta`, `EntryType`, per-type payloads, `EntryPayload` enum, `TagPayload` · **Depends on:** `ReimplementCore.md` + `ImplementCrypto.md` (both complete)

---

## Why this plan

Crypto and storage are in place, but nothing speaks about *what* gets encrypted. Every use case (`CreateEntry`, `CopyField`, `UpdateEntry`, `ChangePassword`) needs typed payload structs it can serialize to JSON, hand to `CryptoProvider::encrypt_entry`, and then deserialize back after decryption. Without these types, use cases would juggle raw `serde_json::Value`, scatter `password` / `totp_secret` field lookups across the codebase, and provide no compile-time guarantees that secrets are zeroized.

This plan stops at: **every payload shape from the vault spec is a Rust type with serde impls, secrets are `SecretString`-wrapped, the `EntryPayload` enum round-trips through the existing `XChaCha20CryptoProvider`, and golden JSON vectors pin the wire format so silent schema drift is impossible.**

## Authoritative references

| Doc | What it pins down |
|---|---|
| `docs/Design/Vedge/Core/vault …md` § Domain types | Full `CommonMeta`, every `*Payload` struct, `EntryPayload` enum shape |
| `docs/Design/Vedge/Core/Database Schema — Vault Storage Design …md` § Payload structure | JSON wire format for each entry type, `CommonMeta` fields, forward-compat rules |

## Architectural commitments (binding)

- **Payloads live in `domain/vault/payloads/`** — a new sibling to `domain/vault/entities/`. One file per payload type plus an `entry_payload.rs` for the enum. Still pure domain: no `async`, no `sea-orm`, no `chacha20poly1305`, no `tokio`.
- **Secrets are `secrecy::SecretString`**, not `String`. Any field the spec marks as a secret (passwords, TOTP seeds, PEM keys, card numbers, PINs, CVVs, national IDs, env var values) uses `SecretString`. Non-secret strings (names, URLs, filenames, cardholder names) stay as `String`.
- **Serde is asymmetric.** Deserialization is automatic via `secrecy`'s `Deserialize` impl for `SecretString` — safe, builds a zeroizing wrapper. Serialization requires `#[serde(serialize_with = "expose_secret_string")]` on every secret field. One helper in `domain/vault/payloads/serde_secret.rs`, called by name at every use site. This enforces *explicit intent* to expose — accidental `serde_json::to_string(&payload)` on a naïve type cannot leak secrets.
- **JSON wire format is flat** — `CommonMeta` fields sit at the top level alongside type-specific fields, with `entry_type` acting as the discriminator. Matches the schema spec verbatim (`{ "name": "...", "entry_type": "Login", "url": "...", "username": "...", "password": "..." }`). `EntryPayload` implements `Serialize`/`Deserialize` manually by routing on `entry_type` through an intermediate `serde_json::Value` — no `#[serde(tag = "...")]`, which would force nesting.
- **`payload_schema` is a u32 in `CommonMeta`, default `1`.** Deserialization of a payload with `payload_schema > CURRENT_PAYLOAD_SCHEMA` returns `VaultError::UnsupportedPayloadSchema(n)`. Adding a new entry type does **not** bump `payload_schema` — only incompatible field changes within an existing type do.
- **`EntryType::Unknown(String)` preserves forward-compatibility.** When an old client encounters a new entry type, it wraps the unknown string in `Unknown` and stores the raw JSON body in an `unknown_fields: serde_json::Value` field on a dedicated `UnknownPayload`. The entry stays visible in the list (via metadata) but cannot be decrypted into a typed payload. We do *not* round-trip write an unknown payload — read-only preservation is enough.
- **No `#[derive(Zeroize)]` on outer payload structs.** Each secret field is a `SecretString` that zeroizes on drop; the plaintext JSON `Vec<u8>` returned by `CryptoProvider::decrypt_entry` is already `Zeroizing<Vec<u8>>`. That's two zeroize events — enough. Outer structs stay simple.
- **Tag payloads live here too.** `TagPayload { name, color, sort_order }` is a small plain struct — no secrets, but it's serialized/encrypted the same way and belongs alongside its siblings.

## Out of scope for this plan

- `IndexEntry` and `VaultIndex` — these are the non-secret in-memory projection consumed by the UI. They belong in the use-case plan alongside `VaultSession`.
- `TagMeta` (the in-memory decrypted form of a TagRow) — same story, use-case plan.
- `UnlockVault`, `CreateEntry`, `CopyField`, `ChangePassword` and the rest of the use cases — next plan.
- Migration helpers when `payload_schema` eventually bumps — not needed until there's a v2.
- Validation rules (email format, ISO 3166 country codes, PEM structure) — payloads are storage types, not form validators. Validation lives at the use-case boundary.

---

## Phase 0 — Dependencies

**Goal:** `secrecy` can serialize and deserialize. Build stays green.

1. **`Cargo.toml`** — enable the serde feature on `secrecy`:
   ```toml
   secrecy = { version = "0.10", features = ["serde"] }
   ```
   No new crates. `serde_json` is already in the dependency set.
2. **Verify** — `cargo build -p vedge-core` clean; 45 existing tests still pass.

**Done when:** the new feature resolves without breaking anything.

---

## Phase 1 — `CommonMeta`, `EntryType`, shared helpers

**Goal:** the base types every payload embeds, plus the serde-secret helper.

**Files:**

- `domain/vault/payloads.rs` — module index: `pub mod common_meta; pub mod entry_type; pub mod serde_secret; ...` (more added in later phases).
- `domain/vault/payloads/common_meta.rs`:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  pub struct CommonMeta {
      pub name: String,
      pub entry_type: EntryType,
      #[serde(default)]
      pub url: Option<String>,
      #[serde(default)]
      pub favicon_url: Option<String>,
      #[serde(default)]
      pub tag_ids: Vec<TagId>,
      #[serde(default)]
      pub folder_id: Option<EntryId>,
      #[serde(default)]
      pub is_favorite: bool,
      #[serde(default)]
      pub notes: Option<String>,
      #[serde(default = "default_payload_schema")]
      pub payload_schema: u32,
  }

  pub const CURRENT_PAYLOAD_SCHEMA: u32 = 1;
  fn default_payload_schema() -> u32 { CURRENT_PAYLOAD_SCHEMA }
  ```
- `domain/vault/payloads/entry_type.rs`:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  #[serde(rename_all = "PascalCase")]
  pub enum EntryType {
      Login,
      Card,
      SshKey,
      ApiKey,
      EnvVars,
      Note,
      Document,
      Identity,
      Folder,
      #[serde(untagged)]
      Unknown(String),
  }
  ```
  `#[serde(untagged)]` on the fallback arm lets serde capture any unrecognized string.
- `domain/vault/payloads/serde_secret.rs`:
  ```rust
  use secrecy::{ExposeSecret, SecretBox, SecretString};
  use serde::Serializer;

  /// Serialize a `SecretString` by exposing it. Only call from within a payload
  /// struct whose output is immediately encrypted. Every call site must be
  /// reviewable as "this is the encryption boundary".
  pub fn expose_secret_string<S: Serializer>(
      secret: &SecretString,
      ser: S,
  ) -> Result<S::Ok, S::Error> {
      secret.expose_secret().serialize(ser)
  }

  /// Same for optional secrets.
  pub fn expose_optional_secret_string<S: Serializer>(
      secret: &Option<SecretString>,
      ser: S,
  ) -> Result<S::Ok, S::Error>;

  /// Same for `Vec<SecretString>` (used by recovery_codes).
  pub fn expose_secret_string_vec<S: Serializer>(
      secrets: &[SecretString],
      ser: S,
  ) -> Result<S::Ok, S::Error>;
  ```
- `domain/vault/errors.rs` — add:
  ```rust
  #[error("unsupported payload schema: {0}")]
  UnsupportedPayloadSchema(u32),
  #[error("unsupported entry type: {0}")]
  UnsupportedEntryType(String),
  #[error("malformed payload: {0}")]
  MalformedPayload(String),
  ```
- `domain/vault.rs` — re-export `CommonMeta`, `EntryType`, `CURRENT_PAYLOAD_SCHEMA`.

**Tests** (inline `#[cfg(test)]` in each file):
- `CommonMeta` round-trips through serde_json with all optional fields both present and absent.
- `EntryType` serializes as PascalCase strings: `EntryType::Login` → `"Login"`, `EntryType::SshKey` → `"SshKey"`.
- Unknown type: `serde_json::from_str::<EntryType>("\"Passkey\"")` → `EntryType::Unknown("Passkey")`.
- `payload_schema` defaults to 1 when missing from input JSON.

**Done when:** the two structs + helper compile; unit tests pass.

---

## Phase 2 — Per-entry payload types

**Goal:** every entry kind from the spec has a Rust struct with serde impls. Field names and visibility exactly match the schema.

Each file lives under `domain/vault/payloads/` and follows this skeleton:

```rust
use secrecy::SecretString;
use serde::{Deserialize, Serialize};

use crate::domain::vault::payloads::common_meta::CommonMeta;
use crate::domain::vault::payloads::serde_secret::expose_secret_string;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginPayload {
    #[serde(flatten)]
    pub meta: CommonMeta,

    pub username: String,
    #[serde(serialize_with = "expose_secret_string")]
    pub password: SecretString,
    #[serde(default, serialize_with = "expose_optional_secret_string")]
    pub totp_secret: Option<SecretString>,
    #[serde(default, serialize_with = "expose_secret_string_vec")]
    pub recovery_codes: Vec<SecretString>,
}
```

The complete file list:

| File | Struct | Secret fields | Non-secret fields |
|---|---|---|---|
| `payloads/login.rs` | `LoginPayload` | `password`, `totp_secret`, `recovery_codes` | `username` |
| `payloads/card.rs` | `CardPayload` | `number`, `cvv`, `pin` | `cardholder_name`, `expiry_month` (`u8`), `expiry_year` (`u16`) |
| `payloads/ssh_key.rs` | `SshKeyPayload` | `private_key_pem`, `passphrase` | `public_key`, `fingerprint`, `key_type` |
| `payloads/api_key.rs` | `ApiKeyPayload` | `key`, `secret` | `endpoint`, `expiry` (`Option<String>`), `key_type` |
| `payloads/env_vars.rs` | `EnvVarsPayload` + `EnvVar` | `EnvVar.value` | `EnvVar.key`, `EnvVarsPayload.vars` |
| `payloads/note.rs` | `NotePayload` | `content` | — |
| `payloads/document.rs` | `DocumentPayload` | — (blob is encrypted separately) | `filename`, `mime_type`, `size_bytes` (`u64`), `blob_nonce` (`[u8; 24]` — serialized as base64) |
| `payloads/identity.rs` | `IdentityPayload` + `Address` | `national_id` | `first_name`, `last_name`, `email`, `phone`, `address`, `date_of_birth` |
| `payloads/folder.rs` | `FolderPayload` | — | just `meta` |
| `payloads/unknown.rs` | `UnknownPayload` | — | `meta`, `unknown_fields: serde_json::Value` |

**Notes per type:**

- **`DocumentPayload.blob_nonce`** is a 24-byte nonce that keys the external `.vedge_blobs/{entry_id}.blob` file. Serialize it as a base64 string (standard alphabet, no padding) so JSON stays text-clean. Add a small `serde_nonce` helper module in `payloads/` for the base64 codec.
- **`EnvVar`** is a public struct in `env_vars.rs` so callers can construct `Vec<EnvVar>` directly.
- **`Address`** is a public struct in `identity.rs`, mirroring spec fields (`line1`, `line2`, `city`, `state`, `postal_code`, `country` as ISO 3166 alpha-2 — stored as a plain `String`, no validation here).
- **`UnknownPayload`** exists to preserve forward-compat. When `EntryPayload::deserialize` sees a type it doesn't recognize, it stores the full JSON object minus `CommonMeta` into `unknown_fields`. Read-only: there is no write path for `UnknownPayload`.

**Tests** (one file per payload, as inline `#[cfg(test)]`):
- Round-trip: build a struct → `serde_json::to_vec` → `serde_json::from_slice` → same struct (secret values compared via `ExposeSecret`).
- Secret field appears in serialized JSON as plaintext (required — that's what encryption bytes consume).
- `Debug` on the struct does **not** print raw secret bytes. `SecretString`'s Debug is `"[REDACTED alloc::string::String]"` by default — assert the output does not contain the known password string.

**Done when:** all payload types compile, their unit tests pass, every secret field has `serialize_with` wired up.

---

## Phase 3 — `EntryPayload` enum + manual serde

**Goal:** one enum to rule them all, serializing flatly with `entry_type` as the discriminator.

**File:** `domain/vault/payloads/entry_payload.rs`.

```rust
#[derive(Debug, Clone)]
pub enum EntryPayload {
    Login(LoginPayload),
    Card(CardPayload),
    SshKey(SshKeyPayload),
    ApiKey(ApiKeyPayload),
    EnvVars(EnvVarsPayload),
    Note(NotePayload),
    Document(DocumentPayload),
    Identity(IdentityPayload),
    Folder(FolderPayload),
    Unknown(UnknownPayload),
}

impl EntryPayload {
    pub fn meta(&self) -> &CommonMeta { /* match each variant */ }
    pub fn meta_mut(&mut self) -> &mut CommonMeta { /* match each variant */ }
    pub fn entry_type(&self) -> &EntryType { &self.meta().entry_type }

    /// Serialize to JSON bytes for encryption. Intent-explicit name — grep
    /// for this to find every encryption boundary.
    pub fn to_encryptable_json(&self) -> Result<Zeroizing<Vec<u8>>, VaultError>;

    /// Deserialize from decrypted plaintext bytes. Enforces payload_schema
    /// and maps unknown entry_type into `Unknown(UnknownPayload)`.
    pub fn from_decrypted_json(bytes: &[u8]) -> Result<Self, VaultError>;
}
```

**Serialization** — delegate to each inner struct's derived `Serialize`. Implement `Serialize` on `EntryPayload` with a `match` that calls `inner.serialize(serializer)`. The inner struct already has `entry_type` inside its flattened `CommonMeta` — no extra discriminator field needed.

**Deserialization** — two-pass approach:

```rust
impl<'de> Deserialize<'de> for EntryPayload {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // 1. Parse into a generic Value so we can peek entry_type.
        let value = serde_json::Value::deserialize(d)?;

        // 2. Validate payload_schema.
        let schema = value.get("payload_schema")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as u32;
        if schema > CURRENT_PAYLOAD_SCHEMA {
            return Err(de::Error::custom(format!(
                "unsupported payload_schema: {schema}"
            )));
        }

        // 3. Route on entry_type.
        let entry_type = value.get("entry_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| de::Error::missing_field("entry_type"))?;

        match entry_type {
            "Login"   => Ok(Self::Login(serde_json::from_value(value).map_err(de::Error::custom)?)),
            "Card"    => Ok(Self::Card(serde_json::from_value(value).map_err(de::Error::custom)?)),
            // ...
            other => Ok(Self::Unknown(UnknownPayload {
                meta: serde_json::from_value(value.clone()).map_err(de::Error::custom)?,
                unknown_fields: value,
            })),
        }
    }
}
```

**`to_encryptable_json` / `from_decrypted_json`** — thin wrappers around `serde_json` that map errors into `VaultError::MalformedPayload` / `VaultError::UnsupportedPayloadSchema` / `VaultError::UnsupportedEntryType`. `to_encryptable_json` returns `Zeroizing<Vec<u8>>` because the bytes contain serialized secrets and must be wiped after the encryption call consumes them.

**Tests:**
- Each variant round-trips: construct → `to_encryptable_json` → `from_decrypted_json` → equal (via `ExposeSecret` on secret fields).
- Unknown entry_type survives a round-trip via `UnknownPayload` and preserves fields.
- `payload_schema = 999` → `UnsupportedPayloadSchema(999)`.
- Missing `entry_type` field → `MalformedPayload(_)`.

**Done when:** the enum compiles, all variants round-trip, the `Unknown` fallback works.

---

## Phase 4 — Golden JSON + crypto round-trip integration

**Goal:** the wire format is pinned. Future refactors that silently change JSON field names, types, or ordering will break loudly.

**File:** `tests/payload_wire_format.rs`.

1. For each payload variant, build a concrete instance with every field populated (no `None` defaults). Serialize it. Compare against a `const` JSON string committed in the test. If the serialization is non-deterministic (map ordering), use `serde_json::to_value` + pretty-sorted compare or assert individual fields instead.
2. For each variant, deserialize the same JSON string and assert the struct matches.
3. A single golden byte-for-byte comparison: `LoginPayload` with a known `CommonMeta` + known secret — the committed JSON includes the plaintext secret (that's what encrypted bytes carry — we're asserting JSON format, not confidentiality).

**File:** `tests/payload_crypto_round_trip.rs`.

Integration with the existing crypto layer:

```rust
#[test]
fn login_payload_round_trips_through_xchacha20() {
    let crypto = XChaCha20CryptoProvider::new();
    let dek = crypto.generate_dek();
    let id = EntryId::new();
    let aad = entry_aad(&id, 1).unwrap();

    let payload_in = EntryPayload::Login(LoginPayload {
        meta: CommonMeta { name: "github".into(), entry_type: EntryType::Login, .. },
        username: "alice".into(),
        password: SecretString::from("hunter2"),
        totp_secret: None,
        recovery_codes: vec![],
    });

    let bytes = payload_in.to_encryptable_json().unwrap();
    let (nonce, ct) = crypto.encrypt_entry(&dek, &bytes, &aad).unwrap();

    let pt = crypto.decrypt_entry(&dek, &nonce, &ct, &aad).unwrap();
    let payload_out = EntryPayload::from_decrypted_json(&pt).unwrap();

    let EntryPayload::Login(l) = payload_out else { panic!("wrong variant") };
    assert_eq!(l.username, "alice");
    assert_eq!(l.password.expose_secret(), "hunter2");
}
```

Repeat this pattern for each non-trivial variant (Card, SshKey, ApiKey, EnvVars, Note, Document, Identity, Folder). Document can use synthetic `blob_nonce` bytes — this test doesn't touch the blob store.

**Done when:** golden tests and crypto round-trip tests pass; total crate test count rises from 45 to ~60-65.

---

## Phase 5 — Re-exports + workspace verification

**Goal:** the payload surface is reachable from outside the crate.

1. **`domain/vault.rs`** re-exports the payload module:
   ```rust
   pub use payloads::{
       Address, ApiKeyPayload, CardPayload, CommonMeta, CURRENT_PAYLOAD_SCHEMA,
       DocumentPayload, EntryPayload, EntryType, EnvVar, EnvVarsPayload, FolderPayload,
       IdentityPayload, LoginPayload, NotePayload, SshKeyPayload, TagPayload,
       UnknownPayload,
   };
   ```
2. **Verification matrix:**
   - `cargo build -p vedge-core` — zero warnings.
   - `cargo test -p vedge-core` — all crates' tests pass, count in the 60-65 range.
   - `cargo build --workspace` — green aside from the two pre-existing `vedge-app` warnings.
3. **Memory update** — amend `memory/project_core_reimpl.md` to record that payloads are in place and the use-case plan is the next blocker.

**Done when:** verification matrix passes; no new warnings anywhere in the workspace.

---

## What comes after this plan

- **Use-case plan** — `UnlockVault`, `CreateEntry`, `UpdateEntry`, `CopyField`, `ChangePassword`, `RunMaintenance`. Introduces `VaultSession` (owns the KEK + `VaultIndex`), `IndexEntry` (the lightweight non-secret projection of `CommonMeta`), and wires everything through `spawn_blocking` for Argon2id.
- **Blob store plan** — `BlobStore` trait + filesystem impl for `.vedge_blobs/{ulid}.blob`. Consumes the `blob_aad` helper and `DocumentPayload.blob_nonce`.
- **Shell rewire plan** — `vedge-tauri` + `vedge-cli` repointed at the new use cases.

---

## Cross-cutting rules

- **Never derive `Serialize` on a struct containing a bare `SecretString` without `serialize_with`.** The build won't catch this (serde happily refuses to serialize `SecretString` by default), but a missing attribute will silently drop the secret field from the serialized output, breaking encryption. Code review rule: every `SecretString`/`Option<SecretString>`/`Vec<SecretString>` field must have a matching `serialize_with`.
- **`to_encryptable_json` / `from_decrypted_json` are the only JSON boundaries for `EntryPayload`.** No direct `serde_json::to_string(&payload)` at call sites — always go through these named methods so zeroize + error mapping stay consistent.
- **`Debug` output must never include plaintext secrets.** `SecretString` guarantees this by default. If we ever introduce a custom secret type, its `Debug` impl must follow the same redaction rule.
- **Unknown fields are preserved on deserialize, not on serialize.** If an old client reads a new entry type with unknown fields, it keeps them in `UnknownPayload.unknown_fields`. If that client ever tries to *re-encrypt* the payload (UpdateEntry on an unknown type), the use case must refuse — we don't own the schema well enough to write it back. This is a use-case-plan concern but noted here so the payload layer stays read-only for Unknown.
- **No validation in the payload layer.** Email shape, card number Luhn check, PEM validity — all belong to the use-case or UI layer. Payload types are storage schemas.
- **Golden JSON vectors are committed.** Changing any committed wire-format string is a deliberate schema change and triggers a `payload_schema` bump discussion.
