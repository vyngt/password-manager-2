use vedge_core::business::domain::entities::theme::{ColorScheme, UpdateColorScheme};
use vedge_core::infra::data::sqlite::entities::theme::color_scheme;
use vedge_core::infra::data::sqlite::mappers::db_theme::ColorSchemeMapper;
use chrono::Utc;
use uuid::Uuid;

#[test]
fn test_color_scheme_mapper_to_domain() {
    // Arrange
    let id = Uuid::new_v4();
    let now = Utc::now();
    let db_model = color_scheme::Model {
        id,
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
    let domain_entity = ColorSchemeMapper::to_domain(db_model);

    // Assert
    assert_eq!(domain_entity.id, id);
    assert_eq!(domain_entity.name, "Test Theme");
    assert_eq!(domain_entity.color_primary, "#3B82F6");
    assert_eq!(domain_entity.color_secondary, "#6B7280");
    assert_eq!(domain_entity.color_success, "#10B981");
    assert_eq!(domain_entity.color_danger, "#EF4444");
    assert_eq!(domain_entity.color_warning, "#F59E0B");
    assert_eq!(domain_entity.color_foreground, "#1F2937");
    assert_eq!(domain_entity.color_background, "#FFFFFF");
}

#[test]
fn test_color_scheme_mapper_to_persistence() {
    // Arrange
    let id = Uuid::new_v4();
    let domain_entity = ColorScheme {
        id,
        name: "Test Theme".to_string(),
        color_primary: "#3B82F6".to_string(),
        color_secondary: "#6B7280".to_string(),
        color_success: "#10B981".to_string(),
        color_danger: "#EF4444".to_string(),
        color_warning: "#F59E0B".to_string(),
        color_foreground: "#1F2937".to_string(),
        color_background: "#FFFFFF".to_string(),
    };

    // Act
    let active_model = ColorSchemeMapper::to_persistence(domain_entity);

    // Assert
    assert_eq!(active_model.id, sea_orm::ActiveValue::Set(id));
    assert_eq!(
        active_model.name,
        sea_orm::ActiveValue::Set("Test Theme".to_string())
    );
    assert_eq!(
        active_model.color_primary,
        sea_orm::ActiveValue::Set("#3B82F6".to_string())
    );
    assert_eq!(
        active_model.color_secondary,
        sea_orm::ActiveValue::Set("#6B7280".to_string())
    );
    assert_eq!(
        active_model.color_success,
        sea_orm::ActiveValue::Set("#10B981".to_string())
    );
    assert_eq!(
        active_model.color_danger,
        sea_orm::ActiveValue::Set("#EF4444".to_string())
    );
    assert_eq!(
        active_model.color_warning,
        sea_orm::ActiveValue::Set("#F59E0B".to_string())
    );
    assert_eq!(
        active_model.color_foreground,
        sea_orm::ActiveValue::Set("#1F2937".to_string())
    );
    assert_eq!(
        active_model.color_background,
        sea_orm::ActiveValue::Set("#FFFFFF".to_string())
    );

    // Check that timestamps are set
    assert!(matches!(
        active_model.created_at,
        sea_orm::ActiveValue::Set(_)
    ));
    assert!(matches!(
        active_model.updated_at,
        sea_orm::ActiveValue::Set(_)
    ));
}

#[test]
fn test_color_scheme_mapper_to_update() {
    // Arrange
    let id = Uuid::new_v4();
    let update_entity = UpdateColorScheme {
        name: "Updated Theme".to_string(),
        color_primary: "#60A5FA".to_string(),
        color_secondary: "#9CA3AF".to_string(),
        color_success: "#34D399".to_string(),
        color_danger: "#F87171".to_string(),
        color_warning: "#FBBF24".to_string(),
        color_foreground: "#F9FAFB".to_string(),
        color_background: "#111827".to_string(),
    };

    // Act
    let active_model = ColorSchemeMapper::to_update(id, update_entity);

    // Assert
    assert_eq!(active_model.id, sea_orm::ActiveValue::Set(id));
    assert_eq!(
        active_model.name,
        sea_orm::ActiveValue::Set("Updated Theme".to_string())
    );
    assert_eq!(
        active_model.color_primary,
        sea_orm::ActiveValue::Set("#60A5FA".to_string())
    );
    assert_eq!(
        active_model.color_secondary,
        sea_orm::ActiveValue::Set("#9CA3AF".to_string())
    );
    assert_eq!(
        active_model.color_success,
        sea_orm::ActiveValue::Set("#34D399".to_string())
    );
    assert_eq!(
        active_model.color_danger,
        sea_orm::ActiveValue::Set("#F87171".to_string())
    );
    assert_eq!(
        active_model.color_warning,
        sea_orm::ActiveValue::Set("#FBBF24".to_string())
    );
    assert_eq!(
        active_model.color_foreground,
        sea_orm::ActiveValue::Set("#F9FAFB".to_string())
    );
    assert_eq!(
        active_model.color_background,
        sea_orm::ActiveValue::Set("#111827".to_string())
    );

    // Check that updated_at is set but created_at is not (for updates)
    assert!(matches!(
        active_model.updated_at,
        sea_orm::ActiveValue::Set(_)
    ));
    assert!(matches!(
        active_model.created_at,
        sea_orm::ActiveValue::NotSet
    ));
}

#[test]
fn test_color_scheme_mapper_roundtrip() {
    // Arrange
    let id = Uuid::new_v4();
    let original_domain = ColorScheme {
        id,
        name: "Roundtrip Test".to_string(),
        color_primary: "#8B5CF6".to_string(),
        color_secondary: "#A78BFA".to_string(),
        color_success: "#10B981".to_string(),
        color_danger: "#F59E0B".to_string(),
        color_warning: "#EF4444".to_string(),
        color_foreground: "#1F2937".to_string(),
        color_background: "#F3F4F6".to_string(),
    };

    // Act
    let _active_model = ColorSchemeMapper::to_persistence(original_domain.clone());

    // Create a mock database model with the same data
    let now = Utc::now();
    let db_model = color_scheme::Model {
        id: original_domain.id,
        name: original_domain.name.clone(),
        color_primary: original_domain.color_primary.clone(),
        color_secondary: original_domain.color_secondary.clone(),
        color_success: original_domain.color_success.clone(),
        color_danger: original_domain.color_danger.clone(),
        color_warning: original_domain.color_warning.clone(),
        color_foreground: original_domain.color_foreground.clone(),
        color_background: original_domain.color_background.clone(),
        created_at: now,
        updated_at: now,
    };

    let converted_domain = ColorSchemeMapper::to_domain(db_model);

    // Assert
    assert_eq!(converted_domain.id, original_domain.id);
    assert_eq!(converted_domain.name, original_domain.name);
    assert_eq!(
        converted_domain.color_primary,
        original_domain.color_primary
    );
    assert_eq!(
        converted_domain.color_secondary,
        original_domain.color_secondary
    );
    assert_eq!(
        converted_domain.color_success,
        original_domain.color_success
    );
    assert_eq!(converted_domain.color_danger, original_domain.color_danger);
    assert_eq!(
        converted_domain.color_warning,
        original_domain.color_warning
    );
    assert_eq!(
        converted_domain.color_foreground,
        original_domain.color_foreground
    );
    assert_eq!(
        converted_domain.color_background,
        original_domain.color_background
    );
}
