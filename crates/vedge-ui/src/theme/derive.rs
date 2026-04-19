use super::color_group::derive_color_group;
use super::color_space::{ColorError, hex_to_oklch, hex_to_srgb, oklch_to_hex};
use super::contrast::{contrast_ratio, relative_luminance};
use super::mix::oklch_mix;
use super::shadows::compute_shadows;
use super::types::ThemeConfig;
use super::types::ThemeTokens;

// Surface mix ratios (background → foreground)
const SURFACE_1_RATIO: f32 = 0.04;
const SURFACE_2_RATIO: f32 = 0.08;
const SURFACE_3_RATIO: f32 = 0.14;
const SURFACE_4_RATIO: f32 = 0.22;
const BORDER_RATIO: f32 = 0.12;
const BORDER_STRONG_RATIO: f32 = 0.24;

// Text wash ratios (foreground → background)
const TEXT_SECONDARY_WASH: f32 = 0.30;
const TEXT_TERTIARY_WASH: f32 = 0.55;

// Focus ring minimum contrast against background
const FOCUS_RING_MIN_CONTRAST: f32 = 3.0;

/// Derive all CSS token values from a `ThemeConfig`.
///
/// This is the main entry point for the color system. It parses the 3 required
/// inputs (+ optional semantic overrides) to OKLCH, then computes all surfaces,
/// text tiers, color groups, shadows, and the focus ring.
pub fn derive_tokens(config: &ThemeConfig) -> Result<ThemeTokens, ColorError> {
    let bg = hex_to_oklch(&config.background)?;
    let fg = hex_to_oklch(&config.foreground)?;
    let primary = hex_to_oklch(&config.primary)?;
    let danger = hex_to_oklch(config.danger_or_default())?;
    let warning = hex_to_oklch(config.warning_or_default())?;
    let success = hex_to_oklch(config.success_or_default())?;

    // --- Surfaces ---
    let surface_1 = oklch_mix(bg, fg, SURFACE_1_RATIO);
    let surface_2 = oklch_mix(bg, fg, SURFACE_2_RATIO);
    let surface_3 = oklch_mix(bg, fg, SURFACE_3_RATIO);
    let surface_4 = oklch_mix(bg, fg, SURFACE_4_RATIO);
    let border = oklch_mix(bg, fg, BORDER_RATIO);
    let border_strong = oklch_mix(bg, fg, BORDER_STRONG_RATIO);

    // --- Text tiers ---
    let text_secondary = oklch_mix(fg, bg, TEXT_SECONDARY_WASH);
    let text_tertiary = oklch_mix(fg, bg, TEXT_TERTIARY_WASH);

    // --- Color groups (unified derivation) ---
    let primary_group = derive_color_group(primary, bg)?;
    let danger_group = derive_color_group(danger, bg)?;
    let warning_group = derive_color_group(warning, bg)?;
    let success_group = derive_color_group(success, bg)?;

    // --- Focus ring ---
    let mut focus_ring_hex = primary_group.base.clone();
    let bg_lum = relative_luminance(hex_to_srgb(&config.background)?);
    let focus_lum = relative_luminance(hex_to_srgb(&focus_ring_hex)?);
    if contrast_ratio(focus_lum, bg_lum) < FOCUS_RING_MIN_CONTRAST {
        focus_ring_hex = adjust_for_minimum_contrast(
            &focus_ring_hex,
            &config.background,
            FOCUS_RING_MIN_CONTRAST,
        )?;
    }

    // --- Shadows ---
    let shadows = compute_shadows(bg_lum);

    Ok(ThemeTokens {
        color_background: oklch_to_hex(bg),
        color_surface_1: oklch_to_hex(surface_1),
        color_surface_2: oklch_to_hex(surface_2),
        color_surface_3: oklch_to_hex(surface_3),
        color_surface_4: oklch_to_hex(surface_4),
        color_border: oklch_to_hex(border),
        color_border_strong: oklch_to_hex(border_strong),

        color_text_primary: oklch_to_hex(fg),
        color_text_secondary: oklch_to_hex(text_secondary),
        color_text_tertiary: oklch_to_hex(text_tertiary),

        color_primary: primary_group.base,
        color_primary_hover: primary_group.hover,
        color_primary_muted: primary_group.muted,
        color_primary_foreground: primary_group.foreground,
        color_primary_text: primary_group.text,

        color_danger: danger_group.base,
        color_danger_hover: danger_group.hover,
        color_danger_muted: danger_group.muted,
        color_danger_foreground: danger_group.foreground,
        color_danger_text: danger_group.text,

        color_warning: warning_group.base,
        color_warning_hover: warning_group.hover,
        color_warning_muted: warning_group.muted,
        color_warning_foreground: warning_group.foreground,
        color_warning_text: warning_group.text,

        color_success: success_group.base,
        color_success_hover: success_group.hover,
        color_success_muted: success_group.muted,
        color_success_foreground: success_group.foreground,
        color_success_text: success_group.text,

        color_focus_ring: focus_ring_hex,

        shadow_sm: shadows.shadow_sm,
        shadow_md: shadows.shadow_md,
        shadow_lg: shadows.shadow_lg,
        shadow_scrim: shadows.shadow_scrim,
    })
}

