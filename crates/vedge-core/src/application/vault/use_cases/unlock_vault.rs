//! `UnlockVault` — turns `(vault_path, master_password, secret_key?)` into a
//! live [`VaultSession`].
//!
//! ## Security flow (per spec)
//!
//! 1. Load `vault_config` from the `.vdb`.
//! 2. Resolve the Secret Key (caller input overrides keychain).
//! 3. **On `spawn_blocking`**: 2SKD → Argon2id → HKDF (KEK + `verify_hash`).
//!    Argon2id is CPU-bound and would starve the async executor.
//! 4. Constant-time compare `verify_hash` before building the index —
//!    fail-fast on wrong credentials avoids loading secrets into RAM.
//! 5. Wrap the 32-byte KEK in a mlock'd [`SecretMem`] region.
//! 6. Decrypt every entry + tag row into a fresh `VaultIndex`; per-row DEKs
//!    live only inside each loop iteration and `Drop`-zeroize automatically.
//! 7. Append `AuditEvent::Unlocked`; update `last_unlocked_at`.
//! 8. Hand back a [`VaultSession`].
//!
//! All secret buffers (password bytes, Secret Key, Master Key, per-entry DEKs,
//! plaintext payload bytes) are either `Zeroizing<T>` or owned by types that
//! `Zeroize`-on-drop. No secret outlives its scope.

use std::path::PathBuf;
use std::sync::Arc;

use tracing::instrument;
use zeroize::Zeroizing;

use crate::application::vault::ports::clipboard::ClipboardProvider;
use crate::application::vault::ports::crypto::CryptoProvider;
use crate::application::vault::ports::factories::{BlobStoreFactory, VaultRepositoryFactory};
use crate::application::vault::ports::kdf::KeyDerivationProvider;
use crate::application::vault::ports::keychain::KeychainProvider;
use crate::application::vault::ports::repository::VaultRepository;
use crate::application::vault::session::VaultSession;
use crate::domain::shared::{Timestamp, VaultId, now};
use crate::domain::vault::aad::tag_aad;
use crate::domain::vault::crypto_constants::{KEK_LEN, SECRET_KEY_LEN};
use crate::domain::vault::entities::{AuditAction, AuditEvent, EntryRow, TagRow, VaultConfig};
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::index::{IndexEntry, TagMeta, VaultIndex};
use crate::domain::vault::payloads::TagPayload;
use crate::infrastructure::crypto::secret_mem::SecretMem;

pub struct UnlockVaultInput {
    pub vault_path: PathBuf,
    pub master_password: Zeroizing<String>,
    /// If `None`, the Secret Key is read from the OS keychain via
    /// [`KeychainProvider`]. Caller-supplied overrides (for testing,
    /// emergency-kit recovery flows) are kept in a zeroizing wrapper to
    /// match the keychain return type.
    pub secret_key: Option<Zeroizing<[u8; SECRET_KEY_LEN]>>,
}

/// Use-case wiring. `repo_factory` and `blob_factory` construct per-vault
/// infrastructure inside `execute`; the other ports are app-lifetime.
pub struct UnlockVault {
    pub repo_factory: Arc<dyn VaultRepositoryFactory>,
    pub blob_factory: Arc<dyn BlobStoreFactory>,
    pub crypto: Arc<dyn CryptoProvider>,
    pub clipboard: Arc<dyn ClipboardProvider>,
    pub kdf: Arc<dyn KeyDerivationProvider>,
    pub keychain: Arc<dyn KeychainProvider>,
}

/// Backfill an intrinsic `vault_uuid` on first open of a pre-4.6 vault (created
/// before the column existed). Idempotent: a vault minted on/after 4.6a already
/// carries one, so this only ever fires once per legacy vault. Both unlock paths
/// (password + biometric) already persist `updated_config` unconditionally, so the
/// backfill rides that same `save_config`. Slice 4.6a.
fn backfill_vault_uuid(config: &mut VaultConfig) {
    if config.vault_uuid.is_none() {
        config.vault_uuid = Some(ulid::Ulid::new().to_string());
    }
}

