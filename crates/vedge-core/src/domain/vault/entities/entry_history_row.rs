use crate::domain::shared::{EntryId, Timestamp};

use super::EntryRow;

/// A superseded version of an entry's encrypted payload.
///
/// Stores **only** the ciphertext plus the metadata needed to decrypt it — and
/// deliberately **no key material of its own**. A snapshot is decrypted by
/// unwrapping the DEK from the *live* `entries` row: the per-entry DEK is stable
/// for the entry's whole life, and `change_password` re-wraps it O(1) without
/// ever rewriting ciphertext, so snapshots stay decryptable across a
/// master-password change. Storing a per-snapshot `dek_wrapped` would silently
/// brick history on the next password change — so we don't.
///
/// AAD binds each snapshot to `entry_aad(entry_id, version)` using the
/// snapshot's *own* version, so it authenticates exactly as it did when live and
/// cannot be replayed as a different entry or version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryHistoryRow {
    /// ULID primary key for the snapshot row.
    pub id: String,
    /// The entry this snapshot belongs to (link to `entries.id`).
    pub entry_id: EntryId,
    /// The entry's version at the moment this payload was superseded.
    pub version: i64,
    pub cipher_suite: i32,
    pub nonce: [u8; 24],
    pub ciphertext: Vec<u8>,
    /// When this version became current (the superseded row's `updated_at`).
    pub changed_at: Timestamp,
}

impl EntryHistoryRow {
    /// Snapshot the pre-update row *before* it is overwritten. A pure byte-copy
    /// — the existing ciphertext/nonce are carried over verbatim (no decrypt at
    /// capture). `changed_at` is the superseded row's `updated_at`: the moment
    /// this version became current.
    #[must_use]
    pub fn from_superseded(existing: &EntryRow) -> Self {
        Self {
            id: ulid::Ulid::new().to_string(),
            entry_id: existing.id.clone(),
            version: existing.version,
            cipher_suite: existing.cipher_suite,
            nonce: existing.nonce,
            ciphertext: existing.ciphertext.clone(),
            changed_at: existing.updated_at,
        }
    }
}
