use super::registry::Registry;
use crate::config::settings::Settings;

pub struct AppState {
    pub registry: Registry,
    pub settings: Settings,
}
