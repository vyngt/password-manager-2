use std::fs;
use std::path::PathBuf;

use tauri::{App, Manager};

pub const APP_DIR: &str = ".v_pwm";
pub const VAULT_DATA: &str = "vault.dat";
pub const VAULT_KEY: &str = "key";
pub const THEME_DATA: &str = "theme.dat";

pub const DEV_APP_DIR: &str = "local";
pub const DEV_VAULT_DATA: &str = "vault_dev.dat";
pub const DEV_THEME_DATA: &str = "theme_dev.dat";
pub const DEV_VAULT_KEY: &str = "key_dev";

pub struct Settings {
    home_dir: PathBuf,
    app_dir: String,
    vault_data: String,
    vault_key: String,
    theme_data: String,
}

fn get_home_dir(app: &mut App) -> Option<PathBuf> {
    if cfg!(debug_assertions) {
        return None;
    }

    match app.path().home_dir() {
        Err(_) => None,
        Ok(dir) => Some(dir),
    }
}

impl Settings {
    // Load env

    fn from_dev() -> Self {
        Settings {
            app_dir: DEV_APP_DIR.to_string(),
            vault_data: DEV_VAULT_DATA.to_string(),
            vault_key: DEV_VAULT_KEY.to_string(),
            theme_data: DEV_THEME_DATA.to_string(),
            home_dir: PathBuf::new(),
        }
    }

    fn from_prod(app: &mut App) -> Self {
        Settings {
            app_dir: APP_DIR.to_string(),
            vault_data: VAULT_DATA.to_string(),
            vault_key: VAULT_KEY.to_string(),
            theme_data: THEME_DATA.to_string(),
            home_dir: get_home_dir(app).unwrap(),
        }
    }

    pub fn from_env(app: &mut App) -> Self {
        if cfg!(debug_assertions) {
            Self::from_dev()
        } else {
            Self::from_prod(app)
        }
    }

    // APIs

    pub fn get_app_dir(&self) -> PathBuf {
        let home_dir = self.home_dir.clone();
        home_dir.join(&self.app_dir)
    }

    pub fn get_vault_path(&self) -> PathBuf {
        let app_dir = self.get_app_dir();
        app_dir.join(&self.vault_data)
    }

    pub fn get_vault_key_path(&self) -> PathBuf {
        let app_dir = self.get_app_dir();
        app_dir.join(&self.vault_key)
    }

    pub fn get_theme_path(&self) -> PathBuf {
        let app_dir = self.get_app_dir();
        app_dir.join(&self.theme_data)
    }

    pub fn init_home_dir(&self) {
        let app_dir = self.get_app_dir();
        fs::create_dir_all(app_dir).unwrap();
    }
}
