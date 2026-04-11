/// Internal HSV + Alpha color representation.
/// H: 0.0–360.0 (degrees), S: 0.0–100.0 (%), V: 0.0–100.0 (%), A: 0.0–100.0 (%).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HsvColor {
    pub h: f64,
    pub s: f64,
    pub v: f64,
    pub a: f64,
}

impl Default for HsvColor {
    fn default() -> Self {
        Self {
            h: 0.0,
            s: 0.0,
            v: 0.0,
            a: 100.0,
        }
    }
}

/// Output color format for the consumer.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum ColorFormat {
    #[default]
    Hex,
    Rgb,
    Hsl,
}

/// Trigger display mode.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum TriggerMode {
    #[default]
    SwatchInput,
    SwatchOnly,
}

/// Preset color swatch item.
#[derive(Debug, Clone, PartialEq)]
pub struct SwatchItem {
    pub value: String,
    pub label: Option<String>,
}

// ---------------------------------------------------------------------------
// HSV <-> RGB
// ---------------------------------------------------------------------------

/// Convert HSV (h: 0–360, s: 0–100, v: 0–100) to RGB (0–255 each).
pub fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let s = s / 100.0;
    let v = v / 100.0;
    let c = v * s;
    let h_prime = h / 60.0;
    let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());
    let (r1, g1, b1) = if h_prime < 1.0 {
        (c, x, 0.0)
    } else if h_prime < 2.0 {
        (x, c, 0.0)
    } else if h_prime < 3.0 {
        (0.0, c, x)
    } else if h_prime < 4.0 {
        (0.0, x, c)
    } else if h_prime < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let m = v - c;
    (
        ((r1 + m) * 255.0).round() as u8,
        ((g1 + m) * 255.0).round() as u8,
        ((b1 + m) * 255.0).round() as u8,
    )
}

/// Convert RGB (0–255 each) to HSV (h: 0–360, s: 0–100, v: 0–100).
pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let r = r as f64 / 255.0;
    let g = g as f64 / 255.0;
    let b = b as f64 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };

    let s = if max == 0.0 { 0.0 } else { delta / max * 100.0 };
    let v = max * 100.0;

    (h, s, v)
}

// ---------------------------------------------------------------------------
// HSV <-> HSL (direct, no RGB intermediary)
// ---------------------------------------------------------------------------

/// Convert HSV (h: 0–360, s: 0–100, v: 0–100) to HSL (h: 0–360, s: 0–100, l: 0–100).
pub fn hsv_to_hsl(h: f64, s: f64, v: f64) -> (f64, f64, f64) {
    let s = s / 100.0;
    let v = v / 100.0;
    let l = v * (1.0 - s / 2.0);
    let s_hsl = if l == 0.0 || l == 1.0 {
        0.0
    } else {
        (v - l) / l.min(1.0 - l)
    };
    (h, s_hsl * 100.0, l * 100.0)
}

/// Convert HSL (h: 0–360, s: 0–100, l: 0–100) to HSV (h: 0–360, s: 0–100, v: 0–100).
pub fn hsl_to_hsv(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    let s = s / 100.0;
    let l = l / 100.0;
    let v = l + s * l.min(1.0 - l);
    let s_hsv = if v == 0.0 { 0.0 } else { 2.0 * (1.0 - l / v) };
    (h, s_hsv * 100.0, v * 100.0)
}

// ---------------------------------------------------------------------------
// HsvColor methods
// ---------------------------------------------------------------------------

impl HsvColor {
    pub fn new(h: f64, s: f64, v: f64, a: f64) -> Self {
        Self { h, s, v, a }
    }

    /// Parse a hex color string (#RGB, #RRGGBB, #RRGGBBAA).
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim().trim_start_matches('#');
        // Expand 3-char shorthand to 6-char
        let hex = if hex.len() == 3 {
            let mut expanded = String::with_capacity(6);
            for c in hex.chars() {
                expanded.push(c);
                expanded.push(c);
            }
            expanded
        } else {
            hex.to_string()
        };

        if hex.len() != 6 && hex.len() != 8 {
            return None;
        }

        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        let a = if hex.len() == 8 {
            u8::from_str_radix(&hex[6..8], 16).ok()? as f64 / 255.0 * 100.0
        } else {
            100.0
        };

