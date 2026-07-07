//! Pure conversions + selectors between the theme IPC DTOs, the UI engine's
//! `ThemeConfig`, and the list/editor views. Kept free of Leptos so they stay
//! host-testable (the same split as `playground/theme_api.rs`, which this
//! adapts).
//!
//! Field-name bridge: the IPC DTOs use `root_background` / `root_foreground` /
//! `root_primary` + `danger_base` / `warning_base` / `success_base`; the engine
//! `ThemeConfig` drops the prefixes to `background` / `foreground` / `primary` +
//! `danger` / `warning` / `success`. The three status colors are `Option` on
//! both sides.

use vedge_ipc::{CreateCustomThemeInputDto, ThemeDto, UpdateCustomThemeInputDto};
use vedge_ui::theme::{Severity, ThemeConfig, ThemeValidation};

/// Extract the six editable colors out of a persisted theme.
#[must_use]
pub fn theme_config_from_dto(dto: &ThemeDto) -> ThemeConfig {
    ThemeConfig {
        background: dto.root_background.clone(),
        foreground: dto.root_foreground.clone(),
        primary: dto.root_primary.clone(),
        danger: dto.danger_base.clone(),
        warning: dto.warning_base.clone(),
        success: dto.success_base.clone(),
    }
}

/// Build a `ThemeConfig` from the editor's six raw string fields. Blank optional
/// fields collapse to `None` so the engine substitutes its defaults (the spec's
/// "omitted → engine default").
#[must_use]
pub fn config_from_fields(
    background: String,
    foreground: String,
    primary: String,
    danger: String,
    warning: String,
    success: String,
) -> ThemeConfig {
    ThemeConfig {
        background,
        foreground,
        primary,
        danger: non_empty(danger),
        warning: non_empty(warning),
        success: non_empty(success),
    }
}

fn non_empty(s: String) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

/// Build a create-input from the editor's current config + a chosen name.
#[must_use]
pub fn create_input_from(name: String, cfg: &ThemeConfig) -> CreateCustomThemeInputDto {
    CreateCustomThemeInputDto {
        name,
        root_background: cfg.background.clone(),
        root_foreground: cfg.foreground.clone(),
        root_primary: cfg.primary.clone(),
        danger_base: cfg.danger.clone(),
        warning_base: cfg.warning.clone(),
        success_base: cfg.success.clone(),
    }
}

/// Build an update-input for an existing custom theme.
#[must_use]
pub fn update_input_from(id: String, name: String, cfg: &ThemeConfig) -> UpdateCustomThemeInputDto {
    UpdateCustomThemeInputDto {
        id,
        name,
        root_background: cfg.background.clone(),
        root_foreground: cfg.foreground.clone(),
        root_primary: cfg.primary.clone(),
        danger_base: cfg.danger.clone(),
        warning_base: cfg.warning.clone(),
        success_base: cfg.success.clone(),
    }
}

/// The five preview swatches for a theme row: background, text, primary, then
/// danger + warning resolved to the engine defaults when unset.
#[must_use]
pub fn theme_swatches(dto: &ThemeDto) -> [String; 5] {
    let cfg = theme_config_from_dto(dto);
    [
        cfg.background.clone(),
        cfg.foreground.clone(),
        cfg.primary.clone(),
        cfg.danger_or_default().to_owned(),
        cfg.warning_or_default().to_owned(),
    ]
}

/// Whether `dto` is the currently-active theme.
#[must_use]
pub fn is_active(dto: &ThemeDto, active_id: Option<&str>) -> bool {
    active_id == Some(dto.id.as_str())
}

