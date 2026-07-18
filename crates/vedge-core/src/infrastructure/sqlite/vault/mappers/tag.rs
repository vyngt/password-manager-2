use crate::domain::shared::TagId;
use crate::domain::vault::entities::TagRow;
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::sqlite::vault::entities::tag::Model;

use super::{fixed_bytes, string_to_ts, ts_to_string};

pub fn model_to_domain(model: Model) -> Result<TagRow, VaultError> {
    Ok(TagRow {
        id: TagId::from_raw(model.id),
        nonce: fixed_bytes::<24>(&model.nonce, "tag.nonce")?,
        ciphertext: model.ciphertext,
        dek_wrapped: model
            .dek_wrapped
            .map(|v| fixed_bytes::<40>(&v, "tag.dek_wrapped"))
            .transpose()?,
        created_at: string_to_ts(&model.created_at)?,
        updated_at: string_to_ts(&model.updated_at)?,
    })
}

#[must_use]
pub fn domain_to_model(tag: &TagRow) -> Model {
    Model {
        id: tag.id.as_str().to_owned(),
        nonce: tag.nonce.to_vec(),
        ciphertext: tag.ciphertext.clone(),
        dek_wrapped: tag.dek_wrapped.map(|w| w.to_vec()),
        created_at: ts_to_string(&tag.created_at),
        updated_at: ts_to_string(&tag.updated_at),
    }
}
