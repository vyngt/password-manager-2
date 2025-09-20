use backend_lib::v2::infra::data::sqlite::datasource::connection::DataSourceConnection;
use backend_lib::v2::infra::data::sqlite::datasource::migration::VaultMigrator;

use sea_orm_cli::{DateTimeCrate, run_generate_command};
use sea_orm_migration::MigratorTrait;

use std::sync::Arc;

const ENTITY_OUTPUT_PATH: &str = "../backend/src/v2/infra/data/sqlite/entities/vault";
const DB_PATH: &str = "local/vault_unencrypt.db";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = Arc::new(init_connection().await);

    refresh(source.clone()).await;

    generate_entities().await;

    Ok(())
}

async fn init_connection() -> DataSourceConnection {
    DataSourceConnection::new(DB_PATH).await
}

async fn refresh(con: Arc<DataSourceConnection>) {
    VaultMigrator::refresh(con.conn()).await.unwrap();
}

async fn generate_entities() {
    run_generate_command(
        sea_orm_cli::GenerateSubcommands::Entity {
            database_url: format!("sqlite://{}", DB_PATH),
            output_dir: ENTITY_OUTPUT_PATH.to_string(),
            compact_format: true,
            expanded_format: false,
            include_hidden_tables: false,
            tables: vec![],
            ignore_tables: vec!["seaql_migrations".to_string()],
            max_connections: 1,
            database_schema: None,
            with_serde: "none".to_string(),
            serde_skip_deserializing_primary_key: false,
            serde_skip_hidden_column: false,
            with_copy_enums: false,
            date_time_crate: DateTimeCrate::Chrono,
            frontend_format: false,
            acquire_timeout: 30,
            with_prelude: String::from("all"),
            lib: false,
            model_extra_derives: vec![],
            model_extra_attributes: vec![],
            enum_extra_derives: vec![],
            enum_extra_attributes: vec![],
            seaography: false,
            impl_active_model_behavior: true,
        },
        true,
    )
    .await
    .unwrap();
}
