use crate::domain::shared::Timestamp;
use crate::domain::vault::kdf_params::KdfParams;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultConfig {
    pub id: String,
    pub magic: String,
    pub schema_version: i32,
    pub vault_salt: [u8; 32],
    pub kdf_params: KdfParams,
    pub verify_hash: [u8; 32],
    pub preferred_cipher_suite: i32,
    pub trash_retention_days: i32,
    pub audit_retention_days: i32,
    pub created_at: Timestamp,
    pub last_unlocked_at: Option<Timestamp>,
    /// Intrinsic vault identity (ULID) — answers "which *vault* is this?", distinct
    /// from `VaultId(PathBuf)` ("which *file* is open?"). Minted on create, backfilled
    /// on first open of a pre-4.6 vault. `None` only for a not-yet-backfilled vault.
    /// A *copy* of a vault is the same vault (same uuid) — a future "duplicate as a new
    /// vault" feature must mint a fresh one. Slice 4.6a; see arch note 13 - Vault Identity.
    pub vault_uuid: Option<String>,
}
