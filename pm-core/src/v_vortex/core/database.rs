use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::v_vortex::utils::errors::{VortexError, VortexResult};

#[derive(Clone)]
pub enum ConnectionType {
    Pool(Pool<ConnectionManager<SqliteConnection>>),
    Single(Arc<Mutex<SqliteConnection>>),
}

pub struct Database {
    connections: HashMap<String, ConnectionType>,
}

#[derive(Clone)]
pub enum ConnectionConfig {
    Pool(String),   // URL
    Single(String), // URL
}

impl Database {
    pub fn new(config: HashMap<String, ConnectionConfig>) -> VortexResult<Self> {
        let mut connections = HashMap::new();
        for (db_name, conn_config) in config {
            match conn_config {
                ConnectionConfig::Pool(db_url) => {
                    let manager = ConnectionManager::<SqliteConnection>::new(&db_url);
                    let pool = Pool::builder()
                        .max_size(10)
                        .build(manager)
                        .map_err(|e| VortexError::Pool(e))?;
                    connections.insert(db_name, ConnectionType::Pool(pool));
                }
                ConnectionConfig::Single(db_url) => {
                    let conn = SqliteConnection::establish(&db_url)?;
                    connections.insert(db_name, ConnectionType::Single(Arc::new(Mutex::new(conn))));
                }
            }
        }
        Ok(Self { connections })
    }

    pub fn get_pool(&self, db_name: &str) -> Option<&Pool<ConnectionManager<SqliteConnection>>> {
        self.connections.get(db_name).and_then(|conn| match conn {
            ConnectionType::Pool(pool) => Some(pool),
            _ => None,
        })
    }

    pub fn execute_raw(&self, db_name: &str, query: &str) -> VortexResult<usize> {
        match self.connections.get(db_name) {
            Some(ConnectionType::Pool(pool)) => {
                let conn = &mut pool.get()?;
                diesel::sql_query(query).execute(conn).map_err(Into::into)
            }
            Some(ConnectionType::Single(conn)) => {
                let mut conn = conn
                    .lock()
                    .map_err(|_| VortexError::DatabaseErr("Mutex lock failed".into()))?;
                diesel::sql_query(query)
                    .execute(&mut *conn)
                    .map_err(Into::into)
            }
            None => Err(VortexError::ModelNotFound(db_name.to_string())),
        }
    }

    /// Get connection from pool
    pub fn get_connection(
        &self,
        db_name: &str,
    ) -> VortexResult<PooledConnection<ConnectionManager<SqliteConnection>>> {
        match self.connections.get(db_name) {
            Some(ConnectionType::Pool(pool)) => pool.get().map_err(Into::into),
            Some(ConnectionType::Single(_)) => Err(VortexError::DatabaseErr(
                "Single connection mode does not support pooled connections".into(),
            )),
            None => Err(VortexError::ModelNotFound(db_name.to_string())),
        }
    }

    pub fn with_connection<F, T>(&self, db_name: &str, f: F) -> VortexResult<T>
    where
        F: FnOnce(&mut SqliteConnection) -> VortexResult<T>,
    {
        match self.connections.get(db_name) {
            Some(ConnectionType::Pool(pool)) => {
                let conn = &mut pool.get()?;
                f(conn)
            }
            Some(ConnectionType::Single(conn)) => {
                let mut conn = conn
                    .lock()
                    .map_err(|_| VortexError::DatabaseErr("Mutex lock failed".into()))?;
                f(&mut conn)
            }
            None => Err(VortexError::ModelNotFound(db_name.to_string())),
        }
    }
}
