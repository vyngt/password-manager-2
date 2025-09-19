use crate::v2::business::domain::services::VaultService;
use crate::v2::infra::data::sqlite::DataSourceConnection;
use async_trait::async_trait;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::{path::PathBuf, sync::Arc};

use argon2::{
    Argon2,
    password_hash::{
        PasswordHasher, SaltString,
        rand_core::{OsRng, RngCore},
    },
};

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng as AesOsRng, consts::U12},
};

use crate::v2::errors::{AppError, AppResult};

pub struct VaultServiceImpl {
    encryption_datasource: Arc<DataSourceConnection>,
    encryption_key_path: PathBuf,
}

impl VaultServiceImpl {
    pub fn new(
        encryption_datasource: Arc<DataSourceConnection>,
        encryption_key_path: PathBuf,
    ) -> Self {
        Self {
            encryption_datasource,
            encryption_key_path,
        }
    }

    async fn check_if_unlocked(&self) -> bool {
        let stmt = "SELECT id FROM pre_setup where id=1;";
        match self.encryption_datasource.execute_raw(&stmt).await {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    async fn apply_migrations(&self) -> bool {
        use crate::v2::infra::data::sqlite::datasource::migration::VaultMigrator;
        use sea_orm_migration::MigratorTrait;

        match VaultMigrator::up(self.encryption_datasource.conn(), None).await {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}

// Encryption Operation
impl VaultServiceImpl {
    fn generate_salt(&self) -> SaltString {
        SaltString::generate(&mut OsRng)
    }

    fn generate_nonce(&self) -> Nonce<U12> {
        Aes256Gcm::generate_nonce(&mut AesOsRng)
    }

    fn generate_dek(&self) -> String {
        let mut dek = [0u8; 32];
        OsRng.fill_bytes(&mut dek);
        dek.iter().map(|b| format!("{:02x}", b)).collect()
    }

    ///
    /// Save DEK
    ///
    /// Content: <salt><nonce><encrypted_dek>
    ///
    /// Size (114 bytes):
    ///  - salt: 22
    ///  - nonce: 12
    ///  - encrypted_dek: rest (by calculate -> 80)
    ///
    fn save_dek(&self, encrypted_dek: Vec<u8>, salt: SaltString, nonce: Nonce<U12>) {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.encryption_key_path)
            .unwrap();

        file.write_all(salt.as_str().as_bytes()).unwrap();
        file.write_all(&nonce).unwrap();
        file.write_all(&encrypted_dek).unwrap();
    }

    /// Return <salt><nonce><encrypted_dek>
    fn load_dek(&self) -> AppResult<(SaltString, Nonce<U12>, Vec<u8>)> {
        let file = OpenOptions::new()
            .read(true)
            .open(&self.encryption_key_path);

        match file {
            Ok(mut f) => {
                let mut buf = Vec::new();
                f.read_to_end(&mut buf).unwrap();

                let salt_buf = &buf[0..22];
                let nonce_buf = &buf[22..34];
                let encrypted_dek = &buf[34..].to_vec();

                let salt_str = String::from_utf8(salt_buf.to_vec()).unwrap();

                let salt = SaltString::from_b64(&salt_str).unwrap();
                let nonce = Nonce::from_slice(&nonce_buf).to_owned();

                Ok((salt, nonce, encrypted_dek.to_owned()))
            }
            Err(_) => Err(AppError::FileNotFoundError),
        }
    }

    fn encrypt_dek(&self, dek: String, kek: &[u8], nonce: &Nonce<U12>) -> AppResult<Vec<u8>> {
        let kek_bytes = &kek[0..32];

        let cipher = Aes256Gcm::new_from_slice(kek_bytes);

        match cipher {
            Ok(cipher) => match cipher.encrypt(&nonce, dek.as_bytes().iter().as_slice()) {
                Ok(value) => Ok(value),
                Err(_) => Err(AppError::EncryptionEncryptError),
            },
            Err(e) => Err(AppError::EncryptionCipherError(e.to_string())),
        }
    }

    fn decrypt_dek(
        &self,
        encrypted_dek: Vec<u8>,
        kek: &[u8],
        nonce: &Nonce<U12>,
    ) -> AppResult<String> {
        let kek_bytes = &kek[0..32];
        let cipher = Aes256Gcm::new_from_slice(kek_bytes);
        match cipher {
            Ok(cipher) => match cipher.decrypt(&nonce, encrypted_dek.as_slice()) {
                Ok(value) => {
                    let dek = String::from_utf8(value);
                    match dek {
                        Ok(dek) => Ok(dek),
                        Err(e) => Err(AppError::UnknownError(e.to_string())),
                    }
                }
                Err(_) => Err(AppError::EncryptionEncryptError),
            },
            Err(e) => Err(AppError::EncryptionCipherError(e.to_string())),
        }
    }

    fn construct_kek(&self, password: &str, salt: &SaltString) -> Vec<u8> {
        let argon2 = Argon2::default();
        let derive_kek = argon2.hash_password(password.as_bytes(), salt).unwrap();
        derive_kek.hash.unwrap().as_ref().to_owned()
    }

    fn perform_new_key(&self, password: &str) -> AppResult<String> {
        let salt = self.generate_salt();
        let dek = self.generate_dek();
        let nonce = self.generate_nonce();
        let kek = self.construct_kek(password, &salt);
        let encrypted_dek = self.encrypt_dek(dek.clone(), &kek, &nonce)?;
        self.save_dek(encrypted_dek, salt, nonce);
        Ok(dek)
    }

    fn perform_decrypt_key(&self, password: &str) -> AppResult<String> {
        match self.load_dek() {
            Ok((salt, nonce, encrypted_dek)) => {
                let kek = self.construct_kek(password, &salt);
                let dek = self.decrypt_dek(encrypted_dek, &kek, &nonce)?;
                Ok(dek)
            }
            Err(e) => Err(e),
        }
    }
}

#[async_trait]
impl VaultService for VaultServiceImpl {
    async fn unlock(&self, key: &str) -> bool {
        let dek = match self.perform_decrypt_key(key) {
            Ok(key) => key,
            Err(e) => match e {
                AppError::FileNotFoundError => match self.perform_new_key(key) {
                    Ok(key) => key,
                    Err(_) => {
                        return false;
                    }
                },
                _ => {
                    return false;
                }
            },
        };

        let stmt = format!("PRAGMA key='{dek}';");
        let result = self.encryption_datasource.execute_raw(&stmt).await;
        match result {
            Ok(_) => {
                if self.apply_migrations().await && self.check_if_unlocked().await {
                    return true;
                }

                false
            }
            Err(_) => false,
        }
    }

    async fn change_key(&self, new_key: &str) -> bool {
        let new_dek = match self.perform_new_key(new_key) {
            Ok(key) => key,
            Err(e) => {
                println!("{:?}", e);
                return false;
            }
        };

        let stmt = format!("PRAGMA rekey='{new_dek}';");
        let result = self.encryption_datasource.execute_raw(&stmt).await;
        match result {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}
