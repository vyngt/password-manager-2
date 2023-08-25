use super::{state::AppDBState, state::AppState};

use crate::db::models::user::User;
use argon2::{Argon2, PasswordHasher};
use diesel::SqliteConnection;
use password_hash::{rand_core::OsRng, PasswordHash, PasswordVerifier, SaltString};

pub fn check_identity(password: &str, password_hashed: &str) -> bool {
    let b_pw = password.as_bytes();
    let parsed_hash = PasswordHash::new(&password_hashed);
    let parsed_hash = match parsed_hash {
        Ok(ph) => ph,
        Err(_) => return false,
    };

    Argon2::default()
        .verify_password(b_pw, &parsed_hash)
        .is_ok()
}

fn get_profile_content(conn: &mut SqliteConnection) -> String {
    User::fetch(conn)
}

#[tauri::command]
pub fn is_first_time(conn: tauri::State<AppDBState>) -> bool {
    let mut c = conn.0.lock().unwrap();
    let c = &mut c.db;
    get_profile_content(c).len() == 0
}

pub fn write_profile(conn: &mut SqliteConnection, password: String) -> bool {
    let user = User::get_profile(conn);
    match user {
        Some(u) => User::set_profile(conn, u, &password),
        None => User::create(conn, &password),
    }
}

pub fn set_master_password(conn: &mut SqliteConnection, password: &str) -> bool {
    let b_pw = password.as_bytes();
    let salt = SaltString::generate(&mut OsRng);
    let hashed = Argon2::default().hash_password(b_pw, &salt);

    let ok = match hashed {
        Ok(value) => write_profile(conn, value.to_string()),
        Err(_e) => false,
    };

    ok
}

fn get_master_password(conn: &mut SqliteConnection) -> String {
    get_profile_content(conn)
}

#[tauri::command]
pub fn login(
    password: &str,
    conn: tauri::State<AppDBState>,
    app_state: tauri::State<AppState>,
) -> bool {
    let mut state = app_state.0.lock().unwrap();
    let state = &mut state;

    let mut c = conn.0.lock().unwrap();
    let c = &mut c.db;

    let password_hashed = get_master_password(c);
    if check_identity(password, &password_hashed) {
        state.set_key(password);
        true
    } else {
        state.reset();
        false
    }
}

#[tauri::command]
pub fn register(
    password: &str,
    conn: tauri::State<AppDBState>,
    app_state: tauri::State<AppState>,
) -> bool {
    let mut state = app_state.0.lock().unwrap();
    let state = &mut state;

    let mut c = conn.0.lock().unwrap();
    let c = &mut c.db;

    let ok = set_master_password(c, password);
    if ok {
        state.set_key(password);
    }

    ok
}
