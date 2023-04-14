use std::{fs, path};

use super::config;

pub fn is_first_time() -> bool {
    let home_dir = config::get_home_dir();

    match home_dir {
        Some(dir) => {
            let profile_path = path::Path::new(&dir);
            let profile_path = profile_path.join(config::APP_DIR).join(config::PROFILE);
            let profile_path = profile_path.to_str().unwrap();

            let contents = fs::read_to_string(&profile_path);

            match contents {
                Ok(contents) => return contents.trim().len() != 0,
                Err(_) => {}
            }
        }
        None => {}
    }

    true
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

pub fn register(password: &str) {}
