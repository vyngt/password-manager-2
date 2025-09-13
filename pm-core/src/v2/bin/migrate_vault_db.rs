use dotenvy::dotenv;
use pm::v2::infra::data::sqlite::datasource::migration::VaultMigrator;
use sea_orm::{ConnectOptions, Database};
use sea_orm_migration::MigratorTrait;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let vault_db = env::var("DEV_VAULT_DB")?;

    let url = format!("sqlite://{}?mode=rwc", &vault_db);
    let opt = ConnectOptions::new(url);
    let db = Database::connect(opt).await?;

    VaultMigrator::refresh(&db).await?;

    Ok(())
}
