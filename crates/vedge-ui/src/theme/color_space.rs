use std::fmt;

/// Error returned when a hex color string cannot be parsed.
#[derive(Debug, Clone)]
pub enum ColorError {
    InvalidHex(String),
}

impl fmt::Display for ColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColorError::InvalidHex(s) => write!(f, "invalid hex color: {s}"),
        }
    }
}

impl std::error::Error for ColorError {}

// ---------------------------------------------------------------------------
// Color types
// ---------------------------------------------------------------------------

/// sRGB color with channels in `[0.0, 1.0]` (gamma-compressed).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Srgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

/// Linear-light sRGB (gamma decoded).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinSrgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

/// OKLCH polar color (perceptually uniform).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oklch {
    /// Lightness `0.0..1.0`
    pub l: f32,
    /// Chroma `0.0..~0.4`
    pub c: f32,
    /// Hue `0.0..360.0` (degrees). Undefined when `c ≈ 0`.
    pub h: f32,
}

// ---------------------------------------------------------------------------
// Hex ↔ sRGB
// ---------------------------------------------------------------------------

pub fn hex_to_srgb(hex: &str) -> Result<Srgb, ColorError> {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return Err(ColorError::InvalidHex(format!("#{hex}")));
    }
    let r = u8::from_str_radix(&hex[0..2], 16)
        .map_err(|_| ColorError::InvalidHex(format!("#{hex}")))?;
    let g = u8::from_str_radix(&hex[2..4], 16)
        .map_err(|_| ColorError::InvalidHex(format!("#{hex}")))?;
    let b = u8::from_str_radix(&hex[4..6], 16)
        .map_err(|_| ColorError::InvalidHex(format!("#{hex}")))?;
    Ok(Srgb {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
    })
}

