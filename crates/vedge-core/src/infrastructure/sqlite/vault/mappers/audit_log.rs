use crate::domain::shared::{DeviceId, EntryId};
use crate::domain::vault::entities::{AuditAction, AuditEvent};
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::sqlite::vault::entities::audit_log::Model;

use super::{string_to_ts, ts_to_string};

pub fn model_to_domain(model: Model) -> Result<AuditEvent, VaultError> {
    let action = AuditAction::parse(&model.action)
        .ok_or_else(|| VaultError::UnknownAuditAction(model.action.clone()))?;
    Ok(AuditEvent {
        id: model.id,
        entry_id: model.entry_id.map(EntryId::from_raw),
        action,
        occurred_at: string_to_ts(&model.occurred_at)?,
        device_id: model.device_id.map(DeviceId::from_raw),
    })
}

pub fn domain_to_model(event: &AuditEvent) -> Model {
    Model {
        id: event.id.clone(),
        entry_id: event.entry_id.as_ref().map(|e| e.as_str().to_owned()),
        action: event.action.as_str().to_owned(),
        occurred_at: ts_to_string(&event.occurred_at),
        device_id: event.device_id.as_ref().map(|d| d.as_str().to_owned()),
    }
}
