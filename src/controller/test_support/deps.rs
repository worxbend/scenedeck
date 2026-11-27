//! Fake `ControllerDependencies` providers.

use crate::controller::dependencies::{AppConfigProvider, ObsPasswordProvider};
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

impl ObsPasswordProvider for NoServices {
    fn obs_password(&self, _connection_id: &str) -> Result<Option<String>, AppError> {
        panic!("fixture must not access the keyring")
    }
}
