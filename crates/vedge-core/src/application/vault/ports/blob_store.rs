use async_trait::async_trait;
use zeroize::Zeroizing;

use crate::domain::shared::EntryId;
use crate::domain::vault::crypto_constants::{DEK_LEN, NONCE_LEN};
use crate::domain::vault::errors::VaultError;

/// Encrypted sidecar storage for `Document` entry bodies.
///
/// Each blob lives in a file adjacent to the `.vdb`; its contents are encrypted
/// with the entry's own DEK under a blob-specific AAD (see
/// [`crate::domain::vault::aad::blob_aad`]). Authentication-style failures
/// (wrong DEK, wrong nonce, tampered file) collapse to
/// [`VaultError::DecryptionFailed`] — the caller cannot distinguish between
/// them.
#[async_trait]
pub trait BlobStore: Send + Sync {
    /// Encrypt `plaintext` with `dek` under the blob AAD, write to
    /// `{root}/{entry_id}.blob`. Returns the 24-byte nonce the caller must
    /// embed in `DocumentPayload.blob_nonce`.
    async fn write_blob(
        &self,
        entry_id: &EntryId,
        dek: &[u8; DEK_LEN],
        plaintext: &[u8],
    ) -> Result<[u8; NONCE_LEN], VaultError>;

    /// Decrypt the blob into a zeroizing buffer. Returns `DecryptionFailed`
    /// for any authentication failure or file-corruption cause.
    async fn read_blob(
        &self,
        entry_id: &EntryId,
        dek: &[u8; DEK_LEN],
        blob_nonce: &[u8; NONCE_LEN],
    ) -> Result<Zeroizing<Vec<u8>>, VaultError>;

    /// Delete the blob file. No error if already missing.
    async fn delete_blob(&self, entry_id: &EntryId) -> Result<(), VaultError>;

    /// List entry IDs whose blob files exist on disk. Used by `RunMaintenance`
    /// for orphan detection; the returned IDs are derived from filenames, not
    /// cryptographically verified.
    async fn list_blobs(&self) -> Result<Vec<EntryId>, VaultError>;
}
