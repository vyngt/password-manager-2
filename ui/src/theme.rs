pub mod color_group;
pub mod color_space;
pub mod contrast;
pub mod derive;
pub mod inject;
pub mod mix;
pub mod provider;
pub mod scale;
pub mod shadows;
pub mod types;
pub mod validate;

// Re-export public API
pub use color_space::{ColorError, Oklch};
pub use contrast::compute_primary_foreground;
pub use derive::derive_tokens;
pub use provider::{ThemeState, use_theme};
pub use types::{ContrastResult, Severity, ThemeConfig, ThemeTokens, ThemeValidation};
pub use validate::{validate_theme_config, validate_tokens};

pub use inject::inject_css_vars;
