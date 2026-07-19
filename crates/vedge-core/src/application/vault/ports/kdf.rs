use zeroize::Zeroizing;

use crate::domain::vault::crypto_constants::{
    KEK_LEN, MASTER_KEY_LEN, SECRET_KEY_LEN, VAULT_SALT_LEN, VERIFY_HASH_LEN,
};
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::kdf_params::KdfParams;

/// Argon2id + HKDF-SHA256 operations for the unlock flow.
///
/// [`derive_master_key`] is deliberately **synchronous and CPU-bound** (~500ms with
/// default params). Callers running inside an async runtime must wrap it in
/// `tokio::task::spawn_blocking` so the executor is not starved.
pub trait KeyDerivationProvider: Send + Sync {
    /// 2SKD preprocessing: `HKDF-SHA256(ikm = master_password, salt = secret_key,
    /// info = "vedge-v1-2skd")` — produces a 32-byte, uniformly distributed input for
    /// Argon2id regardless of password length / character class.
    ///
    /// Returns `KeyDerivationFailed` only if the underlying HKDF expand reports an
    /// `InvalidLength` — structurally impossible for 32-byte output with SHA-256, but
    /// we propagate rather than panic.
    fn preprocess_2skd(
        &self,
        master_password: &[u8],
        secret_key: &[u8; SECRET_KEY_LEN],
    ) -> Result<Zeroizing<[u8; 32]>, VaultError>;

    /// Derive the Master Key via Argon2id. Blocking — call via `spawn_blocking`.
    fn derive_master_key(
        &self,
        input: &[u8; 32],
        vault_salt: &[u8; VAULT_SALT_LEN],
        params: &KdfParams,
    ) -> Result<Zeroizing<[u8; MASTER_KEY_LEN]>, VaultError>;

    /// HKDF-expand the Master Key to the KEK.
    fn derive_kek(
        &self,
        master_key: &[u8; MASTER_KEY_LEN],
    ) -> Result<Zeroizing<[u8; KEK_LEN]>, VaultError>;

    /// HKDF-expand the Master Key to the **recovery** KEK (slice 5.7). Identical to
    /// [`derive_kek`](Self::derive_kek) but with the `vedge-v1-recovery-kek` info string —
    /// the one domain-separation point that makes `KEK_rec` ≠ the password-derived KEK,
    /// even though both share `preprocess_2skd`/`derive_master_key`. The Master Key here is
    /// derived from `2SKD(Recovery Key, Secret Key)`, not from the master password.
    fn derive_recovery_kek(
        &self,
        master_key: &[u8; MASTER_KEY_LEN],
    ) -> Result<Zeroizing<[u8; KEK_LEN]>, VaultError>;

    /// HKDF-expand the Master Key to the `verify_hash`. Stored on disk — **not secret**.
    fn derive_verify_hash(
        &self,
        master_key: &[u8; MASTER_KEY_LEN],
    ) -> Result<[u8; VERIFY_HASH_LEN], VaultError>;

    /// HKDF-expand the Master Key to the sync auth key.
    fn derive_sync_auth(
        &self,
        master_key: &[u8; MASTER_KEY_LEN],
    ) -> Result<Zeroizing<[u8; 32]>, VaultError>;
}
