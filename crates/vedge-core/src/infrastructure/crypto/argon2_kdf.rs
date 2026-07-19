use argon2::{Algorithm, Argon2, Params, Version};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::application::vault::ports::kdf::KeyDerivationProvider;
use crate::domain::vault::crypto_constants::{
    HKDF_INFO_2SKD, HKDF_INFO_KEK, HKDF_INFO_RECOVERY_KEK, HKDF_INFO_SYNC_AUTH, HKDF_INFO_VERIFY,
    KEK_LEN, MASTER_KEY_LEN, SECRET_KEY_LEN, VAULT_SALT_LEN, VERIFY_HASH_LEN,
};
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::kdf_params::KdfParams;

pub struct Argon2idKdfProvider;

impl Argon2idKdfProvider {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for Argon2idKdfProvider {
    fn default() -> Self {
        Self::new()
    }
}

fn hkdf_expand<const N: usize>(ikm: &[u8], info: &[u8]) -> Result<[u8; N], VaultError> {
    let hk = Hkdf::<Sha256>::new(None, ikm);
    let mut out = [0u8; N];
    hk.expand(info, &mut out)
        .map_err(|e| VaultError::KeyDerivationFailed(format!("hkdf expand: {e}")))?;
    Ok(out)
}

impl KeyDerivationProvider for Argon2idKdfProvider {
    fn preprocess_2skd(
        &self,
        master_password: &[u8],
        secret_key: &[u8; SECRET_KEY_LEN],
    ) -> Result<Zeroizing<[u8; 32]>, VaultError> {
        // 2SKD: HKDF-SHA256(ikm=password, salt=secret_key, info="vedge-v1-2skd").
        let hk = Hkdf::<Sha256>::new(Some(secret_key), master_password);
        let mut out = [0u8; 32];
        hk.expand(HKDF_INFO_2SKD, &mut out)
            .map_err(|e| VaultError::KeyDerivationFailed(format!("hkdf expand: {e}")))?;
        Ok(Zeroizing::new(out))
    }

    fn derive_master_key(
        &self,
        input: &[u8; 32],
        vault_salt: &[u8; VAULT_SALT_LEN],
        params: &KdfParams,
    ) -> Result<Zeroizing<[u8; MASTER_KEY_LEN]>, VaultError> {
        if params.alg != "argon2id" {
            return Err(VaultError::KeyDerivationFailed(format!(
                "unsupported kdf alg: {}",
                params.alg
            )));
        }

        let a2_params = Params::new(params.m, params.t, params.p, Some(MASTER_KEY_LEN))
            .map_err(|e| VaultError::KeyDerivationFailed(e.to_string()))?;
        let a2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, a2_params);

        let mut out = [0u8; MASTER_KEY_LEN];
        a2.hash_password_into(input, vault_salt, &mut out)
            .map_err(|e| VaultError::KeyDerivationFailed(e.to_string()))?;
        Ok(Zeroizing::new(out))
    }

    fn derive_kek(
        &self,
        master_key: &[u8; MASTER_KEY_LEN],
    ) -> Result<Zeroizing<[u8; KEK_LEN]>, VaultError> {
        Ok(Zeroizing::new(hkdf_expand::<KEK_LEN>(
            master_key,
            HKDF_INFO_KEK,
        )?))
    }

    fn derive_recovery_kek(
        &self,
        master_key: &[u8; MASTER_KEY_LEN],
    ) -> Result<Zeroizing<[u8; KEK_LEN]>, VaultError> {
        Ok(Zeroizing::new(hkdf_expand::<KEK_LEN>(
            master_key,
            HKDF_INFO_RECOVERY_KEK,
        )?))
    }

    fn derive_verify_hash(
        &self,
        master_key: &[u8; MASTER_KEY_LEN],
    ) -> Result<[u8; VERIFY_HASH_LEN], VaultError> {
        hkdf_expand::<VERIFY_HASH_LEN>(master_key, HKDF_INFO_VERIFY)
    }

    fn derive_sync_auth(
        &self,
        master_key: &[u8; MASTER_KEY_LEN],
    ) -> Result<Zeroizing<[u8; 32]>, VaultError> {
        Ok(Zeroizing::new(hkdf_expand::<32>(
            master_key,
            HKDF_INFO_SYNC_AUTH,
        )?))
    }
}
