//! `resolve_active_theme` + `set_active_theme` — the two-repo orchestration
//! that binds `themes` and `app_settings` so the shell doesn't have to
//! juggle the fallback policy.
//!
//! ## Contract
//!
//! - The "active theme" is identified by the [`ThemeId`] stored under the
//!   `app.active_theme_id` key in `app_settings`. Value is a JSON string.
//! - If the key is missing, or the referenced theme no longer exists,
//!   `resolve_active_theme` silently falls back to [`DEFAULT_THEME_ID`]
//!   (`builtin-light`) — seeded by migration `m20260418_090003_create_themes`.
//! - `set_active_theme` validates that the target theme exists *before*
//!   writing the setting, so we can't land in a state where the active
//!   pointer dangles.

use tracing::instrument;

use crate::application::app::ports::{AppSettingRepository, ThemeRepository};
use crate::domain::app::entities::Theme;
use crate::domain::app::errors::AppDbError;
use crate::domain::shared::{now, ThemeId};

/// Validate a color string is exactly `#RRGGBB` — leading `#`, six ASCII
/// hex digits. No alpha, no HSL, no named colors (matches the built-in
/// seed palette in `tokens.css`).
pub fn validate_hex_color(s: &str) -> Result<(), AppDbError> {
    let bytes = s.as_bytes();
    let Some((first, rest)) = bytes.split_first() else {
        return Err(AppDbError::InvalidSettingValue {
            key: "theme.color".into(),
            reason: "empty string".into(),
        });
    };
    if rest.len() != 6 || *first != b'#' {
        return Err(AppDbError::InvalidSettingValue {
            key: "theme.color".into(),
            reason: format!("expected #RRGGBB, got {s:?}"),
        });
    }
    for &b in rest {
        if !b.is_ascii_hexdigit() {
            return Err(AppDbError::InvalidSettingValue {
                key: "theme.color".into(),
                reason: format!("non-hex digit in {s:?}"),
            });
        }
    }
    Ok(())
}

/// Key under which the active-theme ID is stored in `app_settings`.
pub const ACTIVE_THEME_SETTING_KEY: &str = "app.active_theme_id";

/// Seeded by migration; used as the fallback when the setting is missing
/// or points at a deleted theme.
pub const DEFAULT_THEME_ID: &str = "builtin-light";

/// Read the active theme. Falls back to [`DEFAULT_THEME_ID`] on any of:
/// setting missing, setting value not a JSON string, or the referenced
/// theme not present in the `themes` table.
///
/// Never returns `Err` for a "not configured" case — the fallback is
/// silent. Propagates storage errors.
#[instrument(skip_all)]
pub async fn resolve_active_theme(
    themes: &dyn ThemeRepository,
    settings: &dyn AppSettingRepository,
) -> Result<Theme, AppDbError> {
    let configured_id = match settings.get(ACTIVE_THEME_SETTING_KEY).await? {
        Some(row) => row.value.as_str().map(ToOwned::to_owned),
        None => None,
    };

    if let Some(id) = configured_id {
        let theme_id = ThemeId::from_raw(id);
        match themes.get(&theme_id).await {
            Ok(theme) => return Ok(theme),
            Err(AppDbError::ThemeNotFound(_)) => {
                // Fall through to default.
            }
            Err(e) => return Err(e),
        }
    }

    themes.get(&ThemeId::from_raw(DEFAULT_THEME_ID)).await
}

/// Mark a theme as active. Validates that the theme exists *first* so
/// the setting never points at a missing row.
#[instrument(skip_all, fields(theme_id = %theme_id))]
pub async fn set_active_theme(
    themes: &dyn ThemeRepository,
    settings: &dyn AppSettingRepository,
    theme_id: &ThemeId,
) -> Result<(), AppDbError> {
    // `get` returns ThemeNotFound if missing — propagate directly.
    let _ = themes.get(theme_id).await?;

    let value = serde_json::Value::String(theme_id.as_str().to_owned());
    settings
        .set(ACTIVE_THEME_SETTING_KEY, value, now())
        .await
}

// ---- Pass-through reads ------------------------------------------------------