impl UnlockVault {
    #[instrument(skip_all, fields(vault_path = %input.vault_path.display()))]
    pub async fn execute(&self, input: UnlockVaultInput) -> Result<VaultSession, VaultError> {
        // ---- 0. Per-vault infrastructure ------------------------------------
        // The repo + blob store are tied to this specific `.vdb`, so they're
        // constructed here via the factory ports and owned by the resulting
        // session — they die when the session drops.
        let repo = self.repo_factory.open(&input.vault_path).await?;
        let blob = self
            .blob_factory
            .create(&input.vault_path, Arc::clone(&self.crypto))?;

        // ---- 1. Load config --------------------------------------------------
        let config = repo.load_config().await?;
        if config.magic != "VEDG" {
            return Err(VaultError::BadMagic);
        }
        let vault_id = VaultId::new(input.vault_path.clone());

        // ---- 2. Resolve Secret Key ------------------------------------------
        // The keychain is keyed on the intrinsic `vault_uuid` (slice 5.2.0), read from
        // the plaintext config loaded above. A migrated/created vault always carries one;
        // a legacy vault with `vault_uuid == None` has no locatable keychain entry, so the
        // user must supply the Secret Key (Emergency Kit) — surfaced as "not found".
        let secret_key = if let Some(sk) = input.secret_key {
            sk
        } else {
            let uuid = config
                .vault_uuid
                .as_deref()
                .ok_or(VaultError::KeychainEntryNotFound)?;
            self.keychain.read_secret_key(uuid)?
        };

        // ---- 3. spawn_blocking KDF ------------------------------------------
        // Move secrets into the blocking thread so they drop (and zeroize)
        // inside that thread regardless of how this async task is cancelled.
        let kdf = Arc::clone(&self.kdf);
        let vault_salt = config.vault_salt;
        let kdf_params = config.kdf_params.clone();
        let master_password = input.master_password;

        let (kek_zeroizing, verify_hash) = tokio::task::spawn_blocking(
            move || -> Result<(Zeroizing<[u8; KEK_LEN]>, [u8; 32]), VaultError> {
                let input_bytes = kdf.preprocess_2skd(master_password.as_bytes(), &secret_key)?;
                let master_key = kdf.derive_master_key(&input_bytes, &vault_salt, &kdf_params)?;
                let verify = kdf.derive_verify_hash(&master_key)?;
                let kek = kdf.derive_kek(&master_key)?;
                // master_password, secret_key, input_bytes, master_key all
                // drop here — each zeroizes its backing buffer.
                Ok((kek, verify))
            },
        )
        .await
        .map_err(|e| VaultError::KeyDerivationFailed(format!("spawn_blocking join: {e}")))??;

        // ---- 4. Verify before anything else ----------------------------------
        if !self
            .crypto
            .verify_hash_matches(&verify_hash, &config.verify_hash)
        {
            // `kek_zeroizing` drops here, zeroing the derived KEK we never
            // use. The plaintext password / Secret Key are already gone.
            return Err(VaultError::WrongCredentials);
        }

        // ---- 5. Pin the KEK in mlock'd memory --------------------------------
        // Brief stack copy from `*kek_zeroizing` → SecretMem::new's `value`
        // param → internal `Box<T>`. The temporary isn't zeroized, but
        // `kek_zeroizing` zeroizes its source on drop immediately after.
        let kek = SecretMem::new(*kek_zeroizing)?;
        drop(kek_zeroizing);

        // ---- 6. Build VaultIndex --------------------------------------------
        // Password path: a decrypt failure here is genuine corruption (the
        // constant-time verify_hash compare above already validated the password),
        // so it passes through unchanged.
        let index = self
            .build_index(repo.as_ref(), kek.expose(), std::convert::identity)
            .await?;

        // ---- 7. Audit + update last_unlocked_at -----------------------------
        let when = now();
        let event = AuditEvent {
            id: ulid::Ulid::new().to_string(),
            entry_id: None,
            action: AuditAction::Unlocked,
            occurred_at: when,
            device_id: None,
        };
        repo.append_audit(&event).await?;

        let mut updated_config = config;
        updated_config.last_unlocked_at = Some(when);
        backfill_vault_uuid(&mut updated_config);
        repo.save_config(&updated_config).await?;

        // ---- 7b. Rollback compare (non-fatal) — AFTER backfill so the uuid is set.
        let rollback_warning = self
            .detect_and_audit_rollback(repo.as_ref(), &updated_config, when)
            .await;

        // ---- 8. Build session ------------------------------------------------
        let mut session = VaultSession::assemble(
            vault_id,
            kek,
            index,
            updated_config,
            repo,
            Arc::clone(&self.crypto),
            blob,
            Arc::clone(&self.clipboard),
        );
        session.rollback_warning = rollback_warning;
        Ok(session)
    }

