use zeroize::Zeroizing;

use crate::domain::vault::crypto_constants::{
    DEK_LEN, DEK_WRAPPED_LEN, KEK_LEN, NONCE_LEN, SECRET_KEY_LEN, VAULT_SALT_LEN, VERIFY_HASH_LEN,
};
use crate::domain::vault::errors::VaultError;

pub type Nonce = [u8; NONCE_LEN];

/// Stateless AEAD + key-wrap operations. Takes key material as parameters — never holds
/// it. Callers are responsible for zeroizing inputs after use; outputs are pre-wrapped in
/// `Zeroizing` where secret.
///
/// ## Error policy
///
/// All authentication-style failures (wrong key, wrong AAD, tampered ciphertext,
/// corrupted wrapped DEK) collapse to [`VaultError::DecryptionFailed`]. The impl must
/// never distinguish these cases — doing so leaks information about which secret was
/// wrong.
pub trait CryptoProvider: Send + Sync {
    /// Encrypt an entry payload under a per-entry DEK.
    ///
    /// `aad` is the entry's binding context (entry ULID bytes + version LE), produced
    /// by [`crate::domain::vault::aad::entry_aad`]. Returns the 24-byte nonce used and
    /// the ciphertext with a 16-byte Poly1305 tag appended.
    fn encrypt_entry(
        &self,
        dek: &[u8; DEK_LEN],
        payload: &[u8],
        aad: &[u8],
    ) -> Result<(Nonce, Vec<u8>), VaultError>;

    /// Decrypt an entry ciphertext. Returns plaintext wrapped in `Zeroizing` so the
    /// caller cannot forget to zero it.
    fn decrypt_entry(
        &self,
        dek: &[u8; DEK_LEN],
        nonce: &[u8; NONCE_LEN],
        ciphertext: &[u8],
        aad: &[u8],
    ) -> Result<Zeroizing<Vec<u8>>, VaultError>;

    /// Encrypt a tag payload under the vault's KEK (no per-row DEK for tags).
    fn encrypt_tag(
        &self,
        kek: &[u8; KEK_LEN],
        payload: &[u8],
        aad: &[u8],
    ) -> Result<(Nonce, Vec<u8>), VaultError>;

    /// Decrypt a tag ciphertext.
    fn decrypt_tag(
        &self,
        kek: &[u8; KEK_LEN],
        nonce: &[u8; NONCE_LEN],
        ciphertext: &[u8],
        aad: &[u8],
    ) -> Result<Zeroizing<Vec<u8>>, VaultError>;

    /// Wrap a DEK under the KEK using AES-256 Key Wrap (RFC 3394). Output is exactly
    /// 40 bytes: 32-byte DEK + 8-byte integrity check.
    fn wrap_dek(
        &self,
        dek: &[u8; DEK_LEN],
        kek: &[u8; KEK_LEN],
    ) -> Result<[u8; DEK_WRAPPED_LEN], VaultError>;

    /// Unwrap a DEK. The raw DEK must never outlive this call frame — return type is
    /// `Zeroizing` so `Drop` zeros the buffer.
    fn unwrap_dek(
        &self,
        wrapped: &[u8; DEK_WRAPPED_LEN],
        kek: &[u8; KEK_LEN],
    ) -> Result<Zeroizing<[u8; DEK_LEN]>, VaultError>;

    /// Generate a fresh 32-byte DEK from the OS CSPRNG.
    fn generate_dek(&self) -> Zeroizing<[u8; DEK_LEN]>;

    /// Generate a fresh 24-byte nonce from the OS CSPRNG. Nonce is not secret —
    /// no zeroize wrapper.
    fn generate_nonce(&self) -> Nonce;

    /// Generate a fresh 16-byte Secret Key from the OS CSPRNG. Wrapped in
    /// `Zeroizing` — this is the vault's root secret alongside the password.
    fn generate_secret_key(&self) -> Zeroizing<[u8; SECRET_KEY_LEN]>;

    /// Generate a fresh 32-byte vault salt from the OS CSPRNG. Not secret —
    /// no zeroize wrapper.
    fn generate_vault_salt(&self) -> [u8; VAULT_SALT_LEN];

    /// Constant-time equality for `verify_hash` values. Callers must never use `==`.
    fn verify_hash_matches(
        &self,
        candidate: &[u8; VERIFY_HASH_LEN],
        expected: &[u8; VERIFY_HASH_LEN],
    ) -> bool;
}