/// List every theme (built-in + custom). Pass-through to the repo — lives
/// in the use-case layer so future instrumentation / filtering / sorting
/// can be added without touching the shell.
#[instrument(skip_all)]
pub async fn list_themes(repo: &dyn ThemeRepository) -> Result<Vec<Theme>, AppDbError> {
    repo.list().await
}

/// Look up a theme by ID. Pass-through to the repo.
#[instrument(skip_all, fields(id = %id))]
pub async fn get_theme(repo: &dyn ThemeRepository, id: &ThemeId) -> Result<Theme, AppDbError> {
    repo.get(id).await
}

// ---- Custom theme CRUD -------------------------------------------------------

/// Input to [`create_custom_theme`]. `is_built_in` is not exposed —
/// user-created themes are always custom.
#[derive(Debug, Clone)]
pub struct CreateCustomThemeInput {
    pub name: String,
    pub root_background: String,
    pub root_foreground: String,
    pub root_primary: String,
    pub danger_base: Option<String>,
    pub warning_base: Option<String>,
    pub success_base: Option<String>,
}

/// Create a user-defined theme. Validates colors + name, forces
/// `is_built_in = false`. Returns the freshly-minted [`ThemeId`].
#[instrument(skip_all, fields(name = %input.name))]
pub async fn create_custom_theme(
    themes: &dyn ThemeRepository,
    input: CreateCustomThemeInput,
) -> Result<ThemeId, AppDbError> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppDbError::InvalidSettingValue {
            key: "theme.name".into(),
            reason: "empty".into(),
        });
    }
    validate_hex_color(&input.root_background)?;
    validate_hex_color(&input.root_foreground)?;
    validate_hex_color(&input.root_primary)?;
    if let Some(ref c) = input.danger_base {
        validate_hex_color(c)?;
    }
    if let Some(ref c) = input.warning_base {
        validate_hex_color(c)?;
    }
    if let Some(ref c) = input.success_base {
        validate_hex_color(c)?;
    }

    let id = ThemeId::new();
    let when = now();
    let theme = Theme {
        id: id.clone(),
        name: name.to_owned(),
        is_built_in: false,
        root_background: input.root_background,
        root_foreground: input.root_foreground,
        root_primary: input.root_primary,
        danger_base: input.danger_base,
        warning_base: input.warning_base,
        success_base: input.success_base,
        created_at: when,
        updated_at: when,
    };
    themes.upsert(&theme).await?;
    Ok(id)
}

/// Input to [`update_custom_theme`]. Only mutable fields; `id` + `name`
/// + color fields. `created_at` is preserved from the existing row.
#[derive(Debug, Clone)]
pub struct UpdateCustomThemeInput {
    pub id: ThemeId,
    pub name: String,
    pub root_background: String,
    pub root_foreground: String,
    pub root_primary: String,
    pub danger_base: Option<String>,
    pub warning_base: Option<String>,
    pub success_base: Option<String>,
}

/// Mutate an existing custom theme. Rejects built-ins with
/// [`AppDbError::BuiltInThemeImmutable`]. Preserves `created_at`, bumps
/// `updated_at` to now.
#[instrument(skip_all, fields(theme_id = %input.id))]
pub async fn update_custom_theme(
    themes: &dyn ThemeRepository,
    input: UpdateCustomThemeInput,
) -> Result<(), AppDbError> {
    let existing = themes.get(&input.id).await?;
    if existing.is_built_in {
        return Err(AppDbError::BuiltInThemeImmutable);
    }
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppDbError::InvalidSettingValue {
            key: "theme.name".into(),
            reason: "empty".into(),
        });
    }
    validate_hex_color(&input.root_background)?;
    validate_hex_color(&input.root_foreground)?;
    validate_hex_color(&input.root_primary)?;
    if let Some(ref c) = input.danger_base {
        validate_hex_color(c)?;
    }
    if let Some(ref c) = input.warning_base {
        validate_hex_color(c)?;
    }
    if let Some(ref c) = input.success_base {
        validate_hex_color(c)?;
    }

    let updated = Theme {
        id: existing.id,
        name: name.to_owned(),
        is_built_in: false,
        root_background: input.root_background,
        root_foreground: input.root_foreground,
        root_primary: input.root_primary,
        danger_base: input.danger_base,
        warning_base: input.warning_base,
        success_base: input.success_base,
        created_at: existing.created_at,
        updated_at: now(),
    };
    themes.upsert(&updated).await
}

