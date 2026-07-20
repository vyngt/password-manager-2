use ulid::Ulid;

use crate::domain::shared::{EntryId, TagId};
use crate::domain::vault::crypto_constants::BLOB_AAD_SUFFIX;
use crate::domain::vault::errors::VaultError;

fn entry_ulid_bytes(id: &EntryId) -> Result<[u8; 16], VaultError> {
    Ulid::from_string(id.as_str())
        .map(|u| u.to_bytes())
        .map_err(|e| VaultError::InvalidEntryId(format!("{e}")))
}

fn tag_ulid_bytes(id: &TagId) -> Result<[u8; 16], VaultError> {
    Ulid::from_string(id.as_str())
        .map(|u| u.to_bytes())
        .map_err(|e| VaultError::InvalidTagId(format!("{e}")))
}

/// AAD for an entry ciphertext: entry ULID (16 bytes) || version as u64-LE (8 bytes).
pub fn entry_aad(id: &EntryId, version: i64) -> Result<Vec<u8>, VaultError> {
    // Version is monotonic and non-negative by construction (starts at 1, only
    // increases). A negative value means the DB row is corrupt.
    let version_u64 = u64::try_from(version).map_err(|_| {
        VaultError::MalformedPayload(format!("entry version must be non-negative: {version}"))
    })?;
    let mut aad = Vec::with_capacity(24);
    aad.extend_from_slice(&entry_ulid_bytes(id)?);
    aad.extend_from_slice(&version_u64.to_le_bytes());
    Ok(aad)
}

/// AAD for a tag ciphertext: tag ULID (16 bytes).
pub fn tag_aad(id: &TagId) -> Result<Vec<u8>, VaultError> {
    Ok(tag_ulid_bytes(id)?.to_vec())
}

/// AAD for a document blob: entry ULID (16 bytes) || b"blob" (4 bytes).
/// Distinct from entry AAD to prevent cross-context ciphertext substitution.
pub fn blob_aad(id: &EntryId) -> Result<Vec<u8>, VaultError> {
    let mut aad = Vec::with_capacity(20);
    aad.extend_from_slice(&entry_ulid_bytes(id)?);
    aad.extend_from_slice(BLOB_AAD_SUFFIX);
    Ok(aad)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_entry_id() -> EntryId {
        // Known-good ULID for deterministic tests.
        EntryId::from_raw("01ARZ3NDEKTSV4RRFFQ69G5FAV")
    }

    fn fixed_tag_id() -> TagId {
        TagId::from_raw("01H0000000000000000000TAG1")
    }

    #[test]
    fn entry_aad_is_24_bytes_and_version_sensitive() {
        let id = fixed_entry_id();
        let a = entry_aad(&id, 1).unwrap();
        let b = entry_aad(&id, 2).unwrap();
        assert_eq!(a.len(), 24);
        assert_eq!(b.len(), 24);
        assert_eq!(a[..16], b[..16]);
        assert_ne!(a[16..], b[16..]);
    }

    #[test]
    fn entry_aad_ulid_prefix_matches_little_endian_version() {
        let id = fixed_entry_id();
        let a = entry_aad(&id, 7).unwrap();
        assert_eq!(&a[16..], &7u64.to_le_bytes());
    }

    #[test]
    fn tag_aad_is_16_bytes() {
        let aad = tag_aad(&fixed_tag_id()).unwrap();
        assert_eq!(aad.len(), 16);
    }

    #[test]
    fn blob_aad_is_20_bytes_with_suffix() {
        let aad = blob_aad(&fixed_entry_id()).unwrap();
        assert_eq!(aad.len(), 20);
        assert_eq!(&aad[16..], BLOB_AAD_SUFFIX);
    }

    #[test]
    fn blob_aad_differs_from_entry_aad() {
        let id = fixed_entry_id();
        let entry = entry_aad(&id, 1).unwrap();
        let blob = blob_aad(&id).unwrap();
        assert_ne!(entry, blob);
    }

    #[test]
    fn malformed_id_returns_error() {
        let bad = EntryId::from_raw("not-a-ulid");
        let err = entry_aad(&bad, 1).unwrap_err();
        assert!(matches!(err, VaultError::InvalidEntryId(_)));
    }
}
