//! Per-row tag DEK crypto (slice 5.6.0).
//!
//! Tags used to be sealed directly under the vault KEK; a password change then left
//! them under the old KEK and bricked the vault. They now carry a per-row DEK wrapped
//! under the KEK, exactly like entries. These are the shared primitives for that:
//!
//! - [`seal_tag_payload`] — the WRITE side: mint a DEK, seal the plaintext under it,
//!   wrap the DEK under the KEK. Used by create/rename and by every rewrap. **No write
//!   path uses the KEK as an AEAD key** — the KEK only wraps the DEK here.
//! - [`open_tag_row`] — the READ side, dual-path: a DEK-sealed row (`dek_wrapped =
//!   Some`) unwraps its DEK and decrypts under it; a LEGACY row (`None`) decrypts
//!   directly under the KEK via the one surviving [`CryptoProvider::decrypt_legacy_tag`]
//!   door. This is the ONLY place the KEK is used as an AEAD key, and only to READ.
//! - [`rewrap_tag_row`] — move a row from `old_kek` to `new_kek`, always producing a
//!   DEK-sealed row so a caller can never leave a tag under the old KEK.

use zeroize::Zeroizing;

use crate::application::vault::ports::crypto::{CryptoProvider, Nonce};
use crate::domain::vault::aad::tag_aad;
use crate::domain::vault::crypto_constants::{DEK_WRAPPED_LEN, KEK_LEN};
use crate::domain::vault::entities::TagRow;
use crate::domain::vault::errors::VaultError;

/// Seal a tag payload under a fresh per-row DEK; return `(nonce, ciphertext, wrapped_dek)`.
/// The DEK is generated, used, and dropped (zeroized) inside this frame — it never leaves
/// except as the KEK-wrapped `dek_wrapped`.
pub(crate) fn seal_tag_payload(
    crypto: &dyn CryptoProvider,
    kek: &[u8; KEK_LEN],
    aad: &[u8],
    plaintext: &[u8],
) -> Result<(Nonce, Vec<u8>, [u8; DEK_WRAPPED_LEN]), VaultError> {
    let dek = crypto.generate_dek();
    let (nonce, ciphertext) = crypto.encrypt_entry(&dek, plaintext, aad)?;
    let wrapped = crypto.wrap_dek(&dek, kek)?;
    Ok((nonce, ciphertext, wrapped))
}

/// Decrypt a tag row's payload under `kek`, choosing the path from `dek_wrapped`:
/// `Some` → unwrap the DEK and decrypt under it; `None` → legacy KEK-direct decrypt.
/// The returned plaintext zeroizes on drop.
pub(crate) fn open_tag_row(
    crypto: &dyn CryptoProvider,
    kek: &[u8; KEK_LEN],
    row: &TagRow,
) -> Result<Zeroizing<Vec<u8>>, VaultError> {
    let aad = tag_aad(&row.id)?;
    match row.dek_wrapped {
        Some(wrapped) => {
            let dek = crypto.unwrap_dek(&wrapped, kek)?;
            crypto.decrypt_entry(&dek, &row.nonce, &row.ciphertext, &aad)
        }
        None => crypto.decrypt_legacy_tag(kek, &row.nonce, &row.ciphertext, &aad),
    }
}

/// If `row` is a legacy KEK-sealed tag (`dek_wrapped = None`), return the migrated
/// DEK-sealed row: decrypt under `kek`, re-seal under a fresh per-row DEK wrapped by the
/// **same** `kek`. Returns `None` when the row already carries a DEK (nothing to migrate) —
/// so a caller writes only on the one-time transition, keeping the migration idempotent.
pub(crate) fn migrate_legacy_tag(
    crypto: &dyn CryptoProvider,
    kek: &[u8; KEK_LEN],
    row: &TagRow,
) -> Result<Option<TagRow>, VaultError> {
    if row.dek_wrapped.is_some() {
        return Ok(None);
    }
    let aad = tag_aad(&row.id)?;
    let plaintext = crypto.decrypt_legacy_tag(kek, &row.nonce, &row.ciphertext, &aad)?;
    let (nonce, ciphertext, dek_wrapped) = seal_tag_payload(crypto, kek, &aad, &plaintext)?;
    Ok(Some(TagRow {
        nonce,
        ciphertext,
        dek_wrapped: Some(dek_wrapped),
        ..row.clone()
    }))
}

/// Re-wrap (or migrate) a tag row from `old_kek` to `new_kek`, always yielding a
/// DEK-sealed row readable under `new_kek`.
///
/// - `dek_wrapped = Some` → unwrap the DEK under `old_kek`, re-wrap under `new_kek`;
///   the ciphertext (under the unchanged DEK) is untouched.
/// - `dek_wrapped = None` (a legacy KEK-sealed row — a pre-5.6.0 snapshot, or a live
///   straggler whose at-unlock migration did not persist) → legacy-decrypt under
///   `old_kek`, then re-seal under a fresh DEK wrapped by `new_kek`.
///
/// Either way the result carries `dek_wrapped = Some` under `new_kek`, so a caller can
/// never leave a tag under the old KEK (the brick this slice closes).
pub(crate) fn rewrap_tag_row(
    crypto: &dyn CryptoProvider,
    row: &TagRow,
    old_kek: &[u8; KEK_LEN],
    new_kek: &[u8; KEK_LEN],
) -> Result<TagRow, VaultError> {
    if let Some(wrapped) = row.dek_wrapped {
        // Already DEK-sealed: re-wrap the DEK; ciphertext (under the unchanged DEK) is untouched.
        let dek = crypto.unwrap_dek(&wrapped, old_kek)?;
        let new_wrapped = crypto.wrap_dek(&dek, new_kek)?;
        Ok(TagRow {
            dek_wrapped: Some(new_wrapped),
            ..row.clone()
        })
    } else {
        // Legacy KEK-sealed: decrypt under `old_kek`, then re-seal under a fresh DEK by `new_kek`.
        let aad = tag_aad(&row.id)?;
        let plaintext = crypto.decrypt_legacy_tag(old_kek, &row.nonce, &row.ciphertext, &aad)?;
        let (nonce, ciphertext, new_wrapped) = seal_tag_payload(crypto, new_kek, &aad, &plaintext)?;
        Ok(TagRow {
            nonce,
            ciphertext,
            dek_wrapped: Some(new_wrapped),
            ..row.clone()
        })
    }
}
