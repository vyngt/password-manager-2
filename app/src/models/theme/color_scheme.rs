use crate::db::schema::theme::color_scheme;
use crate::v_vortex::core::model::{Model, QueryableModel, SerializeModel, VortexModel};
use diesel::prelude::*;

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Queryable,
    QueryableByName,
    Selectable,
    PartialEq,
    AsChangeset,
)]
#[diesel(table_name = color_scheme)]
pub struct ColorScheme {
    pub id: i64,
    pub name: String,
    pub primary: String,
    pub secondary: String,
    pub success: String,
    pub danger: String,
    pub warning: String,
    pub foreground: String,
    pub background: String,
}

impl Model for ColorScheme {
    fn table_name() -> &'static str {
        "color_scheme"
    }

    fn database_name() -> &'static str {
        "theme"
    }
}
impl SerializeModel for ColorScheme {}
impl QueryableModel for ColorScheme {}
impl VortexModel for ColorScheme {}

#[derive(serde::Serialize, Queryable, Selectable, PartialEq)]
#[diesel(table_name = color_scheme)]
pub struct ColorSchemeOut {
    pub primary: String,
    pub secondary: String,
    pub success: String,
    pub danger: String,
    pub warning: String,
    pub foreground: String,
    pub background: String,
}

#[derive(Insertable, serde::Serialize, serde::Deserialize)]
#[diesel(table_name = color_scheme)]
pub struct ColorSchemeCreate<'a> {
    pub name: &'a str,
    pub primary: &'a str,
    pub secondary: &'a str,
    pub success: &'a str,
    pub danger: &'a str,
    pub warning: &'a str,
    pub foreground: &'a str,
    pub background: &'a str,
}
