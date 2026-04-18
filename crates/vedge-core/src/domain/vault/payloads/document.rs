use serde::{Deserialize, Serialize};

use crate::domain::vault::crypto_constants::NONCE_LEN;
use crate::domain::vault::payloads::common_meta::CommonMeta;
use crate::domain::vault::payloads::serde_nonce;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentPayload {
    #[serde(flatten)]
    pub meta: CommonMeta,

    pub filename: String,
    pub mime_type: String,
    pub size_bytes: u64,

    /// Nonce used to encrypt the external `.vedge_blobs/{entry_id}.blob` file.
    /// Serialized as base64. Not a secret — just needs to be unique per write.
    #[serde(with = "serde_nonce")]
    pub blob_nonce: [u8; NONCE_LEN],
}
