use super::{config, state::AppState};

use argon2::{Argon2, PasswordHasher};
use password_hash::{rand_core::OsRng, PasswordHash, PasswordVerifier, SaltString};
use std::{fs, path};

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

fn get_profile_content() -> String {
    let home_dir = config::get_home_dir();

    match home_dir {
        Some(dir) => {
            let profile_path = path::Path::new(&dir);
            let profile_path = profile_path.join(config::APP_DIR).join(config::PROFILE);
            let profile_path = profile_path.to_str().unwrap();

            let contents = fs::read_to_string(&profile_path);

            match contents {
                Ok(contents) => return String::from(contents.trim()),
                Err(_) => {}
            }
        }
        None => {}
    }

    String::from("")
}

#[tauri::command]
pub fn is_first_time() -> bool {
    get_profile_content().len() != 0
}

pub fn write_profile(password: &str) -> std::io::Result<()> {
    let home_dir = config::get_home_dir();

    match home_dir {
        Some(dir) => {
            let profile_path = path::Path::new(&dir);
            let profile_path = profile_path.join(config::APP_DIR).join(config::PROFILE);
            let profile_path = profile_path.to_str().unwrap();

            fs::write(profile_path, password)?;
        }
        None => {}
    }
    Ok(())
}

pub fn set_master_password(password: &str) -> std::io::Result<()> {
    let b_pw = password.as_bytes();
    let salt = SaltString::generate(&mut OsRng);
    let hashed = Argon2::default().hash_password(b_pw, &salt);

    let result = match hashed {
        Ok(value) => value.to_string(),
        Err(_e) => String::from(""),
    };

    write_profile(&result).unwrap();
    Ok(())
}

fn get_master_password() -> String {
    get_profile_content()
}

#[tauri::command]
pub fn login(password: &str, app_state: tauri::State<AppState>) -> bool {
    let mut state = app_state.0.lock().unwrap();
    let state = &mut state;

    let password_hashed = get_master_password();
    if check_identity(password, &password_hashed) {
        state.set_key(password);
        true
    } else {
        state.reset();
        false
    }
}

#[tauri::command]
pub fn register(password: &str) -> bool {
    set_master_password(password).is_ok()
}
