use super::color_space::{ColorError, Oklch, oklch_to_hex};
use super::contrast::compute_primary_foreground;
use super::scale::generate_color_scale;

/// A 5-token color group derived from a single base color.
///
/// Used identically for primary, danger, warning, and success.
/// The only difference between groups is the input base color.
#[derive(Debug, Clone)]
pub struct ColorGroup {
    pub base: String,
    pub hover: String,
    pub muted: String,
    pub foreground: String,
    pub text: String,
}

/// Derive a 5-token color group from a base color and the theme's background.
///
/// Internally generates a full 10-step scale, then extracts:
/// - `base`       = scale[6] (600 — the input color)
/// - `hover`      = scale[7] (700 — one shade darker)
/// - `muted`      = scale[0] (50  — lightest tint)
/// - `foreground` = auto white/black for text on `base`
/// - `text`       = scale[8] (800 — dark enough for readability on neutral surfaces)
pub fn derive_color_group(base: Oklch, background: Oklch) -> Result<ColorGroup, ColorError> {
    let scale = generate_color_scale(base, background);

    let base_hex = oklch_to_hex(scale[6]);
    let foreground = compute_primary_foreground(&base_hex)?;

    Ok(ColorGroup {
        base: base_hex,
        hover: oklch_to_hex(scale[7]),
        muted: oklch_to_hex(scale[0]),
        foreground: foreground.to_owned(),
        text: oklch_to_hex(scale[8]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::color_space::hex_to_oklch;

    #[test]
    fn primary_group_blue() {
        let base = hex_to_oklch("#2563EB").unwrap();
        let bg = hex_to_oklch("#FFFFFF").unwrap();
        let group = derive_color_group(base, bg).unwrap();

        // Blue primary should get white foreground
        assert_eq!(group.foreground, "#FFFFFF");
        // Muted should be very light (close to white)
        let muted = hex_to_oklch(&group.muted).unwrap();
        assert!(muted.l > 0.9, "muted L={} should be > 0.9", muted.l);
        // Hover should be darker than base
        let hover = hex_to_oklch(&group.hover).unwrap();
        assert!(hover.l < base.l, "hover should be darker than base");
    }

    #[test]
    fn danger_group() {
        let base = hex_to_oklch("#DC2626").unwrap();
        let bg = hex_to_oklch("#FFFFFF").unwrap();
        let group = derive_color_group(base, bg).unwrap();
        // Red should get white foreground
        assert_eq!(group.foreground, "#FFFFFF");
    }

    #[test]
    fn yellow_group_gets_dark_foreground() {
        let base = hex_to_oklch("#EAB308").unwrap();
        let bg = hex_to_oklch("#FFFFFF").unwrap();
        let group = derive_color_group(base, bg).unwrap();
        // Bright yellow should get dark foreground
        assert_eq!(group.foreground, "#111827");
    }
}
