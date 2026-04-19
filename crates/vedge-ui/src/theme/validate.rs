use super::color_space::{ColorError, hex_to_srgb};
use super::contrast::{contrast_ratio, relative_luminance};
use super::derive::derive_tokens;
use super::types::{ContrastResult, Severity, ThemeConfig, ThemeTokens, ThemeValidation};

/// Validate a `ThemeConfig` by deriving tokens then checking all contrast pairs.
pub fn validate_theme_config(config: &ThemeConfig) -> Result<ThemeValidation, ColorError> {
    let tokens = derive_tokens(config)?;
    validate_tokens(&tokens)
}

/// Validate pre-derived `ThemeTokens` against WCAG contrast requirements.
///
/// Checks per the Color System spec (section 6.1):
///
/// | Pair                           | Threshold | Severity |
/// |--------------------------------|-----------|----------|
/// | text.primary / background      | >= 7.0    | Block    |
/// | text.secondary / background    | >= 4.5    | Block    |
/// | text.tertiary / background     | >= 3.0    | Warn     |
/// | primary-foreground / primary   | >= 4.5    | Block    |
/// | focus-ring / background        | >= 3.0    | Block    |
/// | focus-ring / surface-1         | >= 3.0    | Warn     |
/// | danger-foreground / danger     | >= 4.5    | Block    |
/// | text.primary / surface-4       | >= 4.5    | Warn     |
/// | background / surface-1         | >= 1.05   | Warn     |
/// | surface-1 / surface-2          | >= 1.05   | Warn     |
pub fn validate_tokens(tokens: &ThemeTokens) -> Result<ThemeValidation, ColorError> {
    let mut checks = Vec::new();

    // Helper: compute contrast ratio between two hex colors
    let cr = |hex_a: &str, hex_b: &str| -> Result<f32, ColorError> {
        let la = relative_luminance(hex_to_srgb(hex_a)?);
        let lb = relative_luminance(hex_to_srgb(hex_b)?);
        Ok(contrast_ratio(la, lb))
    };

    // Text checks
    check(
        &mut checks,
        "Body text on background",
        cr(&tokens.color_text_primary, &tokens.color_background)?,
        7.0,
        Severity::Block,
        "Body text is too faint on this background.",
    );

    check(
        &mut checks,
        "Secondary text on background",
        cr(&tokens.color_text_secondary, &tokens.color_background)?,
        4.5,
        Severity::Block,
        "Secondary text is too faint on this background.",
    );

    check(
        &mut checks,
        "Tertiary text on background",
        cr(&tokens.color_text_tertiary, &tokens.color_background)?,
        3.0,
        Severity::Warn,
        "Small captions may be slightly difficult to read.",
    );

    // Primary foreground on primary
    check(
        &mut checks,
        "Text on primary button",
        cr(&tokens.color_primary_foreground, &tokens.color_primary)?,
        4.5,
        Severity::Block,
        "Text on your primary button is hard to read.",
    );

    // Focus ring
    check(
        &mut checks,
        "Focus ring on background",
        cr(&tokens.color_focus_ring, &tokens.color_background)?,
        3.0,
        Severity::Block,
        "Focus indicator is not visible enough on this background.",
    );

    check(
        &mut checks,
        "Focus ring on surface-1",
        cr(&tokens.color_focus_ring, &tokens.color_surface_1)?,
        3.0,
        Severity::Warn,
        "Focus indicator may be hard to see on sidebar/card surfaces.",
    );

    // Danger foreground
    check(
        &mut checks,
        "Text on danger button",
        cr(&tokens.color_danger_foreground, &tokens.color_danger)?,
        4.5,
        Severity::Block,
        "Text on danger button is hard to read.",
    );

    // Text on surface-4
    check(
        &mut checks,
        "Body text on surface-4",
        cr(&tokens.color_text_primary, &tokens.color_surface_4)?,
        4.5,
        Severity::Warn,
        "Body text may be hard to read on floating surfaces.",
    );

    // Surface differentiation
    check(
        &mut checks,
        "Background vs surface-1",
        cr(&tokens.color_background, &tokens.color_surface_1)?,
        1.05,
        Severity::Warn,
        "Background and first surface layer are indistinguishable.",
    );

    check(
        &mut checks,
        "Surface-1 vs surface-2",
        cr(&tokens.color_surface_1, &tokens.color_surface_2)?,
        1.05,
        Severity::Warn,
        "Surface layers 1 and 2 are indistinguishable.",
    );

    let is_valid = checks
        .iter()
        .all(|c| c.pass || c.severity != Severity::Block);

    Ok(ThemeValidation { checks, is_valid })
}

fn check(
    checks: &mut Vec<ContrastResult>,
    label: &str,
    ratio: f32,
    required: f32,
    severity: Severity,
    fail_message: &str,
) {
    let pass = ratio >= required;
    let message = if pass {
        format!("{label}: {ratio:.1}:1 (meets {required:.1}:1)")
    } else {
        format!("{fail_message} Current contrast: {ratio:.1}:1. Required: {required:.1}:1.")
    };

    checks.push(ContrastResult {
        pair_label: label.to_string(),
        ratio,
        required,
        pass,
        severity,
        message,
    });
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

    #[test]
    fn light_theme_is_valid() {
        let validation = validate_theme_config(&light_preset()).unwrap();
        assert!(
            validation.is_valid,
            "light preset should be valid. Failures: {:?}",
            validation
                .checks
                .iter()
                .filter(|c| !c.pass)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn low_contrast_blocks() {
        let config = ThemeConfig {
            background: "#808080".into(),
            foreground: "#707070".into(),
            primary: "#2563EB".into(),
            danger: None,
            warning: None,
            success: None,
        };
        let validation = validate_theme_config(&config).unwrap();
        assert!(!validation.is_valid, "low contrast should block");
        assert!(
            validation
                .checks
                .iter()
                .any(|c| c.severity == Severity::Block && !c.pass),
            "should have at least one Block failure"
        );
    }

    #[test]
    fn dark_theme_is_valid() {
        let config = ThemeConfig {
            background: "#09090B".into(),
            foreground: "#FAFAFA".into(),
            primary: "#3B82F6".into(),
            danger: None,
            warning: None,
            success: None,
        };
        let validation = validate_theme_config(&config).unwrap();
        assert!(
            validation.is_valid,
            "dark preset should be valid. Failures: {:?}",
            validation
                .checks
                .iter()
                .filter(|c| !c.pass)
                .collect::<Vec<_>>()
        );
    }
}
