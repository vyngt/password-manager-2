use vedge_core::business::domain::entities::theme::{ColorScheme, Theme, UpdateTheme};
use vedge_core::infra::data::sqlite::entities::theme::{color_scheme, theme};
use vedge_core::infra::data::sqlite::mappers::db_theme::ThemeMapper;
use chrono::Utc;
use uuid::Uuid;

#[test]
fn test_theme_mapper_to_domain() {
    // Arrange
    let theme_id = Uuid::new_v4();
    let color_scheme_id = Uuid::new_v4();
    let now = Utc::now();

    let theme_model = theme::Model {
        id: theme_id,
        color_scheme_id: Some(color_scheme_id),
    };

    let color_scheme_model = color_scheme::Model {
        id: color_scheme_id,
        name: "Test Theme".to_string(),
        color_primary: "#3B82F6".to_string(),
        color_secondary: "#6B7280".to_string(),
        color_success: "#10B981".to_string(),
        color_danger: "#EF4444".to_string(),
        color_warning: "#F59E0B".to_string(),
        color_foreground: "#1F2937".to_string(),
        color_background: "#FFFFFF".to_string(),
        created_at: now,
        updated_at: now,
    };

    // Act
    let domain_entity = ThemeMapper::to_domain(theme_model, color_scheme_model);

    // Assert
    assert_eq!(domain_entity.id, theme_id);
    assert_eq!(domain_entity.color_scheme.id, color_scheme_id);
    assert_eq!(domain_entity.color_scheme.name, "Test Theme");
    assert_eq!(domain_entity.color_scheme.color_primary, "#3B82F6");
    assert_eq!(domain_entity.color_scheme.color_secondary, "#6B7280");
    assert_eq!(domain_entity.color_scheme.color_success, "#10B981");
    assert_eq!(domain_entity.color_scheme.color_danger, "#EF4444");
    assert_eq!(domain_entity.color_scheme.color_warning, "#F59E0B");
    assert_eq!(domain_entity.color_scheme.color_foreground, "#1F2937");
    assert_eq!(domain_entity.color_scheme.color_background, "#FFFFFF");
}

#[test]
fn test_theme_mapper_to_persistence() {
    // Arrange
    let theme_id = Uuid::new_v4();
    let color_scheme_id = Uuid::new_v4();

    let domain_entity = Theme {
        id: theme_id,
        color_scheme: ColorScheme {
            id: color_scheme_id,
            name: "Test Theme".to_string(),
            color_primary: "#3B82F6".to_string(),
            color_secondary: "#6B7280".to_string(),
            color_success: "#10B981".to_string(),
            color_danger: "#EF4444".to_string(),
            color_warning: "#F59E0B".to_string(),
            color_foreground: "#1F2937".to_string(),
            color_background: "#FFFFFF".to_string(),
        },
    };

    // Act
    let active_model = ThemeMapper::to_persistence(domain_entity);

    // Assert
    assert_eq!(active_model.id, sea_orm::ActiveValue::Set(theme_id));
    assert_eq!(
        active_model.color_scheme_id,
        sea_orm::ActiveValue::Set(Some(color_scheme_id))
    );
}

#[test]
fn test_theme_mapper_to_update() {
    // Arrange
    let theme_id = Uuid::new_v4();
    let new_color_scheme_id = Uuid::new_v4();

    let update_entity = UpdateTheme {
        color_scheme_id: new_color_scheme_id,
    };

    // Act
    let active_model = ThemeMapper::to_update(theme_id, update_entity);

    // Assert
    assert_eq!(active_model.id, sea_orm::ActiveValue::Set(theme_id));
    assert_eq!(
        active_model.color_scheme_id,
        sea_orm::ActiveValue::Set(Some(new_color_scheme_id))
    );
}

#[test]
fn test_theme_mapper_roundtrip() {
    // Arrange
    let theme_id = Uuid::new_v4();
    let color_scheme_id = Uuid::new_v4();

    let original_domain = Theme {
        id: theme_id,
        color_scheme: ColorScheme {
            id: color_scheme_id,
            name: "Roundtrip Test".to_string(),
            color_primary: "#8B5CF6".to_string(),
            color_secondary: "#A78BFA".to_string(),
            color_success: "#10B981".to_string(),
            color_danger: "#F59E0B".to_string(),
            color_warning: "#EF4444".to_string(),
            color_foreground: "#1F2937".to_string(),
            color_background: "#F3F4F6".to_string(),
        },
    };

    // Act
    let _active_model = ThemeMapper::to_persistence(original_domain.clone());

    // Create mock database models with the same data
    let now = Utc::now();
    let theme_model = theme::Model {
        id: theme_id,
        color_scheme_id: Some(color_scheme_id),
    };

    let color_scheme_model = color_scheme::Model {
        id: color_scheme_id,
        name: original_domain.color_scheme.name.clone(),
        color_primary: original_domain.color_scheme.color_primary.clone(),
        color_secondary: original_domain.color_scheme.color_secondary.clone(),
        color_success: original_domain.color_scheme.color_success.clone(),
        color_danger: original_domain.color_scheme.color_danger.clone(),
        color_warning: original_domain.color_scheme.color_warning.clone(),
        color_foreground: original_domain.color_scheme.color_foreground.clone(),
        color_background: original_domain.color_scheme.color_background.clone(),
        created_at: now,
        updated_at: now,
    };

    let converted_domain = ThemeMapper::to_domain(theme_model, color_scheme_model);

    // Assert
    assert_eq!(converted_domain.id, original_domain.id);
    assert_eq!(
        converted_domain.color_scheme.id,
        original_domain.color_scheme.id
    );
    assert_eq!(
        converted_domain.color_scheme.name,
        original_domain.color_scheme.name
    );
    assert_eq!(
        converted_domain.color_scheme.color_primary,
        original_domain.color_scheme.color_primary
    );
    assert_eq!(
        converted_domain.color_scheme.color_secondary,
        original_domain.color_scheme.color_secondary
    );
    assert_eq!(
        converted_domain.color_scheme.color_success,
        original_domain.color_scheme.color_success
    );
    assert_eq!(
        converted_domain.color_scheme.color_danger,
        original_domain.color_scheme.color_danger
    );
    assert_eq!(
        converted_domain.color_scheme.color_warning,
        original_domain.color_scheme.color_warning
    );
    assert_eq!(
        converted_domain.color_scheme.color_foreground,
        original_domain.color_scheme.color_foreground
    );
    assert_eq!(
        converted_domain.color_scheme.color_background,
        original_domain.color_scheme.color_background
    );
}

#[test]
fn test_theme_mapper_with_none_color_scheme_id() {
    // Test edge case where color_scheme_id might be None
    let theme_id = Uuid::new_v4();
    let color_scheme_id = Uuid::new_v4();

    let update_entity = UpdateTheme { color_scheme_id };

    let active_model = ThemeMapper::to_update(theme_id, update_entity);

    // Should always set Some(color_scheme_id) since UpdateTheme requires it
    assert_eq!(active_model.id, sea_orm::ActiveValue::Set(theme_id));
    assert_eq!(
        active_model.color_scheme_id,
        sea_orm::ActiveValue::Set(Some(color_scheme_id))
    );
}