/// Binary-search the lightness of `color_hex` until it achieves at least
/// `min_contrast` against `surface_hex`.
fn adjust_for_minimum_contrast(
    color_hex: &str,
    surface_hex: &str,
    min_contrast: f32,
) -> Result<String, ColorError> {
    let surface_lum = relative_luminance(hex_to_srgb(surface_hex)?);
    let mut oklch = hex_to_oklch(color_hex)?;

    // Determine direction: if surface is light, darken the color; otherwise lighten
    let surface_is_light = surface_lum > 0.5;

    let mut low = 0.0f32;
    let mut high = 1.0f32;

    for _ in 0..16 {
        let mid = (low + high) / 2.0;
        let adjusted = if surface_is_light {
            // Darken: reduce L
            super::color_space::Oklch {
                l: oklch.l - mid * oklch.l,
                ..oklch
            }
        } else {
            // Lighten: increase L toward 1.0
            super::color_space::Oklch {
                l: oklch.l + mid * (1.0 - oklch.l),
                ..oklch
            }
        };
        let hex = oklch_to_hex(adjusted);
        let lum = relative_luminance(hex_to_srgb(&hex)?);
        if contrast_ratio(lum, surface_lum) >= min_contrast {
            high = mid;
        } else {
            low = mid;
        }
    }

    let final_l = if surface_is_light {
        oklch.l - high * oklch.l
    } else {
        oklch.l + high * (1.0 - oklch.l)
    };
    oklch.l = final_l.clamp(0.0, 1.0);

    Ok(oklch_to_hex(oklch))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn light_preset() -> ThemeConfig {
        ThemeConfig {
            background: "#FFFFFF".into(),
            foreground: "#111827".into(),
            primary: "#2563EB".into(),
            danger: None,
            warning: None,
            success: None,
        }
    }

    fn dark_preset() -> ThemeConfig {
        ThemeConfig {
            background: "#09090B".into(),
            foreground: "#FAFAFA".into(),
            primary: "#3B82F6".into(),
            danger: None,
            warning: None,
            success: None,
        }
    }

    #[test]
    fn light_theme_derives_successfully() {
        let tokens = derive_tokens(&light_preset()).unwrap();
        assert_eq!(tokens.color_background, "#FFFFFF");
        assert_eq!(tokens.color_primary_foreground, "#FFFFFF");
        // Surfaces should be slightly off-white
        let s1 = hex_to_oklch(&tokens.color_surface_1).unwrap();
        assert!(s1.l > 0.95 && s1.l < 1.0);
    }

    #[test]
    fn dark_theme_derives_successfully() {
        let tokens = derive_tokens(&dark_preset()).unwrap();
        // Background should be near-black (OKLCH L is perceptual, ~0.16 for #09090B)
        let bg = hex_to_oklch(&tokens.color_background).unwrap();
        assert!(bg.l < 0.2, "bg L={} should be < 0.2", bg.l);
        // Shadows should be glow borders
        assert_eq!(tokens.shadow_sm, "none");
    }

    #[test]
    fn custom_danger_override() {
        let config = ThemeConfig {
            danger: Some("#FF0000".into()),
            ..light_preset()
        };
        let tokens = derive_tokens(&config).unwrap();
        // Should use the override, not the default
        let danger = hex_to_oklch(&tokens.color_danger).unwrap();
        let default_danger = hex_to_oklch("#DC2626").unwrap();
        // They should be different
        assert!(
            (danger.h - default_danger.h).abs() > 1.0 || (danger.c - default_danger.c).abs() > 0.01,
            "custom danger should differ from default"
        );
    }

    #[test]
    fn focus_ring_has_minimum_contrast() {
        let tokens = derive_tokens(&light_preset()).unwrap();
        let bg_lum = relative_luminance(hex_to_srgb(&tokens.color_background).unwrap());
        let fr_lum = relative_luminance(hex_to_srgb(&tokens.color_focus_ring).unwrap());
        let cr = contrast_ratio(bg_lum, fr_lum);
        assert!(cr >= 3.0, "focus ring contrast {cr:.2} should be >= 3.0");
    }
}
