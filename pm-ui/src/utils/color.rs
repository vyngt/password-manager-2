use crate::constants::Color;
use crate::types::color::RgbColor;

pub fn get_css_var_color(color: &Color) -> RgbColor {
    fn parse_rgb_value(value: &str) -> Option<RgbColor> {
        let s = value.trim().to_lowercase();

        // Extract inside parentheses if present, e.g. rgb(255 255 255)
        let inner = if let (Some(open), Some(close)) = (s.find('('), s.rfind(')')) {
            &s[open + 1..close]
        } else {
            s.as_str()
        };

        // Normalize separators: commas and slashes to spaces
        let normalized = inner.replace(',', " ").replace('/', " ");

        let mut nums = normalized
            .split_whitespace()
            .filter_map(|t| t.parse::<u16>().ok())
            .collect::<Vec<_>>();

        if nums.len() < 3 {
            return None;
        }

        // Clamp to 0..=255 and take first three
        nums.truncate(3);
        let r = nums[0].min(255) as u8;
        let g = nums[1].min(255) as u8;
        let b = nums[2].min(255) as u8;
        Some(RgbColor { r, g, b })
    }

    fn load_css_var(var: &str, default: RgbColor) -> RgbColor {
        use web_sys::window;

        if let Some(win) = window() {
            if let Some(doc) = win.document() {
                if let Some(root) = doc.document_element() {
                    if let Ok(Some(style)) = win.get_computed_style(&root) {
                        if let Ok(value) = style.get_property_value(var) {
                            if !value.trim().is_empty() {
                                if let Some(rgb) = parse_rgb_value(&value) {
                                    return rgb;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Non-web targets or any failure: fall back to provided default
        default
    }

    match color {
        Color::Primary => load_css_var(
            "--color-primary",
            RgbColor {
                r: 244,
                g: 31,
                b: 198,
            },
        ),
        Color::Secondary => load_css_var(
            "--color-secondary",
            RgbColor {
                r: 3,
                g: 169,
                b: 244,
            },
        ),
        Color::Success => load_css_var(
            "--color-success",
            RgbColor {
                r: 30,
                g: 200,
                b: 135,
            },
        ),
        Color::Danger => load_css_var(
            "--color-danger",
            RgbColor {
                r: 244,
                g: 37,
                b: 99,
            },
        ),
        Color::Warning => load_css_var(
            "--color-warning",
            RgbColor {
                r: 224,
                g: 193,
                b: 36,
            },
        ),
        Color::Background => load_css_var("--color-background", RgbColor { r: 0, g: 0, b: 0 }),
        Color::Foreground => load_css_var(
            "--color-foreground",
            RgbColor {
                r: 255,
                g: 255,
                b: 255,
            },
        ),
    }
}
