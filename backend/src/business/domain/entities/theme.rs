use uuid::Uuid;
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ColorScheme {
    pub id: Uuid,
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
pub struct UpdateColorScheme {
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
pub struct Theme {
    pub id: Uuid,

    // Relation
    pub color_scheme: ColorScheme,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct UpdateTheme {
    pub color_scheme_id: Uuid,
}
