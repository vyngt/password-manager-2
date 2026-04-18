# Implementing `vedge-core` crypto layer

**Status:** Planning · **Scope:** `CryptoProvider`, `KeyDerivationProvider`, `KeychainProvider` · **Depends on:** `ReimplementCore.md` (database layer — complete)

---

## Why this plan

The database layer (app.db + .vdb) is in place and tested. Nothing above it can be built without the cryptography contract: entry/tag encryption, AES Key Wrap for per-row DEKs, Argon2id + HKDF for unlock, and OS keychain access for the Secret Key. Payloads, `VaultSession`, and all use cases (`UnlockVault`, `CreateEntry`, `CopyField`, `ChangePassword`, `RunMaintenance`) depend on this layer.

This plan stops at: **three ports (CryptoProvider, KeyDerivationProvider, KeychainProvider) defined in the application layer, with concrete infrastructure implementations, proven end-to-end by an integration test that simulates a full unlock flow without touching the database.**

## Authoritative references

| Doc | What it pins down |
|---|---|
| `docs/Design/Vedge/Core/Key Hierarchy …md` | Every algorithm, parameter, HKDF info string, storage location, threat model |
| `docs/Design/Vedge/Core/vault …md` § Ports | Exact trait signatures for the three crypto ports |

Read the Key Hierarchy doc front to back before starting. The values below are verbatim from it — if any one of them is wrong, migration is hard.

## Architectural commitments (binding)

- **Ports live in `application/vault/ports/`** as a directory of per-concept files (`repository.rs`, `crypto.rs`, `kdf.rs`, `keychain.rs`). The existing single-file `ports.rs` is expanded to a directory in Phase 0. Domain is unchanged except for new error variants and AAD helpers.
- **All key material implements `zeroize::Zeroize`.** Stack-local keys use `zeroize::Zeroizing<[u8; N]>`. Long-lived keys (KEK) use a mlock-protected `SecretBox`. Raw `[u8; 32]` / `[u8; 16]` is acceptable only inside infrastructure-layer function bodies that explicitly zeroize on exit.
- **AAD construction is a domain concern.** `domain/vault/aad.rs` holds pure functions `entry_aad(id, version)`, `tag_aad(id)`, `blob_aad(id)`. The `CryptoProvider` port takes `aad: &[u8]` and is agnostic to format. Changing AAD format invalidates every ciphertext — therefore it belongs to the domain contract, not the infrastructure.
- **HKDF domain-separation strings are domain constants** in `domain/vault/crypto_constants.rs` (`vedge-v1-2skd`, `vedge-v1-kek`, `vedge-v1-verify`, `vedge-v1-sync-auth`). Any change is a schema bump.
- **KDF is sync, spawn_blocking at the call site.** `KeyDerivationProvider::derive_master_key` is a synchronous method. Use-cases wrap Argon2id calls in `tokio::task::spawn_blocking` — the port itself stays runtime-agnostic, and `vedge-core` still has no `tokio` runtime dependency at the library level.
- **Constant-time comparison is provided by the port, not exposed as a utility.** `CryptoProvider::verify_hash_matches(candidate, expected) -> bool` hides the `subtle` dependency. Callers never see `==` on key material.
- **Secret Key size is 128 bits (16 bytes).** Fixed by spec. Not parameterizable at runtime.
- **Linux keychain fallback is deferred.** `OsKeychainProvider` uses the `keyring` crate on all three platforms; if no daemon/credential-store is available, operations return `VaultError::KeychainUnavailable`. The "encrypted file in ~/.config/vedge/" fallback from the spec is a separate plan — out of scope here.

## Out of scope for this plan

- `EntryPayload` and `CommonMeta` (payload plan).
- `VaultSession`, `VaultIndex`, high-level use cases (`UnlockVault`, `CreateEntry`, etc.).
- `BlobStore` for documents.
- Emergency Kit PDF export of the Secret Key.
- Shamir splitting / vault sharing / FIPS mode.
- Linux keychain fallback.

