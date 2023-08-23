use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::db::models::item::{GItem, Item, NewItem};

impl GItem {
    pub fn default() -> GItem {
        GItem {
            id: 0,
            name: String::from(""),
            url: String::from(""),
            username: String::from(""),
        }
    }
}

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

    pub fn default() -> Item {
        Item {
            id: 0,
            name: String::from(""),
            url: String::from(""),
            username: String::from(""),
            password: String::from(""),
        }
    }

    pub fn update(
        conn: &mut SqliteConnection,
        _id: &i32,
        _name: &str,
        _url: &str,
        _username: &str,
        _password: &str,
    ) -> bool {
        use crate::db::schema::items::dsl::*;

        diesel::update(items)
            .filter(id.eq(_id))
            .set((
                name.eq(_name),
                url.eq(_url),
                username.eq(_username),
                password.eq(_password),
            ))
            .execute(conn)
            .unwrap();

        true
    }

    pub fn delete(conn: &mut SqliteConnection, _id: &i32) -> bool {
        use crate::db::schema::items::dsl::*;

        diesel::delete(items.filter(id.eq(_id)))
            .execute(conn)
            .unwrap();

        true
    }

    pub fn get(conn: &mut SqliteConnection, _id: &i32) -> Option<Item> {
        use crate::db::schema::items::dsl::*;

        let mut result = items
            .filter(id.eq(_id))
            .load::<Item>(conn)
            .unwrap_or(vec![]);

        if result.len() == 1 {
            result.pop()
        } else {
            None
        }
    }

    pub fn all(conn: &mut SqliteConnection) -> Vec<GItem> {
        use crate::db::schema::items::dsl::*;
        let results = items
            .select((id, name, url, username))
            .load(conn)
            .unwrap_or(vec![]);
        return results;
    }
}
