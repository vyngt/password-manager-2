use backend::business::domain::entities::vault_item::{
    VaultItemData, VaultItemDataCredential, VaultItemKind,
};
use backend::business::usecases::{
    CreateVaultItemInput, CreateVaultItemUseCase, GetVaultItemInput, GetVaultItemUseCase,
    UpdateVaultItemInput, UpdateVaultItemUseCase,
};
use backend::infra::data::sqlite::datasource::connection::DataSourceConnection;
use backend::infra::data::sqlite::datasource::migration::VaultMigrator;
use backend::infra::data::sqlite::repositories::vault_item::VaultItemRepositoryImpl;
use backend::shared::base::BaseUseCase;
use dotenvy::dotenv;
use sea_orm_migration::MigratorTrait;
use std::env;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let con = Arc::new(init_connection().await);
    // set_encrypt_key(con.clone()).await;
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

// async fn set_encrypt_key(conn: Arc<DataSourceConnection>) {
//     let x = conn.execute_raw("PRAGMA key=123456;").await.unwrap();
//     println!("X> {:?}", x);
// }

async fn test_insert(con: Arc<DataSourceConnection>) {
    println!("Testing insert");
    let repo = Arc::new(VaultItemRepositoryImpl::new(con.clone()));

    let usecase_1 = CreateVaultItemUseCase::new(repo.clone());
    let usecase_2 = GetVaultItemUseCase::new(repo.clone());
    let usecase_3 = UpdateVaultItemUseCase::new(repo.clone());

    let data_1 = CreateVaultItemInput {
        title: "Hello World".to_string(),
        kind: VaultItemKind::Credential,
        data: VaultItemData::Credential(VaultItemDataCredential {
            url: "testAS".to_string(),
            identifier: "testsss".to_string(),
            password: "p@ssw0rdsad".to_string(),
        }),
    };

    let data_2 = GetVaultItemInput {
        id: "2dbd714d-69f4-42f8-a564-d7d1ca77e0f7".to_string(),
    };

    let data_3 = UpdateVaultItemInput {
        id: "2dbd714d-69f4-42f8-a564-d7d1ca77e0f7".to_string(),
        title: "Cooked".to_string(),
        kind: VaultItemKind::Credential,
        data: VaultItemData::Credential(VaultItemDataCredential {
            url: "Updated1".to_string(),
            identifier: "Updated2".to_string(),
            password: "Updated Password".to_string(),
        }),
    };

    let x = usecase_1.execute(data_1).await.unwrap();
    let z = usecase_3.execute(data_3).await.unwrap();
    let y = usecase_2.execute(data_2).await.unwrap();

    println!("X> {:?}", x);
    println!("Z> {:?}", z);
    println!("Y> {:?}", y);
}
