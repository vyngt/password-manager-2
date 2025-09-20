use backend::business::domain::services::VaultService;
use backend::infra::data::sqlite::datasource::connection::DataSourceConnection;
use backend::infra::service::vault::VaultServiceImpl;

use std::path::PathBuf;
use std::sync::Arc;

const DB_PATH: &str = "local/vault.db";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // WARN For testing purpose
    let source = Arc::new(init_connection().await);

    let vault_service = VaultServiceImpl::new(source.clone(), PathBuf::from("local/key"));

    println!("Unlock");
    let result = vault_service.unlock("111111").await;
    println!("Result> {}", result);

    let result = vault_service.change_key("111111").await;
    println!("Result> {}", result);

    let result = vault_service.change_key("234567").await;
    println!("Result> {}", result);

    Ok(())
}

async fn init_connection() -> DataSourceConnection {
    DataSourceConnection::new(DB_PATH).await
}
