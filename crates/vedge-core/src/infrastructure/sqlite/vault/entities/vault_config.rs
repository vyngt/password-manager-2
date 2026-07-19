use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "vault_config")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub magic: String,
    pub schema_version: i32,
    pub vault_salt: Vec<u8>,
    pub kdf_params: String,
    pub verify_hash: Vec<u8>,
    pub preferred_cipher_suite: i32,
    pub trash_retention_days: i32,
    pub audit_retention_days: i32,
    pub created_at: String,
    pub last_unlocked_at: Option<String>,
    /// Intrinsic vault identity (ULID). Nullable: minted on create, backfilled on
    /// first open of a pre-4.6 vault (slice 4.6a).
    pub vault_uuid: Option<String>,
    /// Monotonic write counter for rollback detection (slice 5.2c). `NOT NULL
    /// DEFAULT 0`; bumped by the repo layer on every content write.
    pub commit_counter: i64,
    /// Optional override of the default `<home>/snapshots` store location (slice 5.2.1).
    pub backup_dir: Option<String>,
    /// Opt-in snapshot retention (Decision ⑯). `None` ⇒ keep everything (the default).
    pub backup_keep_count: Option<i32>,
    /// When a snapshot was last taken (RFC-3339 millis). Written by a targeted update.
    pub last_snapshot_at: Option<String>,
    /// When a `.vbk` was last written. 🔴 SEPARATE from `last_snapshot_at` (Decision ⑧).
    pub last_backup_at: Option<String>,
    /// The Recovery Key slot (slice 5.7): `AES-KW(KEK_rec, KEK)`, 40 bytes. Nullable BLOB;
    /// `NULL` ⇒ recovery off. Set/cleared by targeted repo updates; never crosses to WASM.
    pub recovery_slot: Option<Vec<u8>>,
    /// When the master password's KEK was last refreshed (RFC-3339 millis). Nullable;
    /// written by the credential paths in the rewrap txn / by a targeted update (slice 5.9).
    pub last_password_change_at: Option<String>,
    /// When the Secret Key was last rotated (RFC-3339 millis). Nullable; written only on a
    /// real SK change, not a recovery reset (slice 5.9).
    pub last_secret_key_rotation_at: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
