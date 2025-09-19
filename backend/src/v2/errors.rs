use thiserror::Error;

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
}

pub type AppResult<T> = std::result::Result<T, AppError>;