---

## Phase 0 — Dependencies and port-directory refactor

**Goal:** crates wired up, `application/vault/ports.rs` split into a directory, empty port files + infra stubs in place. Must compile with zero warnings.

1. **Cargo.toml additions** under `[dependencies]`:
   - `chacha20poly1305 = { version = "0.10", default-features = false, features = ["alloc", "getrandom"] }` — XChaCha20-Poly1305 AEAD.
   - `aes = "0.8"` + `aes-kw = "0.2"` — AES-256 Key Wrap (RFC 3394).
   - `hkdf = "0.12"` + `sha2 = "0.10"` — HKDF-SHA256.
   - `argon2 = "0.5"` — Argon2id KDF.
   - `zeroize = { version = "1", features = ["zeroize_derive"] }` — `Zeroize` + `Zeroizing<T>`.
   - `secrecy = "0.10"` — `SecretBox<T>` wrapper. mlock behaviour is added in Phase 5 via a thin wrapper; `secrecy` alone does not mlock.
   - `subtle = "2"` — `ConstantTimeEq`.
   - `rand = "0.8"` + `rand_core = "0.6"` — `OsRng` for nonces and DEK generation. (Pin to what `chacha20poly1305 = 0.10` wants.)
   - `keyring = { version = "3", default-features = false, features = ["apple-native", "windows-native", "sync-secret-service"] }` — platform-native keychain, no unnecessary async.
   - `ulid = "1"` is already present — we rely on `Ulid::from_string` / `to_bytes` for AAD construction.
2. **Port directory:** delete `application/vault/ports.rs` file, create `application/vault/ports.rs` with `pub mod repository; pub mod crypto; pub mod kdf; pub mod keychain;` plus `pub use` re-exports. Move the existing `VaultRepository` trait into `application/vault/ports/repository.rs` unchanged. The three new files start empty (one-line `// populated in Phase 2` stubs).
3. **Infrastructure scaffolding:**
   - `infrastructure/crypto.rs` + `infrastructure/crypto/{xchacha20.rs, argon2_kdf.rs}` — empty stubs.
   - `infrastructure/keychain.rs` + `infrastructure/keychain/{os.rs, memory.rs}` — empty stubs.
   - Register both as `pub mod crypto; pub mod keychain;` in `infrastructure.rs` alongside `sqlite`.
4. **Verify:** `cargo build -p vedge-core` clean, `cargo test -p vedge-core` still green (12 tests from the database layer must not regress).

**Done when:** new deps resolve, module tree matches, all 12 database tests still pass.

---

## Phase 1 — Domain additions (errors + AAD + constants)

**Goal:** domain has the vocabulary the crypto layer needs. No crypto imports in domain.

1. **`domain/vault/crypto_constants.rs`** — HKDF info strings as `&'static [u8]`:
   ```rust
   pub const HKDF_INFO_2SKD:      &[u8] = b"vedge-v1-2skd";
   pub const HKDF_INFO_KEK:       &[u8] = b"vedge-v1-kek";
   pub const HKDF_INFO_VERIFY:    &[u8] = b"vedge-v1-verify";
   pub const HKDF_INFO_SYNC_AUTH: &[u8] = b"vedge-v1-sync-auth";

   pub const SECRET_KEY_LEN: usize = 16;    // 128 bits
   pub const VAULT_SALT_LEN: usize = 32;
   pub const MASTER_KEY_LEN: usize = 32;
   pub const KEK_LEN:        usize = 32;
   pub const DEK_LEN:        usize = 32;
   pub const DEK_WRAPPED_LEN: usize = 40;   // RFC 3394 (32 + 8 integrity)
   pub const NONCE_LEN:      usize = 24;    // XChaCha20 extended nonce
   pub const VERIFY_HASH_LEN: usize = 32;
   ```
