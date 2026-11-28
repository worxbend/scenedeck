//! Thin async wrapper around `obws::Client`.
//!
//! Only this adapter layer imports from `obws`. External OBS types are
//! converted to domain types here or in `mapper.rs` before returning.

use std::future::Future;
use std::sync::Arc;

use futures_util::{stream, StreamExt};
use obws::client::ConnectConfig;
use obws::requests::EventSubscription;
use obws::Client;

use crate::domain::audio::{volume_db_to_mul, MAX_VOLUME_DB, MIN_VOLUME_DB};
use crate::domain::connection::credential::ObsPassword;
use crate::domain::obs::{ConnectionInfo, ObsNamedList};
use crate::domain::output::OutputStatus;
use crate::domain::scene::SceneInventory;
use crate::domain::stats::{ObsStats, StreamHealth};
use crate::infra::error::AppError;
use crate::obs::event::ObsEventStream;
use crate::obs::mapper;

pub(super) const MAX_CONCURRENT_OBS_REQUESTS: usize = super::requests::MAX_CONCURRENT_REQUESTS;

/// Run independent OBS requests concurrently while retaining input ordering.
///
/// Ordered buffering makes multiple failures deterministic: the earliest
/// input's error wins after already admitted requests have been drained.
pub(super) async fn collect_bounded_ordered<I, F, Fut, T, E>(
    inputs: I,
    concurrency: usize,
    request: F,
) -> Result<Vec<T>, E>
where
    I: IntoIterator,
    F: FnMut(I::Item) -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    // Drain admitted requests even after an error: dropping siblings would
    // abandon obws registrations and unnecessarily retire the connection.
    stream::iter(inputs)
        .map(request)
        .buffered(concurrency.max(1))
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect()
}

/// Cheaply cloneable handle to an active OBS WebSocket session.
#[derive(Clone)]
pub struct ObsClient {
    pub(super) inner: Arc<Client>,
    requests: Arc<super::requests::RequestGate>,
    images: Arc<super::requests::RequestGate>,
    endpoint: String,
    connection_id: String,
    observation: Arc<std::sync::atomic::AtomicU64>,
}

impl ObsClient {
    /// Connect to OBS, returning the client and an owned event stream.
    ///
    /// The subscription adds `INPUT_VOLUME_METERS` to the usual set. That is a
    /// high-volume event — OBS pushes every active input's levels about twenty
    /// times a second — and it is the only source of the mixer volume meters,
    /// so it has to be asked for at connect time.
    pub async fn connect(
        host: &str,
        port: u16,
        password: Option<&ObsPassword>,
    ) -> Result<(Self, ObsEventStream), AppError> {
        let client = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            Client::connect_with_config(ConnectConfig {
                host,
                port,
                password: password.map(ObsPassword::expose),
                event_subscriptions: Some(
                    EventSubscription::ALL | EventSubscription::INPUT_VOLUME_METERS,
                ),
                broadcast_capacity: obws::client::DEFAULT_BROADCAST_CAPACITY,
                connect_timeout: obws::client::DEFAULT_CONNECT_TIMEOUT,
                dangerous: None,
            }),
        )
        .await
        .map_err(AppError::connection)?
        .map_err(AppError::connection)?;

        let events = client.events().map_err(AppError::connection)?;

