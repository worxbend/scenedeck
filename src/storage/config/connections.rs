//! Saved OBS endpoints. Credentials are addressed separately by immutable ID.

use serde::{Deserialize, Serialize};

use super::{AppConfig, ObsConfig};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SavedConnection {
    pub id: String,
    pub name: String,
    pub obs: ObsConfig,
}

impl AppConfig {
    /// A fresh or legacy configuration presents its original endpoint as OBS.
    pub(crate) fn connection_profiles(&self) -> Vec<SavedConnection> {
        if self.connections.is_empty() {
            vec![SavedConnection {
                id: self.connection_id.clone(),
                name: "OBS".into(),
                obs: self.obs.clone(),
            }]
        } else {
            self.connections.clone()
        }
    }

    pub(super) fn normalize_connections(&mut self) -> bool {
        let mut changed = false;
        if self.connections.is_empty() {
            self.connections = self.connection_profiles();
            changed = true;
        }
        // Partial legacy configs may omit their schema version. Capture the
        // original default endpoint before any profile edit can redirect it.
        if self.legacy_registry_endpoint.is_none() {
            if let Some(original) = self
                .connections
                .iter()
                .find(|profile| profile.id == "default")
            {
                self.legacy_registry_endpoint =
                    Some(format!("{}:{}", original.obs.host, original.obs.port));
                changed = true;
            }
        }
        if let Some(profile) = self
            .connections
            .iter()
            .find(|profile| profile.id == self.connection_id)
        {
            if self.obs != profile.obs {
                self.obs = profile.obs.clone();
                changed = true;
            }
        }
        changed
    }

    /// Persist a new connection or edit its label/endpoint without changing ID.
    pub(crate) fn save_connection(&mut self, profile: SavedConnection) -> bool {
        if profile.id.is_empty()
            || profile.name.trim().is_empty()
            || profile.obs.host.trim().is_empty()
            || profile.obs.port == 0
        {
            return false;
        }
        let normalized = self.normalize_connections();
        if profile.id == self.connection_id {
            self.obs = profile.obs.clone();
        }
        if let Some(existing) = self
            .connections
            .iter_mut()
            .find(|entry| entry.id == profile.id)
        {
            if *existing == profile {
                return normalized;
            }
            *existing = profile;
        } else {
            self.connections.push(profile);
        }
        true
    }

    pub(crate) fn select_connection(&mut self, id: &str) -> bool {
        let profiles = self.connection_profiles();
        let Some(profile) = profiles.iter().find(|profile| profile.id == id) else {
            return false;
        };
        if self.connection_id == id {
            return false;
        }
        self.normalize_connections();
        self.connection_id = profile.id.clone();
        self.obs = profile.obs.clone();
        self.connections = profiles;
        true
    }
}

/// Reject ambiguous credential identities before any profile can be selected.
pub(super) fn deserialize_connections<'de, D>(
    deserializer: D,
) -> Result<Vec<SavedConnection>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let profiles = Vec::<SavedConnection>::deserialize(deserializer)?;
    let mut ids = std::collections::HashSet::new();
    if profiles.iter().any(|profile| {
        profile.id.is_empty() || profile.name.trim().is_empty() || !ids.insert(&profile.id)
    }) {
        return Err(serde::de::Error::custom(
            "saved connection IDs and names must be nonempty; IDs must be unique",
        ));
    }
    Ok(profiles)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_endpoint_and_renamed_connection_keep_identity_across_selection() {
        let mut config = AppConfig {
            obs: ObsConfig {
                host: "studio.local".into(),
                port: 4456,
            },
            ..Default::default()
        };
        let original = config.connection_profiles()[0].clone();
        let guest = SavedConnection {
            id: "guest-id".into(),
            name: "Event laptop".into(),
            obs: ObsConfig {
                host: "event.local".into(),
                port: 4455,
            },
        };
        assert!(config.save_connection(guest));
        assert!(config.select_connection("guest-id"));
        assert_eq!(config.obs.host, "event.local");
        assert!(config.save_connection(SavedConnection {
            name: "Streaming PC".into(),
            ..original.clone()
        }));
        assert_eq!(config.connection_id, "guest-id");
        assert!(config.select_connection(&original.id));
        assert_eq!(config.obs, original.obs);
        assert_eq!(config.connection_profiles()[0].name, "Streaming PC");
        assert!(!config.select_connection("missing"));
        let restored: AppConfig =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        assert_eq!(restored.connection_id, "default");
        assert_eq!(restored.connection_profiles(), config.connection_profiles());
    }

    #[test]
    fn duplicate_or_blank_profile_identity_is_not_silently_rebound() {
        for ids in [["same", "same"], ["", "other"]] {
            let profiles: Vec<_> = ids
                .iter()
                .map(|id| SavedConnection {
                    id: (*id).into(),
                    name: "OBS".into(),
                    obs: ObsConfig::default(),
                })
                .collect();
            let mut raw = serde_json::to_value(AppConfig::default()).unwrap();
            raw["connections"] = serde_json::to_value(profiles).unwrap();
            assert!(serde_json::from_value::<AppConfig>(raw).is_err());
        }
    }
}
