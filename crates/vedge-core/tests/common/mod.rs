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
use vedge_core::domain::shared::{
    BLOBS_DIR, EntryId, SNAPSHOTS_DIR, TagId, VAULT_FILE, VaultId, now,
};
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
    /// The vault **home** directory (`…/work.vedge`) — the value tests pass as
    /// `vault_path` (slice 5.2.0). The `.vdb` lives at `home.join(VAULT_FILE)`.
    pub home: PathBuf,
    pub vault_id: VaultId,
    /// The intrinsic `vault_uuid` this vault carries; the keychain Secret Key is
    /// seeded under it (slice 5.2.0 — the keychain is uuid-keyed, not path-keyed).
    pub vault_uuid: String,
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
        // The vault is a `<name>.vedge/` home (slice 5.2.0): provision its fixed members
        // before opening the DB / blob store.
        let home = tempdir.path().join("work.vedge");
        std::fs::create_dir_all(home.join(BLOBS_DIR)).unwrap();
        std::fs::create_dir_all(home.join(SNAPSHOTS_DIR)).unwrap();
        let vault_id = VaultId::new(&home);

        let crypto = Arc::new(XChaCha20CryptoProvider::new());
        let kdf = Arc::new(Argon2idKdfProvider::new());
        let keychain = Arc::new(MemoryKeychainProvider::new());
        let biometric = Arc::new(MemoryBiometricAuthenticator::new());
        let clipboard = Arc::new(MemoryClipboardProvider::new());

        let master_password: Zeroizing<String> =
            Zeroizing::new("correct horse battery staple".into());
        let secret_key = [0x55u8; SECRET_KEY_LEN];
        // A normal, migrated/created vault carries an intrinsic uuid, and its keychain
        // Secret Key is stored under `secret:{uuid}` (slice 5.2.0). Backfill-precondition
        // tests that need a pre-4.6 (uuid `None`) vault clear it themselves.
        let vault_uuid = ulid::Ulid::new().to_string();
        keychain.store_secret_key(&vault_uuid, &secret_key).unwrap();

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
            vault_uuid: Some(vault_uuid.clone()),
            commit_counter: 0,
            backup_dir: None,
            backup_keep_count: None,
            last_snapshot_at: None,
            last_backup_at: None,
            recovery_slot: None,
        };

        // Open DB, run migrations, seed config.
        let db = VaultDbConnection::open(&home.join(VAULT_FILE))
            .await
            .unwrap();
        let repo = Arc::new(SqliteVaultRepository::new(db.handle()));
        repo.save_config(&config).await.unwrap();

        let blob = Arc::new(
            FilesystemBlobStore::new(&home, Arc::clone(&crypto) as Arc<dyn CryptoProvider>)
                .unwrap(),
        );

        Self {
            tempdir,
            home,
            vault_id,
            vault_uuid,
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

    /// Seed a DEK-sealed tag (the current format, slice 5.6.0) with the given
    /// (already normalized) name.
    pub async fn seed_tag(&self, normalized_name: &str) -> TagId {
        let id = TagId::new();
        let bytes = tag_bytes(normalized_name);
        let aad = tag_aad(&id).unwrap();
        let dek = self.crypto.generate_dek();
        let (nonce, ciphertext) = self.crypto.encrypt_entry(&dek, &bytes, &aad).unwrap();
        let dek_wrapped = self.crypto.wrap_dek(&dek, &self.kek).unwrap();

        let row = TagRow {
            id: id.clone(),
            nonce,
            ciphertext,
            dek_wrapped: Some(dek_wrapped),
            created_at: now(),
            updated_at: now(),
        };
        self.repo.insert_tag(&row).await.unwrap();
        id
    }

    /// Seed a LEGACY tag sealed directly under the KEK (pre-5.6.0, `dek_wrapped =
    /// None`) — simulates a v1 vault's tag so the at-unlock migration can be tested.
    pub async fn seed_legacy_tag(&self, normalized_name: &str) -> TagId {
        let id = TagId::new();
        let bytes = tag_bytes(normalized_name);
        let aad = tag_aad(&id).unwrap();
        // KEK-as-AEAD-key: the pre-5.6.0 shape. `encrypt_entry(kek, …)` is byte-identical
        // to the retired `encrypt_tag` (both call the same AEAD helper).
        let (nonce, ciphertext) = self.crypto.encrypt_entry(&self.kek, &bytes, &aad).unwrap();

        let row = TagRow {
            id: id.clone(),
            nonce,
            ciphertext,
            dek_wrapped: None,
            created_at: now(),
            updated_at: now(),
        };
        self.repo.insert_tag(&row).await.unwrap();
        id
    }
}

fn tag_bytes(normalized_name: &str) -> Vec<u8> {
    let payload = TagPayload {
        name: normalized_name.to_owned(),
        color: None,
        sort_order: 0,
    };
    serde_json::to_vec(&payload).unwrap()
}

pub fn tempdir_path() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().to_path_buf();
    (dir, p)
}

pub fn as_path(h: &Harness) -> &Path {
    &h.home
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
