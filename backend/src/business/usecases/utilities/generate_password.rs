use std::sync::Arc;

use crate::business::domain::services::{GeneratePasswordOptions, UtilitiesService};
use crate::errors::AppResult;
use crate::shared::base::BaseUseCase;

pub struct GeneratePasswordInput {
    len: i32,
    upper: bool,
    lower: bool,
    digits: bool,
    special: bool,
}

pub struct GeneratePasswordUseCase {
    utilities_service: Arc<dyn UtilitiesService>,
}

impl GeneratePasswordUseCase {
    pub fn new(utilities_service: Arc<dyn UtilitiesService>) -> Self {
        Self { utilities_service }
    }
}

impl BaseUseCase<GeneratePasswordInput, String> for GeneratePasswordUseCase {
    async fn execute(&self, input: GeneratePasswordInput) -> AppResult<String> {
        let password = self
            .utilities_service
            .generate_password(GeneratePasswordOptions {
                len: input.len,
                upper: input.upper,
                lower: input.lower,
                digits: input.digits,
                special: input.special,
            })
            .await;

        Ok(password)
    }
}
