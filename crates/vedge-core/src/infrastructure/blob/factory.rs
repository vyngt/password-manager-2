//! `FilesystemBlobStoreFactory` — constructs a `FilesystemBlobStore` rooted at a vault
//! home's `blobs/` directory. Implementation of the `BlobStoreFactory` port.

use std::path::Path;
use std::sync::Arc;

use crate::application::vault::ports::BlobStoreFactory;
use crate::application::vault::ports::blob_store::BlobStore;
use crate::application::vault::ports::crypto::CryptoProvider;
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
        home: &Path,
        crypto: Arc<dyn CryptoProvider>,
    ) -> Result<Arc<dyn BlobStore>, VaultError> {
        Ok(Arc::new(FilesystemBlobStore::new(home, crypto)?))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::infrastructure::crypto::XChaCha20CryptoProvider;

    #[tokio::test]
    async fn factory_opens_the_home_blobs_dir() {
        // Blobs live at `<home>/blobs/` — a fixed member of the home, no stem.
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("my.vedge");
        std::fs::create_dir_all(home.join("blobs")).unwrap();

        let crypto: Arc<dyn CryptoProvider> = Arc::new(XChaCha20CryptoProvider::new());
        let factory = FilesystemBlobStoreFactory::new();
        let store = factory.create(&home, crypto).unwrap();
        assert_eq!(store.blob_dir(), home.join("blobs"));
    }

    #[tokio::test]
    async fn factory_fails_loudly_on_a_missing_blobs_dir() {
        // A home with no `blobs/` is a half-copied vault: fail, never synthesize empty.
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("my.vedge");
        std::fs::create_dir_all(&home).unwrap();

        let crypto: Arc<dyn CryptoProvider> = Arc::new(XChaCha20CryptoProvider::new());
        let factory = FilesystemBlobStoreFactory::new();
        assert!(factory.create(&home, crypto).is_err());
    }
}
