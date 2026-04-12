use vedge_core::business::domain::entities::vault_item::{
    UpdateVaultItem, VaultItem, VaultItemData, VaultItemDataCredential, VaultItemKind,
};
use vedge_core::infra::data::sqlite::entities::vault::vault_item;
use vedge_core::infra::data::sqlite::mappers::db_vault::VaultMapper;
use chrono::Utc;
use uuid::Uuid;

#[test]
fn test_vault_mapper_to_domain() {
    // Arrange
    let id = Uuid::new_v4();
    let now = Utc::now();
    let credential_data = VaultItemDataCredential {
        identifier: "test@example.com".to_string(),
        password: "password123".to_string(),
        url: "https://example.com".to_string(),
    };
    let json_data =
        serde_json::to_string(&VaultItemData::Credential(credential_data.clone())).unwrap();

    let db_model = vault_item::Model {
        id,
        title: "Test Credential".to_string(),
        kind: "Credential".to_string(),
        data: json_data,
        created_at: now,
        updated_at: now,
    };

    // Act
    let domain_entity = VaultMapper::to_domain(db_model);

    // Assert
    assert_eq!(domain_entity.id, id);
    assert_eq!(domain_entity.title, "Test Credential");
    assert_eq!(domain_entity.kind, VaultItemKind::Credential);
    assert!(domain_entity.created_at.is_some());
    assert!(domain_entity.updated_at.is_some());

    match domain_entity.data {
        VaultItemData::Credential(cred) => {
            assert_eq!(cred.identifier, "test@example.com");
            assert_eq!(cred.password, "password123");
            assert_eq!(cred.url, "https://example.com");
        }
    }
}