        let (h, s, v) = rgb_to_hsv(r, g, b);
        Some(Self { h, s, v, a })
    }

    /// Parse an `rgb(r, g, b)` or `rgba(r, g, b, a)` string.
    pub fn from_rgb_str(input: &str) -> Option<Self> {
        let s = input.trim();
        let inner = if s.starts_with("rgba(") {
            s.strip_prefix("rgba(")?.strip_suffix(')')?
        } else if s.starts_with("rgb(") {
            s.strip_prefix("rgb(")?.strip_suffix(')')?
        } else {
            return None;
        };

        let parts: Vec<&str> = inner.split([',', ' ']).filter(|p| !p.is_empty()).collect();
        if parts.len() < 3 || parts.len() > 4 {
            return None;
        }

        let r: u8 = parts[0].trim().parse().ok()?;
        let g: u8 = parts[1].trim().parse().ok()?;
        let b: u8 = parts[2].trim().parse().ok()?;
        let a = if parts.len() == 4 {
            let a_val: f64 = parts[3].trim().parse().ok()?;
            if a_val <= 1.0 { a_val * 100.0 } else { a_val }
        } else {
            100.0
        };

        let (h, s, v) = rgb_to_hsv(r, g, b);
        Some(Self { h, s, v, a })
    }

    /// Parse an `hsl(h, s%, l%)` or `hsla(h, s%, l%, a)` string.
    pub fn from_hsl_str(input: &str) -> Option<Self> {
        let s = input.trim();
        let inner = if s.starts_with("hsla(") {
            s.strip_prefix("hsla(")?.strip_suffix(')')?
        } else if s.starts_with("hsl(") {
            s.strip_prefix("hsl(")?.strip_suffix(')')?
        } else {
            return None;
        };

        let parts: Vec<&str> = inner
            .split([',', ' '])
            .map(|p| p.trim_end_matches('%'))
            .filter(|p| !p.is_empty())
            .collect();
        if parts.len() < 3 || parts.len() > 4 {
            return None;
        }

        let h: f64 = parts[0].trim().parse().ok()?;
        let s_hsl: f64 = parts[1].trim().parse().ok()?;
        let l: f64 = parts[2].trim().parse().ok()?;
        let a = if parts.len() == 4 {
            let a_val: f64 = parts[3].trim().parse().ok()?;
            if a_val <= 1.0 { a_val * 100.0 } else { a_val }
        } else {
            100.0
        };

        let (h, s_hsv, v) = hsl_to_hsv(h, s_hsl, l);
        Some(Self { h, s: s_hsv, v, a })
    }

    /// Format as hex string. Includes alpha component when `alpha_enabled` and `a < 100`.
    pub fn to_hex(&self, alpha_enabled: bool) -> String {
        let (r, g, b) = hsv_to_rgb(self.h, self.s, self.v);
        if alpha_enabled && self.a < 100.0 {
            let a = (self.a / 100.0 * 255.0).round() as u8;
            format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
        } else {
            format!("#{:02X}{:02X}{:02X}", r, g, b)
        }
    }

    /// Format as `rgb(r, g, b)` or `rgba(r, g, b, a)`.
    pub fn to_rgb_string(&self, alpha_enabled: bool) -> String {
        let (r, g, b) = hsv_to_rgb(self.h, self.s, self.v);
        if alpha_enabled && self.a < 100.0 {
            let a = (self.a / 100.0 * 10.0).round() / 10.0; // one decimal place
            format!("rgba({}, {}, {}, {})", r, g, b, a)
        } else {
            format!("rgb({}, {}, {})", r, g, b)
        }
    }

    /// Format as `hsl(h, s%, l%)` or `hsla(h, s%, l%, a)`.
    pub fn to_hsl_string(&self, alpha_enabled: bool) -> String {
        let (h, s, l) = hsv_to_hsl(self.h, self.s, self.v);
        let h = h.round() as i32;
        let s = s.round() as i32;
        let l = l.round() as i32;
        if alpha_enabled && self.a < 100.0 {
            let a = (self.a / 100.0 * 10.0).round() / 10.0;
            format!("hsla({}, {}%, {}%, {})", h, s, l, a)
        } else {
            format!("hsl({}, {}%, {}%)", h, s, l)
        }
    }

    /// Master output: format to the consumer's chosen format.
    pub fn format_output(&self, format: ColorFormat, alpha_enabled: bool) -> String {
        match format {
            ColorFormat::Hex => self.to_hex(alpha_enabled),
            ColorFormat::Rgb => self.to_rgb_string(alpha_enabled),
            ColorFormat::Hsl => self.to_hsl_string(alpha_enabled),
        }
    }

    /// Get RGB components as (u8, u8, u8).
    pub fn to_rgb(&self) -> (u8, u8, u8) {
        hsv_to_rgb(self.h, self.s, self.v)
    }

    /// Get HSL components as (h, s, l) in display ranges.
    #[allow(dead_code)]
    pub fn to_hsl(&self) -> (f64, f64, f64) {
        hsv_to_hsl(self.h, self.s, self.v)
    }
}