    /// Reconstitute a [`VaultSession`] from a KEK released by the biometric gate,
    /// **without** the master password.
    ///
    /// Mirrors [`execute`](Self::execute) from step 5 onward — the KDF/verify steps
    /// (2–4) are skipped because the caller already holds a validated-by-hardware KEK.
    /// The KEK is still validated against the vault's own ciphertext: the first entry
    /// or tag that fails to decrypt surfaces as [`VaultError::WrongCredentials`] (so the
    /// UI falls back to the password screen) and no partial session is returned. A
    /// vault with no entries and no tags has nothing to validate against and opens
    /// directly — there is no secret to protect.
    #[instrument(skip_all, fields(vault_path = %vault_path.display()))]
    pub async fn unlock_with_kek(
        &self,
        vault_path: PathBuf,
        kek_bytes: Zeroizing<[u8; KEK_LEN]>,
    ) -> Result<VaultSession, VaultError> {
        self.open_with_kek(vault_path, kek_bytes, Some(AuditAction::BiometricUnlocked))
            .await
    }

    /// Like [`unlock_with_kek`](Self::unlock_with_kek), but writes **no** unlock audit row.
    ///
    /// For the seamless in-place revert re-entry (slice 5.2.3): `revert_to_snapshot` has
    /// already recorded `BackupRestored`, which is the honest event. Reusing the audited path
    /// would forge a `BiometricUnlocked` row after every revert — including on password-only
    /// vaults that were never enrolled. The KEK is still validated against the vault's own
    /// ciphertext (a stale/failed-rewrap snapshot surfaces as `WrongCredentials`).
    #[instrument(skip_all, fields(vault_path = %vault_path.display()))]
    pub async fn unlock_with_kek_quiet(
        &self,
        vault_path: PathBuf,
        kek_bytes: Zeroizing<[u8; KEK_LEN]>,
    ) -> Result<VaultSession, VaultError> {
        self.open_with_kek(vault_path, kek_bytes, None).await
    }

