use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use super::super::define::ModelCRUD;
use crate::models::core::item::{Item, ItemCreate, ItemOut};

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

    fn delete(conn: &mut SqliteConnection, _id: i32) -> bool {
        use crate::db::schema::core::item::dsl::*;

        match diesel::delete(item.filter(id.eq(_id))).execute(conn) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn get(conn: &mut SqliteConnection, _id: i32) -> Option<Self> {
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
    pub fn get_multi(conn: &mut SqliteConnection, limit: i64, offset: i64) -> Vec<ItemOut> {
        use crate::db::schema::core::item::dsl::*;

        item.select(ItemOut::as_select())
            .order(id.asc())
            .limit(limit)
            .offset(offset)
            .load(conn)
            .unwrap_or(vec![])
    }

    pub fn filter_by_name(
        conn: &mut SqliteConnection,
        _query: &str,
        limit: i64,
        offset: i64,
    ) -> Vec<ItemOut> {
        use crate::db::schema::core::item::dsl::*;
        let results = item
            .filter(name.like(&format!("%{}%", _query)))
            .select(ItemOut::as_select())
            .order(id.asc())
            .limit(limit)
            .offset(offset)
            .load(conn)
            .unwrap_or(vec![]);

        results
    }
}
