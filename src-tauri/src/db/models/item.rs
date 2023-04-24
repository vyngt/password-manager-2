use crate::db::schema::items;
use diesel::prelude::*;

#[derive(serde::Serialize, Queryable, Selectable, PartialEq)]
#[diesel(table_name = items)]
pub struct Item {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub username: String,
    pub password: String,
}

#[derive(serde::Serialize, Queryable, Selectable, PartialEq)]
#[diesel(table_name = items)]
pub struct GItem {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub username: String,
}

#[derive(Insertable)]
#[diesel(table_name = items)]
pub struct NewItem<'a> {
    pub name: &'a str,
    pub url: &'a str,
    pub username: &'a str,
    pub password: &'a str,
}
