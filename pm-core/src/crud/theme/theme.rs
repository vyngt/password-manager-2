use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::models::theme::theme::Theme;

impl Theme {
    pub fn create(conn: &mut SqliteConnection) -> bool {
        use crate::db::schema::theme::theme;
        let default_id: i64 = 1;
        let default_cs_id: i64 = 1;

        let data = Theme {
            id: default_id,
            color_scheme_id: Some(default_cs_id),
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
        use crate::db::schema::theme::theme::dsl::*;
        match theme
            .select(Theme::as_select())
            .limit(1)
            .get_result::<Theme>(conn)
        {
            Ok(result) => Some(result),
            Err(_) => None,
        }
    }

    pub fn get_theme(conn: &mut SqliteConnection) -> Theme {
        Self::create(conn);
        Self::get(conn).unwrap()
    }

    pub fn update(conn: &mut SqliteConnection, data: Theme) -> bool {
        use crate::db::schema::theme::theme;
        match diesel::update(theme::table).set(data).execute(conn) {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}
