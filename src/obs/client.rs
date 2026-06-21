//! Thin async wrapper around `obws::Client`.
//!
//! This is the only module that imports from `obws`.  All external OBS types
//! are converted to domain types here or in `mapper.rs` before returning.

use std::collections::HashSet;
use std::sync::Arc;

use obws::Client;

use crate::controller::event::ConnectionInfo;
use crate::domain::audio::{AudioInput, AudioSourceScope};
use crate::domain::graph::SceneGraph;
use crate::domain::obs::ObsNamedList;
use crate::domain::output::{OutputRunState, OutputStatus};
use crate::domain::scene::SceneInventory;
use crate::infra::error::AppError;
use crate::obs::mapper;

/// Cheaply cloneable handle to an active OBS WebSocket session.
#[derive(Clone)]
pub struct ObsClient {
    inner: Arc<Client>,
}

impl ObsClient {
    /// Connect to OBS, returning the client and an owned event stream.
    pub async fn connect(
        host: &str,
        port: u16,
        password: Option<&str>,
    ) -> Result<(Self, obws::events::EventStream), AppError> {
        let client = Client::connect(host, port, password)
            .await
            .map_err(|e| AppError::Connection(e.to_string()))?;

        let events = client
            .events()
            .map_err(|e| AppError::Connection(e.to_string()))?;

        Ok((
            Self {
                inner: Arc::new(client),
            },
            events,
        ))
    }

    pub async fn get_version(&self) -> Result<ConnectionInfo, AppError> {
        self.inner
            .general()
            .version()
            .await
            .map(|v| mapper::map_version(&v))
            .map_err(|e| AppError::Request(e.to_string()))
    }

    pub async fn get_scene_inventory(&self) -> Result<SceneInventory, AppError> {
        self.inner
            .scenes()
            .list()
            .await
            .map(mapper::map_scenes)
            .map_err(|e| AppError::Request(e.to_string()))
    }

    /// Build the scene dependency graph by listing each scene's items and
    /// keeping only nested-scene sources (`OBS_SOURCE_TYPE_SCENE`).
    pub async fn get_scene_graph(&self, scene_names: &[String]) -> Result<SceneGraph, AppError> {
        use obws::responses::scene_items::SourceType;

        let mut graph = SceneGraph::default();

        for name in scene_names {
            let items = self
                .inner
                .scene_items()
                .list(obws::requests::scenes::SceneId::Name(name))
                .await
                .map_err(|e| AppError::Request(e.to_string()))?;

            let children: Vec<String> = items
                .into_iter()
                .filter(|item| item.source_type == SourceType::Scene)
                .map(|item| item.source_name)
                .collect();

            // Only record scenes that actually nest other scenes.
            if !children.is_empty() {
                graph.edges.insert(name.clone(), children);
            }
        }

        Ok(graph)
    }

    pub async fn set_current_program_scene(&self, name: &str) -> Result<(), AppError> {
        self.inner
            .scenes()
            .set_current_program_scene(obws::requests::scenes::SceneId::Name(name))
            .await
            .map_err(|e| AppError::Request(e.to_string()))
    }

    pub async fn get_profiles(&self) -> Result<ObsNamedList, AppError> {
        self.inner
            .profiles()
            .list()
            .await
            .map(|profiles| ObsNamedList {
                items: profiles.profiles,
                current: Some(profiles.current),
            })
            .map_err(|e| AppError::Request(e.to_string()))
    }

    pub async fn set_current_profile(&self, name: &str) -> Result<(), AppError> {
        self.inner
            .profiles()
            .set_current(name)
            .await
            .map_err(|e| AppError::Request(e.to_string()))
    }

    pub async fn create_profile(&self, name: &str) -> Result<(), AppError> {
        self.inner
            .profiles()
            .create(name)
            .await
            .map_err(|e| AppError::Request(e.to_string()))
    }

    pub async fn remove_profile(&self, name: &str) -> Result<(), AppError> {
        self.inner
            .profiles()
            .remove(name)
            .await
            .map_err(|e| AppError::Request(e.to_string()))
    }

    pub async fn get_scene_collections(&self) -> Result<ObsNamedList, AppError> {
        self.inner
            .scene_collections()
            .list()
            .await
            .map(|collections| ObsNamedList {
                items: collections.collections,
                current: Some(collections.current),
            })
            .map_err(|e| AppError::Request(e.to_string()))
    }

