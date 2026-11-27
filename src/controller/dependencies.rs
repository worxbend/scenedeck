//! Explicit dependencies used by the application controller.
//!
//! The controller orchestrates OBS work but should not know how configuration
//! or secrets are persisted. These traits keep those boundary details
//! injectable without introducing a larger application framework.

use std::sync::Arc;

use crate::infra::error::AppError;
use crate::storage::config::{read_config, AppConfig, ConfigStartupNotice, LoadedConfig};
use crate::storage::secret;

pub trait AppConfigProvider: Send + Sync {
    fn load_config(&self) -> Result<AppConfig, AppError>;
}

pub trait ObsPasswordProvider: Send + Sync {
    fn obs_password(&self, _connection_id: &str) -> Result<Option<String>, AppError>;
}

#[derive(Clone)]
pub struct ControllerDependencies {
    config_provider: Arc<dyn AppConfigProvider>,
    password_provider: Arc<dyn ObsPasswordProvider>,
}

impl ControllerDependencies {
    pub fn new(
        config_provider: Arc<dyn AppConfigProvider>,
        password_provider: Arc<dyn ObsPasswordProvider>,
    ) -> Self {
        Self {
            config_provider,
            password_provider,
        }
    }

    pub fn load_config(&self) -> Result<AppConfig, AppError> {
        self.config_provider.load_config()
    }

    pub fn obs_password(&self, connection_id: &str) -> Result<Option<String>, AppError> {
        self.password_provider.obs_password(connection_id)
    }
}

impl Default for ControllerDependencies {
    fn default() -> Self {
        Self::new(
            Arc::new(FileAppConfigProvider),
            Arc::new(KeyringObsPasswordProvider),
        )
    }
}

pub struct FileAppConfigProvider;

impl AppConfigProvider for FileAppConfigProvider {
    fn load_config(&self) -> Result<AppConfig, AppError> {
        stored_config(read_config())
    }
}

/// Defaults stand in for a config file that does not exist yet, never for one
/// that exists but could not be read or parsed: the operator's real settings
/// (OBS endpoint, audio filter) are still in that file.
fn stored_config(loaded: LoadedConfig) -> Result<AppConfig, AppError> {
    match loaded.startup_notice {
        None | Some(ConfigStartupNotice::FirstLaunch) => Ok(loaded.config),
        Some(ConfigStartupNotice::ReadFailed(detail))
        | Some(ConfigStartupNotice::ParseFailed { detail, .. }) => Err(AppError::storage(detail)),
    }
}

pub struct KeyringObsPasswordProvider;

impl ObsPasswordProvider for KeyringObsPasswordProvider {
    fn obs_password(&self, connection_id: &str) -> Result<Option<String>, AppError> {
        secret::get_obs_password(connection_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::test_support::deps::StaticConfigProvider;
    use crate::domain::appearance::{ThemeMode, ThemePreference};
    use crate::storage::config::{LiveConfig, ObsConfig};

    struct StaticPasswordProvider(Option<String>);

    impl ObsPasswordProvider for StaticPasswordProvider {
        fn obs_password(&self, connection_id: &str) -> Result<Option<String>, AppError> {
            assert_eq!(connection_id, "studio");
            Ok(self.0.clone())
        }
    }

    #[test]
    fn dependencies_delegate_to_injected_providers() {
        let config = AppConfig {
            connection_id: "studio".into(),
            version: 1,
            obs: ObsConfig {
                host: "192.0.2.10".to_string(),
                port: 4455,
            },
            live: LiveConfig {
                audio_inputs: vec!["Mic".to_string()],
                ..Default::default()
            },
            appearance: ThemePreference {
                mode: ThemeMode::Dark,
                ..ThemePreference::default()
            },
            ..AppConfig::default()
        };

        let dependencies = ControllerDependencies::new(
            Arc::new(StaticConfigProvider(config)),
            Arc::new(StaticPasswordProvider(Some("secret".to_string()))),
        );

        let loaded = dependencies.load_config().unwrap();
        assert_eq!(loaded.obs.host, "192.0.2.10");
        assert_eq!(
            dependencies.obs_password(&loaded.connection_id).unwrap(),
            Some("secret".to_string())
        );
    }

    fn loaded(startup_notice: Option<ConfigStartupNotice>) -> LoadedConfig {
        LoadedConfig {
            config: AppConfig {
                connection_id: "studio".into(),
                ..AppConfig::default()
            },
            startup_notice,
        }
    }

    #[test]
    fn stored_config_serves_a_read_file_and_first_launch_defaults() {
        for notice in [None, Some(ConfigStartupNotice::FirstLaunch)] {
            let config = stored_config(loaded(notice.clone()));

            assert!(
                matches!(&config, Ok(config) if config.connection_id == "studio"),
                "{notice:?}: got {config:?}"
            );
        }
    }

    #[test]
    fn stored_config_refuses_an_unreadable_or_corrupt_file() {
        for (notice, detail) in [
            (
                ConfigStartupNotice::ReadFailed("permission denied".into()),
                "permission denied",
            ),
            (
                ConfigStartupNotice::ParseFailed {
                    detail: "expected value at line 1".into(),
                    backup: None,
                },
                "expected value at line 1",
            ),
        ] {
            let config = stored_config(loaded(Some(notice)));

            assert!(
                matches!(&config, Err(AppError::Storage(found)) if found == detail),
                "got {config:?}"
            );
        }
    }
}
