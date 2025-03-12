use diesel::associations::HasTable;
use diesel::sqlite::Sqlite;

use diesel::{Queryable, QueryableByName};
use serde::{Deserialize, Serialize};

pub trait Model: Send + 'static {
    fn table_name() -> &'static str
    where
        Self: Sized;

    fn database_name() -> &'static str
    where
        Self: Sized;

    fn relations() -> Vec<(&'static str, &'static str)>
    where
        Self: Sized,
    {
        vec![]
    }
}

pub trait SerializeModel: Model + Serialize + Deserialize<'static> {}
pub trait QueryableModel: Model + QueryableByName<Sqlite> {}
pub trait VortexModel: Model + QueryableByName<Sqlite> + 'static {}
