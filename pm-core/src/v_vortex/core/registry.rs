use diesel::associations::HasTable;
use diesel::prelude::*;
use diesel::sqlite::Sqlite;

use std::collections::HashMap;

use super::model::Model;
use crate::v_vortex::utils::errors::VortexResult;

pub trait ModelFetcher {
    fn fetch_all(&self, conn: &mut SqliteConnection) -> VortexResult<Vec<Box<dyn Model>>>;
}

pub struct Registry {
    models: HashMap<&'static str, (Box<dyn ModelFetcher>, &'static str)>, // (fetcher, database_name)
}

struct Fetcher<M> {
    _phantom: std::marker::PhantomData<M>,
}

impl<M> ModelFetcher for Fetcher<M>
where
    M: Model + QueryableByName<Sqlite> + 'static + HasTable,
    M: diesel::Queryable<<M as HasTable>::Table, Sqlite>,
    <M as HasTable>::Table: diesel::Table
        + diesel::query_dsl::RunQueryDsl<SqliteConnection>
        + diesel::query_dsl::LoadQuery<'static, SqliteConnection, M>
        + 'static,
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
