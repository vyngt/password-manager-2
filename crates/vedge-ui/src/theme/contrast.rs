use super::color_space::{ColorError, Srgb, hex_to_srgb, srgb_to_linear};

/// WCAG 2.1 relative luminance from sRGB `[0,1]` channels.
///
/// Applies gamma decode (linearization) then weights:
/// `L = 0.2126·R + 0.7152·G + 0.0722·B`
pub fn relative_luminance(c: Srgb) -> f32 {
    let lin = srgb_to_linear(c);
    0.2126 * lin.r + 0.7152 * lin.g + 0.0722 * lin.b
}

/// WCAG 2.1 contrast ratio between two relative luminance values.
///
/// Returns a value in `[1.0, 21.0]`.
pub fn contrast_ratio(l1: f32, l2: f32) -> f32 {
    let lighter = l1.max(l2);
    let darker = l1.min(l2);
    (lighter + 0.05) / (darker + 0.05)
}

/// Contrast ratio between two hex colors.
pub fn contrast_ratio_hex(hex_a: &str, hex_b: &str) -> Result<f32, ColorError> {
    let la = relative_luminance(hex_to_srgb(hex_a)?);
    let lb = relative_luminance(hex_to_srgb(hex_b)?);
    Ok(contrast_ratio(la, lb))
}

/// Auto-select white (`#FFFFFF`) or near-black (`#111827`) foreground
/// for text on the given background color, based on which achieves
/// higher contrast.
///
/// Near-black `#111827` is used instead of pure black because it's softer
/// and tonally compatible with the text system.
pub fn compute_primary_foreground(background_hex: &str) -> Result<&'static str, ColorError> {
    let bg_lum = relative_luminance(hex_to_srgb(background_hex)?);

    let white_contrast = (1.0 + 0.05) / (bg_lum + 0.05);
    // #111827 has luminance ≈ 0.012
    let near_black_lum = relative_luminance(Srgb {
        r: 0x11 as f32 / 255.0,
        g: 0x18 as f32 / 255.0,
        b: 0x27 as f32 / 255.0,
    });
    let black_contrast = (bg_lum + 0.05) / (near_black_lum + 0.05);

    if white_contrast >= black_contrast {
        Ok("#FFFFFF")
    } else {
        Ok("#111827")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn luminance_white() {
        let l = relative_luminance(Srgb {
            r: 1.0,
            g: 1.0,
            b: 1.0,
        });
        assert!(approx_eq(l, 1.0, 0.001));
    }

    #[test]
    fn luminance_black() {
        let l = relative_luminance(Srgb {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        });
        assert!(approx_eq(l, 0.0, 0.001));
    }

    #[test]
    fn contrast_black_white() {
        assert!(approx_eq(contrast_ratio(1.0, 0.0), 21.0, 0.01));
    }

    #[test]
    fn contrast_same() {
        assert!(approx_eq(contrast_ratio(0.5, 0.5), 1.0, 0.01));
    }

    #[test]
    fn foreground_blue_gets_white() {
        // Blue #2563EB is dark enough that white text wins
        assert_eq!(compute_primary_foreground("#2563EB").unwrap(), "#FFFFFF");
    }

    #[test]
    fn foreground_yellow_gets_black() {
        // Yellow #FFFF00 is very bright — dark text wins
        assert_eq!(compute_primary_foreground("#FFFF00").unwrap(), "#111827");
    }

    #[test]
    fn foreground_white_bg_gets_black() {
        assert_eq!(compute_primary_foreground("#FFFFFF").unwrap(), "#111827");
    }

    #[test]
    fn foreground_black_bg_gets_white() {
        assert_eq!(compute_primary_foreground("#000000").unwrap(), "#FFFFFF");
    }

    #[test]
    fn contrast_ratio_hex_works() {
        let cr = contrast_ratio_hex("#FFFFFF", "#000000").unwrap();
        assert!(approx_eq(cr, 21.0, 0.01));
    }
}
