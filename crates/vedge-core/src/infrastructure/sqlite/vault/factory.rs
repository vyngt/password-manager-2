//! `SqliteVaultRepositoryFactory` — opens a vault home's `vault.vdb`, runs migrations,
//! returns a live `Arc<dyn VaultRepository>`. Implementation of the
//! `VaultRepositoryFactory` port.

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;

use crate::application::vault::ports::VaultRepositoryFactory;
use crate::application::vault::ports::repository::VaultRepository;
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::sqlite::vault::{SqliteVaultRepository, VaultDbConnection};

pub struct SqliteVaultRepositoryFactory;

impl SqliteVaultRepositoryFactory {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for SqliteVaultRepositoryFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VaultRepositoryFactory for SqliteVaultRepositoryFactory {
    async fn open(&self, home: &Path) -> Result<Arc<dyn VaultRepository>, VaultError> {
        // Crash recovery FIRST — reconcile any interrupted restore of this home before
        // SQLite opens. `VaultDbConnection::open` uses `mode=rwc`, which would otherwise
        // fabricate an empty vault at a path a mid-restore crash left missing. This is the
        // single chokepoint every vault open passes through (slice 5.2b; the swap is now a
        // single home-directory rename as of 5.2.0).
        crate::infrastructure::backup::journal::recover_if_pending(home)?;
        let vault_file = home.join(crate::domain::shared::VAULT_FILE);
        let db = VaultDbConnection::open(&vault_file)
            .await
            .map_err(VaultError::Storage)?;
        Ok(Arc::new(SqliteVaultRepository::new(db.handle())))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[tokio::test]
    async fn factory_opens_fresh_vault() {
        let dir = tempfile::tempdir().unwrap();
        // The arg is now the home directory; SQLite creates `vault.vdb` inside it via
        // `mode=rwc`, but the home dir itself must already exist.
        let home = dir.path().join("smoke.vedge");
        std::fs::create_dir_all(&home).unwrap();
        let factory = SqliteVaultRepositoryFactory::new();
        let repo = factory.open(&home).await.unwrap();
        // Fresh vault — migrations ran but no entries inserted.
        let rows = repo.all_entries().await.unwrap();
        assert!(rows.is_empty());
    }
}