    /// Shared body of the two keyless-open paths. `audit_action` is the unlock event to
    /// record, or `None` to suppress it (the seamless-revert reopen). Both paths remap a
    /// decrypt failure to `WrongCredentials`.
    async fn open_with_kek(
        &self,
        vault_path: PathBuf,
        kek_bytes: Zeroizing<[u8; KEK_LEN]>,
        audit_action: Option<AuditAction>,
    ) -> Result<VaultSession, VaultError> {
        // ---- 0. Per-vault infrastructure ------------------------------------
        let repo = self.repo_factory.open(&vault_path).await?;
        let blob = self
            .blob_factory
            .create(&vault_path, Arc::clone(&self.crypto))?;

        // ---- 1. Load config --------------------------------------------------
        let config = repo.load_config().await?;
        if config.magic != "VEDG" {
            return Err(VaultError::BadMagic);
        }
        let vault_id = VaultId::new(vault_path.clone());

        // ---- 5. Pin the KEK in mlock'd memory --------------------------------
        let kek = SecretMem::new(*kek_bytes)?;
        drop(kek_bytes);

        // ---- 6. Build VaultIndex (also validates the KEK) -------------------
        // A wrong/stale KEK can't decrypt any row; the first AEAD failure is remapped
        // to `WrongCredentials` so the shell treats it exactly like a bad password.
        let index = self
            .build_index(repo.as_ref(), kek.expose(), kek_validation_error)
            .await?;

        // ---- 7. Audit (optional) + update last_unlocked_at ------------------
        let when = now();
        if let Some(action) = audit_action {
            let event = AuditEvent {
                id: ulid::Ulid::new().to_string(),
                entry_id: None,
                action,
                occurred_at: when,
                device_id: None,
            };
            repo.append_audit(&event).await?;
        }

        let mut updated_config = config;
        updated_config.last_unlocked_at = Some(when);
        backfill_vault_uuid(&mut updated_config);
        repo.save_config(&updated_config).await?;

        // ---- 7b. Rollback compare (non-fatal) — same as the password path.
        let rollback_warning = self
            .detect_and_audit_rollback(repo.as_ref(), &updated_config, when)
            .await;

        // ---- 8. Build session ------------------------------------------------
        let mut session = VaultSession::assemble(
            vault_id,
            kek,
            index,
            updated_config,
            repo,
            Arc::clone(&self.crypto),
            blob,
            Arc::clone(&self.clipboard),
        );
        session.rollback_warning = rollback_warning;
        Ok(session)
    }

    /// Build a fresh [`VaultIndex`] by decrypting every tag + entry row under `kek`.
    ///
    /// The single copy of the unlock O(n) loop, shared by the password path
    /// ([`execute`](Self::execute)) and the keyless path
    /// ([`open_with_kek`](Self::open_with_kek)). The two differ **only** in
    /// `remap_err`: the password path passes [`std::convert::identity`] (a decrypt
    /// failure is genuine corruption — the password was already verified against
    /// `verify_hash`), and the keyless path passes [`kek_validation_error`] (a
    /// decrypt failure means a wrong/stale KEK → `WrongCredentials`). Per-row DEKs
    /// live only inside `decrypt_entry_row` and zeroize there. Folded from the two
    /// verbatim copies in slice 5.3c.
    async fn build_index(
        &self,
        repo: &dyn VaultRepository,
        kek: &[u8; KEK_LEN],
        remap_err: fn(VaultError) -> VaultError,
    ) -> Result<VaultIndex, VaultError> {
        let mut index = VaultIndex::new();

        let tag_rows = repo.all_tags().await?;
        for tag_row in tag_rows {
            let meta = self.decrypt_tag_row(&tag_row, kek).map_err(remap_err)?;
            index.insert_tag(meta);
        }

        let entry_rows = repo.all_entries().await?;
        for row in entry_rows {
            let entry = self.decrypt_entry_row(&row, kek).map_err(remap_err)?;
            index.insert_entry(entry);
        }

        Ok(index)
    }

    /// Decrypt one entry row → `IndexEntry` projection. DEK + plaintext are
    /// zeroized via Drop at the end of this function.
    #[instrument(skip_all, fields(entry_id = %row.id))]
    fn decrypt_entry_row(
        &self,
        row: &EntryRow,
        kek: &[u8; KEK_LEN],
    ) -> Result<IndexEntry, VaultError> {
        let dek = self.crypto.unwrap_dek(&row.dek_wrapped, kek)?;
        let payload = super::refs::decrypt_row_with_dek(self.crypto.as_ref(), &dek, row)?;
        let idx = IndexEntry::from_payload(&payload, row);
        // `dek` and `payload` zeroize their secret bytes via Drop; `idx` carries
        // only non-secret metadata.
        drop(payload);
        drop(dek);
        Ok(idx)
    }

