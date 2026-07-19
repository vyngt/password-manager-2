//! Per-row tag DEK crypto (slice 5.6.0).
//!
//! Tags used to be sealed directly under the vault KEK; a password change then left
//! them under the old KEK and bricked the vault. They now carry a per-row DEK wrapped
//! under the KEK, exactly like entries. These are the shared primitives for that:
//!
//! - [`seal_tag_payload`] — the WRITE side: mint a DEK, seal the plaintext under it,
//!   wrap the DEK under the KEK. Used by create/rename and by every rewrap. **No path
//!   uses the KEK as an AEAD key** — the KEK only wraps the DEK.
//! - [`open_tag_row`] — the READ side: unwrap the row's DEK under the KEK, decrypt under it.
//! - [`rewrap_tag_row`] — move a row from `old_kek` to `new_kek`, re-wrapping its DEK.
//!
//! 🔴 **The legacy KEK-as-AEAD read path was RETIRED in slice 5.9 ②**, behind a prove-absence
//! audit (`scripts/tag_legacy_audit.ps1`): `decrypt_legacy_tag` and the at-unlock `migrate_legacy_tag`
//! are gone, so a pre-5.6.0 tag (`dek_wrapped = None`) no longer decrypts — it is a hard error. The
//! security property is now unconditional: **no code uses the KEK as an AEAD key.** Every live tag is
//! DEK-sealed (5.6.0's at-unlock migration ran for every reachable vault before the audit went green);
//! to migrate a straggler, open the vault with a pre-5.9 build first.

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

/// Decrypt a tag row's payload: unwrap its per-row DEK under `kek`, decrypt under the DEK.
/// The returned plaintext zeroizes on drop.
///
/// A `dek_wrapped = None` row is a **retired** pre-5.6.0 KEK-sealed tag (slice 5.9 ②): the
/// KEK-as-AEAD read path is gone, so it can no longer be decrypted — `DecryptionFailed`. Unreachable
/// for a vault whose tags migrated before the retirement audit went green; open with a pre-5.9 build
/// to migrate a straggler.
pub(crate) fn open_tag_row(
    crypto: &dyn CryptoProvider,
    kek: &[u8; KEK_LEN],
    row: &TagRow,
) -> Result<Zeroizing<Vec<u8>>, VaultError> {
    let aad = tag_aad(&row.id)?;
    let wrapped = row.dek_wrapped.ok_or(VaultError::DecryptionFailed)?;
    let dek = crypto.unwrap_dek(&wrapped, kek)?;
    crypto.decrypt_entry(&dek, &row.nonce, &row.ciphertext, &aad)
}

/// Re-wrap a tag row's per-row DEK from `old_kek` to `new_kek`. The ciphertext (under the unchanged
/// DEK) is untouched, so the result is readable under `new_kek`.
///
/// A `dek_wrapped = None` row is a **retired** pre-5.6.0 KEK-sealed tag (slice 5.9 ②) — there is no
/// legacy re-seal path any more, so it is a hard `DecryptionFailed`. `rewrap_snapshots::rewrap_one`
/// (its only legacy caller) aborts the whole snapshot on this, exactly as before; unreachable once
/// every reachable vault's tags migrated before the retirement audit went green.
pub(crate) fn rewrap_tag_row(
    crypto: &dyn CryptoProvider,
    row: &TagRow,
    old_kek: &[u8; KEK_LEN],
    new_kek: &[u8; KEK_LEN],
) -> Result<TagRow, VaultError> {
    let wrapped = row.dek_wrapped.ok_or(VaultError::DecryptionFailed)?;
    let dek = crypto.unwrap_dek(&wrapped, old_kek)?;
    let new_wrapped = crypto.wrap_dek(&dek, new_kek)?;
    Ok(TagRow {
        dek_wrapped: Some(new_wrapped),
        ..row.clone()
    })
}