        Ok((
            Self {
                inner: Arc::new(client),
                requests: Arc::new(super::requests::RequestGate::default()),
                images: Arc::new(super::requests::RequestGate::optional_image()),
                endpoint: format!("{host}:{port}"),
                connection_id: "default".into(),
                observation: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            },
            // Wrapped here so the raw `obws` stream never leaves this module.
            ObsEventStream::new(events),
        ))
    }

    /// Whether two handles share the same WebSocket connection, including across reconnects to one endpoint.
    pub(crate) fn same_connection(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    /// Fence this connection before cancelling its tasks. Existing uncertain
    /// requests are not retried; drop all handles to release the socket.
    pub fn retire(&self) {
        self.requests.retire();
        self.images.retire();
    }

    /// Close ordinary admission synchronously, without dropping admitted work.
    pub(crate) fn begin_disconnect(&self) {
        self.requests.close();
        self.images.close();
    }

    /// Drain admitted controls, then issue one private shutdown handle. The
    /// closed ordinary lane can never be reopened or used to replay commands.
    pub(crate) async fn disconnect_client(&self) -> Result<Self, AppError> {
        self.requests
            .drain_for_shutdown()
            .await
            .map_err(AppError::request)?;
        Ok(Self {
            requests: Arc::new(super::requests::RequestGate::shutdown()),
            ..self.clone()
        })
    }

    pub(crate) fn retire_after_deadline(&self) {
        self.requests
            .retire_with_reason(super::requests::RetirementReason::Deadline);
    }

    pub(crate) fn retirement_reason(&self) -> Option<String> {
        self.requests.retirement_reason()
    }

    /// Complete when cancellation or a response deadline retires this socket's
    /// admission. The session owner must then stop its tasks and drop handles.
    pub async fn wait_retired(&self) {
        self.requests.wait_retired().await;
    }

    pub(super) async fn request<T>(
        &self,
        request: impl Future<Output = obws::error::Result<T>>,
    ) -> obws::error::Result<T> {
        self.requests.run(request).await
    }

    pub(super) async fn mutation<T>(
        &self,
        valid: &(impl Fn() -> bool + Sync),
        request: impl Future<Output = obws::error::Result<T>>,
    ) -> obws::error::Result<T> {
        self.requests.run_if_valid(request, valid).await
    }

    pub(crate) fn next_observation(&self) -> u64 {
        self.observation
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel)
            + 1
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Bind connection metadata before publishing or cloning the live session.
    pub(crate) fn with_connection_id(mut self, connection_id: String) -> Self {
        self.connection_id = connection_id;
        self
    }

    pub async fn get_version(&self) -> Result<ConnectionInfo, AppError> {
        self.request(self.inner.general().version())
            .await
            .map(|v| {
                let mut info = mapper::map_version(&v);
                info.endpoint = self.endpoint.clone();
                info.connection_id = self.connection_id.clone();
                info
            })
            .map_err(AppError::request)
    }

    pub async fn get_scene_inventory(&self) -> Result<SceneInventory, AppError> {
        self.request(self.inner.scenes().list())
            .await
            .map(mapper::map_scenes)
            .map_err(AppError::request)
    }

    #[cfg(test)]
    pub async fn set_current_program_scene(&self, name: &str) -> Result<(), AppError> {
        self.execute_control(&crate::domain::control::ControlAction::SetProgram(
            name.into(),
        ))
        .await
    }

    pub async fn get_profiles(&self) -> Result<ObsNamedList, AppError> {
        self.request(self.inner.profiles().list())
            .await
            .map(|profiles| ObsNamedList {
                items: profiles.profiles,
                current: Some(profiles.current),
            })
            .map_err(AppError::request)
    }

    pub async fn set_current_profile(&self, name: &str) -> Result<(), AppError> {
        self.request(self.inner.profiles().set_current(name))
            .await
            .map_err(AppError::request)
    }

    pub async fn get_scene_collections(&self) -> Result<ObsNamedList, AppError> {
        self.request(self.inner.scene_collections().list())
            .await
            .map(|collections| ObsNamedList {
                items: collections.collections,
                current: Some(collections.current),
            })
            .map_err(AppError::request)
    }

    pub async fn set_current_scene_collection(&self, name: &str) -> Result<(), AppError> {
        self.request(self.inner.scene_collections().set_current(name))
            .await
            .map_err(AppError::request)
    }

    pub async fn get_stream_status(&self) -> Result<OutputStatus, AppError> {
        self.request(self.inner.streaming().status())
            .await
            .map(mapper::map_stream_status)
            .map_err(AppError::request)
    }

    /// OBS process performance counters (`GetStats`) — CPU, memory, FPS,
    /// render/output frame timing. Available regardless of streaming state.
    pub async fn get_obs_stats(&self) -> Result<ObsStats, AppError> {
        let mut stats = self
            .request(self.inner.general().stats())
            .await
            .map(mapper::map_stats)
            .map_err(AppError::request)?;
        match self.request(self.inner.config().video_settings()).await {
            Ok(settings) => stats.configured_fps = mapper::configured_fps(&settings),
            Err(error) => tracing::debug!(%error, "configured video FPS unavailable"),
        }
        Ok(stats)
    }

    /// Stream output health (`GetStreamStatus`) — congestion, dropped frames,
    /// and the cumulative byte counter used to derive a rolling bitrate. OBS
    /// reports zeroed counters while the stream is inactive.
    pub async fn get_stream_health(&self) -> Result<StreamHealth, AppError> {
        self.request(self.inner.streaming().status())
            .await
            .map(mapper::map_stream_health)
            .map_err(AppError::request)
    }

    pub(crate) async fn set_streaming_if(
        &self,
        active: bool,
        valid: &(impl Fn() -> bool + Sync),
    ) -> Result<(), AppError> {
        let streaming = self.inner.streaming();
        if active {
            self.mutation(valid, streaming.start()).await
        } else {
            self.mutation(valid, streaming.stop()).await
        }
        .map_err(AppError::request)
    }

    pub async fn get_record_status(&self) -> Result<OutputStatus, AppError> {
        self.request(self.inner.recording().status())
            .await
            .map(mapper::map_record_status)
            .map_err(AppError::request)
    }

    pub(crate) async fn set_recording_if(
        &self,
        active: bool,
        valid: &(impl Fn() -> bool + Sync),
    ) -> Result<Option<String>, AppError> {
        let recording = self.inner.recording();
        if active {
            self.mutation(valid, recording.start())
                .await
                .map(|()| None)
                .map_err(AppError::request)
        } else {
            self.mutation(valid, recording.stop())
                .await
                .map(Some)
                .map_err(AppError::request)
        }
    }
}

