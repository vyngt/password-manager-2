use crate::domain::shared::{TagId, Timestamp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagRow {
    pub id: TagId,
    pub nonce: [u8; 24],
    pub ciphertext: Vec<u8>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
