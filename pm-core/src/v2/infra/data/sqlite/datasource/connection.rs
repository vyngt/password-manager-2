use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, Statement};

#[derive(Clone)]
pub struct DataSourceConnection {
    connection: DatabaseConnection,
}

impl DataSourceConnection {
    pub async fn new(url: &str) -> Self {
        let sqlite_url = format!("sqlite://{}?mode=rwc", url);
        let opt = ConnectOptions::new(sqlite_url);
        let db = Database::connect(opt).await.unwrap();

        Self { connection: db }
    }

    pub fn conn(&self) -> &DatabaseConnection {
        &self.connection
    }

    pub async fn execute_raw(&self, query: &str) -> Result<sea_orm::ExecResult, sea_orm::DbErr> {
        self.connection
            .execute(Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                query,
            ))
            .await
    }
}
