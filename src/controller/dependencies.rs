//! Explicit dependencies used by the application controller.
//!
//! The controller orchestrates OBS work but should not know how configuration
//! or secrets are persisted. These traits keep those boundary details
//! injectable without introducing a larger application framework.

use std::sync::Arc;

use crate::infra::error::AppError;
use crate::storage::config::{read_config, AppConfig};
use crate::storage::secret;

pub trait AppConfigProvider: Send + Sync {
    fn load_config(&self) -> AppConfig;
}

pub trait ObsPasswordProvider: Send + Sync {
    fn obs_password(&self) -> Result<Option<String>, AppError>;
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

    pub fn load_config(&self) -> AppConfig {
        self.config_provider.load_config()
    }

    pub fn obs_password(&self) -> Result<Option<String>, AppError> {
        self.password_provider.obs_password()
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
    fn load_config(&self) -> AppConfig {
        read_config().config
    }
}

pub struct KeyringObsPasswordProvider;

impl ObsPasswordProvider for KeyringObsPasswordProvider {
    fn obs_password(&self) -> Result<Option<String>, AppError> {
        secret::get_obs_password()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::appearance::ThemeMode;
    use crate::storage::config::{LiveConfig, ObsConfig};

    struct StaticConfigProvider(AppConfig);

    impl AppConfigProvider for StaticConfigProvider {
        fn load_config(&self) -> AppConfig {
            self.0.clone()
        }
    }

    struct StaticPasswordProvider(Option<String>);

    impl ObsPasswordProvider for StaticPasswordProvider {
        fn obs_password(&self) -> Result<Option<String>, AppError> {
            Ok(self.0.clone())
        }
    }

    #[test]
    fn dependencies_delegate_to_injected_providers() {
        let config = AppConfig {
            version: 1,
            obs: ObsConfig {
                host: "192.0.2.10".to_string(),
                port: 4455,
            },
            live: LiveConfig {
                audio_inputs: vec!["Mic".to_string()],
                ..Default::default()
            },
            theme_mode: ThemeMode::Dark,
        };

        let dependencies = ControllerDependencies::new(
            Arc::new(StaticConfigProvider(config)),
            Arc::new(StaticPasswordProvider(Some("secret".to_string()))),
        );

        assert_eq!(dependencies.load_config().obs.host, "192.0.2.10");
        assert_eq!(
            dependencies.obs_password().unwrap(),
            Some("secret".to_string())
        );
    }
}