impl ObsClient {
    /// Read a bounded, explicitly scoped operating snapshot. Optional facilities can fail independently.
    pub async fn get_control_snapshot(
        &self,
        scene: Option<&str>,
        include_sources: bool,
    ) -> Result<crate::domain::control::ControlSnapshot, AppError> {
        use crate::domain::control::ControlSnapshot;
        let mut scene_list = self
            .request(self.inner.scenes().list())
            .await
            .map_err(AppError::request)?;
        let preview = scene_list.current_preview_scene.take().map(|s| s.name);
        let inventory = mapper::map_scenes(scene_list);
        let mut snapshot = ControlSnapshot {
            scenes: inventory.scenes,
            ..Default::default()
        };
        snapshot.studio.program = inventory.current_id;
        snapshot.studio.preview = preview;
        snapshot.scene = scene
            .map(str::to_owned)
            .or_else(|| snapshot.studio.program.clone());
        snapshot.studio.enabled = self
            .request(self.inner.ui().studio_mode_enabled())
            .await
            .ok();
        if let Ok(transitions) = self.request(self.inner.transitions().list()).await {
            snapshot.studio.transitions = transitions
                .transitions
                .into_iter()
                .map(|t| t.id.name)
                .collect();
        }
        if let Ok(transition) = self.request(self.inner.transitions().current()).await {
            snapshot.studio.transition = Some(transition.id.name);
            snapshot.studio.transition_duration_ms =
                transition.duration.map(|d| d.whole_milliseconds() as i64);
            snapshot.studio.transition_fixed = transition.fixed;
        }
        snapshot.replay_active = self.request(self.inner.replay_buffer().status()).await.ok();
        snapshot.last_replay_path = self
            .request(self.inner.replay_buffer().last_replay())
            .await
            .ok()
            .filter(|s| !s.is_empty());
        snapshot.virtual_camera_active = self.request(self.inner.virtual_cam().status()).await.ok();
        if let Some(scene) = snapshot.scene.as_ref().filter(|_| include_sources) {
            let (items, warnings) = self.scan_source_items(scene, 4096).await;
            snapshot.items = items;
            snapshot.warnings = warnings;
        }
        Ok(snapshot)
    }

