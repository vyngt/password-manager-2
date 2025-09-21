use async_trait::async_trait;
use backend::business::domain::services::{GeneratePasswordOptions, UtilitiesService};

// Mock implementation of UtilitiesService for testing
pub struct MockUtilitiesService {
    pub should_fail: bool,
    pub generated_password: Option<String>,
}

impl MockUtilitiesService {
    pub fn new() -> Self {
        Self {
            should_fail: false,
            generated_password: None,
        }
    }

    pub fn with_password(mut self, password: String) -> Self {
        self.generated_password = Some(password);
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
}
