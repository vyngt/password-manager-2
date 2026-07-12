//! `CreateVault` — provisions a brand-new encrypted vault and hands back a
//! live, **unlocked** [`VaultSession`] plus the generated Secret Key formatted
//! for the Emergency Kit.
//!
//! ## Relationship to `UnlockVault`
//!
//! This is the mirror of [`UnlockVault`](super::unlock_vault::UnlockVault).
//! Instead of *loading* + verifying `vault_config`, it *generates* the salt +
//! Secret Key, derives key material through the **identical** 2SKD → Argon2id
//! → HKDF chain, and *writes* the initial `vault_config` row. A vault created
//! here therefore unlocks through the unchanged `UnlockVault` with the same
//! password + Secret Key.
//!
//! ## Security flow
//!
//! 1. Refuse to overwrite an existing path (guard *before* opening the DB).
//! 2. Generate a fresh 16-byte Secret Key + 32-byte vault salt from the CSPRNG.
//! 3. **On `spawn_blocking`**: 2SKD → Argon2id → HKDF (KEK + `verify_hash`).
//! 4. Pin the 32-byte KEK in an mlock'd [`SecretMem`] region.
//! 5. Build the initial `VaultConfig` (`id = "default"`, magic `VEDG`).
//! 6. Provision the `.vdb` + blob store via the factory ports; persist config.
//! 7. Store the Secret Key in the OS keychain — a failure here does **not**
//!    void the vault (the user keeps the Emergency Kit).
//! 8. Append `AuditAction::Created`.
//! 9. Assemble and return the session + Emergency-Kit display string.
//!
//! All secret buffers (password bytes, Secret Key, Master Key, KEK) are either
//! `Zeroizing<T>` or owned by types that `Zeroize`-on-drop. No secret outlives
//! its scope.

use std::path::PathBuf;
use std::sync::Arc;

use tracing::instrument;
use zeroize::Zeroizing;

use crate::application::vault::ports::clipboard::ClipboardProvider;
use crate::application::vault::ports::crypto::CryptoProvider;
use crate::application::vault::ports::factories::{BlobStoreFactory, VaultRepositoryFactory};
use crate::application::vault::ports::kdf::KeyDerivationProvider;
use crate::application::vault::ports::keychain::KeychainProvider;
use crate::application::vault::session::VaultSession;
use crate::domain::shared::{VaultId, now};
use crate::domain::vault::crypto_constants::{KEK_LEN, SECRET_KEY_LEN};
use crate::domain::vault::entities::{AuditAction, AuditEvent, VaultConfig};
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::index::VaultIndex;
use crate::domain::vault::kdf_params::KdfParams;
use crate::domain::vault::recovery::format_secret_key;
use crate::infrastructure::crypto::secret_mem::SecretMem;

pub struct CreateVaultInput {
    pub vault_path: PathBuf,
    pub master_password: Zeroizing<String>,
    /// `None` → generate a fresh 16-byte Secret Key from the CSPRNG.
    /// `Some(..)` for deterministic tests / future provisioning flows.
    pub secret_key: Option<Zeroizing<[u8; SECRET_KEY_LEN]>>,
    /// `None` → [`KdfParams::argon2id_default`] (the production profile).
    /// `Some(..)` lets tests inject fast parameters — mirrors `secret_key`'s
    /// override escape hatch.
    pub kdf_params: Option<KdfParams>,
}

/// Successful creation — an unlocked session plus the Emergency-Kit string.
///
/// The `session` is always valid; `keychain_stored` reports whether the Secret
/// Key was also written to the OS keychain (the vault file is valid regardless
/// — see [`recover_vault`] for the same contract).
///
/// [`recover_vault`]: super::recover_vault::recover_vault
pub struct CreateVaultOutput {
    pub session: VaultSession,
    /// `format_secret_key(&secret_key)` — the `A3-XXXXX-…` Emergency-Kit
    /// string. Shown **once**; never persisted in plaintext.
    pub secret_key_display: String,
    /// `true` if `keychain.store_secret_key` succeeded; `false` if the vault
    /// was created but persisting the key did not. In the `false` case,
    /// [`keychain_error`](Self::keychain_error) carries the error's display
    /// string.
    pub keychain_stored: bool,
    pub keychain_error: Option<String>,
}

// Manual `Debug` — never expose the session internals or the Emergency-Kit
// string (which *is* the Secret Key in display form).
impl std::fmt::Debug for CreateVaultOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateVaultOutput")
            .field("keychain_stored", &self.keychain_stored)
            .field("keychain_error", &self.keychain_error)
            .finish_non_exhaustive()
    }
}

/// Use-case wiring for `create_vault`.
///
/// `repo_factory` and `blob_factory` construct per-vault infrastructure inside
/// `execute`; the other ports are app-lifetime. Identical port set to
/// [`UnlockVault`](super::unlock_vault::UnlockVault).
pub struct CreateVault {
    pub repo_factory: Arc<dyn VaultRepositoryFactory>,
    pub blob_factory: Arc<dyn BlobStoreFactory>,
    pub crypto: Arc<dyn CryptoProvider>,
    pub clipboard: Arc<dyn ClipboardProvider>,
    pub kdf: Arc<dyn KeyDerivationProvider>,
    pub keychain: Arc<dyn KeychainProvider>,
}

