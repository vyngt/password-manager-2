use sea_orm::{ConnectionTrait, Statement, TransactionTrait};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const SEED_ISO_TIMESTAMP: &str = "2026-01-01T00:00:00+00:00";

// Semantic base colors — match the canonical palette in `vedge-ui/src/styles/tokens.css`.
// Hover / muted / foreground / text variants are derived at runtime from these bases
// plus `root_background` per the theme spec ("muted = mix(base, root_background, 0.85)").
const DANGER_BASE: &str = "#dc2626";
const WARNING_BASE: &str = "#d97706";
const SUCCESS_BASE: &str = "#16a34a";

const SEED_ROWS: &[SeedTheme] = &[
    SeedTheme {
        id: "builtin-light",
        name: "Light",
        root_background: "#FFFFFF",
        root_foreground: "#111418",
        root_primary: "#1D9E75",
        danger_base: DANGER_BASE,
        warning_base: WARNING_BASE,
        success_base: SUCCESS_BASE,
    },
    SeedTheme {
        id: "builtin-dark",
        name: "Dark",
        root_background: "#0E0F12",
        root_foreground: "#E6E8EB",
        root_primary: "#1D9E75",
        danger_base: DANGER_BASE,
        warning_base: WARNING_BASE,
        success_base: SUCCESS_BASE,
    },
];

struct SeedTheme {
    id: &'static str,
    name: &'static str,
    root_background: &'static str,
    root_foreground: &'static str,
    root_primary: &'static str,
    danger_base: &'static str,
    warning_base: &'static str,
    success_base: &'static str,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = db.get_database_backend();
        let tx = db.begin().await?;

        let create = Table::create()
            .table(Themes::Table)
            .if_not_exists()
            .col(ColumnDef::new(Themes::Id).text().not_null().primary_key())
            .col(ColumnDef::new(Themes::Name).text().not_null())
            .col(
                ColumnDef::new(Themes::IsBuiltIn)
                    .integer()
                    .not_null()
                    .default(0),
            )
            .col(ColumnDef::new(Themes::RootBackground).text().not_null())
            .col(ColumnDef::new(Themes::RootForeground).text().not_null())
            .col(ColumnDef::new(Themes::RootPrimary).text().not_null())
            .col(ColumnDef::new(Themes::DangerBase).text())
            .col(ColumnDef::new(Themes::WarningBase).text())
            .col(ColumnDef::new(Themes::SuccessBase).text())
            .col(ColumnDef::new(Themes::CreatedAt).text().not_null())
            .col(ColumnDef::new(Themes::UpdatedAt).text().not_null())
            .to_owned();
        tx.execute(backend.build(&create)).await?;

        for seed in SEED_ROWS {
            let stmt = Statement::from_sql_and_values(
                backend,
                r"INSERT OR IGNORE INTO themes
                   (id, name, is_built_in, root_background, root_foreground, root_primary,
                    danger_base, warning_base, success_base, created_at, updated_at)
                   VALUES (?, ?, 1, ?, ?, ?, ?, ?, ?, ?, ?)",
                [
                    seed.id.into(),
                    seed.name.into(),
                    seed.root_background.into(),
                    seed.root_foreground.into(),
                    seed.root_primary.into(),
                    seed.danger_base.into(),
                    seed.warning_base.into(),
                    seed.success_base.into(),
                    SEED_ISO_TIMESTAMP.into(),
                    SEED_ISO_TIMESTAMP.into(),
                ],
            );
            tx.execute(stmt).await?;
        }

        tx.commit().await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Themes::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Themes {
    Table,
    Id,
    Name,
    IsBuiltIn,
    RootBackground,
    RootForeground,
    RootPrimary,
    DangerBase,
    WarningBase,
    SuccessBase,
    CreatedAt,
    UpdatedAt,
}
