use thiserror::Error;

#[derive(Error, Debug)]
pub enum VortexError {
    #[error("Database error: {0}")]
    Database(#[from] diesel::result::Error),
    #[error("Pool error: {0}")]
    Pool(#[from] diesel::r2d2::PoolError),
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    #[error("Connection Error: {0}")]
    Connection(#[from] diesel::ConnectionError),
    #[error("Database error: {0}")]
    DatabaseErr(String),
}

pub type VortexResult<T> = std::result::Result<T, VortexError>;
