use anyhow::Result;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DataOperation {
    #[error("Save Error")]
    SaveError,
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Cipher Error: {0}")]
    EncryptionCipherError(String),
    #[error("Decrypt Error")]
    EncryptionDecryptError,
    #[error("Encryption Error")]
    EncryptionEncryptError,
    #[error("File Not Found")]
    FileNotFoundError,
    #[error("Unknown Error: {0}")]
    UnknownError(String),
    #[error("Database Error")]
    DatabaseError(#[from] sea_orm::error::DbErr),
    #[error("Data Operation Error")]
    DataOperationError(#[from] DataOperation),
}

pub type AppResult<T> = Result<T, AppError>;
