use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::db::models::color_scheme::{ColorScheme, CreateColorScheme};

impl ColorScheme {
    pub fn create(conn: &mut SqliteConnection, data: CreateColorScheme) -> Option<ColorScheme> {
        use crate::db::schema::color_scheme;

        match diesel::insert_into(color_scheme::table)
            .values(&data)
            .get_result::<ColorScheme>(conn)
        {
            Ok(e) => Some(e),
            Err(_) => None,
        }
    }

    pub fn get(conn: &mut SqliteConnection, _id: &i32) -> Option<ColorScheme> {
        use crate::db::schema::color_scheme::dsl::*;
        let mut result = color_scheme
            .filter(id.eq(_id))
            .load::<ColorScheme>(conn)
            .unwrap_or(vec![]);
        if result.len() == 1 {
            result.pop()
        } else {
            None
        }
    }

    pub fn update(conn: &mut SqliteConnection, data: ColorScheme) -> bool {
        use crate::db::schema::color_scheme;
        match diesel::update(color_scheme::table).set(data).execute(conn) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub fn delete(conn: &mut SqliteConnection, _id: &i32) -> bool {
        use crate::db::schema::color_scheme::dsl::*;

        match diesel::delete(color_scheme.filter(id.eq(_id))).execute(conn) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub fn all(conn: &mut SqliteConnection) -> Vec<ColorScheme> {
        use crate::db::schema::color_scheme::dsl::*;
        let results = color_scheme
            .select(ColorScheme::as_select())
            .load::<ColorScheme>(conn)
            .unwrap_or(vec![]);
        results
    }
}
