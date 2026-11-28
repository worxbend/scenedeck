//! Fake `ControllerDependencies` providers.

use crate::controller::dependencies::AppConfigProvider;
use crate::domain::connection::credential::{CredentialStore, ObsPassword};
use crate::infra::error::AppError;
use crate::storage::config::AppConfig;

/// Serves one fixed config without touching the filesystem.
pub(crate) struct StaticConfigProvider(pub(crate) AppConfig);

impl AppConfigProvider for StaticConfigProvider {
    fn load_config(&self) -> Result<AppConfig, AppError> {
        Ok(self.0.clone())
    }
}

/// Reports the config file as unusable, the way a corrupt or unreadable
/// `config.json` reaches the controller.
pub(crate) struct UnusableConfigProvider;

impl AppConfigProvider for UnusableConfigProvider {
    fn load_config(&self) -> Result<AppConfig, AppError> {
        Err(AppError::storage("config.json is corrupt"))
    }
}

/// Fails the test on any config read or keyring access, for fixtures that
/// must reach neither.
pub(crate) struct NoServices;

impl AppConfigProvider for NoServices {
    fn load_config(&self) -> Result<AppConfig, AppError> {
        panic!("fixture must not read configuration")
    }
}

impl CredentialStore for NoServices {
    fn read(&self, _connection_id: &str) -> Result<Option<ObsPassword>, AppError> {
        panic!("fixture must not access the keyring")
    }

    fn write(&self, _connection_id: &str, _password: &ObsPassword) -> Result<(), AppError> {
        panic!("fixture must not access the keyring")
    }

    fn delete(&self, _connection_id: &str) -> Result<(), AppError> {
        panic!("fixture must not access the keyring")
    }
}

/// A keyring with no saved password, for fixtures that may read it but
/// must never change it.
pub(crate) struct NoPassword;

impl CredentialStore for NoPassword {
    fn read(&self, _connection_id: &str) -> Result<Option<ObsPassword>, AppError> {
        Ok(None)
    }

    fn write(&self, _connection_id: &str, _password: &ObsPassword) -> Result<(), AppError> {
        panic!("fixture must not write a password")
    }

    fn delete(&self, _connection_id: &str) -> Result<(), AppError> {
        panic!("fixture must not delete a password")
    }
}
