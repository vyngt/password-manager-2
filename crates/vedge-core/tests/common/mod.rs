//! Shared test harness: build an on-disk vault (config + encrypted entry +
//! encrypted tag) using the real crypto + KDF providers. Returned as a
//! ready-to-unlock bundle.
//!
//! Kept in `tests/common/` so every integration test in Phases 4-10 can
//! reuse it without copy-paste.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::needless_pass_by_value
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use secrecy::SecretString;
use tempfile::TempDir;
use zeroize::Zeroizing;

use vedge_core::application::vault::ports::{
    BlobStoreFactory, ClipboardProvider, CryptoProvider, KeyDerivationProvider, KeychainProvider,
    VaultRepository, VaultRepositoryFactory,
};
use vedge_core::application::vault::use_cases::UnlockVault;
use vedge_core::domain::shared::{EntryId, TagId, VaultId, now};
use vedge_core::domain::vault::aad::{entry_aad, tag_aad};
use vedge_core::domain::vault::crypto_constants::{KEK_LEN, SECRET_KEY_LEN, VAULT_SALT_LEN};
use vedge_core::domain::vault::entities::{EntryRow, TagRow, VaultConfig};
use vedge_core::domain::vault::kdf_params::KdfParams;
use vedge_core::domain::vault::payloads::{
    CommonMeta, EntryPayload, EntryType, LoginPayload, TagPayload,
};
use vedge_core::infrastructure::biometric::MemoryBiometricAuthenticator;
use vedge_core::infrastructure::blob::FilesystemBlobStore;
use vedge_core::infrastructure::clipboard::MemoryClipboardProvider;
use vedge_core::infrastructure::crypto::{Argon2idKdfProvider, XChaCha20CryptoProvider};
use vedge_core::infrastructure::keychain::MemoryKeychainProvider;
use vedge_core::infrastructure::sqlite::vault::{SqliteVaultRepository, VaultDbConnection};

/// Test-profile Argon2id parameters — trims memory from 256 MiB to 8 KiB so
/// the unit/integration suite stays fast. Production uses
/// [`KdfParams::argon2id_default`].
pub fn fast_kdf_params() -> KdfParams {
    KdfParams {
        alg: "argon2id".into(),
        m: 8,
        t: 1,
        p: 1,
        version: 1,
    }
}

pub struct Harness {
    pub tempdir: TempDir,
    pub vdb_path: PathBuf,
    pub vault_id: VaultId,
    pub master_password: Zeroizing<String>,
    pub secret_key: [u8; SECRET_KEY_LEN],
    pub crypto: Arc<XChaCha20CryptoProvider>,
    pub kdf: Arc<Argon2idKdfProvider>,
    pub keychain: Arc<MemoryKeychainProvider>,
    pub biometric: Arc<MemoryBiometricAuthenticator>,
    pub repo: Arc<SqliteVaultRepository>,
    pub blob: Arc<FilesystemBlobStore>,
    pub clipboard: Arc<MemoryClipboardProvider>,
    pub kdf_params: KdfParams,
    pub config: VaultConfig,
    /// KEK derived during harness setup — kept so tests can encrypt
    /// additional rows without re-running the KDF.
    pub kek: [u8; KEK_LEN],
}

impl Harness {
    /// Build a fresh temp vault with `VaultConfig` + one `Login` entry and
    /// one tag pre-seeded. Registers the Secret Key in the memory keychain.
    pub async fn fresh() -> Self {
        let tempdir = tempfile::tempdir().unwrap();
        let vdb_path = tempdir.path().join("work.vdb");
        let vault_id = VaultId::new(&vdb_path);

        let crypto = Arc::new(XChaCha20CryptoProvider::new());
        let kdf = Arc::new(Argon2idKdfProvider::new());
        let keychain = Arc::new(MemoryKeychainProvider::new());
        let biometric = Arc::new(MemoryBiometricAuthenticator::new());
        let clipboard = Arc::new(MemoryClipboardProvider::new());

        let master_password: Zeroizing<String> =
            Zeroizing::new("correct horse battery staple".into());
        let secret_key = [0x55u8; SECRET_KEY_LEN];
        keychain.store_secret_key(&vault_id, &secret_key).unwrap();

        let kdf_params = fast_kdf_params();
        let vault_salt = [0x22u8; VAULT_SALT_LEN];

        // Derive KEK + verify_hash up front so we can craft ciphertexts
        // before running UnlockVault.
        let input_bytes = kdf
            .preprocess_2skd(master_password.as_bytes(), &secret_key)
            .unwrap();
        let mk = kdf
            .derive_master_key(&input_bytes, &vault_salt, &kdf_params)
            .unwrap();
        let verify_hash = kdf.derive_verify_hash(&mk).unwrap();
        let kek_z = kdf.derive_kek(&mk).unwrap();
        let kek: [u8; KEK_LEN] = *kek_z;

        let config = VaultConfig {
            id: "default".into(),
            magic: "VEDG".into(),
            schema_version: 1,
            vault_salt,
            kdf_params: kdf_params.clone(),
            verify_hash,
            preferred_cipher_suite: 1,
            trash_retention_days: 30,
            audit_retention_days: 90,
            created_at: now(),
            last_unlocked_at: None,
            vault_uuid: None,
            commit_counter: 0,
        };

        // Open DB, run migrations, seed config.
        let db = VaultDbConnection::open(&vdb_path).await.unwrap();
        let repo = Arc::new(SqliteVaultRepository::new(db.handle()));
        repo.save_config(&config).await.unwrap();

        let blob = Arc::new(
            FilesystemBlobStore::new(&vdb_path, Arc::clone(&crypto) as Arc<dyn CryptoProvider>)
                .unwrap(),
        );

        Self {
            tempdir,
            vdb_path,
            vault_id,
            master_password,
            secret_key,
            crypto,
            kdf,
            keychain,
            biometric,
            repo,
            blob,
            clipboard,
            kdf_params,
            config,
            kek,
        }
    }

