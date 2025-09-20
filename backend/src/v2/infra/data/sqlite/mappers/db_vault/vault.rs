use crate::v2::business::domain::entities::vault_record::{VaultRecord, VaultRecordUpdate};
use crate::v2::infra::data::sqlite::entities::vault::vault;
use chrono::{DateTime, Utc};
use sea_orm::ActiveValue::Set;
use uuid::Uuid;

pub struct VaultMapper;

impl VaultMapper {
    pub fn to_domain(data: vault::Model) -> VaultRecord {
        VaultRecord {
            id: data.id,
            name: data.name,
            url: data.url,
            login: data.login,
            key_pass: data.key_pass,
            created_at: Some(data.created_at.to_rfc3339()),
            updated_at: Some(data.updated_at.to_rfc3339()),
        }
    }

    pub fn to_persistence(data: VaultRecord) -> vault::ActiveModel {
        vault::ActiveModel {
            id: Set(data.id),
            name: Set(data.name),
            url: Set(data.url),
            login: Set(data.login),
            key_pass: Set(data.key_pass),
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

    pub fn to_update(id: Uuid, data: VaultRecordUpdate) -> vault::ActiveModel {
        vault::ActiveModel {
            id: Set(id),
            name: Set(data.name),
            url: Set(data.url),
            login: Set(data.login),
            key_pass: Set(data.key_pass),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
    }
}