2. **`domain/vault/aad.rs`** — pure byte-format helpers, no crypto deps:
   ```rust
   pub fn entry_aad(id: &EntryId, version: i64) -> Result<Vec<u8>, VaultError>;
   pub fn tag_aad(id: &TagId)                    -> Result<Vec<u8>, VaultError>;
   pub fn blob_aad(id: &EntryId)                 -> Result<Vec<u8>, VaultError>;
   ```
   Each parses the ULID string via `ulid::Ulid::from_string`, serializes to its 16-byte form, and appends the suffix (`version.to_le_bytes()` for entries, `b"blob"` for blobs, nothing for tags). Parse failures return `VaultError::InvalidEntryId` / `InvalidTagId`.
3. **`domain/vault/errors.rs`** — add variants:
   ```rust
   WrongCredentials,
   DecryptionFailed,
   EncryptionFailed,
   KeyDerivationFailed(String),        // carries the underlying reason (without key material)
   MlockFailed,
   KeychainUnavailable,
   KeychainAccessDenied,
   KeychainEntryNotFound,
   InvalidEntryId(String),
   InvalidTagId(String),
   ```
   These are error kinds only — `Display` impls must never include bytes of the password, Secret Key, or any derived key material.
4. **`domain/vault.rs`** — re-export `aad::*` and `crypto_constants::*`.

**Done when:** `cargo build -p vedge-core` clean, new modules have unit tests for AAD construction (golden vectors: same ULID + same version → same AAD bytes; different version → different bytes).

---

## Phase 2 — Ports: CryptoProvider, KeyDerivationProvider, KeychainProvider

**Goal:** the three traits are defined, documented, and re-exported. No implementations yet.

1. **`application/vault/ports/crypto.rs`:**
   ```rust
   pub trait CryptoProvider: Send + Sync {
       fn encrypt_entry(
           &self,
           dek:     &[u8; DEK_LEN],
           payload: &[u8],
           aad:     &[u8],
       ) -> Result<(Nonce, Vec<u8>), VaultError>;

       fn decrypt_entry(
           &self,
           dek:        &[u8; DEK_LEN],
           nonce:      &[u8; NONCE_LEN],
           ciphertext: &[u8],
           aad:        &[u8],
       ) -> Result<Zeroizing<Vec<u8>>, VaultError>;

       fn encrypt_tag(
           &self,
           kek:     &[u8; KEK_LEN],
           payload: &[u8],
           aad:     &[u8],
       ) -> Result<(Nonce, Vec<u8>), VaultError>;

       fn decrypt_tag(
           &self,
           kek:        &[u8; KEK_LEN],
           nonce:      &[u8; NONCE_LEN],
           ciphertext: &[u8],
           aad:        &[u8],
       ) -> Result<Zeroizing<Vec<u8>>, VaultError>;

       fn wrap_dek(
           &self,
           dek: &[u8; DEK_LEN],
           kek: &[u8; KEK_LEN],
       ) -> Result<[u8; DEK_WRAPPED_LEN], VaultError>;

       fn unwrap_dek(
           &self,
           wrapped: &[u8; DEK_WRAPPED_LEN],
           kek:     &[u8; KEK_LEN],
       ) -> Result<Zeroizing<[u8; DEK_LEN]>, VaultError>;

       fn generate_dek(&self)   -> Zeroizing<[u8; DEK_LEN]>;
       fn generate_nonce(&self) -> [u8; NONCE_LEN];

       fn verify_hash_matches(&self, candidate: &[u8; VERIFY_HASH_LEN], expected: &[u8; VERIFY_HASH_LEN]) -> bool;
   }

   pub type Nonce = [u8; NONCE_LEN];
   ```
   - Return type is `Zeroizing<Vec<u8>>` (not raw `Vec<u8>`) for decrypted plaintext so callers cannot forget to zeroize.
   - `verify_hash_matches` is the only constant-time helper we expose — it uses `subtle::ConstantTimeEq` inside the impl.