/// Clone an existing theme (built-in or custom) into a new row with a
/// fresh ULID and `is_built_in = false`. `new_name` defaults to
/// `"{source.name} (copy)"` when `None`.
#[instrument(skip_all, fields(source_id = %source_id))]
pub async fn duplicate_theme(
    themes: &dyn ThemeRepository,
    source_id: &ThemeId,
    new_name: Option<String>,
) -> Result<ThemeId, AppDbError> {
    let source = themes.get(source_id).await?;
    let name = new_name
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{} (copy)", source.name));

    let id = ThemeId::new();
    let when = now();
    let theme = Theme {
        id: id.clone(),
        name,
        is_built_in: false,
        root_background: source.root_background,
        root_foreground: source.root_foreground,
        root_primary: source.root_primary,
        danger_base: source.danger_base,
        warning_base: source.warning_base,
        success_base: source.success_base,
        created_at: when,
        updated_at: when,
    };
    themes.upsert(&theme).await?;
    Ok(id)
}

/// Delete a custom theme.
///
/// If it happens to be the active theme, clears the active-theme setting
/// *first* so [`resolve_active_theme`] can land on the default cleanly
/// even if the theme delete fails afterward.
#[instrument(skip_all, fields(theme_id = %theme_id))]
pub async fn delete_custom_theme(
    themes: &dyn ThemeRepository,
    settings: &dyn AppSettingRepository,
    theme_id: &ThemeId,
) -> Result<(), AppDbError> {
    let existing = themes.get(theme_id).await?;
    if existing.is_built_in {
        return Err(AppDbError::BuiltInThemeImmutable);
    }

    // Reset the active-theme setting *before* the delete so partial
    // failure leaves us in a valid state (setting absent → fallback
    // resolves to default).
    if let Some(row) = settings.get(ACTIVE_THEME_SETTING_KEY).await?
        && row.value.as_str() == Some(theme_id.as_str())
    {
        settings.delete(ACTIVE_THEME_SETTING_KEY).await?;
    }

    themes.delete(theme_id).await
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )]

    use super::*;
    use crate::infrastructure::sqlite::app::{
        AppDbConnection, SqliteAppSettingRepository, SqliteThemeRepository,
    };
    use std::sync::Arc;

    async fn fixture() -> (
        tempfile::TempDir,
        Arc<SqliteThemeRepository>,
        Arc<SqliteAppSettingRepository>,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let db = AppDbConnection::open(&dir.path().join("app.db"))
            .await
            .unwrap();
        let themes = Arc::new(SqliteThemeRepository::new(db.handle()));
        let settings = Arc::new(SqliteAppSettingRepository::new(db.handle()));
        (dir, themes, settings)
    }

    #[tokio::test]
    async fn resolve_returns_default_when_setting_missing() {
        let (_dir, themes, settings) = fixture().await;
        let theme = resolve_active_theme(&*themes, &*settings).await.unwrap();
        assert_eq!(theme.id.as_str(), DEFAULT_THEME_ID);
        assert!(theme.is_built_in);
    }

    #[tokio::test]
    async fn resolve_returns_default_when_setting_points_at_missing_theme() {
        let (_dir, themes, settings) = fixture().await;
        settings
            .set(
                ACTIVE_THEME_SETTING_KEY,
                serde_json::Value::String("does-not-exist".into()),
                now(),
            )
            .await
            .unwrap();
        let theme = resolve_active_theme(&*themes, &*settings).await.unwrap();
        assert_eq!(theme.id.as_str(), DEFAULT_THEME_ID);
    }

    #[tokio::test]
    async fn set_active_theme_rejects_missing_theme() {
        let (_dir, themes, settings) = fixture().await;
        let err = set_active_theme(
            &*themes,
            &*settings,
            &ThemeId::from_raw("ghost-theme"),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppDbError::ThemeNotFound(_)));
    }

    #[tokio::test]
    async fn set_then_resolve_round_trips() {
        let (_dir, themes, settings) = fixture().await;
        let dark = ThemeId::from_raw("builtin-dark");
        set_active_theme(&*themes, &*settings, &dark).await.unwrap();
        let theme = resolve_active_theme(&*themes, &*settings).await.unwrap();
        assert_eq!(theme.id, dark);
    }

    // ---- validate_hex_color --------------------------------------------------

    #[test]
    fn validate_accepts_uppercase_hex() {
        assert!(validate_hex_color("#00FF00").is_ok());
    }

    #[test]
    fn validate_accepts_lowercase_hex() {
        assert!(validate_hex_color("#abcdef").is_ok());
    }

    #[test]
    fn validate_rejects_too_short() {
        assert!(validate_hex_color("#abc").is_err());
    }

    #[test]
    fn validate_rejects_missing_hash() {
        assert!(validate_hex_color("00FF00").is_err());
    }

    #[test]
    fn validate_rejects_non_hex_char() {
        let err = validate_hex_color("#ZZZZZZ").unwrap_err();
        assert!(matches!(err, AppDbError::InvalidSettingValue { .. }));
    }

    #[test]
    fn validate_rejects_empty_string() {
        assert!(validate_hex_color("").is_err());
    }

    // ---- create_custom_theme ------------------------------------------------

    fn good_create_input() -> CreateCustomThemeInput {
        CreateCustomThemeInput {
            name: "Midnight".into(),
            root_background: "#0E0F12".into(),
            root_foreground: "#E6E8EB".into(),
            root_primary: "#1D9E75".into(),
            danger_base: Some("#dc2626".into()),
            warning_base: None,
            success_base: None,
        }
    }

    #[tokio::test]
    async fn create_custom_persists_and_is_not_builtin() {
        let (_dir, themes, _settings) = fixture().await;
        let id = create_custom_theme(&*themes, good_create_input())
            .await
            .unwrap();
        let got = themes.get(&id).await.unwrap();
        assert_eq!(got.name, "Midnight");
        assert!(!got.is_built_in);
        assert_eq!(got.root_background, "#0E0F12");
    }

    #[tokio::test]
    async fn create_custom_rejects_empty_name() {
        let (_dir, themes, _) = fixture().await;
        let mut input = good_create_input();
        input.name = "   ".into();
        let err = create_custom_theme(&*themes, input).await.unwrap_err();
        assert!(matches!(err, AppDbError::InvalidSettingValue { .. }));
    }

    #[tokio::test]
    async fn create_custom_rejects_bad_color() {
        let (_dir, themes, _) = fixture().await;
        let mut input = good_create_input();
        input.root_background = "bad".into();
        let err = create_custom_theme(&*themes, input).await.unwrap_err();
        assert!(matches!(err, AppDbError::InvalidSettingValue { .. }));
    }

    #[tokio::test]
    async fn create_custom_optional_bases_round_trip_as_none() {
        let (_dir, themes, _) = fixture().await;
        let input = CreateCustomThemeInput {
            name: "Plain".into(),
            root_background: "#111111".into(),
            root_foreground: "#eeeeee".into(),
            root_primary: "#1234ab".into(),
            danger_base: None,
            warning_base: None,
            success_base: None,
        };
        let id = create_custom_theme(&*themes, input).await.unwrap();
        let got = themes.get(&id).await.unwrap();
        assert!(got.danger_base.is_none());
        assert!(got.warning_base.is_none());
        assert!(got.success_base.is_none());
    }

    // ---- update_custom_theme ------------------------------------------------

    #[tokio::test]
    async fn update_custom_preserves_created_at() {
        let (_dir, themes, _) = fixture().await;
        let id = create_custom_theme(&*themes, good_create_input())
            .await
            .unwrap();
        let before = themes.get(&id).await.unwrap();
        // Sleep 1ms to ensure updated_at can diverge observably.
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;

        update_custom_theme(
            &*themes,
            UpdateCustomThemeInput {
                id: id.clone(),
                name: "Midnight v2".into(),
                root_background: "#000000".into(),
                root_foreground: "#FFFFFF".into(),
                root_primary: "#123456".into(),
                danger_base: None,
                warning_base: None,
                success_base: None,
            },
        )
        .await
        .unwrap();

        let after = themes.get(&id).await.unwrap();
        assert_eq!(after.name, "Midnight v2");
        assert_eq!(after.created_at, before.created_at);
        assert!(after.updated_at >= before.updated_at);
    }

    #[tokio::test]
    async fn update_custom_rejects_builtin() {
        let (_dir, themes, _) = fixture().await;
        let err = update_custom_theme(
            &*themes,
            UpdateCustomThemeInput {
                id: ThemeId::from_raw("builtin-light"),
                name: "Hijacked".into(),
                root_background: "#000000".into(),
                root_foreground: "#FFFFFF".into(),
                root_primary: "#123456".into(),
                danger_base: None,
                warning_base: None,
                success_base: None,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppDbError::BuiltInThemeImmutable));
    }

    #[tokio::test]
    async fn update_custom_rejects_bad_color() {
        let (_dir, themes, _) = fixture().await;
        let id = create_custom_theme(&*themes, good_create_input())
            .await
            .unwrap();
        let err = update_custom_theme(
            &*themes,
            UpdateCustomThemeInput {
                id,
                name: "x".into(),
                root_background: "not-a-color".into(),
                root_foreground: "#FFFFFF".into(),
                root_primary: "#123456".into(),
                danger_base: None,
                warning_base: None,
                success_base: None,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppDbError::InvalidSettingValue { .. }));
    }

    // ---- duplicate_theme ----------------------------------------------------

    #[tokio::test]
    async fn duplicate_produces_distinct_custom() {
        let (_dir, themes, _) = fixture().await;
        let source = ThemeId::from_raw("builtin-dark");
        let new_id = duplicate_theme(&*themes, &source, None).await.unwrap();
        assert_ne!(new_id.as_str(), source.as_str());
        let clone = themes.get(&new_id).await.unwrap();
        assert!(!clone.is_built_in);
        assert_eq!(clone.name, "Dark (copy)");
        assert_eq!(clone.root_background, "#0E0F12");
    }

    #[tokio::test]
    async fn duplicate_uses_explicit_name_override() {
        let (_dir, themes, _) = fixture().await;
        let new_id = duplicate_theme(
            &*themes,
            &ThemeId::from_raw("builtin-light"),
            Some("My Light".into()),
        )
        .await
        .unwrap();
        let clone = themes.get(&new_id).await.unwrap();
        assert_eq!(clone.name, "My Light");
    }

    // ---- delete_custom_theme ------------------------------------------------

    #[tokio::test]
    async fn delete_custom_removes_row_and_leaves_settings_untouched() {
        let (_dir, themes, settings) = fixture().await;
        let id = create_custom_theme(&*themes, good_create_input())
            .await
            .unwrap();
        delete_custom_theme(&*themes, &*settings, &id).await.unwrap();
        let err = themes.get(&id).await.unwrap_err();
        assert!(matches!(err, AppDbError::ThemeNotFound(_)));
        // Setting never written in this flow.
        assert!(settings.get(ACTIVE_THEME_SETTING_KEY).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_active_custom_clears_setting_and_resolves_to_default() {
        let (_dir, themes, settings) = fixture().await;
        let id = create_custom_theme(&*themes, good_create_input())
            .await
            .unwrap();
        set_active_theme(&*themes, &*settings, &id).await.unwrap();

        delete_custom_theme(&*themes, &*settings, &id).await.unwrap();

        assert!(settings.get(ACTIVE_THEME_SETTING_KEY).await.unwrap().is_none());
        let resolved = resolve_active_theme(&*themes, &*settings).await.unwrap();
        assert_eq!(resolved.id.as_str(), DEFAULT_THEME_ID);
    }

    #[tokio::test]
    async fn delete_builtin_rejected() {
        let (_dir, themes, settings) = fixture().await;
        let err = delete_custom_theme(
            &*themes,
            &*settings,
            &ThemeId::from_raw("builtin-dark"),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppDbError::BuiltInThemeImmutable));
        // Still present.
        assert!(themes.get(&ThemeId::from_raw("builtin-dark")).await.is_ok());
    }
}
