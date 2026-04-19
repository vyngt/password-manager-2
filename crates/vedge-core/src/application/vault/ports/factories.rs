//! Factories for per-vault infrastructure.
//!
//! The `UnlockVault` use case owns the lifecycle of a vault's `SQLite`
//! connection and blob store — so it constructs them through these
//! factory ports rather than taking pre-built Arcs as input.
//!
//! Why factories instead of direct construction: the `application`
//! layer cannot name `infrastructure` types without violating the
//! dependency direction. Factories are plain trait objects, implemented
//! in `infrastructure/*`, and wired in the shell's composition root.

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;

use crate::application::vault::ports::blob_store::BlobStore;
use crate::application::vault::ports::crypto::CryptoProvider;
use crate::application::vault::ports::repository::VaultRepository;
use crate::domain::vault::errors::VaultError;

/// Open the vault at `vault_path`, run any pending schema migrations,
/// and return a live repository handle. The returned `Arc` owns the
/// DB connection; dropping it closes the connection.
#[async_trait]
pub trait VaultRepositoryFactory: Send + Sync {
    async fn open(
        &self,
        vault_path: &Path,
    ) -> Result<Arc<dyn VaultRepository>, VaultError>;
}

/// Construct the blob-store sidecar for a vault file. Infallible at
/// trait level; the filesystem impl may still return a `VaultError` if
/// the sidecar directory can't be created.
pub trait BlobStoreFactory: Send + Sync {
    fn create(
        &self,
        vault_path: &Path,
        crypto: Arc<dyn CryptoProvider>,
    ) -> Result<Arc<dyn BlobStore>, VaultError>;
}
