use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use std::collections::HashMap;

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
}
