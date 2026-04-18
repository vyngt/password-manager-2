use crate::domain::shared::EntryId;
use crate::domain::vault::entities::EntryRow;
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::sqlite::vault::entities::entry::Model;

use super::{fixed_bytes, string_to_ts, string_to_ts_opt, ts_to_string};

pub fn model_to_domain(model: Model) -> Result<EntryRow, VaultError> {
    Ok(EntryRow {
        id: EntryId::from_raw(model.id),
        version: model.version,
        cipher_suite: model.cipher_suite,
        dek_wrapped: fixed_bytes::<40>(&model.dek_wrapped, "entry.dek_wrapped")?,
        nonce: fixed_bytes::<24>(&model.nonce, "entry.nonce")?,
        ciphertext: model.ciphertext,
        created_at: string_to_ts(&model.created_at)?,
        updated_at: string_to_ts(&model.updated_at)?,
        accessed_at: string_to_ts_opt(model.accessed_at.as_deref())?,
        is_trashed: model.is_trashed != 0,
        trashed_at: string_to_ts_opt(model.trashed_at.as_deref())?,
    })
}

pub fn domain_to_model(row: &EntryRow) -> Model {
    Model {
        id: row.id.as_str().to_owned(),
        version: row.version,
        cipher_suite: row.cipher_suite,
        dek_wrapped: row.dek_wrapped.to_vec(),
        nonce: row.nonce.to_vec(),
        ciphertext: row.ciphertext.clone(),
        created_at: ts_to_string(&row.created_at),
        updated_at: ts_to_string(&row.updated_at),
        accessed_at: row.accessed_at.as_ref().map(ts_to_string),
        is_trashed: i32::from(row.is_trashed),
        trashed_at: row.trashed_at.as_ref().map(ts_to_string),
    }
}