2. **`application/vault/ports/kdf.rs`:**
   ```rust
   pub trait KeyDerivationProvider: Send + Sync {
       fn preprocess_2skd(
           &self,
           master_password: &[u8],
           secret_key:      &[u8; SECRET_KEY_LEN],
       ) -> Zeroizing<[u8; 32]>;

       /// Blocking. Callers must run this via spawn_blocking.
       fn derive_master_key(
           &self,
           input:      &[u8; 32],
           vault_salt: &[u8; VAULT_SALT_LEN],
           params:     &KdfParams,
       ) -> Result<Zeroizing<[u8; MASTER_KEY_LEN]>, VaultError>;

       fn derive_kek(&self, master_key: &[u8; MASTER_KEY_LEN])          -> Zeroizing<[u8; KEK_LEN]>;
       fn derive_verify_hash(&self, master_key: &[u8; MASTER_KEY_LEN])  -> [u8; VERIFY_HASH_LEN];
       fn derive_sync_auth(&self, master_key: &[u8; MASTER_KEY_LEN])    -> Zeroizing<[u8; 32]>;
   }
   ```
   `KdfParams` is already in the domain (`domain/vault/kdf_params.rs`).

3. **`application/vault/ports/keychain.rs`:**
   ```rust
   pub trait KeychainProvider: Send + Sync {
       fn read_secret_key(&self, vault_id: &VaultId)
           -> Result<Zeroizing<[u8; SECRET_KEY_LEN]>, VaultError>;

       fn store_secret_key(&self, vault_id: &VaultId, key: &[u8; SECRET_KEY_LEN])
           -> Result<(), VaultError>;

       fn delete_secret_key(&self, vault_id: &VaultId)
           -> Result<(), VaultError>;
   }
   ```
   Sync because platform keychain APIs are blocking C calls; shells wrap calls in `spawn_blocking` if needed.

4. **`application/vault/ports.rs`** re-exports the three traits alongside `VaultRepository`.

**Done when:** ports compile, rustdoc on each trait explains security invariants (what callers must zeroize, when to spawn_blocking, when a wrong credential returns which error).

---

## Phase 3 — Infrastructure: `XChaCha20CryptoProvider`

**Goal:** a single struct implements `CryptoProvider` using `chacha20poly1305` and `aes-kw`.

**File:** `infrastructure/crypto/xchacha20.rs`.