#[test]
fn test_vault_mapper_to_persistence() {
    // Arrange
    let id = Uuid::new_v4();
    let credential_data = VaultItemDataCredential {
        identifier: "test@example.com".to_string(),
        password: "password123".to_string(),
        url: "https://example.com".to_string(),
    };

    let domain_entity = VaultItem {
        id,
        title: "Test Credential".to_string(),
        kind: VaultItemKind::Credential,
        data: VaultItemData::Credential(credential_data),
        created_at: Some("2023-01-01T00:00:00Z".to_string()),
        updated_at: Some("2023-01-01T00:00:00Z".to_string()),
    };

    // Act
    let active_model = VaultMapper::to_persistence(domain_entity);

    // Assert
    assert_eq!(active_model.id, sea_orm::ActiveValue::Set(id));
    assert_eq!(
        active_model.title,
        sea_orm::ActiveValue::Set("Test Credential".to_string())
    );
    assert_eq!(
        active_model.kind,
        sea_orm::ActiveValue::Set("Credential".to_string())
    );

    // Check that data is properly serialized
    match &active_model.data {
        sea_orm::ActiveValue::Set(data_str) => {
            let parsed_data: VaultItemData = serde_json::from_str(data_str).unwrap();
            match parsed_data {
                VaultItemData::Credential(cred) => {
                    assert_eq!(cred.identifier, "test@example.com");
                    assert_eq!(cred.password, "password123");
                    assert_eq!(cred.url, "https://example.com");
                }
            }
        }
        _ => panic!("Expected Set value for data"),
    }

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
fn test_vault_mapper_to_persistence_with_none_timestamps() {
    // Arrange
    let id = Uuid::new_v4();
    let credential_data = VaultItemDataCredential {
        identifier: "test@example.com".to_string(),
        password: "password123".to_string(),
        url: "https://example.com".to_string(),
    };

    let domain_entity = VaultItem {
        id,
        title: "Test Credential".to_string(),
        kind: VaultItemKind::Credential,
        data: VaultItemData::Credential(credential_data),
        created_at: None,
        updated_at: None,
    };

    // Act
    let active_model = VaultMapper::to_persistence(domain_entity);

    // Assert
    assert_eq!(active_model.id, sea_orm::ActiveValue::Set(id));
    assert_eq!(
        active_model.title,
        sea_orm::ActiveValue::Set("Test Credential".to_string())
    );
    assert_eq!(
        active_model.kind,
        sea_orm::ActiveValue::Set("Credential".to_string())
    );

    // Check that timestamps are set (should use current time when None)
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
fn test_vault_mapper_to_update() {
    // Arrange
    let id = Uuid::new_v4();
    let credential_data = VaultItemDataCredential {
        identifier: "updated@example.com".to_string(),
        password: "newpassword123".to_string(),
        url: "https://updated.com".to_string(),
    };

    let update_entity = UpdateVaultItem {
        title: "Updated Credential".to_string(),
        kind: VaultItemKind::Credential,
        data: VaultItemData::Credential(credential_data),
    };

    // Act
    let active_model = VaultMapper::to_update(id, update_entity);

    // Assert
    assert_eq!(active_model.id, sea_orm::ActiveValue::Set(id));
    assert_eq!(
        active_model.title,
        sea_orm::ActiveValue::Set("Updated Credential".to_string())
    );
    assert_eq!(
        active_model.kind,
        sea_orm::ActiveValue::Set("Credential".to_string())
    );

    // Check that data is properly serialized
    match &active_model.data {
        sea_orm::ActiveValue::Set(data_str) => {
            let parsed_data: VaultItemData = serde_json::from_str(data_str).unwrap();
            match parsed_data {
                VaultItemData::Credential(cred) => {
                    assert_eq!(cred.identifier, "updated@example.com");
                    assert_eq!(cred.password, "newpassword123");
                    assert_eq!(cred.url, "https://updated.com");
                }
            }
        }
        _ => panic!("Expected Set value for data"),
    }

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
fn test_vault_mapper_roundtrip() {
    // Arrange
    let id = Uuid::new_v4();
    let credential_data = VaultItemDataCredential {
        identifier: "roundtrip@example.com".to_string(),
        password: "roundtrip123".to_string(),
        url: "https://roundtrip.com".to_string(),
    };

    let original_domain = VaultItem {
        id,
        title: "Roundtrip Test".to_string(),
        kind: VaultItemKind::Credential,
        data: VaultItemData::Credential(credential_data.clone()),
        created_at: Some("2023-01-01T00:00:00Z".to_string()),
        updated_at: Some("2023-01-01T00:00:00Z".to_string()),
    };

    // Act
    let _active_model = VaultMapper::to_persistence(original_domain.clone());

    // Create a mock database model with the same data
    let now = Utc::now();
    let json_data = serde_json::to_string(&VaultItemData::Credential(credential_data)).unwrap();
    let db_model = vault_item::Model {
        id: original_domain.id,
        title: original_domain.title.clone(),
        kind: original_domain.kind.to_string(),
        data: json_data,
        created_at: now,
        updated_at: now,
    };

    let converted_domain = VaultMapper::to_domain(db_model);

    // Assert
    assert_eq!(converted_domain.id, original_domain.id);
    assert_eq!(converted_domain.title, original_domain.title);
    assert_eq!(converted_domain.kind, original_domain.kind);

    match (converted_domain.data, original_domain.data) {
        (VaultItemData::Credential(converted_cred), VaultItemData::Credential(original_cred)) => {
            assert_eq!(converted_cred.identifier, original_cred.identifier);
            assert_eq!(converted_cred.password, original_cred.password);
            assert_eq!(converted_cred.url, original_cred.url);
        }
    }
}

#[test]
fn test_vault_mapper_json_serialization() {
    // Test that JSON serialization/deserialization works correctly
    let credential_data = VaultItemDataCredential {
        identifier: "json@example.com".to_string(),
        password: "json123".to_string(),
        url: "https://json.com".to_string(),
    };

    let vault_data = VaultItemData::Credential(credential_data);

    // Serialize to JSON
    let json_str = serde_json::to_string(&vault_data).unwrap();

    // Deserialize back
    let parsed_data: VaultItemData = serde_json::from_str(&json_str).unwrap();

    match parsed_data {
        VaultItemData::Credential(cred) => {
            assert_eq!(cred.identifier, "json@example.com");
            assert_eq!(cred.password, "json123");
            assert_eq!(cred.url, "https://json.com");
        }
    }
}

#[test]
fn test_vault_mapper_vault_item_kind_conversion() {
    // Test VaultItemKind string conversion
    assert_eq!(VaultItemKind::Credential.to_string(), "Credential");

    // Test parsing from string
    let kind = VaultItemKind::try_from("Credential").unwrap();
    assert_eq!(kind, VaultItemKind::Credential);

    // Test invalid string
    let result = VaultItemKind::try_from("Invalid");
    assert!(result.is_err());
}
