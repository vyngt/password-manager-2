use async_trait::async_trait;
use vedge_core::business::domain::services::{GeneratePasswordOptions, UtilitiesService};
use vedge_core::errors::AppResult;
use std::path::PathBuf;

// Mock implementation of UtilitiesService for testing
pub struct MockUtilitiesService {
    pub should_fail: bool,
    pub generated_password: Option<String>,
    pub file_content: Option<String>,
}

impl MockUtilitiesService {
    pub fn new() -> Self {
        Self {
            should_fail: false,
            generated_password: None,
            file_content: None,
        }
    }

    pub fn with_password(mut self, password: String) -> Self {
        self.generated_password = Some(password);
        self
    }

    pub fn with_file_content(mut self, content: String) -> Self {
        self.file_content = Some(content);
        self
    }

    pub fn should_fail(mut self, fail: bool) -> Self {
        self.should_fail = fail;
        self
    }
}

#[async_trait]
impl UtilitiesService for MockUtilitiesService {
    async fn generate_password(&self, _opt: GeneratePasswordOptions) -> String {
        if self.should_fail {
            return "".to_string(); // Return empty string to simulate failure
        }

        self.generated_password
            .clone()
            .unwrap_or_else(|| "MockPassword123!".to_string())
    }

    async fn write_to_file(&self, path: PathBuf, _content: String) -> AppResult<()> {
        if self.should_fail {
            return Err(vedge_core::errors::AppError::FileWriteError(
                "Mock write error".to_string(),
            ));
        }

        // Simulate directory check - if path contains "nonexistent", return DirectoryNotFoundError
        if path.to_string_lossy().contains("nonexistent") {
            return Err(vedge_core::errors::AppError::DirectoryNotFoundError);
        }

        Ok(())
    }

    async fn read_from_file(&self, path: PathBuf) -> AppResult<String> {
        if self.should_fail {
            return Err(vedge_core::errors::AppError::FileReadError(
                "Mock read error".to_string(),
            ));
        }

        // Simulate file not found - if path contains "nonexistent", return FileNotFoundError
        if path.to_string_lossy().contains("nonexistent") {
            return Err(vedge_core::errors::AppError::FileNotFoundError);
        }

        Ok(self
            .file_content
            .clone()
            .unwrap_or_else(|| "Mock file content".to_string()))
    }
}
