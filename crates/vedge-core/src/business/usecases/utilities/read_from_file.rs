use std::path::PathBuf;
use std::sync::Arc;

use crate::business::domain::services::UtilitiesService;
use crate::errors::AppResult;
use crate::shared::base::BaseUseCase;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ReadFromFileInput {
    pub path: PathBuf,
}

pub struct ReadFromFileUseCase {
    utilities_service: Arc<dyn UtilitiesService>,
}

impl ReadFromFileUseCase {
    pub fn new(utilities_service: Arc<dyn UtilitiesService>) -> Self {
        Self { utilities_service }
    }
}

impl BaseUseCase<ReadFromFileInput, String> for ReadFromFileUseCase {
    async fn execute(&self, input: ReadFromFileInput) -> AppResult<String> {
        self.utilities_service.read_from_file(input.path).await
    }
}