    /// Decrypt one tag row → `TagMeta`. `payload_bytes` zeroizes via Drop.
    #[instrument(skip_all, fields(tag_id = %row.id))]
    fn decrypt_tag_row(&self, row: &TagRow, kek: &[u8; KEK_LEN]) -> Result<TagMeta, VaultError> {
        let aad = tag_aad(&row.id)?;
        let plaintext = self
            .crypto
            .decrypt_tag(kek, &row.nonce, &row.ciphertext, &aad)?;
        let payload: TagPayload = serde_json::from_slice(&plaintext)
            .map_err(|e| VaultError::MalformedPayload(format!("tag payload: {e}")))?;
        // `plaintext` zeroizes via Drop. TagPayload fields are not secret.
        drop(plaintext);
        Ok(TagMeta {
            id: row.id.clone(),
            name: payload.name,
            color: payload.color,
            sort_order: payload.sort_order,
        })
    }

    /// Compare the freshly-loaded `commit_counter` against this device's keychain
    /// baseline and reconcile the mirror (slice 5.2c).
    ///
    /// Returns `Some(delta)` — where `delta = baseline − file` — **only** on a genuine
    /// rollback: the file's counter is strictly below the highest value this device has
    /// ever mirrored. Every other outcome returns `None` and never warns:
    ///
    /// - **fresh device** (`Ok(None)` — no baseline yet) → establish the baseline, no warn;
    /// - **this device behind** (`file > baseline`, another device advanced the vault, or
    ///   a legit restore re-based elsewhere) → advance the mirror, no warn;
    /// - **equal** → normal;
    /// - **any keychain error** (daemon down / access denied) → no warn.
    ///
    /// The asymmetry is the safety property (Decision: warning, never an error): a missing
    /// or broken mirror must not manufacture a false rollback that alarms the user, and a
    /// rollback check must never fail an unlock. Requires `config.vault_uuid` to be `Some`
    /// (guaranteed after `backfill_vault_uuid`); a `None` uuid can't key the mirror → no warn.
    fn rollback_check(&self, config: &VaultConfig) -> Option<i64> {
        let uuid = config.vault_uuid.as_deref()?;
        let file = config.commit_counter;
        // Keychain unreachable → no baseline to compare, and unlock must not fail.
        let Ok(baseline) = self.keychain.read_commit_baseline(uuid) else {
            return None;
        };
        match baseline {
            Some(b) if file < b => Some(b.saturating_sub(file)), // ROLLBACK — warn
            Some(b) if file == b => None,                        // in sync — normal
            // file > baseline (this device behind) OR None (first sight): (re)establish
            // the mirror at the current file value. Best-effort; a store failure is
            // non-fatal and simply means the mirror catches up on a later unlock.
            _ => {
                if let Err(e) = self.keychain.store_commit_baseline(uuid, file) {
                    tracing::warn!(error = %e, "could not update the rollback baseline mirror");
                }
                None
            }
        }
    }

    /// Run [`Self::rollback_check`]; on a detected rollback, write a best-effort
    /// `RollbackDetected` audit row and return the delta to stamp on the session. Never
    /// fails the unlock — audit-write failures are logged, not propagated (slice 5.2c).
    async fn detect_and_audit_rollback(
        &self,
        repo: &dyn VaultRepository,
        config: &VaultConfig,
        when: Timestamp,
    ) -> Option<i64> {
        let delta = self.rollback_check(config)?;
        let event = AuditEvent {
            id: ulid::Ulid::new().to_string(),
            entry_id: None,
            action: AuditAction::RollbackDetected,
            occurred_at: when,
            device_id: None,
        };
        if let Err(e) = repo.append_audit(&event).await {
            tracing::warn!(error = %e, "rollback detected but its audit row could not be written");
        }
        tracing::warn!(
            delta,
            "vault commit_counter is below the keychain baseline — possible rollback"
        );
        Some(delta)
    }
}

/// Remap an index-build failure during a biometric unlock. A wrong/stale KEK makes AEAD
/// decryption fail ([`VaultError::DecryptionFailed`]); surface that as
/// [`VaultError::WrongCredentials`] so the shell falls back to the password screen.
/// Genuine data errors (malformed payload, storage) pass through unchanged.
fn kek_validation_error(err: VaultError) -> VaultError {
    match err {
        VaultError::DecryptionFailed => VaultError::WrongCredentials,
        other => other,
    }
}
