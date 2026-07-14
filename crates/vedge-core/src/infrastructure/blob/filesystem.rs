//! Filesystem-backed implementation of [`BlobStore`], rooted inside the vault home.
//!
//! Layout (slice 5.2.0): blobs live at `<home>/blobs/` — a fixed member of the
//! `<name>.vedge/` home, no stem derivation.
//!   `<home>/blobs/{entry_ulid}.blob`
//!
//! File format, per the vault storage spec:
//!   `[24 bytes nonce][XChaCha20-Poly1305 ciphertext + 16-byte Poly1305 tag]`
//!
//! AAD is `blob_aad(entry_id) = entry_ulid_bytes || b"blob"` — distinct from
//! the entry-payload AAD to prevent cross-context ciphertext substitution.
//!
//! **Fail-closed:** [`FilesystemBlobStore::new`] requires the `blobs/` directory to
//! already exist and never synthesizes an empty one. A home whose `vault.vdb` is
//! present but whose `blobs/` is missing is a half-copied vault — opening it must fail
//! loudly, never open with phantom-empty documents. `create_vault` / migration / restore
//! provision `blobs/` up front.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::instrument;
use zeroize::Zeroizing;

use crate::application::vault::ports::blob_store::BlobStore;
use crate::application::vault::ports::crypto::CryptoProvider;
use crate::domain::shared::{BLOBS_DIR, EntryId, StorageError};
use crate::domain::vault::aad::blob_aad;
use crate::domain::vault::crypto_constants::{DEK_LEN, NONCE_LEN};
use crate::domain::vault::errors::VaultError;

const BLOB_FILE_EXT: &str = "blob";

pub struct FilesystemBlobStore {
    root: PathBuf,
    crypto: Arc<dyn CryptoProvider>,
}

impl FilesystemBlobStore {
    /// Open the `<home>/blobs/` store. **Requires** the directory to exist (fail-closed:
    /// a missing `blobs/` beside a live `vault.vdb` is a broken vault, not an empty one).
    /// Provisioning is the caller's job (`create_vault`, migration, restore).
    pub fn new(home: &Path, crypto: Arc<dyn CryptoProvider>) -> Result<Self, VaultError> {
        let root = home.join(BLOBS_DIR);
        if !root.is_dir() {
            return Err(VaultError::Storage(StorageError::Io(format!(
                "vault home is missing its blobs/ directory: {}",
                root.display()
            ))));
        }
        Ok(Self { root, crypto })
    }

    /// Build the final path for an entry's blob.
    fn blob_path(&self, entry_id: &EntryId) -> PathBuf {
        let mut p = self.root.join(entry_id.as_str());
        p.set_extension(BLOB_FILE_EXT);
        p
    }

    /// Root directory for the store (`<home>/blobs`). Shells and tests compose against
    /// this; no cryptographic invariant depends on the path staying private.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
}

fn storage_io(op: &str, e: &std::io::Error) -> VaultError {
    VaultError::Storage(StorageError::Io(format!("{op}: {e}")))
}

#[async_trait]
impl BlobStore for FilesystemBlobStore {
    #[instrument(skip_all, fields(entry_id = %entry_id, bytes = plaintext.len()))]
    async fn write_blob(
        &self,
        entry_id: &EntryId,
        dek: &[u8; DEK_LEN],
        plaintext: &[u8],
    ) -> Result<[u8; NONCE_LEN], VaultError> {
        let aad = blob_aad(entry_id)?;
        let (nonce, ciphertext) = self.crypto.encrypt_entry(dek, plaintext, &aad)?;

        // Write `[nonce || ciphertext]` atomically: write to a temp file
        // then rename. Avoids a half-written blob on crash.
        let final_path = self.blob_path(entry_id);
        let tmp_path = final_path.with_extension("blob.tmp");

        {
            let mut f = fs::File::create(&tmp_path)
                .await
                .map_err(|e| storage_io("create tmp blob", &e))?;
            f.write_all(&nonce)
                .await
                .map_err(|e| storage_io("write nonce", &e))?;
            f.write_all(&ciphertext)
                .await
                .map_err(|e| storage_io("write ciphertext", &e))?;
            f.flush().await.map_err(|e| storage_io("flush blob", &e))?;
            // `f` dropped here — handle closed before rename.
        }

        fs::rename(&tmp_path, &final_path)
            .await
            .map_err(|e| storage_io("rename blob into place", &e))?;
        Ok(nonce)
    }

    #[instrument(skip_all, fields(entry_id = %entry_id))]
    async fn read_blob(
        &self,
        entry_id: &EntryId,
        dek: &[u8; DEK_LEN],
        blob_nonce: &[u8; NONCE_LEN],
    ) -> Result<Zeroizing<Vec<u8>>, VaultError> {
        let path = self.blob_path(entry_id);
        let mut f = fs::File::open(&path)
            .await
            .map_err(|e| storage_io("open blob", &e))?;

        // File format: [24 bytes nonce][ciphertext + 16-byte tag].
        // Validate the on-disk nonce matches the caller-supplied one — the
        // latter comes from the decrypted DocumentPayload and is the binding
        // authority. Mismatch is indistinguishable from tampering; collapse
        // to DecryptionFailed.
        let mut disk_nonce = [0u8; NONCE_LEN];
        f.read_exact(&mut disk_nonce)
            .await
            .map_err(|_| VaultError::DecryptionFailed)?;
        if &disk_nonce != blob_nonce {
            return Err(VaultError::DecryptionFailed);
        }

        let mut ciphertext = Vec::new();
        f.read_to_end(&mut ciphertext)
            .await
            .map_err(|e| storage_io("read blob", &e))?;

        let aad = blob_aad(entry_id)?;
        self.crypto
            .decrypt_entry(dek, blob_nonce, &ciphertext, &aad)
    }

    #[instrument(skip_all, fields(entry_id = %entry_id))]
    async fn delete_blob(&self, entry_id: &EntryId) -> Result<(), VaultError> {
        let path = self.blob_path(entry_id);
        match fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(storage_io("delete blob", &e)),
        }
    }

    fn blob_dir(&self) -> &Path {
        &self.root
    }

    #[instrument(skip_all)]
    async fn list_blobs(&self) -> Result<Vec<EntryId>, VaultError> {
        let mut ids = Vec::new();
        let mut rd = match fs::read_dir(&self.root).await {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(ids),
            Err(e) => return Err(storage_io("read_dir", &e)),
        };
        while let Some(entry) = rd
            .next_entry()
            .await
            .map_err(|e| storage_io("next_entry", &e))?
        {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some(BLOB_FILE_EXT) {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                ids.push(EntryId::from_raw(stem.to_owned()));
            }
        }
        Ok(ids)
    }
}