/// The two headline contrast verdicts shown in the editor: text-on-background and
/// text-on-primary. Each is `true` when every *blocking* check for that pair
/// passes. Selects checks by `pair_label` substring, so a label rename in the
/// engine trips the unit test rather than silently mis-reporting.
#[must_use]
pub fn headline_contrast(validation: &ThemeValidation) -> (bool, bool) {
    let text_ok = validation
        .checks
        .iter()
        .filter(|c| c.severity == Severity::Block && c.pair_label.contains("text on background"))
        .all(|c| c.pass);
    let primary_ok = validation
        .checks
        .iter()
        .filter(|c| c.pair_label.contains("primary button"))
        .all(|c| c.pass);
    (text_ok, primary_ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vedge_ui::theme::validate_theme_config;

    fn dto(id: &str, built_in: bool, danger: Option<&str>) -> ThemeDto {
        ThemeDto {
            id: id.to_owned(),
            name: format!("{id} name"),
            is_built_in: built_in,
            root_background: "#FFFFFF".to_owned(),
            root_foreground: "#111827".to_owned(),
            root_primary: "#2563EB".to_owned(),
            danger_base: danger.map(str::to_owned),
            warning_base: None,
            success_base: None,
            created_at: "2026-07-07T00:00:00.000Z".to_owned(),
            updated_at: "2026-07-07T00:00:00.000Z".to_owned(),
        }
    }

    #[test]
    fn config_from_dto_bridges_names_and_options() {
        let cfg = theme_config_from_dto(&dto("t", false, Some("#AA0000")));
        assert_eq!(cfg.background, "#FFFFFF");
        assert_eq!(cfg.foreground, "#111827");
        assert_eq!(cfg.primary, "#2563EB");
        assert_eq!(cfg.danger.as_deref(), Some("#AA0000"));
        assert_eq!(cfg.warning, None);
        assert_eq!(cfg.success, None);
    }

    #[test]
    fn config_from_fields_blanks_become_none() {
        let cfg = config_from_fields(
            "#000000".into(),
            "#FFFFFF".into(),
            "#2563EB".into(),
            "  ".into(),
            String::new(),
            "#16A34A".into(),
        );
        assert_eq!(cfg.danger, None);
        assert_eq!(cfg.warning, None);
        assert_eq!(cfg.success.as_deref(), Some("#16A34A"));
    }

    #[test]
    fn input_from_maps_all_fields() {
        let cfg = theme_config_from_dto(&dto("t", false, Some("#AA0000")));
        let create = create_input_from("Ocean".into(), &cfg);
        assert_eq!(create.name, "Ocean");
        assert_eq!(create.root_background, "#FFFFFF");
        assert_eq!(create.danger_base.as_deref(), Some("#AA0000"));

        let update = update_input_from("id-1".into(), "Ocean".into(), &cfg);
        assert_eq!(update.id, "id-1");
        assert_eq!(update.root_primary, "#2563EB");
        assert_eq!(update.warning_base, None);
    }

    #[test]
    fn swatches_resolve_optional_to_defaults() {
        let s = theme_swatches(&dto("t", true, None));
        assert_eq!(s[0], "#FFFFFF");
        assert_eq!(s[1], "#111827");
        assert_eq!(s[2], "#2563EB");
        assert_eq!(s[3], "#DC2626"); // danger default
        assert_eq!(s[4], "#D97706"); // warning default
    }

    #[test]
    fn is_active_matches_by_id() {
        let d = dto("theme-a", true, None);
        assert!(is_active(&d, Some("theme-a")));
        assert!(!is_active(&d, Some("theme-b")));
        assert!(!is_active(&d, None));
    }

    #[test]
    fn headline_contrast_passes_for_a_good_theme() {
        let cfg = ThemeConfig {
            background: "#FFFFFF".into(),
            foreground: "#111827".into(),
            primary: "#2563EB".into(),
            danger: None,
            warning: None,
            success: None,
        };
        let v = validate_theme_config(&cfg).expect("derivable");
        assert_eq!(headline_contrast(&v), (true, true));
    }

    #[test]
    fn headline_contrast_flags_low_text_contrast() {
        // Near-white text on white → text-on-background fails.
        let cfg = ThemeConfig {
            background: "#FFFFFF".into(),
            foreground: "#EEEEEE".into(),
            primary: "#2563EB".into(),
            danger: None,
            warning: None,
            success: None,
        };
        let v = validate_theme_config(&cfg).expect("derivable");
        assert!(!headline_contrast(&v).0);
    }
}
