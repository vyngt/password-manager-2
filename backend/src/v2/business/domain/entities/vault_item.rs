use uuid::Uuid;

use crate::v2::errors::{AppError, BusinessError};
use codegen::StringEnum;

#[derive(serde::Serialize, serde::Deserialize, Debug, StringEnum)]
pub enum VaultItemKind {
    Credential,
}

// TODO: Write proc macro impl enum string

// impl TryFrom<String> for VaultItemKind {
//     type Error = AppError;
//     fn try_from(value: String) -> Result<Self, Self::Error> {
//         match value.as_str() {
//             "Credential" => Ok(VaultItemKind::Credential),
//             _ => Err(AppError::BusinessError(BusinessError::InputError)),
//         }
//     }
// }

// impl ToString for VaultItemKind {
//     fn to_string(&self) -> String {
//         match self {
//             VaultItemKind::Credential => "Credential".to_string(),
//         }
//     }
// }

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

#[derive(serde::Serialize, serde::Deserialize, Debug)]
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

#[derive(serde::Serialize, serde::Deserialize)]
pub struct VaultItemUpdate {
    pub title: String,
    pub kind: VaultItemKind,
    pub data: VaultItemData,
}
