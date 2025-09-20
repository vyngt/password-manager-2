use std::fs;
use std::path;
use std::path::PathBuf;

use tauri::App;
use tauri::Manager;

pub const APP_DIR: &str = ".v_pwm";
pub const CORE_DATA: &str = "vault.dat";
pub const THEME_DATA: &str = "theme.dat";

pub fn get_home_dir(app: &mut App) -> Option<PathBuf> {
    match app.path().home_dir() {
        Err(_) => None,
        Ok(dir) => Some(dir),
    }
}

pub fn init_config(app: &mut App) {
    let home_dir = get_home_dir(app);

    match home_dir {
        Some(dir) => {
            let app_path = path::Path::new(&dir);
            let app_path = app_path.join(APP_DIR);
            fs::create_dir_all(app_path).unwrap();
        }
        None => (),
    }
}
