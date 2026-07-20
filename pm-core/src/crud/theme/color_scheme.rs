use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use super::super::define::ModelCRUD;
use crate::models::theme::color_scheme::{ColorScheme, ColorSchemeCreate, ColorSchemeOut};
use crate::models::WithCount;

impl ModelCRUD<ColorSchemeCreate<'_>> for ColorScheme {
    fn create(conn: &mut SqliteConnection, data: ColorSchemeCreate<'_>) -> Option<Self> {
        use crate::db::schema::theme::color_scheme;

        match diesel::insert_into(color_scheme::table)
            .values(&data)
            .get_result::<Self>(conn)
        {
            Ok(out) => Some(out),
            Err(_) => None,
        }
    }

    fn update(conn: &mut SqliteConnection, data: Self) -> Option<Self> {
        use crate::db::schema::theme::color_scheme;

        match diesel::update(color_scheme::table)
            .filter(color_scheme::id.eq(data.id))
            .set(data)
            .get_result::<Self>(conn)
        {
            Ok(r) => Some(r),
            Err(_) => None,
        }
    }

    fn delete(conn: &mut SqliteConnection, _id: i64) -> bool {
        use crate::db::schema::theme::color_scheme::dsl::*;

        match diesel::delete(color_scheme.filter(id.eq(_id))).execute(conn) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn get(conn: &mut SqliteConnection, _id: i64) -> Option<Self> {
        use crate::db::schema::theme::color_scheme::dsl::*;

        let mut result: Vec<Self> = color_scheme
            .filter(id.eq(_id))
            .load::<Self>(conn)
            .unwrap_or(vec![]);

        if result.len() == 1 {
            result.pop()
        } else {
            None
        }
    }

    fn all(conn: &mut SqliteConnection) -> Vec<Self> {
        use crate::db::schema::theme::color_scheme::dsl::*;
        color_scheme.load::<Self>(conn).unwrap_or(vec![])
    }
}

impl ColorScheme {
    pub fn get_multi(
        conn: &mut SqliteConnection,
        limit: i64,
        offset: i64,
        term: &str,
    ) -> WithCount<Self> {
        use crate::db::schema::theme::color_scheme::dsl::*;

        let total = match color_scheme
            .count()
            .filter(name.like(&format!("%{}%", term)))
            .get_result(conn)
        {
            Ok(e) => e,
            Err(_) => 0,
        };

        let result = color_scheme
            .select(Self::as_select())
            .filter(name.like(&format!("%{}%", term)))
            .order(id.asc())
            .limit(limit)
            .offset(offset)
            .load(conn)
            .unwrap_or(vec![]);

        WithCount { result, total }
    }

    pub fn get_color_cs(conn: &mut SqliteConnection, _id: i64) -> Option<ColorSchemeOut> {
        use crate::db::schema::theme::color_scheme::dsl::*;

        match color_scheme
            .select(ColorSchemeOut::as_select())
            .filter(id.eq(_id))
            .limit(1)
            .get_result::<ColorSchemeOut>(conn)
        {
            Ok(e) => Some(e),
            Err(_) => None,
        }
    }
}
