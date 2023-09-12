use crate::db::schema::theme;
use diesel::prelude::*;

#[derive(serde::Serialize, serde::Deserialize, Queryable, Selectable, PartialEq, AsChangeset)]
#[diesel(table_name = theme)]
pub struct Theme {
    pub id: i32,
    pub color_scheme_id: i32,
}

#[derive(Insertable, serde::Serialize, serde::Deserialize)]
#[diesel(table_name = theme)]
pub struct CreateTheme {
    pub color_scheme_id: i32,
}
