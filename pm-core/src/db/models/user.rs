use crate::db::schema::user;
use diesel::prelude::*;

#[derive(serde::Serialize, Queryable, Selectable, PartialEq)]
#[diesel(table_name = user)]
pub struct User {
    pub id: i32,
    pub password: String,
}

#[derive(Insertable, serde::Serialize)]
#[diesel(table_name = user)]
pub struct CreateUser {
    pub password: String,
}
