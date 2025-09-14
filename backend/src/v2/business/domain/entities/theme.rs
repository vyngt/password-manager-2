#[derive(serde::Serialize, serde::Deserialize)]
pub struct ColorSchema {
    pub id: String,
    pub name: String,
    pub color_primary: String,
    pub color_secondary: String,
    pub color_success: String,
    pub color_danger: String,
    pub color_warning: String,
    pub color_foreground: String,
    pub color_background: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ThemeManager {
    pub id: i32,

    // Relation
    pub color_scheme: ColorSchema,
}
