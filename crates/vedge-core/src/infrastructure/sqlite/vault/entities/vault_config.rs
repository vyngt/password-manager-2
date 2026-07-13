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
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
