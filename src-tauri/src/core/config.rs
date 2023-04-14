use std::fs;
use std::path;

pub const APP_DIR: &str = ".v_pwm";

pub fn init_config() {
    let home_dir = tauri::api::path::home_dir();

    match home_dir {
        Some(dir) => {
            let app_path = path::Path::new(&dir);
            let app_path = app_path.join(APP_DIR);
            fs::create_dir_all(app_path).unwrap();
        }
        None => (),
    }
}
