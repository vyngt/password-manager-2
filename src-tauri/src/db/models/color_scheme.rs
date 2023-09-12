use crate::db::schema::color_scheme;
use diesel::prelude::*;

#[derive(serde::Serialize, serde::Deserialize, Queryable, Selectable, PartialEq)]
#[diesel(table_name = color_scheme)]
pub struct ColorScheme {
    pub id: i32,
    pub name: String,
    pub primary: String,
    pub secondary: String,
    pub success: String,
    pub warning: String,
    pub foreground: String,
    pub background: String,
}
