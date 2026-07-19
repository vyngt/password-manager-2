use crate::domain::shared::StorageError;
use crate::domain::vault::entities::VaultConfig;
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::kdf_params::KdfParams;
use crate::infrastructure::sqlite::vault::entities::vault_config::Model;

use super::{fixed_bytes, string_to_ts, string_to_ts_opt, ts_to_string};

pub fn model_to_domain(model: Model) -> Result<VaultConfig, VaultError> {
    let kdf_params: KdfParams = serde_json::from_str(&model.kdf_params)
        .map_err(|e| VaultError::InvalidKdfParams(e.to_string()))?;

    Ok(VaultConfig {
        id: model.id,
        magic: model.magic,
        schema_version: model.schema_version,
        vault_salt: fixed_bytes::<32>(&model.vault_salt, "vault_config.vault_salt")?,
        kdf_params,
        verify_hash: fixed_bytes::<32>(&model.verify_hash, "vault_config.verify_hash")?,
        preferred_cipher_suite: model.preferred_cipher_suite,
        trash_retention_days: model.trash_retention_days,
        audit_retention_days: model.audit_retention_days,
        created_at: string_to_ts(&model.created_at)?,
        last_unlocked_at: string_to_ts_opt(model.last_unlocked_at.as_deref())?,
        vault_uuid: model.vault_uuid,
        commit_counter: model.commit_counter,
        backup_dir: model.backup_dir,
        backup_keep_count: model.backup_keep_count,
        last_snapshot_at: string_to_ts_opt(model.last_snapshot_at.as_deref())?,
        last_backup_at: string_to_ts_opt(model.last_backup_at.as_deref())?,
        recovery_slot: model
            .recovery_slot
            .as_deref()
            .map(|b| fixed_bytes::<40>(b, "vault_config.recovery_slot"))
            .transpose()?,
    })
}

pub fn domain_to_model(config: &VaultConfig) -> Result<Model, VaultError> {
    let kdf_params = serde_json::to_string(&config.kdf_params).map_err(|e| {
        VaultError::Storage(StorageError::Serialization(format!("kdf_params: {e}")))
    })?;
    Ok(Model {
        id: config.id.clone(),
        magic: config.magic.clone(),
        schema_version: config.schema_version,
        vault_salt: config.vault_salt.to_vec(),
        kdf_params,
        verify_hash: config.verify_hash.to_vec(),
        preferred_cipher_suite: config.preferred_cipher_suite,
        trash_retention_days: config.trash_retention_days,
        audit_retention_days: config.audit_retention_days,
        created_at: ts_to_string(&config.created_at),
        last_unlocked_at: config.last_unlocked_at.as_ref().map(ts_to_string),
        vault_uuid: config.vault_uuid.clone(),
        commit_counter: config.commit_counter,
        backup_dir: config.backup_dir.clone(),
        backup_keep_count: config.backup_keep_count,
        last_snapshot_at: config.last_snapshot_at.as_ref().map(ts_to_string),
        last_backup_at: config.last_backup_at.as_ref().map(ts_to_string),
        recovery_slot: config.recovery_slot.map(|a| a.to_vec()),
    })
}
