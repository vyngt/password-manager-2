use std::fs;
use std::path;
use std::path::PathBuf;

pub const APP_DIR: &str = ".v_pwm";
pub const PROFILE: &str = "profile";
pub const DATABASE: &str = "vault.db";

pub fn get_home_dir() -> Option<PathBuf> {
    tauri::api::path::home_dir()
}

pub fn init_config() {
    let home_dir = get_home_dir();

    match home_dir {
        Some(dir) => {
            let app_path = path::Path::new(&dir);
            let app_path = app_path.join(APP_DIR);
            fs::create_dir_all(app_path).unwrap();
        }
        None => (),
    }
}
