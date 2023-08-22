use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::db::models::user::{CreateUser, User};

impl User {
    pub fn create(conn: &mut SqliteConnection, password_hash: String) -> bool {
        use crate::db::schema::user;

        let new_user = CreateUser {
            password: password_hash,
        };

        match diesel::insert_into(user::table)
            .values(&new_user)
            .execute(conn)
        {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub fn fetch(conn: &mut SqliteConnection) -> String {
        use crate::db::schema::user::dsl::*;

        let users: Vec<User> = user
            .limit(1)
            .select((id, password))
            .load(conn)
            .unwrap_or(vec![]);

        if *&users.len() != 0 {
            users[0].password.clone()
        } else {
            "".to_string()
        }
    }
}