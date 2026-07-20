use crate::domain::app::entities::AppSetting;
use crate::domain::shared::StorageError;
use crate::infrastructure::sqlite::app::entities::app_setting::Model;

use super::{string_to_ts, ts_to_string};

pub fn model_to_domain(model: Model) -> Result<AppSetting, StorageError> {
    let value = serde_json::from_str(&model.value)
        .map_err(|e| StorageError::Serialization(format!("app_setting.value: {e}")))?;
    Ok(AppSetting {
        key: model.key,
        value,
        updated_at: string_to_ts(&model.updated_at)?,
    })
}

pub fn domain_to_model(setting: &AppSetting) -> Result<Model, StorageError> {
    let value = serde_json::to_string(&setting.value)
        .map_err(|e| StorageError::Serialization(format!("app_setting.value: {e}")))?;
    Ok(Model {
        key: setting.key.clone(),
        value,
        updated_at: ts_to_string(&setting.updated_at),
    })
}
