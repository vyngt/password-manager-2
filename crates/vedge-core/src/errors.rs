use anyhow::Result;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BusinessError {
    #[error("Invalid Input")]
    InputError,
}

#[derive(Error, Debug)]
pub enum DataOperation {
    #[error("Save Error")]
    SaveError,
    #[error("Not found error")]
    NotFoundError,
    #[error("Delete error")]
    DeleteError,
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("UUID Error")]
    UuidError(#[from] uuid::Error),
    #[error("Cipher Error: {0}")]
    EncryptionCipherError(String),
    #[error("Decrypt Error")]
    EncryptionDecryptError,
    #[error("Encryption Error")]
    EncryptionEncryptError,
    #[error("File Not Found")]
    FileNotFoundError,
    #[error("File Write Error: {0}")]
    FileWriteError(String),
    #[error("File Read Error: {0}")]
    FileReadError(String),
    #[error("Directory Not Found")]
    DirectoryNotFoundError,
    #[error("Unknown Error: {0}")]
    UnknownError(String),
    #[error("Database Error")]
    DatabaseError(#[from] sea_orm::error::DbErr),
    #[error("Data Operation Error")]
    DataOperationError(#[from] DataOperation),
    #[error("Business Error")]
    BusinessError(#[from] BusinessError),
}

pub type AppResult<T> = Result<T, AppError>;
