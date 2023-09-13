use crate::db::schema::theme;
use diesel::prelude::*;

#[derive(
    serde::Serialize, serde::Deserialize, Queryable, Selectable, PartialEq, AsChangeset, Insertable,
)]
#[diesel(table_name = theme)]
pub struct Theme {
    pub id: i32,
    pub color_scheme_id: Option<i32>,
}
