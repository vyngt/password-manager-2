use std::path::PathBuf;
use std::sync::Arc;

use crate::business::domain::services::UtilitiesService;
use crate::errors::AppResult;
use crate::shared::base::BaseUseCase;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct WriteToFileInput {
    pub path: PathBuf,
    pub content: String,
}

pub struct WriteToFileUseCase {
    utilities_service: Arc<dyn UtilitiesService>,
}

impl WriteToFileUseCase {
    pub fn new(utilities_service: Arc<dyn UtilitiesService>) -> Self {
        Self { utilities_service }
    }
}

impl BaseUseCase<WriteToFileInput, ()> for WriteToFileUseCase {
    async fn execute(&self, input: WriteToFileInput) -> AppResult<()> {
        self.utilities_service
            .write_to_file(input.path, input.content)
            .await
    }
}
