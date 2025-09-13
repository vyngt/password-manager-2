#[derive(serde::Serialize, serde::Deserialize)]
pub struct ColorSchema {
    pub id: String,
    pub name: String,
    pub primary: String,
    pub secondary: String,
    pub success: String,
    pub danger: String,
    pub warning: String,
    pub foreground: String,
    pub background: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ThemeManager {
    pub id: i32,

    // Relation
    pub color_scheme: ColorSchema,
}
