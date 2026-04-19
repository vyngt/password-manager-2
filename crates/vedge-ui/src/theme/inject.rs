use super::types::ThemeTokens;

/// Inject all theme tokens as CSS custom properties on `:root`.
///
/// Each field in `ThemeTokens` is mapped to its corresponding `--color-*` or
/// `--shadow-*` CSS variable. This overwrites the fallback values defined in
/// `tokens.css`.
///
/// On non-wasm targets this is a no-op.
pub fn inject_css_vars(tokens: &ThemeTokens) {
    #[cfg(target_arch = "wasm32")]
    {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(document) = window.document() else {
            return;
        };
        let Some(root) = document.document_element() else {
            return;
        };

        let html: &web_sys::HtmlElement = wasm_bindgen::JsCast::unchecked_ref(&root);
        let style = html.style();

        let pairs: &[(&str, &str)] = &[
            // Surfaces
            ("--color-background", &tokens.color_background),
            ("--color-surface-1", &tokens.color_surface_1),
            ("--color-surface-2", &tokens.color_surface_2),
            ("--color-surface-3", &tokens.color_surface_3),
            ("--color-surface-4", &tokens.color_surface_4),
            ("--color-border", &tokens.color_border),
            ("--color-border-strong", &tokens.color_border_strong),
            // Text
            ("--color-text-primary", &tokens.color_text_primary),
            ("--color-text-secondary", &tokens.color_text_secondary),
            ("--color-text-tertiary", &tokens.color_text_tertiary),
            // Primary
            ("--color-primary", &tokens.color_primary),
            ("--color-primary-hover", &tokens.color_primary_hover),
            ("--color-primary-muted", &tokens.color_primary_muted),
            (
                "--color-primary-foreground",
                &tokens.color_primary_foreground,
            ),
            ("--color-primary-text", &tokens.color_primary_text),
            // Danger
            ("--color-danger", &tokens.color_danger),
            ("--color-danger-hover", &tokens.color_danger_hover),
            ("--color-danger-muted", &tokens.color_danger_muted),
            ("--color-danger-foreground", &tokens.color_danger_foreground),
            ("--color-danger-text", &tokens.color_danger_text),
            // Warning
            ("--color-warning", &tokens.color_warning),
            ("--color-warning-hover", &tokens.color_warning_hover),
            ("--color-warning-muted", &tokens.color_warning_muted),
            (
                "--color-warning-foreground",
                &tokens.color_warning_foreground,
            ),
            ("--color-warning-text", &tokens.color_warning_text),
            // Success
            ("--color-success", &tokens.color_success),
            ("--color-success-hover", &tokens.color_success_hover),
            ("--color-success-muted", &tokens.color_success_muted),
            (
                "--color-success-foreground",
                &tokens.color_success_foreground,
            ),
            ("--color-success-text", &tokens.color_success_text),
            // Focus
            ("--color-focus-ring", &tokens.color_focus_ring),
            // Shadows
            ("--shadow-sm", &tokens.shadow_sm),
            ("--shadow-md", &tokens.shadow_md),
            ("--shadow-lg", &tokens.shadow_lg),
            ("--shadow-scrim", &tokens.shadow_scrim),
        ];

        for (prop, value) in pairs {
            let _ = style.set_property(prop, value);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    let _ = tokens;
}
