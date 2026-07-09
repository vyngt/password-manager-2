use serde::{Deserialize, Serialize};

/// Default danger base color (red-600).
pub const DEFAULT_DANGER: &str = "#DC2626";
/// Default warning base color (amber-600).
pub const DEFAULT_WARNING: &str = "#D97706";
/// Default success base color (green-600).
pub const DEFAULT_SUCCESS: &str = "#16A34A";

/// User-configurable theme inputs. All values are hex color strings.
///
/// `background`, `foreground`, and `primary` are required.
/// `danger`, `warning`, `success` are optional — the system uses well-known
/// defaults when omitted. The UI Settings page only exposes the 3 required
/// fields; the internal system treats all color groups identically.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThemeConfig {
    pub background: String,
    pub foreground: String,
    pub primary: String,
    #[serde(default)]
    pub danger: Option<String>,
    #[serde(default)]
    pub warning: Option<String>,
    #[serde(default)]
    pub success: Option<String>,
}

impl ThemeConfig {
    pub fn danger_or_default(&self) -> &str {
        self.danger.as_deref().unwrap_or(DEFAULT_DANGER)
    }

    pub fn warning_or_default(&self) -> &str {
        self.warning.as_deref().unwrap_or(DEFAULT_WARNING)
    }

    pub fn success_or_default(&self) -> &str {
        self.success.as_deref().unwrap_or(DEFAULT_SUCCESS)
    }
}

/// Full derived token set — all CSS variable values as hex or CSS strings.
/// Field names map 1:1 to `--color-*` and `--shadow-*` CSS custom properties.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThemeTokens {
    // Surfaces
    pub color_background: String,
    pub color_surface_1: String,
    pub color_surface_2: String,
    pub color_surface_3: String,
    pub color_surface_4: String,
    pub color_border: String,
    pub color_border_strong: String,

    // Text hierarchy
    pub color_text_primary: String,
    pub color_text_secondary: String,
    pub color_text_tertiary: String,

    // Primary
    pub color_primary: String,
    pub color_primary_hover: String,
    pub color_primary_muted: String,
    pub color_primary_foreground: String,
    pub color_primary_text: String,

    // Danger
    pub color_danger: String,
    pub color_danger_hover: String,
    pub color_danger_muted: String,
    pub color_danger_foreground: String,
    pub color_danger_text: String,

    // Warning
    pub color_warning: String,
    pub color_warning_hover: String,
    pub color_warning_muted: String,
    pub color_warning_foreground: String,
    pub color_warning_text: String,

    // Success
    pub color_success: String,
    pub color_success_hover: String,
    pub color_success_muted: String,
    pub color_success_foreground: String,
    pub color_success_text: String,

    // Focus
    pub color_focus_ring: String,

    // Shadows (full CSS value strings, not hex)
    pub shadow_sm: String,
    pub shadow_md: String,
    pub shadow_lg: String,
    pub shadow_scrim: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Block,
    Warn,
}

/// Result of a single contrast check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContrastResult {
    pub pair_label: String,
    pub ratio: f32,
    pub required: f32,
    pub pass: bool,
    pub severity: Severity,
    pub message: String,
}

/// Full validation report for a theme configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeValidation {
    pub checks: Vec<ContrastResult>,
    /// `true` when no `Severity::Block` failures exist.
    pub is_valid: bool,
}
