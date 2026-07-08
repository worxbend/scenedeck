//! Error type shared across OBS, storage, and controller boundaries.

use thiserror::Error;

/// Application-level error normalized for controller/UI handling.
#[derive(Debug, Error, Clone)]
pub enum AppError {
    /// OBS WebSocket connection setup failed.
    #[error("OBS connection failed: {0}")]
    Connection(String),

    /// OBS accepted the connection but a request failed.
    #[error("OBS request failed: {0}")]
    Request(String),

    /// Local configuration failed validation or persistence.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Local storage or secret-service operation failed.
    #[error("Storage error: {0}")]
    Storage(String),
}

impl AppError {
    /// Normalize an upstream connection error.
    pub fn connection(error: impl std::fmt::Display) -> Self {
        Self::Connection(error.to_string())
    }

    /// Normalize an upstream OBS request error.
    pub fn request(error: impl std::fmt::Display) -> Self {
        Self::Request(error.to_string())
    }

    /// Normalize a configuration validation or persistence error.
    pub fn config(error: impl std::fmt::Display) -> Self {
        Self::Config(error.to_string())
    }

    /// Normalize a local storage or secret-service error.
    pub fn storage(error: impl std::fmt::Display) -> Self {
        Self::Storage(error.to_string())
    }

    /// Build a concise notification title for user-visible error toasts.
    pub fn notification_title(&self) -> String {
        format!("SceneDeck error: {self}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_preserve_display_text() {
        assert_eq!(
            AppError::connection("refused").to_string(),
            "OBS connection failed: refused"
        );
        assert_eq!(
            AppError::request("missing scene").to_string(),
            "OBS request failed: missing scene"
        );
        assert_eq!(
            AppError::config("bad role").to_string(),
            "Configuration error: bad role"
        );
        assert_eq!(
            AppError::storage("keyring locked").to_string(),
            "Storage error: keyring locked"
        );
    }

    #[test]
    fn notification_title_uses_application_context() {
        assert_eq!(
            AppError::config("bad role").notification_title(),
            "SceneDeck error: Configuration error: bad role"
        );
        assert_eq!(
            AppError::request("missing scene").notification_title(),
            "SceneDeck error: OBS request failed: missing scene"
        );
    }
}
