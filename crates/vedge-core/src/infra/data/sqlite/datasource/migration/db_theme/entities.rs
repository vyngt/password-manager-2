use sea_orm_migration::prelude::*;

#[derive(Iden)]
pub enum Theme {
    Table,
    Id,
    // ColorScheme
    ColorSchemeId,
}

#[derive(Iden)]
pub enum ColorScheme {
    Table,
    Id,
    Name,
    ColorPrimary,
    ColorSecondary,
    ColorSuccess,
    ColorDanger,
    ColorWarning,
    ColorForeground,
    ColorBackground,
    CreatedAt,
    UpdatedAt,
}
