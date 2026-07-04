use aes_kw::{KeyInit as AesKwKeyInit, KwAes256};
use chacha20poly1305::aead::{Aead, KeyInit as AeadKeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use rand::{Rng, rng};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

use crate::application::vault::ports::crypto::{CryptoProvider, Nonce};
use crate::domain::vault::crypto_constants::{
    DEK_LEN, DEK_WRAPPED_LEN, KEK_LEN, NONCE_LEN, SECRET_KEY_LEN, VAULT_SALT_LEN, VERIFY_HASH_LEN,
};
use crate::domain::vault::errors::VaultError;

/// XChaCha20-Poly1305 AEAD + AES-256 Key Wrap (RFC 3394).
///
/// Stateless; a single instance is safe to share across threads.
pub struct XChaCha20CryptoProvider;

impl XChaCha20CryptoProvider {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for XChaCha20CryptoProvider {
    fn default() -> Self {
        Self::new()
    }
}

fn cipher(key: &[u8; 32]) -> XChaCha20Poly1305 {
    XChaCha20Poly1305::new(key.into())
}

fn aead_encrypt(
    key: &[u8; 32],
    payload: &[u8],
    aad: &[u8],
) -> Result<(Nonce, Vec<u8>), VaultError> {
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rng().fill_bytes(&mut nonce_bytes);
    let nonce = XNonce::from_slice(&nonce_bytes);

    let ct = cipher(key)
        .encrypt(nonce, Payload { msg: payload, aad })
        .map_err(|_| VaultError::EncryptionFailed)?;
    Ok((nonce_bytes, ct))
}

fn aead_decrypt(
    key: &[u8; 32],
    nonce: &[u8; NONCE_LEN],
    ct: &[u8],
    aad: &[u8],
) -> Result<Zeroizing<Vec<u8>>, VaultError> {
    let xnonce = XNonce::from_slice(nonce);
    let pt = cipher(key)
        .decrypt(xnonce, Payload { msg: ct, aad })
        .map_err(|_| VaultError::DecryptionFailed)?;
    Ok(Zeroizing::new(pt))
}

impl CryptoProvider for XChaCha20CryptoProvider {
    fn encrypt_entry(
        &self,
        dek: &[u8; DEK_LEN],
        payload: &[u8],
        aad: &[u8],
    ) -> Result<(Nonce, Vec<u8>), VaultError> {
        aead_encrypt(dek, payload, aad)
    }

    fn decrypt_entry(
        &self,
        dek: &[u8; DEK_LEN],
        nonce: &[u8; NONCE_LEN],
        ciphertext: &[u8],
        aad: &[u8],
    ) -> Result<Zeroizing<Vec<u8>>, VaultError> {
        aead_decrypt(dek, nonce, ciphertext, aad)
    }

    fn encrypt_tag(
        &self,
        kek: &[u8; KEK_LEN],
        payload: &[u8],
        aad: &[u8],
    ) -> Result<(Nonce, Vec<u8>), VaultError> {
        aead_encrypt(kek, payload, aad)
    }

    fn decrypt_tag(
        &self,
        kek: &[u8; KEK_LEN],
        nonce: &[u8; NONCE_LEN],
        ciphertext: &[u8],
        aad: &[u8],
    ) -> Result<Zeroizing<Vec<u8>>, VaultError> {
        aead_decrypt(kek, nonce, ciphertext, aad)
    }

    fn wrap_dek(
        &self,
        dek: &[u8; DEK_LEN],
        kek: &[u8; KEK_LEN],
    ) -> Result<[u8; DEK_WRAPPED_LEN], VaultError> {
        let kw = KwAes256::new(kek.into());
        let mut out = [0u8; DEK_WRAPPED_LEN];
        kw.wrap_key(dek, &mut out)
            .map_err(|_| VaultError::EncryptionFailed)?;
        Ok(out)
    }

    fn unwrap_dek(
        &self,
        wrapped: &[u8; DEK_WRAPPED_LEN],
        kek: &[u8; KEK_LEN],
    ) -> Result<Zeroizing<[u8; DEK_LEN]>, VaultError> {
        let kw = KwAes256::new(kek.into());
        let mut out = [0u8; DEK_LEN];
        kw.unwrap_key(wrapped, &mut out)
            .map_err(|_| VaultError::DecryptionFailed)?;
        Ok(Zeroizing::new(out))
    }

    fn generate_dek(&self) -> Zeroizing<[u8; DEK_LEN]> {
        let mut k = [0u8; DEK_LEN];
        rng().fill_bytes(&mut k);
        Zeroizing::new(k)
    }

    fn generate_nonce(&self) -> Nonce {
        let mut n = [0u8; NONCE_LEN];
        rng().fill_bytes(&mut n);
        n
    }

    fn generate_secret_key(&self) -> Zeroizing<[u8; SECRET_KEY_LEN]> {
        let mut k = [0u8; SECRET_KEY_LEN];
        rng().fill_bytes(&mut k);
        Zeroizing::new(k)
    }

    fn generate_vault_salt(&self) -> [u8; VAULT_SALT_LEN] {
        let mut s = [0u8; VAULT_SALT_LEN];
        rng().fill_bytes(&mut s);
        s
    }

    fn verify_hash_matches(
        &self,
        candidate: &[u8; VERIFY_HASH_LEN],
        expected: &[u8; VERIFY_HASH_LEN],
    ) -> bool {
        candidate.ct_eq(expected).into()
    }
}
