use crate::domain::shared::EntryId;
use crate::domain::vault::entities::EntryHistoryRow;
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::sqlite::vault::entities::entry_history::Model;

use super::{fixed_bytes, string_to_ts, ts_to_string};

pub fn model_to_domain(model: Model) -> Result<EntryHistoryRow, VaultError> {
    Ok(EntryHistoryRow {
        id: model.id,
        entry_id: EntryId::from_raw(model.entry_id),
        version: model.version,
        cipher_suite: model.cipher_suite,
        nonce: fixed_bytes::<24>(&model.nonce, "entry_history.nonce")?,
        ciphertext: model.ciphertext,
        changed_at: string_to_ts(&model.changed_at)?,
    })
}

#[must_use]
pub fn domain_to_model(row: &EntryHistoryRow) -> Model {
    Model {
        id: row.id.clone(),
        entry_id: row.entry_id.as_str().to_owned(),
        version: row.version,
        cipher_suite: row.cipher_suite,
        nonce: row.nonce.to_vec(),
        ciphertext: row.ciphertext.clone(),
        changed_at: ts_to_string(&row.changed_at),
    }
}
