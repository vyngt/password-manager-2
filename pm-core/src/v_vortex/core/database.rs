use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use std::collections::HashMap;

use crate::v_vortex::utils::errors::{VortexError, VortexResult};

pub struct Database {
    pools: HashMap<String, Pool<ConnectionManager<SqliteConnection>>>,
}

impl Database {
    pub fn new(config: HashMap<String, String>) -> Self {
        let mut pools = HashMap::new();
        for (db_name, db_url) in config {
            let manager = ConnectionManager::<SqliteConnection>::new(db_url);
            let pool = Pool::builder()
                .build(manager)
                .expect(&format!("Failed to create pool for {}", db_name));
            pools.insert(db_name, pool);
        }
        Self { pools }
    }

    pub fn get_pool(&self, db_name: &str) -> Option<&Pool<ConnectionManager<SqliteConnection>>> {
        self.pools.get(db_name)
    }

    pub fn execute_raw(&self, db_name: &str, query: &str) -> VortexResult<usize> {
        let pool = self
            .get_pool(db_name)
            .ok_or(VortexError::ModelNotFound(db_name.to_string()))?;
        let conn = &mut pool.get()?;
        diesel::sql_query(query).execute(conn).map_err(Into::into)
    }

    //
    pub fn get_connection(
        &self,
        db_name: &str,
    ) -> VortexResult<PooledConnection<ConnectionManager<SqliteConnection>>> {
        let pool = self
            .get_pool(db_name)
            .ok_or(VortexError::ModelNotFound(db_name.to_string()))?;
        pool.get().map_err(Into::into)
    }
}
