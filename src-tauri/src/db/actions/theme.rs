use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::db::models::theme::{CreateTheme, Theme};

impl CreateTheme {
    pub fn new() -> Self {
        CreateTheme { color_scheme_id: 1 }
    }
}

impl Theme {
    pub fn create(conn: &mut SqliteConnection) -> bool {
        use crate::db::schema::theme;
        let data = CreateTheme::new();
        match diesel::insert_into(theme::table)
            .values(&data)
            .execute(conn)
        {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub fn get(conn: &mut SqliteConnection) -> Option<Theme> {
        use crate::db::schema::theme::dsl::*;
        match theme
            .select(Theme::as_select())
            .limit(1)
            .load::<Theme>(conn)
        {
            Ok(results) => {
                let mut results = results;
                results.pop()
            }
            Err(_) => None,
        }
    }

    pub fn update(conn: &mut SqliteConnection, data: Theme) -> bool {
        use crate::db::schema::theme;
        match diesel::update(theme::table).set(data).execute(conn) {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}
