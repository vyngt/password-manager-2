pub const HKDF_INFO_2SKD: &[u8] = b"vedge-v1-2skd";
pub const HKDF_INFO_KEK: &[u8] = b"vedge-v1-kek";
pub const HKDF_INFO_VERIFY: &[u8] = b"vedge-v1-verify";
pub const HKDF_INFO_SYNC_AUTH: &[u8] = b"vedge-v1-sync-auth";

pub const SECRET_KEY_LEN: usize = 16;
pub const VAULT_SALT_LEN: usize = 32;
pub const MASTER_KEY_LEN: usize = 32;
pub const KEK_LEN: usize = 32;
pub const DEK_LEN: usize = 32;
pub const DEK_WRAPPED_LEN: usize = 40;
pub const NONCE_LEN: usize = 24;
pub const VERIFY_HASH_LEN: usize = 32;

pub const BLOB_AAD_SUFFIX: &[u8] = b"blob";
