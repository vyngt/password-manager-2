use crate::db::schema::theme::theme;
use diesel::prelude::*;

#[derive(
    serde::Serialize, serde::Deserialize, Queryable, Insertable, Selectable, PartialEq, AsChangeset,
)]
#[diesel(table_name = theme)]
pub struct Theme {
    pub id: i64,
    pub color_scheme_id: Option<i64>,
}
