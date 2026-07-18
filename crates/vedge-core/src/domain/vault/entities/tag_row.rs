use crate::domain::shared::{TagId, Timestamp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagRow {
    pub id: TagId,
    pub nonce: [u8; 24],
    pub ciphertext: Vec<u8>,
    /// The per-row DEK, wrapped under the vault KEK (AES-256 Key Wrap, 40 bytes).
    /// `None` = a LEGACY tag sealed directly under the KEK (pre-5.6.0), whose
    /// `ciphertext` is under the KEK itself; it is migrated to a DEK on the next
    /// unlock. `Some` = a DEK-sealed tag: `ciphertext` is under the wrapped DEK.
    pub dek_wrapped: Option<[u8; 40]>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
