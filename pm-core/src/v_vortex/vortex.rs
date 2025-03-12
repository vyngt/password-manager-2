use super::core::database::{ConnectionConfig, Database};
use super::core::model::VortexModel;
use super::core::registry::Registry;
use super::utils::errors::{VortexError, VortexResult};
use diesel::associations::HasTable;
use diesel::prelude::Queryable;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::sqlite::Sqlite;
use diesel::SqliteConnection;

use std::collections::HashMap;

/// Vortex, the new System
pub struct Vortex {
    registry: Registry,
    db: Database,
}

impl Vortex {
    pub fn new(config: HashMap<String, ConnectionConfig>) -> VortexResult<Self> {
        Ok(Self {
            registry: Registry::new(),
            db: Database::new(config)?,
        })
    }

    pub fn register<M>(&mut self)
    where
        M: VortexModel + Send + 'static,
        M: HasTable,
        M: Queryable<<M as HasTable>::Table, Sqlite>,
        <M as HasTable>::Table:
            diesel::query_dsl::LoadQuery<'static, diesel::sqlite::SqliteConnection, M>,
    {
        self.registry.register::<M>();
    }

    pub fn fetch_all<M>(&self) -> VortexResult<Vec<M>>
    where
        M: VortexModel + Send + 'static,
        M: HasTable,
        M: Queryable<<M as HasTable>::Table, Sqlite>,
        <M as HasTable>::Table:
            diesel::query_dsl::LoadQuery<'static, diesel::sqlite::SqliteConnection, M>,
    {
        use diesel::RunQueryDsl;
        let db_name = M::database_name();
        let pool = self
            .db
            .get_pool(db_name)
            .ok_or(VortexError::ModelNotFound(db_name.to_string()))?;
        let conn = &mut pool.get()?;
        let table = <M as HasTable>::table();
        Ok(table.load::<M>(conn)?)
    }

    pub fn execute_raw(&self, db_name: &str, query: &str) -> VortexResult<usize> {
        self.db.execute_raw(db_name, query)
    }

    //
    pub fn get_connection(
        &self,
        db_name: &str,
    ) -> VortexResult<PooledConnection<ConnectionManager<SqliteConnection>>> {
        self.db.get_connection(db_name)
    }

    pub fn with_connection<F, T>(&self, db_name: &str, f: F) -> VortexResult<T>
    where
        F: FnOnce(&mut SqliteConnection) -> VortexResult<T>,
    {
        self.db.with_connection(db_name, f)
    }
}