    pub async fn set_current_scene_collection(&self, name: &str) -> Result<(), AppError> {
        self.inner
            .scene_collections()
            .set_current(name)
            .await
            .map_err(|e| AppError::Request(e.to_string()))
    }

    pub async fn create_scene_collection(&self, name: &str) -> Result<(), AppError> {
        self.inner
            .scene_collections()
            .create(name)
            .await
            .map_err(|e| AppError::Request(e.to_string()))
    }

    pub async fn get_stream_status(&self) -> Result<OutputStatus, AppError> {
        self.inner
            .streaming()
            .status()
            .await
            .map(|status| OutputStatus {
                active: status.active,
                state: if status.reconnecting {
                    OutputRunState::Reconnecting
                } else if status.active {
                    OutputRunState::Active
                } else {
                    OutputRunState::Inactive
                },
                detail: None,
            })
            .map_err(|e| AppError::Request(e.to_string()))
    }

    pub async fn set_streaming(&self, active: bool) -> Result<(), AppError> {
        let streaming = self.inner.streaming();
        if active {
            streaming.start().await
        } else {
            streaming.stop().await
        }
        .map_err(|e| AppError::Request(e.to_string()))
    }

    pub async fn get_record_status(&self) -> Result<OutputStatus, AppError> {
        self.inner
            .recording()
            .status()
            .await
            .map(|status| OutputStatus {
                active: status.active,
                state: if status.paused {
                    OutputRunState::Paused
                } else if status.active {
                    OutputRunState::Active
                } else {
                    OutputRunState::Inactive
                },
                detail: None,
            })
            .map_err(|e| AppError::Request(e.to_string()))
    }

    pub async fn set_recording(&self, active: bool) -> Result<Option<String>, AppError> {
        let recording = self.inner.recording();
        if active {
            recording
                .start()
                .await
                .map(|()| None)
                .map_err(|e| AppError::Request(e.to_string()))
        } else {
            recording
                .stop()
                .await
                .map(Some)
                .map_err(|e| AppError::Request(e.to_string()))
        }
    }

    /// Toggle the mute state of an input using OBS's native toggle request.
    /// Returns the new mute state.
    pub async fn toggle_input_mute(&self, name: &str) -> Result<bool, AppError> {
        self.inner
            .inputs()
            .toggle_mute(obws::requests::inputs::InputId::Name(name))
            .await
            .map_err(|e| AppError::Request(e.to_string()))
    }

    pub async fn set_input_mute(&self, name: &str, muted: bool) -> Result<(), AppError> {
        self.inner
            .inputs()
            .set_muted(obws::requests::inputs::InputId::Name(name), muted)
            .await
            .map_err(|e| AppError::Request(e.to_string()))
    }

    pub async fn set_input_volume(&self, name: &str, volume_mul: f64) -> Result<(), AppError> {
        self.inner
            .inputs()
            .set_volume(
                obws::requests::inputs::InputId::Name(name),
                obws::requests::inputs::Volume::Mul(volume_mul as f32),
            )
            .await
            .map_err(|e| AppError::Request(e.to_string()))
    }

    /// Return audio-capable OBS inputs with mute + volume state.
    ///
    /// If `filter` is empty, all OBS inputs are scanned and only sources that
    /// successfully expose audio mute and volume state are returned. Otherwise
    /// `filter` is used directly as the explicit name list.
    pub async fn get_audio_inputs(&self, filter: &[String]) -> Result<Vec<AudioInput>, AppError> {
        let names: Vec<AudioInputSource> = if filter.is_empty() {
            self.inner
                .inputs()
                .list(None)
                .await
                .map_err(|e| AppError::Request(e.to_string()))?
                .into_iter()
                .map(|input| AudioInputSource::active_scene(input.id.name, Vec::new()))
                .collect()
        } else {
            filter
                .iter()
                .map(|name| AudioInputSource::active_scene(name.clone(), Vec::new()))
                .collect()
        };

        self.get_audio_inputs_by_name(names).await
    }

