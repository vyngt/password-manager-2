use uuid::Uuid;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Folder {
    pub id: Uuid,
    pub name: String,

    pub created_at: Option<String>,
    pub updated_at: Option<String>,

    // Relation
    pub parent_id: Option<Uuid>,
}

impl Folder {
    pub fn new(name: String, parent_id: Option<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            parent_id,
            created_at: None,
            updated_at: None,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct UpdateFolder {
    pub name: Option<String>,
    pub parent_id: Option<Uuid>,
}
