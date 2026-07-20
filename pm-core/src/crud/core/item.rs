use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use super::super::define::ModelCRUD;
use crate::models::core::item::{Item, ItemCreate, ItemOut};
use crate::models::WithCount;

impl ModelCRUD<ItemCreate<'_>> for Item {
    fn create(conn: &mut SqliteConnection, data: ItemCreate<'_>) -> Option<Self> {
        use crate::db::schema::core::item;

        match diesel::insert_into(item::table)
            .values(&data)
            .get_result::<Item>(conn)
        {
            Ok(out) => Some(out),
            Err(_) => None,
        }
    }

    fn update(conn: &mut SqliteConnection, data: Self) -> Option<Self> {
        use crate::db::schema::core::item;

        match diesel::update(item::table)
            .filter(item::id.eq(data.id))
            .set(data)
            .get_result::<Self>(conn)
        {
            Ok(r) => Some(r),
            Err(_) => None,
        }
    }

    fn delete(conn: &mut SqliteConnection, _id: i64) -> bool {
        use crate::db::schema::core::item::dsl::*;

        match diesel::delete(item.filter(id.eq(_id))).execute(conn) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn get(conn: &mut SqliteConnection, _id: i64) -> Option<Self> {
        use crate::db::schema::core::item::dsl::*;

        let mut result: Vec<Self> = item.filter(id.eq(_id)).load::<Item>(conn).unwrap_or(vec![]);

        if result.len() == 1 {
            result.pop()
        } else {
            None
        }
    }

    fn all(conn: &mut SqliteConnection) -> Vec<Self> {
        use crate::db::schema::core::item::dsl::*;
        item.load::<Self>(conn).unwrap_or(vec![])
    }
}

impl Item {
    pub fn get_multi(
        conn: &mut SqliteConnection,
        limit: i64,
        offset: i64,
        term: &str,
    ) -> WithCount<ItemOut> {
        use crate::db::schema::core::item::dsl::*;

        let total = match item
            .count()
            .filter(name.like(&format!("%{}%", term)))
            .get_result(conn)
        {
            Ok(e) => e,
            Err(_) => 0,
        };

        let result = item
            .select(ItemOut::as_select())
            .filter(name.like(&format!("%{}%", term)))
            .order(id.asc())
            .limit(limit)
            .offset(offset)
            .load(conn)
            .unwrap_or(vec![]);

        WithCount { result, total }
    }

    pub fn copy_key(conn: &mut SqliteConnection, _id: i64) -> String {
        use crate::db::schema::core::item::dsl::*;

        let mut result = item
            .select(password)
            .filter(id.eq(_id))
            .load::<String>(conn)
            .unwrap_or(vec![]);

        if result.len() == 1 {
            match result.pop() {
                Some(e) => e,
                None => "".into(),
            }
        } else {
            "".into()
        }
    }
}
