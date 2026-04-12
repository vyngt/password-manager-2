use uuid::Uuid;

use vedge_codegen::StringEnum;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, StringEnum)]
pub enum VaultItemKind {
    Credential,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct VaultItemDataCredential {
    pub identifier: String,
    pub password: String,
    pub url: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(tag = "kind")]
pub enum VaultItemData {
    Credential(VaultItemDataCredential),
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct VaultItem {
    pub id: Uuid,
    pub title: String,
    pub kind: VaultItemKind,
    pub data: VaultItemData,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl VaultItem {
    pub fn new(id: Option<Uuid>, title: String, kind: VaultItemKind, data: VaultItemData) -> Self {
        let id = id.unwrap_or(Uuid::new_v4());
        Self {
            id,
            title,
            kind,
            data,
            created_at: None,
            updated_at: None,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct UpdateVaultItem {
    pub title: String,
    pub kind: VaultItemKind,
    pub data: VaultItemData,
}
