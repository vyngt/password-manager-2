use backend_lib::v2::business::usecases::vault_record::create_vault_record::{
    CreateVaultRecordInput, CreateVaultRecordUseCase,
};
use backend_lib::v2::infra::data::sqlite::datasource::connection::DataSourceConnection;
use backend_lib::v2::infra::data::sqlite::datasource::migration::VaultMigrator;
use backend_lib::v2::infra::data::sqlite::repositories::vault_record::VaultRecordRepositoryImpl;
use backend_lib::v2::shared::base::BaseUseCase;
use dotenvy::dotenv;
use sea_orm_migration::MigratorTrait;
use std::env;
use std::sync::Arc;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let con = Arc::new(init_connection().await);
    set_encrypt_key(con.clone()).await;
    apply_new_migrations(con.clone()).await;
    test_insert(con.clone()).await;

    Ok(())
}

async fn init_connection() -> DataSourceConnection {
    dotenv().ok();
    let vault_db = env::var("DEV_VAULT_DB").unwrap();

    DataSourceConnection::new(&vault_db).await
}

async fn apply_new_migrations(con: Arc<DataSourceConnection>) {
    VaultMigrator::up(con.conn(), None).await.unwrap();
}

async fn set_encrypt_key(conn: Arc<DataSourceConnection>) {
    let x = conn.execute_raw("PRAGMA key=123456;").await.unwrap();
    println!("X> {:?}", x);
}

async fn test_insert(con: Arc<DataSourceConnection>) {
    println!("Testing insert");
    let repo = Arc::new(VaultRecordRepositoryImpl::new(con.clone()));

    let usecase_1 = CreateVaultRecordUseCase::new(repo.clone());
    let usecase_2 = CreateVaultRecordUseCase::new(repo.clone());

    let data_1 = CreateVaultRecordInput {
        name: "test".to_string(),
        url: "test".to_string(),
        login: "test".to_string(),
        key_pass: "test".to_string(),
    };

    let data_2 = CreateVaultRecordInput {
        name: "test".to_string(),
        url: "test2".to_string(),
        login: "test2".to_string(),
        key_pass: "test2".to_string(),
    };

    let x = usecase_1.execute(data_1).await.unwrap();
    let y = usecase_2.execute(data_2).await.unwrap();

    println!("X> {:?}", x);
    println!("Y> {:?}", y);
}
