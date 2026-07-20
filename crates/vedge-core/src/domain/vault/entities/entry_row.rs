use crate::domain::shared::{EntryId, Timestamp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryRow {
    pub id: EntryId,
    pub version: i64,
    pub cipher_suite: i32,
    pub dek_wrapped: [u8; 40],
    pub nonce: [u8; 24],
    pub ciphertext: Vec<u8>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub accessed_at: Option<Timestamp>,
    pub is_trashed: bool,
    pub trashed_at: Option<Timestamp>,
}