    /// Encrypt and insert a `Login` entry directly via the repo + crypto
    /// layer — bypassing the use-case API so tests can set up state
    /// deterministically.
    pub async fn seed_login(&self, name: &str, username: &str, password: &str) -> EntryId {
        let id = EntryId::new();
        let version: i64 = 1;

        let mut meta = CommonMeta::new(name, EntryType::Login);
        meta.url = Some("https://example.com".into());
        let payload = EntryPayload::Login(LoginPayload {
            meta,
            username: username.to_owned(),
            password: SecretString::from(password),
            totp_secret: None,
            totp_params: vedge_core::TotpParams::default(),
            recovery_codes: vec![],
        });
        let payload_bytes = payload.to_encryptable_json().unwrap();

        let dek = self.crypto.generate_dek();
        let aad = entry_aad(&id, version).unwrap();
        let (nonce, ciphertext) = self
            .crypto
            .encrypt_entry(&dek, &payload_bytes, &aad)
            .unwrap();
        let dek_wrapped = self.crypto.wrap_dek(&dek, &self.kek).unwrap();

        let row = EntryRow {
            id: id.clone(),
            version,
            cipher_suite: 1,
            dek_wrapped,
            nonce,
            ciphertext,
            created_at: now(),
            updated_at: now(),
            accessed_at: None,
            is_trashed: false,
            trashed_at: None,
        };
        self.repo.insert_entry(&row).await.unwrap();
        id
    }

    /// Encrypt and insert an entry whose `entry_type` is unrecognized, so it
    /// decodes to `EntryPayload::Unknown`. Bypasses the use-case API (which
    /// refuses to serialize `Unknown`) by encrypting a bare `CommonMeta` whose
    /// `entry_type` is `Unknown(..)`. Used to exercise the reveal/edit guards.
    pub async fn seed_unknown(&self, name: &str, unknown_type: &str) -> EntryId {
        let id = EntryId::new();
        let version: i64 = 1;

        let meta = CommonMeta::new(name, EntryType::Unknown(unknown_type.to_owned()));
        let bytes = serde_json::to_vec(&meta).unwrap();

        let dek = self.crypto.generate_dek();
        let aad = entry_aad(&id, version).unwrap();
        let (nonce, ciphertext) = self.crypto.encrypt_entry(&dek, &bytes, &aad).unwrap();
        let dek_wrapped = self.crypto.wrap_dek(&dek, &self.kek).unwrap();

        let row = EntryRow {
            id: id.clone(),
            version,
            cipher_suite: 1,
            dek_wrapped,
            nonce,
            ciphertext,
            created_at: now(),
            updated_at: now(),
            accessed_at: None,
            is_trashed: false,
            trashed_at: None,
        };
        self.repo.insert_entry(&row).await.unwrap();
        id
    }

    /// Seed a tag with the given (already normalized) name.
    pub async fn seed_tag(&self, normalized_name: &str) -> TagId {
        let id = TagId::new();
        let payload = TagPayload {
            name: normalized_name.to_owned(),
            color: None,
            sort_order: 0,
        };
        let bytes = serde_json::to_vec(&payload).unwrap();
        let aad = tag_aad(&id).unwrap();
        let (nonce, ciphertext) = self.crypto.encrypt_tag(&self.kek, &bytes, &aad).unwrap();

        let row = TagRow {
            id: id.clone(),
            nonce,
            ciphertext,
            created_at: now(),
            updated_at: now(),
        };
        self.repo.insert_tag(&row).await.unwrap();
        id
    }
}

pub fn tempdir_path() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().to_path_buf();
    (dir, p)
}

pub fn as_path(h: &Harness) -> &Path {
    &h.vdb_path
}

/// Construct an `UnlockVault` use case wired to the harness's ports +
/// real infrastructure factories. The factories open a *second*
/// `SQLite` connection to the same `.vdb` that the harness already
/// seeded — `SQLite` allows concurrent connections, so both live side
/// by side for the duration of the test.
pub fn build_unlock(h: &Harness) -> UnlockVault {
    use vedge_core::infrastructure::blob::FilesystemBlobStoreFactory;
    use vedge_core::infrastructure::sqlite::vault::SqliteVaultRepositoryFactory;

    UnlockVault {
        repo_factory: Arc::new(SqliteVaultRepositoryFactory::new())
            as Arc<dyn VaultRepositoryFactory>,
        blob_factory: Arc::new(FilesystemBlobStoreFactory::new()) as Arc<dyn BlobStoreFactory>,
        crypto: Arc::clone(&h.crypto) as Arc<dyn CryptoProvider>,
        clipboard: Arc::clone(&h.clipboard) as Arc<dyn ClipboardProvider>,
        kdf: Arc::clone(&h.kdf) as Arc<dyn KeyDerivationProvider>,
        keychain: Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>,
    }
}
