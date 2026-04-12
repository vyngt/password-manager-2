use vedge_core::errors::AppError;
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl From<AppError> for ApiError {
    fn from(value: AppError) -> Self {
        match value {
            _ => ApiError {
                code: "UNKNOWN".to_string(),
                message: value.to_string(),
            },
        }
    }
}
