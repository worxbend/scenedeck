//! OBS WebSocket password storage via the Linux Secret Service (D-Bus keyring).
//!
//! Passwords are stored in the system keyring (GNOME Keyring / KWallet), never
//! in `config.json`.  The entry is keyed by the app ID and a fixed username.

use keyring::{Entry, Error as KeyringError};

use crate::app_info::APP_ID;
use crate::infra::error::AppError;

const SECRET_USER: &str = "obs-websocket";

fn entry() -> Result<Entry, AppError> {
    Entry::new(APP_ID, SECRET_USER).map_err(AppError::storage)
}

/// Retrieve the stored OBS password, or `None` if none is set.
pub fn get_obs_password() -> Result<Option<String>, AppError> {
    match entry()?.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(e) => Err(AppError::storage(e)),
    }
}

/// Store the OBS password in the keyring (overwriting any existing value).
pub fn set_obs_password(password: &str) -> Result<(), AppError> {
    entry()?.set_password(password).map_err(AppError::storage)
}

/// Delete the stored OBS password.  Succeeds even if none was set.
pub fn delete_obs_password() -> Result<(), AppError> {
    match entry()?.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(e) => Err(AppError::storage(e)),
    }
}
