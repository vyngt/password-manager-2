mod password_generator;

use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;

use crate::business::domain::services::{GeneratePasswordOptions, UtilitiesService};
use crate::errors::{AppError, AppResult};

pub struct UtilitiesServiceImpl;

impl UtilitiesServiceImpl {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl UtilitiesService for UtilitiesServiceImpl {
    async fn generate_password(&self, opt: GeneratePasswordOptions) -> String {
        password_generator::generate_password(opt)
    }

    async fn write_to_file(&self, path: PathBuf, content: String) -> AppResult<()> {
        // Check if parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Err(AppError::DirectoryNotFoundError);
            }
        }

        // Write content to file
        fs::write(&path, content).await.map_err(|e| {
            AppError::FileWriteError(format!("Failed to write to file {:?}: {}", path, e))
        })?;

        Ok(())
    }

    async fn read_from_file(&self, path: PathBuf) -> AppResult<String> {
        // Check if file exists
        if !path.exists() {
            return Err(AppError::FileNotFoundError);
        }

        // Read content from file
        let content = fs::read_to_string(&path).await.map_err(|e| {
            AppError::FileReadError(format!("Failed to read from file {:?}: {}", path, e))
        })?;

        Ok(content)
    }
}