- Struct has no fields — it's pure function dispatch. `pub struct XChaCha20CryptoProvider;`
- `encrypt_entry` / `encrypt_tag`:
  1. Generate nonce via `OsRng::fill_bytes` (or call `self.generate_nonce()` internally).
  2. Build `XChaCha20Poly1305::new(Key::from_slice(key))`.
  3. `aead.encrypt(&XNonce::from_slice(&nonce), aead::Payload { msg: payload, aad })` — returns ciphertext with 16-byte tag appended.
  4. Map any error to `VaultError::EncryptionFailed` (hide the underlying crypto error — don't leak).
- `decrypt_entry` / `decrypt_tag`:
  1. Build cipher.
  2. `aead.decrypt(...)` — authentication failures return `VaultError::DecryptionFailed` (same error for wrong key, wrong AAD, wrong nonce, tampered ciphertext — do not distinguish).
  3. Wrap the result in `Zeroizing::new(plaintext_vec)`.
- `wrap_dek` / `unwrap_dek`: use `aes_kw::KekAes256::new(kek.into())`, then `kek.wrap(dek, &mut buf)` / `kek.unwrap(wrapped, &mut out)`. Output length is exactly 40 (wrap) / 32 (unwrap). Unwrap failure (tampered dek_wrapped) returns `VaultError::DecryptionFailed` — same bucket as AEAD failures, since a distinction would leak information.
- `generate_dek`: `let mut k = [0u8; 32]; OsRng.fill_bytes(&mut k); Zeroizing::new(k)`.
- `generate_nonce`: same pattern, 24 bytes, no zeroizing (nonce is not secret).
- `verify_hash_matches`: `candidate.ct_eq(expected).into()`.

**Error mapping rule:** exactly three output error variants from this impl — `EncryptionFailed`, `DecryptionFailed`, and (only from `wrap_dek` when `kek` length is wrong, which is compile-time impossible given the signature) nothing else. Don't leak underlying library error chains into the `Display`.

**Tests** (`tests/crypto_xchacha20.rs`):
- Round-trip entry encrypt/decrypt returns original plaintext.
- Decrypt with a different DEK → `DecryptionFailed`.
- Decrypt with a different AAD (simulate entry-ID substitution) → `DecryptionFailed`.
- Decrypt with a single tampered byte in ciphertext → `DecryptionFailed`.
- `wrap_dek` + `unwrap_dek` round-trips a DEK exactly.
- `unwrap_dek` with wrong KEK → `DecryptionFailed`.
- Two successive `generate_dek` calls produce different keys (non-zero probabilistic check).
- Two successive `generate_nonce` calls produce different nonces.
- `verify_hash_matches` is constant-time — assert it for equal and unequal inputs (behaviour only; timing isn't testable here).

**Done when:** all tests pass, no warnings, the struct is `Send + Sync`.

---

## Phase 4 — Infrastructure: `Argon2idKdfProvider`

**Goal:** a single struct implements `KeyDerivationProvider` using `argon2` and `hkdf`.

**File:** `infrastructure/crypto/argon2_kdf.rs`.

- `pub struct Argon2idKdfProvider;` — stateless.
- `preprocess_2skd`:
  ```rust
  let hk = Hkdf::<Sha256>::new(Some(secret_key), master_password);
  let mut out = [0u8; 32];
  hk.expand(HKDF_INFO_2SKD, &mut out).expect("32 ≤ 8160");
  Zeroizing::new(out)
  ```
- `derive_master_key`:
  1. Validate `params.alg == "argon2id"`; otherwise return `VaultError::KeyDerivationFailed("unsupported kdf")`.
  2. Build `argon2::Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::new(params.m, params.t, params.p, Some(32))?)`.
  3. `a2.hash_password_into(input, vault_salt, &mut out)` — any error becomes `VaultError::KeyDerivationFailed(e.to_string())` (argon2's error strings do not include the password or salt).
  4. Return `Zeroizing::new(out)`.
- `derive_kek` / `derive_sync_auth`: HKDF-expand with empty salt and the relevant info string, 32-byte output, wrap in `Zeroizing`.
- `derive_verify_hash`: same as `derive_kek` but not zeroizing (hash is stored on disk — not secret).

**Tests** (`tests/crypto_kdf.rs`):
- `preprocess_2skd` is deterministic: same (password, secret_key) → same output.
- `preprocess_2skd` differs when either input differs.
- `derive_master_key` with default params, input = zeros, vault_salt = zeros → pins a known 32-byte output (golden vector, computed once and hard-coded to detect parameter drift).
- `derive_kek(mk) != derive_verify_hash(mk)` — domain separation works.
- `derive_master_key` with weak params (`m=19456, t=2, p=1` — OWASP minimum) succeeds; with `alg="pbkdf2"` → `KeyDerivationFailed("unsupported kdf")`.

**Done when:** tests pass; the golden vector is reproducible on Windows + macOS + Linux runs.

---

## Phase 5 — Infrastructure: `OsKeychainProvider` + `MemoryKeychainProvider`

**Goal:** real OS keychain access plus a test double.

**Files:** `infrastructure/keychain/os.rs`, `infrastructure/keychain/memory.rs`.

1. **`OsKeychainProvider`** — uses `keyring::Entry`:
   ```rust
   pub struct OsKeychainProvider {
       service: String,  // default "vedge"
   }

   impl OsKeychainProvider {
       pub fn new() -> Self { Self { service: "vedge".into() } }
       pub fn with_service(service: impl Into<String>) -> Self { ... }
   }
   ```
   Account key format: `format!("vault:{}", vault_id.path().to_string_lossy())`. The Secret Key is stored as base64 text (keyring stores strings on all platforms except Windows where it can do raw bytes — use base64 uniformly for consistency).
   - `read_secret_key`: `Entry::new(&self.service, &account)?` → `.get_password()` → base64-decode → length-check → `Zeroizing::new([u8; 16])`.
   - Error mapping:
     - `keyring::Error::NoEntry` → `VaultError::KeychainEntryNotFound`
     - `keyring::Error::PlatformFailure`, `NoStorageAccess` → `VaultError::KeychainUnavailable`
     - `keyring::Error::Ambiguous` or any auth denial → `VaultError::KeychainAccessDenied`
     - Base64 decode / length mismatch → `VaultError::KeychainEntryNotFound` (treat malformed as missing — don't leak)
   - Never log the decoded bytes; `Display` on our error never includes them.
2. **`MemoryKeychainProvider`** — for tests, plain `Arc<Mutex<HashMap<VaultId, [u8; 16]>>>`. Exposed under `#[cfg(any(test, feature = "test-support"))]` — but since we're not wiring features yet, make it `pub` with a clear doc-comment that it's for tests only.
3. **No mlock in this phase.** Record the mlock decision in a `// NOTE:` comment in `OsKeychainProvider`: we rely on `Zeroizing` + short-lived heap allocations; proper mlock'd KEK storage is a Phase-6 concern in the *use-case* plan, not here. Keychain reads hand off to the caller inside a single function scope and immediately zeroize.

**Tests** (`tests/crypto_keychain.rs`):
- `MemoryKeychainProvider` store → read → delete → read returns `KeychainEntryNotFound`.
- `MemoryKeychainProvider` read missing → `KeychainEntryNotFound`.
- `OsKeychainProvider` is tested *manually* — do not write automated tests against the user's real keychain. Add a `#[ignore]` integration test that round-trips on a `vedge-test-{ulid}` service so developers can run it with `cargo test -- --ignored` when they want to verify.

**Done when:** memory provider tests pass; the ignored OS test is present and documented.

---

## Phase 6 — End-to-end unlock simulation (integration test)

**Goal:** one test proves the three providers compose correctly. This is the contract the future `UnlockVault` use case will consume.

**File:** `tests/crypto_unlock_flow.rs`.

Scenario (no DB involvement):
1. Generate a fake Secret Key `sk = [0u8; 16]` and vault_salt `vs = [0u8; 32]`.
2. Password = `b"correct horse battery staple"`.
3. `input = kdf.preprocess_2skd(password, sk)`.
4. `master_key = kdf.derive_master_key(input, vs, &KdfParams::argon2id_default())`.
5. `kek = kdf.derive_kek(&master_key)`.
6. `verify_hash = kdf.derive_verify_hash(&master_key)`.
7. `crypto.verify_hash_matches(&verify_hash, &verify_hash)` → `true`.
8. Create a DEK: `dek = crypto.generate_dek()`.
9. `wrapped = crypto.wrap_dek(&dek, &kek)`.
10. Build an `EntryId::new()`, compute `aad = entry_aad(&id, 1)`.
11. `ciphertext = crypto.encrypt_entry(&dek, b"{\"name\":\"test\"}", &aad)`.
12. Simulate unlock: `dek_read = crypto.unwrap_dek(&wrapped, &kek)`, `plaintext = crypto.decrypt_entry(&dek_read, &nonce, &ciphertext, &aad)`.
13. Assert plaintext == `b"{\"name\":\"test\"}"`.
14. **Wrong password branch:** repeat steps 3-6 with `password2 = b"wrong"`. Expect `verify_hash_matches(vh2, original_vh) == false` — do not attempt decryption (mirrors the real use-case behaviour).
15. **AAD substitution branch:** compute `bad_aad = entry_aad(&different_id, 1)`, call `decrypt_entry` with it — expect `VaultError::DecryptionFailed`.

**Done when:** the test passes on Windows; no calls to `app.db` or `.vdb`; total wall time under 5 seconds (Argon2id dominates — one call at 256MiB × 3 iterations).

---

## Phase 7 — lib.rs re-exports + workspace verification

**Goal:** crypto surface is reachable from outside the crate.

1. **`lib.rs`-accessible paths** (through existing `pub mod` chain):
   - `vedge_core::application::vault::ports::{CryptoProvider, KeyDerivationProvider, KeychainProvider, Nonce}`
   - `vedge_core::domain::vault::{aad, crypto_constants, KdfParams, VaultError}`
   - `vedge_core::infrastructure::crypto::{XChaCha20CryptoProvider, Argon2idKdfProvider}`
   - `vedge_core::infrastructure::keychain::{OsKeychainProvider, MemoryKeychainProvider}`
   Add explicit `pub use` lines in `infrastructure/crypto.rs` and `infrastructure/keychain.rs` so consumers don't need to know submodule names.
2. **Verification matrix** (same as the database plan):
   - `cargo build -p vedge-core` — zero warnings.
   - `cargo test -p vedge-core` — must now show 12 + crypto tests (roughly 20-25 total).
   - `cargo build --workspace` — green except for the two pre-existing `dead_code` warnings in `vedge-app`.
3. **Memory update:** amend `memory/project_core_reimpl.md` to note that the crypto layer is in place and `vedge-tauri` / `vedge-tools` rewire is the next blocker.

**Done when:** verification matrix passes; no new warnings anywhere in the workspace.

---

## What comes after this plan

Sketched for alignment; not commitments in this plan.

1. **Payload plan** — `CommonMeta`, per-type `*Payload` structs, `EntryPayload` enum, `payload_schema` versioning, `Secret<String>` wrapping.
2. **Use-case plan** — `UnlockVault`, `CreateEntry`, `UpdateEntry`, `CopyField`, `ChangePassword`, `RunMaintenance`. Introduces `VaultSession` (owns the mlock'd KEK + `VaultIndex`) and `VaultIndex`. This is where `spawn_blocking` around `derive_master_key` lives.
3. **Blob store plan** — `BlobStore` trait + filesystem impl for `.vedge_blobs/{ulid}.blob`, using the `blob_aad` helper already defined in Phase 1.
4. **Shell rewire plan** — `vedge-tauri` and `vedge-cli` point at the new use cases and ports.

---

## Cross-cutting rules

- **No key material in `Display`, `Debug`, or `tracing` output.** Errors must be informative about *what* failed, never *with what bytes*.
- **`Zeroizing<T>` is the default return type for secrets.** Callers that need to extend lifetime beyond a function explicitly take responsibility via `secrecy::SecretBox` — they do not "clone out" of `Zeroizing`.
- **No conditional error distinction on auth failure.** Wrong KEK on unwrap, wrong DEK on decrypt, wrong AAD on decrypt, tampered ciphertext on decrypt — all return the same `DecryptionFailed`. Distinguishing them leaks information.
- **`spawn_blocking` is a caller concern, not the port's concern.** KDF methods stay synchronous; the use-case plan wires them onto `spawn_blocking`.
- **Golden vectors are committed.** The Argon2id + HKDF golden vector in Phase 4 is an intentional tripwire — if someone changes parameters or info strings, the test breaks and they must consciously bump versions.
- **Constants are versioned.** All HKDF info strings contain `v1`. A future cryptographic agility plan introduces `v2` strings; the old ones remain for reading legacy vaults until retired.