impl CreateVault {
    #[instrument(skip_all, fields(vault_path = %input.vault_path.display()))]
    pub async fn execute(&self, input: CreateVaultInput) -> Result<CreateVaultOutput, VaultError> {
        // ---- 1. Refuse to overwrite -----------------------------------------
        // `open` uses SQLite `mode=rwc`, which would *create* the file — so we
        // guard before touching the factory to avoid clobbering a pre-existing
        // vault (or an unrelated file) with a fresh config.
        if input.vault_path.try_exists().unwrap_or(false) {
            return Err(VaultError::VaultAlreadyExists);
        }

        // ---- 2. Generate fresh key material ---------------------------------
        let secret_key = input
            .secret_key
            .unwrap_or_else(|| self.crypto.generate_secret_key());
        let vault_salt = self.crypto.generate_vault_salt();
        let kdf_params = input.kdf_params.unwrap_or_else(KdfParams::argon2id_default);

        // ---- 3. spawn_blocking KDF (identical chain to UnlockVault) ---------
        // Argon2id is CPU-bound; keep it off the async executor. A *clone* of
        // the Secret Key moves into the thread — the original is needed after
        // for the keychain write + Emergency-Kit display. Everything moved in
        // drops (and zeroizes) inside that thread.
        let kdf = Arc::clone(&self.kdf);
        let master_password = input.master_password;
        let secret_key_for_kdf = secret_key.clone();
        let kdf_params_for_derive = kdf_params.clone();

        let (kek_zeroizing, verify_hash) = tokio::task::spawn_blocking(
            move || -> Result<(Zeroizing<[u8; KEK_LEN]>, [u8; 32]), VaultError> {
                let input_bytes =
                    kdf.preprocess_2skd(master_password.as_bytes(), &secret_key_for_kdf)?;
                let master_key =
                    kdf.derive_master_key(&input_bytes, &vault_salt, &kdf_params_for_derive)?;
                let verify = kdf.derive_verify_hash(&master_key)?;
                let kek = kdf.derive_kek(&master_key)?;
                // master_password, secret_key_for_kdf, input_bytes, master_key
                // all drop here — each zeroizes its backing buffer.
                Ok((kek, verify))
            },
        )
        .await
        .map_err(|e| VaultError::KeyDerivationFailed(format!("spawn_blocking join: {e}")))??;

        // ---- 4. Pin the KEK in mlock'd memory -------------------------------
        let kek = SecretMem::new(*kek_zeroizing)?;
        drop(kek_zeroizing);

        // ---- 5. Build the initial config ------------------------------------
        let created_at = now();
        let config = VaultConfig {
            id: "default".to_owned(),
            magic: "VEDG".to_owned(),
            schema_version: 1,
            vault_salt,
            kdf_params,
            verify_hash,
            preferred_cipher_suite: 1,
            trash_retention_days: 30,
            // Matches the migration column default + the canonical schema doc (20.2);
            // was 365 before slice 4.6a (an unreviewed outlier `run_maintenance` reads).
            audit_retention_days: 90,
            created_at,
            last_unlocked_at: Some(created_at),
            // Intrinsic identity, minted once at create; never changes on move/rename.
            vault_uuid: Some(ulid::Ulid::new().to_string()),
        };

        // ---- 6. Provision the .vdb + blob store, persist config -------------
        let repo = self.repo_factory.open(&input.vault_path).await?;
        let blob = self
            .blob_factory
            .create(&input.vault_path, Arc::clone(&self.crypto))?;
        repo.save_config(&config).await?;

        // ---- 7. Store the Secret Key in the OS keychain (non-fatal) ---------
        // Mirrors RecoverVault: a keychain failure does not void the vault —
        // the file is valid and the user still holds the Emergency Kit.
        let vault_id = VaultId::new(input.vault_path.clone());
        let (keychain_stored, keychain_error) =
            match self.keychain.store_secret_key(&vault_id, &secret_key) {
                Ok(()) => (true, None),
                Err(e) => (false, Some(e.to_string())),
            };

        // ---- 8. Audit -------------------------------------------------------
        let event = AuditEvent {
            id: ulid::Ulid::new().to_string(),
            entry_id: None,
            action: AuditAction::Created,
            occurred_at: created_at,
            device_id: None,
        };
        repo.append_audit(&event).await?;

        // ---- 9. Assemble the unlocked session -------------------------------
        // Compute the display string, then drop the last plaintext Secret Key.
        let secret_key_display = format_secret_key(&secret_key);
        drop(secret_key);

        let session = VaultSession::assemble(
            vault_id,
            kek,
            VaultIndex::new(),
            config,
            repo,
            Arc::clone(&self.crypto),
            blob,
            Arc::clone(&self.clipboard),
        );

        Ok(CreateVaultOutput {
            session,
            secret_key_display,
            keychain_stored,
            keychain_error,
        })
    }
}
