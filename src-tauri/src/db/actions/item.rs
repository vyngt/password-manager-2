pub use diesel::prelude::*;
pub use diesel::sqlite::SqliteConnection;

use crate::db::models::item::{Item, NewItem};

impl Item {
    pub fn create(
        conn: &mut SqliteConnection,
        name: &str,
        url: &str,
        username: &str,
        password: &str,
    ) {
        use crate::db::schema::items;
        let new_item = NewItem {
            name,
            url,
            username,
            password,
        };

        diesel::insert_into(items::table)
            .values(&new_item)
            .execute(conn)
            .expect("Error saving new post");
    }

    pub fn all(conn: &mut SqliteConnection) -> Vec<Item> {
        use crate::db::schema::items::dsl::*;
        let results = items.load::<Item>(conn).expect("Error loading items");
        return results;
    }
}
