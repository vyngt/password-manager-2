use std::path::Path;
use std::sync::Arc;

use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;

use crate::domain::shared::StorageError;

use super::migrations::Migrator;

#[derive(Clone)]
pub struct AppDbConnection {
    conn: Arc<DatabaseConnection>,
}

impl AppDbConnection {
    pub async fn open(path: &Path) -> Result<Self, StorageError> {
        let url = format!("sqlite://{}?mode=rwc", path.to_string_lossy());
        let conn = Database::connect(ConnectOptions::new(url))
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;

        Migrator::up(&conn, None)
            .await
            .map_err(|e| StorageError::Migration(e.to_string()))?;

        Ok(Self {
            conn: Arc::new(conn),
        })
    }

    #[must_use]
    pub fn handle(&self) -> Arc<DatabaseConnection> {
        Arc::clone(&self.conn)
    }
}
