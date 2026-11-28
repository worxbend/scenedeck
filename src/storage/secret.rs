//! OBS WebSocket password storage via the Linux Secret Service (D-Bus keyring).
//!
//! Passwords are stored in the system keyring (GNOME Keyring / KWallet), never
//! in `config.json`. Entries are keyed by app ID and immutable connection ID.
//! The migrated default connection retains the original username.

use keyring::{Entry, Error as KeyringError};

use crate::app_info::APP_ID;
use crate::domain::connection::credential::{CredentialStore, ObsPassword};
use crate::infra::error::AppError;

const SECRET_USER: &str = "obs-websocket";

fn secret_user(connection_id: &str) -> Result<String, AppError> {
    if connection_id.is_empty() {
        return Err(AppError::storage(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "connection identity is empty",
        )));
    }
    Ok(if connection_id == "default" {
        SECRET_USER.to_string()
    } else {
        format!("{SECRET_USER}:{connection_id}")
    })
}

fn entry(connection_id: &str) -> Result<Entry, AppError> {
    Entry::new(APP_ID, &secret_user(connection_id)?).map_err(AppError::storage)
}

/// The production [`CredentialStore`]: the Linux Secret Service.
pub struct KeyringCredentialStore;

impl CredentialStore for KeyringCredentialStore {
    fn read(&self, connection_id: &str) -> Result<Option<ObsPassword>, AppError> {
        match entry(connection_id)?.get_password() {
            Ok(password) => Ok(Some(ObsPassword::from(password))),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(e) => Err(AppError::storage(e)),
        }
    }

    fn write(&self, connection_id: &str, password: &ObsPassword) -> Result<(), AppError> {
        entry(connection_id)?
            .set_password(password.expose())
            .map_err(AppError::storage)
    }

    fn delete(&self, connection_id: &str) -> Result<(), AppError> {
        match entry(connection_id)?.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(e) => Err(AppError::storage(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_credentials_have_distinct_keys_without_legacy_fallback() {
        assert_eq!(secret_user("default").unwrap(), "obs-websocket");
        assert_eq!(secret_user("studio").unwrap(), "obs-websocket:studio");
        assert_ne!(
            secret_user("studio").unwrap(),
            secret_user("event").unwrap()
        );
        assert_ne!(
            secret_user("obs-websocket").unwrap(),
            secret_user("default").unwrap()
        );
        assert!(secret_user("").is_err());
    }
}