pub fn srgb_to_hex(c: Srgb) -> String {
    let r = (c.r.clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (c.g.clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (c.b.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{r:02X}{g:02X}{b:02X}")
}

// ---------------------------------------------------------------------------
// sRGB ↔ Linear sRGB (gamma decode / encode)
// ---------------------------------------------------------------------------

fn gamma_decode(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn gamma_encode(c: f32) -> f32 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

pub fn srgb_to_linear(c: Srgb) -> LinSrgb {
    LinSrgb {
        r: gamma_decode(c.r),
        g: gamma_decode(c.g),
        b: gamma_decode(c.b),
    }
}

pub fn linear_to_srgb(c: LinSrgb) -> Srgb {
    Srgb {
        r: gamma_encode(c.r.clamp(0.0, f32::MAX)),
        g: gamma_encode(c.g.clamp(0.0, f32::MAX)),
        b: gamma_encode(c.b.clamp(0.0, f32::MAX)),
    }
}

// ---------------------------------------------------------------------------
// Linear sRGB ↔ OKLAB (via LMS intermediate)
// ---------------------------------------------------------------------------

/// Returns `[L, a, b]` in OKLAB.
pub fn linear_to_oklab(c: LinSrgb) -> [f32; 3] {
    // Step 1: linear sRGB → LMS (cone response)
    let l = 0.4122214708 * c.r + 0.5363325363 * c.g + 0.0514459929 * c.b;
    let m = 0.2119034982 * c.r + 0.6806995451 * c.g + 0.1073969566 * c.b;
    let s = 0.0883024619 * c.r + 0.2817188376 * c.g + 0.6299787005 * c.b;

    // Step 2: cube root (perceptual nonlinearity)
    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    // Step 3: LMS' → OKLAB
    let lab_l = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
    let lab_a = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
    let lab_b = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;

    [lab_l, lab_a, lab_b]
}

/// OKLAB `[L, a, b]` → linear sRGB.
pub fn oklab_to_linear(lab: [f32; 3]) -> LinSrgb {
    let [lab_l, lab_a, lab_b] = lab;

    // OKLAB → LMS' (inverse of step 3)
    let l_ = lab_l + 0.3963377774 * lab_a + 0.2158037573 * lab_b;
    let m_ = lab_l - 0.1055613458 * lab_a - 0.0638541728 * lab_b;
    let s_ = lab_l - 0.0894841775 * lab_a - 1.2914855480 * lab_b;

    // LMS' → LMS (cube, inverse of step 2)
    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    // LMS → linear sRGB (inverse of step 1)
    let r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
    let g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
    let b = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;

    LinSrgb { r, g, b }
}

// ---------------------------------------------------------------------------
// OKLAB ↔ OKLCH (Cartesian ↔ Polar)
// ---------------------------------------------------------------------------

pub const ACHROMATIC_THRESHOLD: f32 = 1e-4;

pub fn oklab_to_oklch(lab: [f32; 3]) -> Oklch {
    let [l, a, b] = lab;
    let c = (a * a + b * b).sqrt();
    let h = if c < ACHROMATIC_THRESHOLD {
        0.0 // hue is undefined for achromatic colors
    } else {
        let h = b.atan2(a).to_degrees();
        if h < 0.0 { h + 360.0 } else { h }
    };
    Oklch { l, c, h }
}

pub fn oklch_to_oklab(c: Oklch) -> [f32; 3] {
    let h_rad = c.h.to_radians();
    [c.l, c.c * h_rad.cos(), c.c * h_rad.sin()]
}

// ---------------------------------------------------------------------------
// Convenience: hex ↔ OKLCH (full pipeline)
// ---------------------------------------------------------------------------

pub fn hex_to_oklch(hex: &str) -> Result<Oklch, ColorError> {
    let srgb = hex_to_srgb(hex)?;
    let lin = srgb_to_linear(srgb);
    let lab = linear_to_oklab(lin);
    Ok(oklab_to_oklch(lab))
}

pub fn oklch_to_hex(c: Oklch) -> String {
    let lab = oklch_to_oklab(c);
    let lin = oklab_to_linear(lab);
    let srgb = linear_to_srgb(lin);
    srgb_to_hex(Srgb {
        r: srgb.r.clamp(0.0, 1.0),
        g: srgb.g.clamp(0.0, 1.0),
        b: srgb.b.clamp(0.0, 1.0),
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn hex_roundtrip_white() {
        let hex = "#FFFFFF";
        let oklch = hex_to_oklch(hex).unwrap();
        assert!(approx_eq(oklch.l, 1.0, 0.01));
        assert!(oklch.c < 0.01); // achromatic
        let back = oklch_to_hex(oklch);
        assert_eq!(back, "#FFFFFF");
    }

    #[test]
    fn hex_roundtrip_black() {
        let hex = "#000000";
        let oklch = hex_to_oklch(hex).unwrap();
        assert!(approx_eq(oklch.l, 0.0, 0.01));
        assert!(oklch.c < 0.01);
        let back = oklch_to_hex(oklch);
        assert_eq!(back, "#000000");
    }

    #[test]
    fn hex_roundtrip_blue() {
        let hex = "#2563EB";
        let result = oklch_to_hex(hex_to_oklch(hex).unwrap());
        // Allow +/- 1 per channel
        let orig = hex_to_srgb(hex).unwrap();
        let back = hex_to_srgb(&result).unwrap();
        assert!(approx_eq(orig.r, back.r, 2.0 / 255.0));
        assert!(approx_eq(orig.g, back.g, 2.0 / 255.0));
        assert!(approx_eq(orig.b, back.b, 2.0 / 255.0));
    }

    #[test]
    fn hex_roundtrip_red() {
        let hex = "#DC2626";
        let result = oklch_to_hex(hex_to_oklch(hex).unwrap());
        let orig = hex_to_srgb(hex).unwrap();
        let back = hex_to_srgb(&result).unwrap();
        assert!(approx_eq(orig.r, back.r, 2.0 / 255.0));
        assert!(approx_eq(orig.g, back.g, 2.0 / 255.0));
        assert!(approx_eq(orig.b, back.b, 2.0 / 255.0));
    }

    #[test]
    fn invalid_hex() {
        assert!(hex_to_srgb("not-a-color").is_err());
        assert!(hex_to_srgb("#GG0000").is_err());
        assert!(hex_to_srgb("#FFF").is_err()); // 3-char not supported
    }

    #[test]
    fn near_black_anchor() {
        // #0C0C0C should have L > 0 and near-zero chroma
        let oklch = hex_to_oklch("#0C0C0C").unwrap();
        assert!(oklch.l > 0.0);
        assert!(oklch.c < 0.01);
    }

    #[test]
    fn gamma_roundtrip() {
        for val in [0.0f32, 0.04, 0.04045, 0.5, 1.0] {
            let decoded = gamma_decode(val);
            let encoded = gamma_encode(decoded);
            assert!(approx_eq(val, encoded, 1e-5), "failed at {val}");
        }
    }
}
