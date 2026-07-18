use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "tags")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
    /// Wrapped per-row DEK (slice 5.6.0). Nullable: a NULL row is a legacy
    /// KEK-sealed tag awaiting the at-unlock migration.
    pub dek_wrapped: Option<Vec<u8>>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
