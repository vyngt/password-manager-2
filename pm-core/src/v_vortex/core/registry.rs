use diesel::associations::HasTable;
use diesel::prelude::*;

use std::collections::HashMap;

use super::model::{Model, VortexModel};
use crate::v_vortex::utils::errors::VortexResult;

pub trait ModelFetcher {
    fn fetch_all(&self, conn: &mut SqliteConnection) -> VortexResult<Vec<Box<dyn Model>>>;
}

///
/// This trait is used to fetch models from the database.
///
/// The fetcher associated with each model is responsible for fetching all the
/// models from the database.
///
///
/// The models data structure stores the fetcher and database name for each
/// model. The key is the table name and the value is a tuple containing the
/// fetcher and database name.
///
/// ```
/// table_name, (fetcher, database_name)
/// ```
pub struct Registry {
    models: HashMap<&'static str, (Box<dyn ModelFetcher>, &'static str)>,
}

struct Fetcher<M> {
    _phantom: std::marker::PhantomData<M>,
}

impl<M> ModelFetcher for Fetcher<M>
where
    M: VortexModel,
    <M as HasTable>::Table:
        diesel::query_dsl::LoadQuery<'static, diesel::sqlite::SqliteConnection, M>,
{
    fn fetch_all(&self, conn: &mut SqliteConnection) -> VortexResult<Vec<Box<dyn Model>>> {
        use diesel::RunQueryDsl;
        let table = <M as HasTable>::table();
        let results = table.load::<M>(conn)?;
        Ok(results
            .into_iter()
            .map(|m| Box::new(m) as Box<dyn Model>)
            .collect())
    }
}

impl Registry {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    pub fn register<M>(&mut self)
    where
        M: VortexModel,
        <M as HasTable>::Table:
            diesel::query_dsl::LoadQuery<'static, diesel::sqlite::SqliteConnection, M>,
    {
        let table_name = M::table_name();
        let db_name = M::database_name();
        let fetcher = Fetcher::<M> {
            _phantom: std::marker::PhantomData,
        };
        self.models.insert(table_name, (Box::new(fetcher), db_name));
    }

    pub fn get_fetcher(&self, table_name: &str) -> Option<&Box<dyn ModelFetcher>> {
        self.models.get(table_name).map(|(fetcher, _)| fetcher)
    }

    pub fn get_db_name(&self, table_name: &str) -> Option<&'static str> {
        self.models.get(table_name).map(|(_, db_name)| *db_name)
    }
}
