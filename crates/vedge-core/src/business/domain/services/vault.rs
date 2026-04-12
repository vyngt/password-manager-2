#[async_trait::async_trait]
pub trait VaultService: Send + Sync {
    async fn unlock(&self, key: &str) -> bool;
    async fn change_key(&self, new_key: &str) -> bool;
}
