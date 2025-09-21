use std::path::PathBuf;

use crate::errors::AppResult;

pub struct GeneratePasswordOptions {
    pub len: i32,
    pub upper: bool,
    pub lower: bool,
    pub digits: bool,
    pub special: bool,
}

#[async_trait::async_trait]
pub trait UtilitiesService: Send + Sync {
    async fn generate_password(&self, opt: GeneratePasswordOptions) -> String;
    async fn write_to_file(&self, path: PathBuf, content: String) -> AppResult<()>;
    async fn read_from_file(&self, path: PathBuf) -> AppResult<String>;
}
