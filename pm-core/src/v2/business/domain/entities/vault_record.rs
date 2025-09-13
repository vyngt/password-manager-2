#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct VaultRecord {
    pub id: uuid::Uuid,
    pub name: String,
    pub url: String,
    pub login: String,
    pub key_pass: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl VaultRecord {
    pub fn new(
        id: Option<uuid::Uuid>,
        name: String,
        url: String,
        login: String,
        key_pass: String,
    ) -> Self {
        let id = id.unwrap_or(uuid::Uuid::new_v4());
        Self {
            id,
            name,
            url,
            login,
            key_pass,
            created_at: None,
            updated_at: None,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct VaultRecordUpdate {
    pub name: String,
    pub url: String,
    pub login: String,
    pub key_pass: String,
}
