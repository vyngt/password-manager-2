//! Windows Hello biometric authenticator — TPM-bound KEK release.
//!
//! Enroll creates a Windows Hello-gated key pair in the TPM (`KeyCredentialManager`),
//! signs a fixed challenge, and derives a symmetric wrap key from the signature
//! (`SHA-256(domain ‖ signature)`). The KEK is then AEAD-sealed under that wrap key (via
//! the shared [`CryptoProvider`]) and the ciphertext stored device-locally in the OS
//! credential store (`keyring`, service `"vedge-biometric"`). Retrieve re-signs the same
//! challenge (Hello prompt) → the same signature (RSA-PKCS1v1.5 Hello signatures are
//! deterministic) → the same wrap key → decrypts the KEK. Resetting Windows Hello deletes
//! the TPM key, so the credential auto-invalidates (`OpenAsync` → `NotFound`), satisfying
//! the spec's "invalidate on biometry change".
//!
//! **Verification note:** the interactive Hello prompt, the sign-determinism, and COM
//! initialization on the calling thread cannot be exercised headless. This follows
//! established Windows Hello credential-signing prior art; it is validated by the manual
//! on-device smoke recorded in the slice 2.8 changelog. The rest of the biometric stack
//! (use cases, lifecycle, UI) is covered headless by `MemoryBiometricAuthenticator`.

use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use keyring::{Entry, Error as KeyringError};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

use windows::Security::Credentials::{
    KeyCredentialCreationOption, KeyCredentialManager, KeyCredentialStatus,
};
use windows::Security::Cryptography::CryptographicBuffer;
use windows::core::HSTRING;

use crate::application::vault::ports::biometric::BiometricAuthenticator;
use crate::application::vault::ports::crypto::CryptoProvider;
use crate::domain::shared::VaultId;
use crate::domain::vault::crypto_constants::{KEK_LEN, NONCE_LEN};
use crate::domain::vault::errors::VaultError;

/// OS credential-store service holding each vault's wrapped-KEK blob.
const SERVICE: &str = "vedge-biometric";
/// Fixed challenge signed by the per-vault Hello key; the signature is the wrap-key IKM.
const CHALLENGE: &[u8] = b"vedge-biometric-kek-challenge-v1";
/// Domain-separation prefix hashed with the signature to derive the 32-byte wrap key.
const WRAP_KEY_DOMAIN: &[u8] = b"vedge-biometric-wrap-key-v1";
/// AAD binding for the wrapped-KEK AEAD.
const WRAP_AAD: &[u8] = b"vedge-biometric-kek-v1";

pub struct WindowsHelloAuthenticator {
    crypto: Arc<dyn CryptoProvider>,
}

impl WindowsHelloAuthenticator {
    #[must_use]
    pub fn new(crypto: Arc<dyn CryptoProvider>) -> Self {
        Self { crypto }
    }

    /// Stable, character-safe identifier for a vault's Hello key + keyring entry:
    /// `vedge-biometric-<url-safe base64 of sha256(path)>`.
    fn key_name(vault_id: &VaultId) -> String {
        let mut hasher = Sha256::new();
        hasher.update(vault_id.path().to_string_lossy().as_bytes());
        let digest = hasher.finalize();
        format!("vedge-biometric-{}", URL_SAFE_NO_PAD.encode(digest))
    }

    fn entry(vault_id: &VaultId) -> Result<Entry, VaultError> {
        Entry::new(SERVICE, &Self::key_name(vault_id)).map_err(map_keyring_err)
    }
}

fn map_keyring_err(_e: KeyringError) -> VaultError {
    // Detail is intentionally hidden — never leak store internals to callers.
    VaultError::BiometricFailed("credential store error".to_owned())
}

fn win_err(context: &str, e: &windows::core::Error) -> VaultError {
    VaultError::BiometricFailed(format!("{context}: {e}"))
}

/// Derive the 32-byte wrap key from a Hello signature: `SHA-256(domain ‖ signature)`.
fn wrap_key_from_signature(sig: &[u8]) -> Zeroizing<[u8; 32]> {
    let mut hasher = Sha256::new();
    hasher.update(WRAP_KEY_DOMAIN);
    hasher.update(sig);
    Zeroizing::new(hasher.finalize().into())
}

