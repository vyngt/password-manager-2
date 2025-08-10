use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_hex(hex: &str) -> Self {
        let hex = hex.trim_start_matches('#');
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
        Self { r, g, b }
    }

    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    pub fn calculate_white_black_text_color(&self, alpha: Option<f32>) -> RgbColor {
        // WCAG 2.1 relative luminance and contrast ratio
        fn srgb_to_linear(c: f32) -> f32 {
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }

        // Treat input as background color with optional alpha, composited over black by default
        let a = alpha.unwrap_or(1.0).clamp(0.0, 1.0);

        // Composite in sRGB space against black (bg = 0.0)
        let r_srgb = a * (self.r as f32 / 255.0);
        let g_srgb = a * (self.g as f32 / 255.0);
        let b_srgb = a * (self.b as f32 / 255.0);

        let r_lin = srgb_to_linear(r_srgb);
        let g_lin = srgb_to_linear(g_srgb);
        let b_lin = srgb_to_linear(b_srgb);
        let l = 0.2126 * r_lin + 0.7152 * g_lin + 0.0722 * b_lin;

        let contrast_with_white = (1.0 + 0.05) / (l + 0.05);
        let contrast_with_black = (l + 0.05) / 0.05;

        if contrast_with_white >= contrast_with_black {
            RgbColor::new(255, 255, 255)
        } else {
            RgbColor::new(0, 0, 0)
        }
    }
}
