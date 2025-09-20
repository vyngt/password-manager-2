use crate::business::domain::entities::vault_item::{
    VaultItem, VaultItemKind, VaultItemUpdate,
};
use crate::infra::data::sqlite::entities::vault::vault_item;
use chrono::{DateTime, Utc};
use sea_orm::ActiveValue::Set;
use serde_json;
use uuid::Uuid;

pub struct VaultMapper;

impl VaultMapper {
    pub fn to_domain(data: vault_item::Model) -> VaultItem {
        VaultItem {
            id: data.id,
            title: data.title,
            kind: VaultItemKind::try_from(data.kind.as_str()).unwrap(),
            data: serde_json::from_str(&data.data).unwrap(),
            created_at: Some(data.created_at.to_rfc3339()),
            updated_at: Some(data.updated_at.to_rfc3339()),
        }
    }

    pub fn to_persistence(data: VaultItem) -> vault_item::ActiveModel {
        vault_item::ActiveModel {
            id: Set(data.id),
            title: Set(data.title),
            kind: Set(data.kind.to_string()),
            data: Set(serde_json::to_string(&data.data).unwrap()),
            created_at: Set(data
                .created_at
                .map(|v| {
                    DateTime::parse_from_rfc3339(&v)
                        .unwrap()
                        .with_timezone(&Utc)
                })
                .unwrap_or(Utc::now())),
            updated_at: Set(Utc::now()),
        }
    }

    pub fn to_update(id: Uuid, data: VaultItemUpdate) -> vault_item::ActiveModel {
        vault_item::ActiveModel {
            id: Set(id),
            title: Set(data.title),
            kind: Set(data.kind.to_string()),
            data: Set(serde_json::to_string(&data.data).unwrap()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
    }
}