/// Sign the fixed challenge with the vault's Hello-gated TPM key, creating it when
/// `create` is true. Shows the Windows Hello prompt. Returns the raw signature bytes.
fn sign_challenge(name: &HSTRING, create: bool) -> Result<Zeroizing<Vec<u8>>, VaultError> {
    let credential = if create {
        let result = KeyCredentialManager::RequestCreateAsync(
            name,
            KeyCredentialCreationOption::ReplaceExisting,
        )
        .map_err(|e| win_err("RequestCreateAsync", &e))?
        .get()
        .map_err(|e| win_err("RequestCreateAsync.get", &e))?;
        match result.Status().map_err(|e| win_err("Status", &e))? {
            KeyCredentialStatus::Success => {}
            KeyCredentialStatus::UserCanceled => return Err(VaultError::BiometricCancelled),
            other => {
                return Err(VaultError::BiometricFailed(format!(
                    "create status {other:?}"
                )));
            }
        }
        result.Credential().map_err(|e| win_err("Credential", &e))?
    } else {
        let result = KeyCredentialManager::OpenAsync(name)
            .map_err(|e| win_err("OpenAsync", &e))?
            .get()
            .map_err(|e| win_err("OpenAsync.get", &e))?;
        match result.Status().map_err(|e| win_err("Status", &e))? {
            KeyCredentialStatus::Success => {}
            KeyCredentialStatus::NotFound => return Err(VaultError::BiometricNotEnrolled),
            other => {
                return Err(VaultError::BiometricFailed(format!(
                    "open status {other:?}"
                )));
            }
        }
        result.Credential().map_err(|e| win_err("Credential", &e))?
    };

    let challenge = CryptographicBuffer::CreateFromByteArray(CHALLENGE)
        .map_err(|e| win_err("CreateFromByteArray", &e))?;
    let sign_result = credential
        .RequestSignAsync(&challenge)
        .map_err(|e| win_err("RequestSignAsync", &e))?
        .get()
        .map_err(|e| win_err("RequestSignAsync.get", &e))?;
    match sign_result.Status().map_err(|e| win_err("Status", &e))? {
        KeyCredentialStatus::Success => {}
        KeyCredentialStatus::UserCanceled => return Err(VaultError::BiometricCancelled),
        other => {
            return Err(VaultError::BiometricFailed(format!(
                "sign status {other:?}"
            )));
        }
    }
    let sig_buf = sign_result.Result().map_err(|e| win_err("Result", &e))?;

    let mut out = windows::core::Array::<u8>::new();
    CryptographicBuffer::CopyToByteArray(&sig_buf, &mut out)
        .map_err(|e| win_err("CopyToByteArray", &e))?;
    Ok(Zeroizing::new(out.to_vec()))
}

impl BiometricAuthenticator for WindowsHelloAuthenticator {
    fn is_available(&self) -> bool {
        // No prompt: capability query only.
        KeyCredentialManager::IsSupportedAsync()
            .and_then(|op| op.get())
            .unwrap_or(false)
    }

    fn is_enrolled(&self, vault_id: &VaultId) -> Result<bool, VaultError> {
        // No prompt: presence of the stored wrapped-KEK blob is the enrollment signal.
        // A missing/reset TPM key is caught at `retrieve` time (→ falls back to password).
        match Self::entry(vault_id)?.get_secret() {
            Ok(mut b) => {
                b.zeroize();
                Ok(true)
            }
            Err(KeyringError::NoEntry) => Ok(false),
            Err(e) => Err(map_keyring_err(e)),
        }
    }

    fn enroll(&self, vault_id: &VaultId, kek: &[u8; KEK_LEN]) -> Result<(), VaultError> {
        let name = HSTRING::from(Self::key_name(vault_id));
        let sig = sign_challenge(&name, true)?;
        let wrap_key = wrap_key_from_signature(&sig);

        // Seal the KEK under the Hello-derived wrap key using the shared AEAD.
        let (nonce, ct) = self.crypto.encrypt_tag(&wrap_key, kek, WRAP_AAD)?;

        let mut blob = Vec::with_capacity(NONCE_LEN.saturating_add(ct.len()));
        blob.extend_from_slice(&nonce);
        blob.extend_from_slice(&ct);
        let result = Self::entry(vault_id)?
            .set_secret(&blob)
            .map_err(map_keyring_err);
        blob.zeroize();
        result
    }

    fn retrieve(&self, vault_id: &VaultId) -> Result<Zeroizing<[u8; KEK_LEN]>, VaultError> {
        let mut blob = match Self::entry(vault_id)?.get_secret() {
            Ok(b) => b,
            Err(KeyringError::NoEntry) => return Err(VaultError::BiometricNotEnrolled),
            Err(e) => return Err(map_keyring_err(e)),
        };
        if blob.len() <= NONCE_LEN {
            blob.zeroize();
            return Err(VaultError::BiometricFailed(
                "stored blob too short".to_owned(),
            ));
        }
        // `split_at` avoids index-slicing lints; the length was just checked above.
        let (nonce_slice, ct_slice) = blob.split_at(NONCE_LEN);
        let mut nonce = [0u8; NONCE_LEN];
        nonce.copy_from_slice(nonce_slice);
        let ct = ct_slice.to_vec();

        // Re-sign (Hello prompt) → same wrap key → decrypt the KEK.
        let name = HSTRING::from(Self::key_name(vault_id));
        let sig = sign_challenge(&name, false)?;
        let wrap_key = wrap_key_from_signature(&sig);

        let pt = self
            .crypto
            .decrypt_tag(&wrap_key, &nonce, &ct, WRAP_AAD)
            .map_err(|_| VaultError::BiometricFailed("wrapped KEK decrypt failed".to_owned()))?;
        blob.zeroize();

        let arr = <[u8; KEK_LEN]>::try_from(pt.as_slice())
            .map_err(|_| VaultError::BiometricFailed("bad KEK length".to_owned()))?;
        Ok(Zeroizing::new(arr))
    }

    fn disable(&self, vault_id: &VaultId) -> Result<(), VaultError> {
        // Remove the stored blob (best-effort — a missing entry is not an error).
        if let Ok(entry) = Self::entry(vault_id) {
            drop(entry.delete_credential());
        }
        // Remove the TPM key (best-effort; a Hello reset may already have removed it).
        let name = HSTRING::from(Self::key_name(vault_id));
        drop(KeyCredentialManager::DeleteAsync(&name).and_then(|op| op.get()));
        Ok(())
    }
}
