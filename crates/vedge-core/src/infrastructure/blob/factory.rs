//! `FilesystemBlobStoreFactory` — constructs a `FilesystemBlobStore` for
//! a given vault path. Implementation of the `BlobStoreFactory` port.

use std::path::Path;
use std::sync::Arc;

use crate::application::vault::ports::blob_store::BlobStore;
use crate::application::vault::ports::crypto::CryptoProvider;
use crate::application::vault::ports::BlobStoreFactory;
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::blob::FilesystemBlobStore;

pub struct FilesystemBlobStoreFactory;

impl FilesystemBlobStoreFactory {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for FilesystemBlobStoreFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl BlobStoreFactory for FilesystemBlobStoreFactory {
    fn create(
        &self,
        vault_path: &Path,
        crypto: Arc<dyn CryptoProvider>,
    ) -> Result<Arc<dyn BlobStore>, VaultError> {
        Ok(Arc::new(FilesystemBlobStore::new(vault_path, crypto)?))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::infrastructure::crypto::XChaCha20CryptoProvider;

    #[tokio::test]
    async fn factory_creates_sidecar_dir() {
        // Blob root derives from the vault's file stem — e.g.
        // `my.vdb` → `my.vedge_blobs/` as a sibling.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("my.vdb");
        std::fs::write(&path, b"stub").unwrap();

        let crypto: Arc<dyn CryptoProvider> = Arc::new(XChaCha20CryptoProvider::new());
        let factory = FilesystemBlobStoreFactory::new();
        let _store = factory.create(&path, crypto).unwrap();
        assert!(dir.path().join("my.vedge_blobs").is_dir());
    }
}
