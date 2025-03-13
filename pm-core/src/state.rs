use super::v_vortex::vortex::Vortex;
use crate::db;
use std::{path::PathBuf, sync::Mutex};

pub struct AppVortexContainer {
    pub vortex: Vortex,
}

pub struct AppVortex(pub Mutex<AppVortexContainer>);

impl AppVortex {
    pub fn new(home_dir: &PathBuf) -> Self {
        let config = db::initialize_db_connections_config(home_dir);
        if let Ok(vortex) = Vortex::new(config) {
            return Self(Mutex::new(AppVortexContainer { vortex }));
        }

        panic!("Failed to create Vortex");
    }
}