/// Parse any supported color string into HsvColor.
/// Tries hex first, then rgb, then hsl.
pub fn parse_color_value(value: &str) -> Option<HsvColor> {
    HsvColor::from_hex(value)
        .or_else(|| HsvColor::from_rgb_str(value))
        .or_else(|| HsvColor::from_hsl_str(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        let colors = ["#000000", "#FFFFFF", "#2563EB", "#FF0000", "#00FF00", "#0000FF"];
        for hex in colors {
            let hsv = HsvColor::from_hex(hex).unwrap();
            let out = hsv.to_hex(false);
            assert_eq!(hex, out, "roundtrip failed for {}", hex);
        }
    }

    #[test]
    fn hex_with_alpha_roundtrip() {
        let hsv = HsvColor::from_hex("#2563EBCC").unwrap();
        assert!((hsv.a - 80.0).abs() < 1.0);
        let out = hsv.to_hex(true);
        assert_eq!(out, "#2563EBCC");
    }

    #[test]
    fn rgb_str_parse() {
        let hsv = HsvColor::from_rgb_str("rgb(37, 99, 235)").unwrap();
        let (r, g, b) = hsv.to_rgb();
        assert_eq!((r, g, b), (37, 99, 235));
    }

    #[test]
    fn rgba_str_parse() {
        let hsv = HsvColor::from_rgb_str("rgba(37, 99, 235, 0.8)").unwrap();
        assert!((hsv.a - 80.0).abs() < 1.0);
    }

    #[test]
    fn hsl_str_parse() {
        let hsv = HsvColor::from_hsl_str("hsl(221, 83%, 53%)").unwrap();
        let out = hsv.to_hsl_string(false);
        assert_eq!(out, "hsl(221, 83%, 53%)");
    }

    #[test]
    fn hsv_to_rgb_black() {
        assert_eq!(hsv_to_rgb(0.0, 0.0, 0.0), (0, 0, 0));
    }

    #[test]
    fn hsv_to_rgb_white() {
        assert_eq!(hsv_to_rgb(0.0, 0.0, 100.0), (255, 255, 255));
    }

    #[test]
    fn hsv_to_rgb_red() {
        assert_eq!(hsv_to_rgb(0.0, 100.0, 100.0), (255, 0, 0));
    }

    #[test]
    fn rgb_to_hsv_roundtrip() {
        let (h, s, v) = rgb_to_hsv(37, 99, 235);
        let (r, g, b) = hsv_to_rgb(h, s, v);
        assert_eq!((r, g, b), (37, 99, 235));
    }

    #[test]
    fn hsl_roundtrip() {
        let (h, s, l) = hsv_to_hsl(221.0, 84.0, 92.0);
        let (h2, s2, v2) = hsl_to_hsv(h, s, l);
        assert!((h2 - 221.0).abs() < 0.01);
        assert!((s2 - 84.0).abs() < 0.01);
        assert!((v2 - 92.0).abs() < 0.01);
    }

    #[test]
    fn parse_color_value_auto() {
        assert!(parse_color_value("#FF0000").is_some());
        assert!(parse_color_value("rgb(255, 0, 0)").is_some());
        assert!(parse_color_value("hsl(0, 100%, 50%)").is_some());
        assert!(parse_color_value("not-a-color").is_none());
    }

    #[test]
    fn shorthand_hex() {
        let hsv = HsvColor::from_hex("#F00").unwrap();
        assert_eq!(hsv.to_hex(false), "#FF0000");
    }
}
