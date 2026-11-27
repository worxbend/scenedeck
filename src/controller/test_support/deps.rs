//! Fake `ControllerDependencies` providers.

use crate::controller::dependencies::{AppConfigProvider, ObsPasswordProvider};
use crate::infra::error::AppError;
use crate::storage::config::AppConfig;

/// Serves one fixed config without touching the filesystem.
pub(crate) struct StaticConfigProvider(pub(crate) AppConfig);

impl AppConfigProvider for StaticConfigProvider {
    fn load_config(&self) -> AppConfig {
        self.0.clone()
    }
}

/// Fails the test on any config read or keyring access, for fixtures that
/// must reach neither.
pub(crate) struct NoServices;

impl AppConfigProvider for NoServices {
    fn load_config(&self) -> AppConfig {
        panic!("fixture must not read configuration")
    }
}

impl ObsPasswordProvider for NoServices {
    fn obs_password(&self, _connection_id: &str) -> Result<Option<String>, AppError> {
        panic!("fixture must not access the keyring")
    }
}
