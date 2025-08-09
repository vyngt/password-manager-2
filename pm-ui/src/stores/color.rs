use crate::constants::Color;
use crate::types::color::RgbColor;
use crate::utils::color::get_css_var_color;
use reactive_stores::Store;
use serde::{Deserialize, Serialize};

#[derive(Debug, Store, Serialize, Deserialize)]
pub struct ColorStore {
    primary: RgbColor,
    secondary: RgbColor,
    success: RgbColor,
    danger: RgbColor,
    warning: RgbColor,
    background: RgbColor,
    foreground: RgbColor,
}

impl ColorStore {
    pub fn new() -> Self {
        Self {
            primary: get_css_var_color(&Color::Primary),
            secondary: get_css_var_color(&Color::Secondary),
            success: get_css_var_color(&Color::Success),
            danger: get_css_var_color(&Color::Danger),
            warning: get_css_var_color(&Color::Warning),
            background: get_css_var_color(&Color::Background),
            foreground: get_css_var_color(&Color::Foreground),
        }
    }
}
