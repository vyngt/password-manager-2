#[derive(serde::Serialize, serde::Deserialize)]
pub struct VaultRecord {
    pub id: String,
    pub name: String,
    pub url: String,
    pub login: String,
    pub key_pass: String,
}

impl VaultRecord {
    pub fn new(
        id: Option<String>,
        name: String,
        url: String,
        login: String,
        key_pass: String,
    ) -> Self {
        let id = id.unwrap_or(uuid::Uuid::new_v4().to_string());
        Self {
            id,
            name,
            url,
            login,
            key_pass,
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
