use super::core::database::Database;
use super::core::model::VortexModel;
use super::core::registry::Registry;
use super::utils::errors::{VortexError, VortexResult};
use diesel::associations::HasTable;

use std::collections::HashMap;

/// Vortex, the new System
pub struct Vortex {
    registry: Registry,
    db: Database,
}

impl Vortex {
    pub fn new(config: HashMap<String, String>) -> Self {
        Self {
            registry: Registry::new(),
            db: Database::new(config),
        }
    }

    pub fn register<M>(&mut self)
    where
        M: VortexModel,
        <M as HasTable>::Table:
            diesel::query_dsl::LoadQuery<'static, diesel::sqlite::SqliteConnection, M>,
    {
        self.registry.register::<M>();
    }

    pub fn fetch_all<M>(&self) -> VortexResult<Vec<M>>
    where
        M: VortexModel,
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
}
