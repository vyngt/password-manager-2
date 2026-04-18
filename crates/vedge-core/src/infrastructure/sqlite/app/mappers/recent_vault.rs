use std::path::PathBuf;

use crate::domain::app::entities::RecentVault;
use crate::domain::shared::StorageError;
use crate::infrastructure::sqlite::app::entities::recent_vault::Model;

use super::{string_to_ts_opt, ts_to_string};

pub fn model_to_domain(model: Model) -> Result<RecentVault, StorageError> {
    Ok(RecentVault {
        id: model.id,
        path: PathBuf::from(model.path),
        display_name: model.display_name,
        last_opened: string_to_ts_opt(model.last_opened.as_deref())?,
        sort_order: model.sort_order,
    })
}

pub fn domain_to_model(vault: &RecentVault) -> Model {
    Model {
        id: vault.id.clone(),
        path: vault.path.to_string_lossy().into_owned(),
        display_name: vault.display_name.clone(),
        last_opened: vault.last_opened.as_ref().map(ts_to_string),
        sort_order: vault.sort_order,
    }
}