    /// Return enabled audio-capable inputs that belong to `scene_name`.
    ///
    /// Nested scenes and groups are followed recursively.  If `filter` is not
    /// empty it limits the scene-derived source list to the configured names.
    pub async fn get_scene_audio_inputs(
        &self,
        scene_name: &str,
        filter: &[String],
    ) -> Result<Vec<AudioInput>, AppError> {
        use obws::responses::scene_items::SourceType;

        let filter: HashSet<&str> = filter.iter().map(String::as_str).collect();
        let mut seen_inputs = HashSet::new();
        let mut sources = Vec::new();

        match self.get_global_audio_input_names().await {
            Ok(global_names) => {
                for name in global_names {
                    if seen_inputs.insert(name.clone()) {
                        sources.push(AudioInputSource {
                            name,
                            scope: AudioSourceScope::Global,
                            parent_scene_path: Vec::new(),
                        });
                    }
                }
            }
            Err(e) => tracing::warn!(%e, "global audio input lookup failed"),
        }

        let mut visited_containers = HashSet::new();
        let mut pending = vec![SceneAudioContainer {
            name: scene_name.to_string(),
            is_group: false,
            required: true,
            path: vec![scene_name.to_string()],
            scope: AudioSourceScope::ActiveScene,
        }];

        while let Some(container) = pending.pop() {
            if !visited_containers.insert((container.name.clone(), container.is_group)) {
                continue;
            }

            let items = if container.is_group {
                self.inner
                    .scene_items()
                    .list_group(obws::requests::scenes::SceneId::Name(&container.name))
                    .await
            } else {
                self.inner
                    .scene_items()
                    .list(obws::requests::scenes::SceneId::Name(&container.name))
                    .await
            };

            let items = match items {
                Ok(items) => items,
                Err(e) if container.required => return Err(AppError::Request(e.to_string())),
                Err(e) => {
                    tracing::debug!(
                        %e,
                        container = container.name,
                        is_group = container.is_group,
                        "nested scene audio scan skipped"
                    );
                    continue;
                }
            };

            for item in items {
                let enabled = self
                    .inner
                    .scene_items()
                    .enabled(
                        obws::requests::scenes::SceneId::Name(&container.name),
                        item.id,
                    )
                    .await
                    .unwrap_or(true);
                if !enabled {
                    continue;
                }

                match item.source_type {
                    SourceType::Input => {
                        if !filter.is_empty() && !filter.contains(item.source_name.as_str()) {
                            continue;
                        }
                        if seen_inputs.insert(item.source_name.clone()) {
                            sources.push(AudioInputSource {
                                name: item.source_name,
                                scope: container.scope,
                                parent_scene_path: container.path.clone(),
                            });
                        }
                    }
                    SourceType::Scene => {
                        let is_group = item.is_group.unwrap_or(false);
                        let mut path = container.path.clone();
                        path.push(item.source_name.clone());
                        pending.push(SceneAudioContainer {
                            name: item.source_name,
                            is_group,
                            required: false,
                            path,
                            scope: if is_group {
                                AudioSourceScope::Group
                            } else {
                                AudioSourceScope::NestedScene
                            },
                        });
                    }
                    _ => {}
                }
            }
        }

        self.get_audio_inputs_by_name(sources).await
    }

    async fn get_global_audio_input_names(&self) -> Result<Vec<String>, AppError> {
        let specials = self
            .inner
            .inputs()
            .specials()
            .await
            .map_err(|e| AppError::Request(e.to_string()))?;

        Ok([
            specials.desktop1,
            specials.desktop2,
            specials.mic1,
            specials.mic2,
            specials.mic3,
            specials.mic4,
        ]
        .into_iter()
        .flatten()
        .collect())
    }

    async fn get_audio_inputs_by_name(
        &self,
        sources: Vec<AudioInputSource>,
    ) -> Result<Vec<AudioInput>, AppError> {
        let mut result = Vec::new();
        for source in sources {
            let muted = match self
                .inner
                .inputs()
                .muted(obws::requests::inputs::InputId::Name(&source.name))
                .await
            {
                Ok(m) => m,
                Err(_) => continue,
            };

            let vol = match self
                .inner
                .inputs()
                .volume(obws::requests::inputs::InputId::Name(&source.name))
                .await
            {
                Ok(v) => v,
                Err(_) => continue,
            };

            result.push(
                AudioInput::new(source.name, muted, vol.mul as f64, vol.db as f64)
                    .with_source_context(source.scope, source.parent_scene_path),
            );
        }

        Ok(result)
    }
}

struct AudioInputSource {
    name: String,
    scope: AudioSourceScope,
    parent_scene_path: Vec<String>,
}

impl AudioInputSource {
    fn active_scene(name: String, parent_scene_path: Vec<String>) -> Self {
        Self {
            name,
            scope: AudioSourceScope::ActiveScene,
            parent_scene_path,
        }
    }
}

struct SceneAudioContainer {
    name: String,
    is_group: bool,
    required: bool,
    path: Vec<String>,
    scope: AudioSourceScope,
}
