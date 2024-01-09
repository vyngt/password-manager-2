use crate::db::schema::core::item;
use diesel::prelude::*;

#[derive(serde::Serialize, serde::Deserialize, Queryable, Selectable, PartialEq, AsChangeset)]
#[diesel(table_name = item)]
pub struct Item {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub username: String,
    pub password: String,
}

#[derive(serde::Serialize, Queryable, Selectable, PartialEq)]
#[diesel(table_name = item)]
pub struct ItemOut {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub username: String,
}

#[derive(Insertable, serde::Serialize, serde::Deserialize)]
#[diesel(table_name = item)]
pub struct ItemCreate<'a> {
    pub name: &'a str,
    pub url: &'a str,
    pub username: &'a str,
    pub password: &'a str,
}