    pub async fn get_scene_thumbnail(
        &self,
        source: &str,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<String, AppError> {
        if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(AppError::request("Thumbnail cancelled"));
        }
        let settings = self
            .images
            .run(self.inner.config().video_settings())
            .await
            .map_err(AppError::request)?;
        if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(AppError::request("Thumbnail cancelled"));
        }
        let (width, height) = thumbnail_dimensions(settings.base_width, settings.base_height);
        self.source_screenshot(source, width, height, &self.images)
            .await
    }

    pub(crate) async fn get_thumbnail_collection(&self) -> Result<String, AppError> {
        self.images
            .run(self.inner.scene_collections().list())
            .await
            .map(|collections| collections.current)
            .map_err(AppError::request)
    }
    pub async fn get_source_screenshot(&self, source: &str) -> Result<String, AppError> {
        self.source_screenshot(source, 960, 540, &self.requests)
            .await
    }
    async fn source_screenshot(
        &self,
        source: &str,
        width: u32,
        height: u32,
        requests: &super::requests::RequestGate,
    ) -> Result<String, AppError> {
        requests
            .run(
                self.inner
                    .sources()
                    .take_screenshot(obws::requests::sources::TakeScreenshot {
                        source: obws::requests::sources::SourceId::Name(source),
                        format: "png",
                        width: Some(width),
                        height: Some(height),
                        compression_quality: Some(-1),
                    }),
            )
            .await
            .map_err(AppError::request)
    }

    pub async fn get_source_detail(
        &self,
        input: &str,
    ) -> Result<crate::domain::control::SourceDetail, AppError> {
        use crate::domain::control::*;
        use obws::requests::{inputs::InputId, sources::SourceId};
        let settings = self
            .request(
                self.inner
                    .inputs()
                    .settings::<serde_json::Value>(InputId::Name(input)),
            )
            .await;
        let mut detail = SourceDetail {
            input: input.to_owned(),
            ..Default::default()
        };
        match settings {
            Ok(settings) => {
                detail.kind = settings.kind;
                if is_text_kind(&detail.kind) {
                    match self
                        .request(
                            self.inner
                                .inputs()
                                .default_settings::<serde_json::Value>(&detail.kind),
                        )
                        .await
                    {
                        Ok(defaults) => {
                            detail.text = settings
                                .settings
                                .get("text")
                                .or_else(|| defaults.get("text"))
                                .and_then(serde_json::Value::as_str)
                                .map(str::to_owned);
                            detail.text_from_file =
                                text_is_file_backed(&settings.settings, &defaults);
                        }
                        Err(error) => detail.warnings.push(format!(
                            "Text defaults unavailable; editing disabled: {error}"
                        )),
                    }
                }
            }
            Err(error) => detail.warnings.push(format!("Input settings: {error}")),
        }
        match self
            .request(self.inner.filters().list(SourceId::Name(input)))
            .await
        {
            Ok(filters) => {
                detail.filters = filters
                    .into_iter()
                    .map(|f| SourceFilter {
                        name: f.name,
                        kind: f.kind,
                        enabled: f.enabled,
                    })
                    .collect()
            }
            Err(error) => detail.warnings.push(format!("Filters: {error}")),
        }
        if matches!(detail.kind.as_str(), "ffmpeg_source" | "vlc_source") {
            match self
                .request(self.inner.media_inputs().status(InputId::Name(input)))
                .await
            {
                Ok(media) => {
                    detail.media = Some(MediaStatus {
                        state: mapper::map_media_state(media.state),
                        duration_ms: media.duration.map(|d| d.whole_milliseconds() as i64),
                        cursor_ms: media.cursor.map(|d| d.whole_milliseconds() as i64),
                    })
                }
                Err(error) => detail.warnings.push(format!("Media: {error}")),
            }
        }
        match self
            .request(self.inner.inputs().muted(InputId::Name(input)))
            .await
        {
            Ok(muted) => detail.muted = Some(muted),
            Err(error) if super::audio::is_non_audio(&error) => {}
            Err(error) => detail
                .warnings
                .push(format!("Audio mute unavailable: {error}")),
        }
        if detail.muted.is_some() {
            match self
                .request(self.inner.inputs().volume(InputId::Name(input)))
                .await
            {
                Ok(volume) => detail.volume_db = Some(f64::from(volume.db)),
                Err(error) => detail
                    .warnings
                    .push(format!("Audio volume unavailable: {error}")),
            }
        }
        if detail.muted.is_some() {
            let audio = async {
                let inputs = self.inner.inputs();
                let (balance, offset, monitor, tracks) = tokio::join!(
                    self.request(inputs.audio_balance(InputId::Name(input))),
                    self.request(inputs.audio_sync_offset(InputId::Name(input))),
                    self.request(inputs.audio_monitor_type(InputId::Name(input))),
                    self.request(inputs.audio_tracks(InputId::Name(input)))
                );
                let (balance, offset, monitor, tracks) = (balance?, offset?, monitor?, tracks?);
                Ok::<_, obws::error::Error>(AudioSettings {
                    balance,
                    sync_offset_ms: offset.whole_milliseconds() as i64,
                    monitor: match monitor {
                        obws::common::MonitorType::MonitorOnly => AudioMonitor::MonitorOnly,
                        obws::common::MonitorType::MonitorAndOutput => {
                            AudioMonitor::MonitorAndOutput
                        }
                        _ => AudioMonitor::Off,
                    },
                    tracks,
                })
            }
            .await;
            match audio {
                Ok(audio) => detail.audio = Some(audio),
                Err(error) => detail.warnings.push(format!("Advanced audio: {error}")),
            }
        }
        Ok(detail)
    }

    /// Directional actions never toggle against potentially stale UI state.
    #[cfg(test)]
    pub async fn execute_control(
        &self,
        action: &crate::domain::control::ControlAction,
    ) -> Result<(), AppError> {
        self.execute_control_if(action, &|| true).await
    }

    pub(crate) async fn execute_control_if(
        &self,
        action: &crate::domain::control::ControlAction,
        valid: &(impl Fn() -> bool + Sync),
    ) -> Result<(), AppError> {
        self.execute_control_with_identity_if(action, None, valid)
            .await
    }

    pub(crate) async fn execute_control_with_identity_if(
        &self,
        action: &crate::domain::control::ControlAction,
        target_uuid: Option<&str>,
        valid: &(impl Fn() -> bool + Sync),
    ) -> Result<(), AppError> {
        use crate::domain::control::{AudioMonitor, ControlAction as A, MediaAction};
        use obws::requests::{inputs::InputId, scenes::SceneId, sources::SourceId};
        let identity = target_uuid
            .map(|value| value.parse().map(InputId::Uuid))
            .transpose()
            .map_err(AppError::request)?;
        let uuid = identity.and_then(|id| id.as_uuid());
        match action {
            A::ActivateScene {
                scene,
                studio_enabled,
            } => {
                return self
                    .activate_scene_if(
                        uuid.map(SceneId::Uuid).unwrap_or(SceneId::Name(scene)),
                        *studio_enabled,
                        valid,
                    )
                    .await;
            }
            A::SetItemEnabled {
                owner,
                item_id,
                enabled,
            } => {
                self.mutation(
                    valid,
                    self.inner
                        .scene_items()
                        .set_enabled(obws::requests::scene_items::SetEnabled {
                            scene: SceneId::Name(owner),
                            item_id: *item_id,
                            enabled: *enabled,
                        }),
                )
                .await
            }
            A::SetStudioMode(enabled) => {
                self.mutation(valid, self.inner.ui().set_studio_mode_enabled(*enabled))
                    .await
            }
            A::SetProgram(name) => {
                self.mutation(
                    valid,
                    self.inner
                        .scenes()
                        .set_current_program_scene(SceneId::Name(name)),
                )
                .await
            }
            A::Take => {
                self.mutation(valid, self.inner.transitions().trigger())
                    .await
            }
            A::SetTransition(name) => {
                self.mutation(valid, self.inner.transitions().set_current(name))
                    .await
            }
            A::SetTransitionDuration(ms) => {
                if !(50..=20000).contains(ms) {
                    return Err(AppError::request("Transition duration must be 50–20000 ms"));
                }
                self.mutation(
                    valid,
                    self.inner
                        .transitions()
                        .set_current_duration(time::Duration::milliseconds(*ms)),
                )
                .await
            }
            A::SetRecordingPaused(paused) => {
                if *paused {
                    self.mutation(valid, self.inner.recording().pause()).await
                } else {
                    self.mutation(valid, self.inner.recording().resume()).await
                }
            }
            A::SplitRecording => {
                self.mutation(valid, self.inner.recording().split_file())
                    .await
            }
            A::RecordChapter(name) => {
                self.mutation(
                    valid,
                    self.inner
                        .recording()
                        .create_chapter(if name.trim().is_empty() {
                            None
                        } else {
                            Some(name)
                        }),
                )
                .await
            }
            A::SetReplayActive(active) => {
                if *active {
                    self.mutation(valid, self.inner.replay_buffer().start())
                        .await
                } else {
                    self.mutation(valid, self.inner.replay_buffer().stop())
                        .await
                }
            }
            A::SaveReplay => {
                self.mutation(valid, self.inner.replay_buffer().save())
                    .await
            }
            A::SetVirtualCamera(active) => {
                if *active {
                    self.mutation(valid, self.inner.virtual_cam().start()).await
                } else {
                    self.mutation(valid, self.inner.virtual_cam().stop()).await
                }
            }
            A::Media { input, action } => {
                self.mutation(
                    valid,
                    self.inner.media_inputs().trigger_action(
                        uuid.map(InputId::Uuid).unwrap_or(InputId::Name(input)),
                        match action {
                            MediaAction::Play => obws::common::MediaAction::Play,
                            MediaAction::Pause => obws::common::MediaAction::Pause,
                            MediaAction::Stop => obws::common::MediaAction::Stop,
                            MediaAction::Restart => obws::common::MediaAction::Restart,
                            MediaAction::Next => obws::common::MediaAction::Next,
                            MediaAction::Previous => obws::common::MediaAction::Previous,
                        },
                    ),
                )
                .await
            }
            A::SeekMedia { input, position_ms } => {
                let status = self
                    .request(
                        self.inner
                            .media_inputs()
                            .status(uuid.map(InputId::Uuid).unwrap_or(InputId::Name(input))),
                    )
                    .await
                    .map_err(AppError::request)?;
                let duration = status.duration.filter(|d| d.is_positive()).ok_or_else(|| {
                    AppError::request("Media duration unavailable; seeking disabled")
                })?;
                let cursor = time::Duration::milliseconds(*position_ms);
                if cursor.is_negative() || cursor > duration {
                    return Err(AppError::request("Media position outside current duration"));
                }
                self.mutation(
                    valid,
                    self.inner.media_inputs().set_cursor(
                        uuid.map(InputId::Uuid).unwrap_or(InputId::Name(input)),
                        cursor,
                    ),
                )
                .await
            }
            A::SetFilterEnabled {
                input,
                filter,
                enabled,
            } => {
                self.mutation(
                    valid,
                    self.inner
                        .filters()
                        .set_enabled(obws::requests::filters::SetEnabled {
                            source: uuid.map(SourceId::Uuid).unwrap_or(SourceId::Name(input)),
                            filter,
                            enabled: *enabled,
                        }),
                )
                .await
            }
            A::SetText { input, text } => {
                let settings = self
                    .request(self.inner.inputs().settings::<serde_json::Value>(
                        uuid.map(InputId::Uuid).unwrap_or(InputId::Name(input)),
                    ))
                    .await
                    .map_err(AppError::request)?;
                if !crate::domain::control::is_text_kind(&settings.kind) {
                    return Err(AppError::request(
                        "Only built-in text sources can be edited",
                    ));
                }
                let defaults = self
                    .request(
                        self.inner
                            .inputs()
                            .default_settings::<serde_json::Value>(&settings.kind),
                    )
                    .await
                    .map_err(AppError::request)?;
                if text_is_file_backed(&settings.settings, &defaults) {
                    return Err(AppError::request(
                        "File-backed text sources cannot be edited",
                    ));
                }
                self.mutation(
                    valid,
                    self.inner
                        .inputs()
                        .set_settings(obws::requests::inputs::SetSettings {
                            input: uuid.map(InputId::Uuid).unwrap_or(InputId::Name(input)),
                            settings: &serde_json::json!({"text":text}),
                            overlay: Some(true),
                        }),
                )
                .await
            }
            A::SetInputMuted { input, muted } => {
                self.mutation(
                    valid,
                    self.inner.inputs().set_muted(
                        uuid.map(InputId::Uuid).unwrap_or(InputId::Name(input)),
                        *muted,
                    ),
                )
                .await
            }
            A::SetInputVolumeDb { input, db } => {
                if !db.is_finite() || !(MIN_VOLUME_DB..=MAX_VOLUME_DB).contains(db) {
                    return Err(AppError::request("Audio level must be -100–0 dB"));
                }
                self.mutation(
                    valid,
                    self.inner.inputs().set_volume(
                        uuid.map(InputId::Uuid).unwrap_or(InputId::Name(input)),
                        obws::requests::inputs::Volume::Mul(volume_db_to_mul(*db) as f32),
                    ),
                )
                .await
            }
            A::SetAudioMonitor { input, monitor } => {
                self.mutation(
                    valid,
                    self.inner.inputs().set_audio_monitor_type(
                        uuid.map(InputId::Uuid).unwrap_or(InputId::Name(input)),
                        match monitor {
                            AudioMonitor::Off => obws::common::MonitorType::None,
                            AudioMonitor::MonitorOnly => obws::common::MonitorType::MonitorOnly,
                            AudioMonitor::MonitorAndOutput => {
                                obws::common::MonitorType::MonitorAndOutput
                            }
                        },
                    ),
                )
                .await
            }
            A::SetAudioSync { input, offset_ms } => {
                if !(-950..=20000).contains(offset_ms) {
                    return Err(AppError::request("Audio sync offset must be -950–20000 ms"));
                }
                self.mutation(
                    valid,
                    self.inner.inputs().set_audio_sync_offset(
                        uuid.map(InputId::Uuid).unwrap_or(InputId::Name(input)),
                        time::Duration::milliseconds(*offset_ms),
                    ),
                )
                .await
            }
            A::SetAudioBalance { input, balance } => {
                if !balance.is_finite() || !(0.0..=1.0).contains(balance) {
                    return Err(AppError::request("Audio balance must be between 0 and 1"));
                }
                self.mutation(
                    valid,
                    self.inner.inputs().set_audio_balance(
                        uuid.map(InputId::Uuid).unwrap_or(InputId::Name(input)),
                        *balance,
                    ),
                )
                .await
            }
            A::SetAudioTracks { input, tracks } => {
                self.mutation(
                    valid,
                    self.inner.inputs().set_audio_tracks(
                        uuid.map(InputId::Uuid).unwrap_or(InputId::Name(input)),
                        tracks.map(Some),
                    ),
                )
                .await
            }
        }
        .map_err(AppError::request)
    }
}

fn thumbnail_dimensions(width: u32, height: u32) -> (u32, u32) {
    let width = width.max(1);
    let height = height.max(1);
    let scale = (384.0 / f64::from(width))
        .min(216.0 / f64::from(height))
        .min(1.0);
    (
        (f64::from(width) * scale).round().max(1.0) as u32,
        (f64::from(height) * scale).round().max(1.0) as u32,
    )
}

fn text_is_file_backed(settings: &serde_json::Value, defaults: &serde_json::Value) -> bool {
    ["read_from_file", "from_file"].iter().any(|key| {
        settings
            .get(*key)
            .or_else(|| defaults.get(*key))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod control_tests;
