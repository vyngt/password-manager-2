use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::db::models::theme::Theme;

impl Theme {
    pub fn create(conn: &mut SqliteConnection) -> bool {
        use crate::db::schema::theme;
        let data = Theme {
            id: 1,
            color_scheme_id: Some(1),
        };
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

    pub fn get_theme(conn: &mut SqliteConnection) -> Theme {
        Self::create(conn);

        Self::get(conn).unwrap()
    }

    pub fn update(conn: &mut SqliteConnection, data: Theme) -> bool {
        use crate::db::schema::theme;
        match diesel::update(theme::table).set(data).execute(conn) {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}
