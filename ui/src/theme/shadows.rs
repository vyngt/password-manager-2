/// Computed shadow tokens as CSS value strings.
pub struct ShadowTokens {
    pub shadow_sm: String,
    pub shadow_md: String,
    pub shadow_lg: String,
    pub shadow_scrim: String,
}

/// Compute shadow tokens based on background luminance.
///
/// - Light backgrounds (luminance > 0.18): traditional drop shadows with
///   opacity scaled by background brightness.
/// - Dark backgrounds (luminance <= 0.18): subtle glow borders using
///   `1px` white rings instead of drop shadows.
pub fn compute_shadows(bg_luminance: f32) -> ShadowTokens {
    if bg_luminance > 0.18 {
        // Light mode: drop shadows
        let alpha_sm = 0.04 + (1.0 - bg_luminance) * 0.06;
        let alpha_md = 0.08 + (1.0 - bg_luminance) * 0.08;
        let alpha_lg = 0.12 + (1.0 - bg_luminance) * 0.08;

        ShadowTokens {
            shadow_sm: format!("0 1px 3px rgba(0, 0, 0, {alpha_sm:.2})"),
            shadow_md: format!("0 4px 12px rgba(0, 0, 0, {alpha_md:.2})"),
            shadow_lg: format!("0 8px 32px rgba(0, 0, 0, {alpha_lg:.2})"),
            shadow_scrim: "rgba(0, 0, 0, 0.50)".into(),
        }
    } else {
        // Dark mode: glow borders
        let border_alpha_md = 0.08 + bg_luminance * 0.04;
        let border_alpha_lg = 0.12 + bg_luminance * 0.04;

        ShadowTokens {
            shadow_sm: "none".into(),
            shadow_md: format!("0 0 0 1px rgba(255, 255, 255, {border_alpha_md:.2})"),
            shadow_lg: format!("0 0 0 1px rgba(255, 255, 255, {border_alpha_lg:.2})"),
            shadow_scrim: "rgba(0, 0, 0, 0.70)".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_mode_shadows() {
        let s = compute_shadows(1.0); // white background
        assert!(s.shadow_sm.contains("rgba(0, 0, 0,"));
        assert!(s.shadow_md.contains("4px 12px"));
        assert!(s.shadow_lg.contains("8px 32px"));
        assert_eq!(s.shadow_scrim, "rgba(0, 0, 0, 0.50)");
    }

    #[test]
    fn dark_mode_shadows() {
        let s = compute_shadows(0.01); // near-black background
        assert_eq!(s.shadow_sm, "none");
        assert!(s.shadow_md.contains("rgba(255, 255, 255,"));
        assert!(s.shadow_lg.contains("rgba(255, 255, 255,"));
        assert_eq!(s.shadow_scrim, "rgba(0, 0, 0, 0.70)");
    }

    #[test]
    fn threshold_boundary() {
        // Exactly at threshold
        let light = compute_shadows(0.19);
        assert!(light.shadow_sm.contains("rgba(0, 0, 0,"));

        let dark = compute_shadows(0.18);
        assert_eq!(dark.shadow_sm, "none");
    }
}
