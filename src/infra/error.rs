use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum AppError {
    #[error("OBS connection failed: {0}")]
    Connection(String),

    #[error("OBS request failed: {0}")]
    Request(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Storage error: {0}")]
    Storage(String),
}
