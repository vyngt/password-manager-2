use crate::db::schema::theme::color_scheme;
use diesel::prelude::*;

#[derive(serde::Serialize, serde::Deserialize, Queryable, Selectable, PartialEq, AsChangeset)]
#[diesel(table_name = color_scheme)]
pub struct ColorScheme {
    pub id: i64,
    pub name: String,
    pub primary: String,
    pub secondary: String,
    pub success: String,
    pub warning: String,
    pub danger: String,
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
    pub warning: &'a str,
    pub danger: &'a str,
    pub foreground: &'a str,
    pub background: &'a str,
}
