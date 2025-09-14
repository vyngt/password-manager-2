use crate::db::schema::theme::theme;
use crate::v_vortex::core::model::{Model, QueryableModel, SerializeModel, VortexModel};
use diesel::prelude::*;

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Queryable,
    QueryableByName,
    Insertable,
    Selectable,
    PartialEq,
    AsChangeset,
)]
#[diesel(table_name = theme)]
pub struct Theme {
    pub id: i64,
    pub color_scheme_id: i64,
}

impl Model for Theme {
    fn table_name() -> &'static str {
        "theme"
    }

    fn database_name() -> &'static str {
        "theme"
    }
}

impl SerializeModel for Theme {}
impl QueryableModel for Theme {}
impl VortexModel for Theme {}
