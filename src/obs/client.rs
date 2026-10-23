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

use crate::controller::event::ConnectionInfo;
use crate::domain::audio::{volume_db_to_mul, MAX_VOLUME_DB, MIN_VOLUME_DB};
use crate::domain::obs::ObsNamedList;
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
        password: Option<&str>,
    ) -> Result<(Self, ObsEventStream), AppError> {
        let client = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            Client::connect_with_config(ConnectConfig {
                host,
                port,
                password,
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

    pub async fn get_version(&self) -> Result<ConnectionInfo, AppError> {
        self.request(self.inner.general().version())
            .await
            .map(|v| {
                let mut info = mapper::map_version(&v);
                info.endpoint = self.endpoint.clone();
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

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use futures_util::{SinkExt, StreamExt};
    use serde_json::json;
    use tokio::net::TcpListener;
    use tokio_websockets::{Message, ServerBuilder};

    use super::{collect_bounded_ordered, ObsClient};
    use crate::obs::event::ObsEvent;

    #[tokio::test]
    async fn connect_negotiates_obs_websocket_and_requests_meter_events() {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind mock OBS server");
        let port = listener.local_addr().expect("mock server address").port();
        let (send_event, event_requested) = tokio::sync::oneshot::channel();
        let (release_server, hold_server) = tokio::sync::oneshot::channel();

        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.expect("accept OBS client");
            let (_, mut websocket) = ServerBuilder::new()
                .accept(tcp)
                .await
                .expect("upgrade mock OBS connection");

            websocket
                .send(Message::text(
                    json!({
                        "op": 0,
                        "d": {
                            "obsWebSocketVersion": "5.5.0",
                            "rpcVersion": 1
                        }
                    })
                    .to_string(),
                ))
                .await
                .expect("send OBS Hello");

            let identify = websocket
                .next()
                .await
                .expect("client sent Identify")
                .expect("read Identify frame");
            let identify: serde_json::Value =
                serde_json::from_str(identify.as_text().expect("Identify is a text frame"))
                    .expect("Identify is valid JSON");

            websocket
                .send(Message::text(
                    json!({
                        "op": 2,
                        "d": { "negotiatedRpcVersion": 1 }
                    })
                    .to_string(),
                ))
                .await
                .expect("send OBS Identified");

            let version_request = websocket
                .next()
                .await
                .expect("client sent GetVersion")
                .expect("read GetVersion frame");
            let version_request: serde_json::Value = serde_json::from_str(
                version_request
                    .as_text()
                    .expect("GetVersion is a text frame"),
            )
            .expect("GetVersion is valid JSON");
            assert_eq!(version_request["op"], 6);
            assert_eq!(version_request["d"]["requestType"], "GetVersion");
            let request_id = version_request["d"]["requestId"].clone();

            websocket
                .send(Message::text(
                    json!({
                        "op": 7,
                        "d": {
                            "requestType": "GetVersion",
                            "requestId": request_id,
                            "requestStatus": { "result": true, "code": 100 },
                            "responseData": {
                                "obsStudioVersion": "31.0.0",
                                "obsWebSocketVersion": "5.5.0",
                                "rpcVersion": 1,
                                "availableRequests": [],
                                "supportedImageFormats": [],
                                "platform": "mock",
                                "platformDescription": "SceneDeck test server"
                            }
                        }
                    })
                    .to_string(),
                ))
                .await
                .expect("send GetVersion response");

            event_requested.await.expect("request mock OBS event");
            websocket
                .send(Message::text(
                    json!({
                        "op": 5,
                        "d": {
                            "eventType": "CurrentProgramSceneChanged",
                            "eventIntent": obws::requests::EventSubscription::SCENES.bits(),
                            "eventData": {
                                "sceneName": "Program",
                                "sceneUuid": "00000000-0000-0000-0000-000000000001"
                            }
                        }
                    })
                    .to_string(),
                ))
                .await
                .expect("send scene-change event");

            let _ = hold_server.await;
            identify
        });

        let (client, mut events) = tokio::time::timeout(
            Duration::from_secs(2),
            ObsClient::connect("127.0.0.1", port, None),
        )
        .await
        .expect("OBS handshake timed out")
        .expect("OBS handshake failed");

        send_event.send(()).expect("request mock OBS event");
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), events.next())
                .await
                .expect("OBS event timed out"),
            Some(ObsEvent::CurrentProgramSceneChanged("Program".to_string()))
        );

        release_server.send(()).expect("release mock OBS server");
        let identify = server.await.expect("mock OBS task panicked");
        assert_eq!(identify["op"], 1);
        assert_eq!(identify["d"]["rpcVersion"], 1);
        let subscriptions = identify["d"]["eventSubscriptions"]
            .as_u64()
            .expect("Identify includes numeric event subscriptions");
        assert_ne!(
            subscriptions
                & u64::from(obws::requests::EventSubscription::INPUT_VOLUME_METERS.bits()),
            0,
            "volume-meter events must be requested during the handshake"
        );

        drop(events);
        drop(client);
    }

    #[tokio::test]
    async fn bounded_collection_preserves_input_order_and_limit() {
        let active = Arc::new(AtomicUsize::new(0));
        let maximum = Arc::new(AtomicUsize::new(0));

        let values = collect_bounded_ordered(0..6, 2, |value| {
            let active = Arc::clone(&active);
            let maximum = Arc::clone(&maximum);
            async move {
                let now_active = active.fetch_add(1, Ordering::SeqCst) + 1;
                maximum.fetch_max(now_active, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis((6 - value) * 2)).await;
                active.fetch_sub(1, Ordering::SeqCst);
                Ok::<_, ()>(value)
            }
        })
        .await
        .unwrap();

        assert_eq!(values, vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(maximum.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn bounded_collection_reports_the_first_error_in_input_order() {
        let error = collect_bounded_ordered(0..4, 4, |value| async move {
            tokio::time::sleep(Duration::from_millis(if value == 1 { 10 } else { 1 })).await;
            match value {
                1 => Err("first"),
                2 => Err("second"),
                _ => Ok(value),
            }
        })
        .await
        .unwrap_err();

        assert_eq!(error, "first");
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
        let scenes = self
            .request(self.inner.scenes().list())
            .await
            .map_err(AppError::request)?;
        let mut snapshot = ControlSnapshot::default();
        snapshot.studio.program = scenes.current_program_scene.map(|s| s.name);
        snapshot.studio.preview = scenes.current_preview_scene.map(|s| s.name);
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
                        state: format!("{:?}", media.state),
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
        use crate::domain::control::{AudioMonitor, ControlAction as A, MediaAction};
        use obws::requests::{inputs::InputId, scenes::SceneId, sources::SourceId};
        match action {
            A::ActivateScene {
                scene,
                studio_enabled,
            } => {
                return self.activate_scene_if(scene, *studio_enabled, valid).await;
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
                        InputId::Name(input),
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
                    .request(self.inner.media_inputs().status(InputId::Name(input)))
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
                    self.inner
                        .media_inputs()
                        .set_cursor(InputId::Name(input), cursor),
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
                            source: SourceId::Name(input),
                            filter,
                            enabled: *enabled,
                        }),
                )
                .await
            }
            A::SetText { input, text } => {
                let settings = self
                    .request(
                        self.inner
                            .inputs()
                            .settings::<serde_json::Value>(InputId::Name(input)),
                    )
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
                            input: InputId::Name(input),
                            settings: &serde_json::json!({"text":text}),
                            overlay: Some(true),
                        }),
                )
                .await
            }
            A::SetInputMuted { input, muted } => {
                self.mutation(
                    valid,
                    self.inner.inputs().set_muted(InputId::Name(input), *muted),
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
                        InputId::Name(input),
                        obws::requests::inputs::Volume::Mul(volume_db_to_mul(*db) as f32),
                    ),
                )
                .await
            }
            A::SetAudioMonitor { input, monitor } => {
                self.mutation(
                    valid,
                    self.inner.inputs().set_audio_monitor_type(
                        InputId::Name(input),
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
                        InputId::Name(input),
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
                    self.inner
                        .inputs()
                        .set_audio_balance(InputId::Name(input), *balance),
                )
                .await
            }
            A::SetAudioTracks { input, tracks } => {
                self.mutation(
                    valid,
                    self.inner
                        .inputs()
                        .set_audio_tracks(InputId::Name(input), tracks.map(Some)),
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
mod control_tests {
    use super::*;
    use crate::domain::control::{ControlAction, MediaAction};
    use futures_util::{SinkExt, StreamExt};
    use serde_json::{json, Value};
    use tokio::net::TcpListener;
    use tokio_websockets::{Message, ServerBuilder};

    pub(super) async fn mock_client(
        script: Vec<(&'static str, Value)>,
    ) -> (ObsClient, tokio::task::JoinHandle<Vec<Value>>) {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let (_, mut socket) = ServerBuilder::new().accept(tcp).await.unwrap();
            socket
                .send(Message::text(
                    json!({"op":0,"d":{"obsWebSocketVersion":"5.5.0","rpcVersion":1}}).to_string(),
                ))
                .await
                .unwrap();
            socket.next().await.unwrap().unwrap();
            socket
                .send(Message::text(
                    json!({"op":2,"d":{"negotiatedRpcVersion":1}}).to_string(),
                ))
                .await
                .unwrap();
            let mut requests = Vec::new();
            let version = json!({"obsStudioVersion":"31.0.0","obsWebSocketVersion":"5.5.0","rpcVersion":1,"availableRequests":[],"supportedImageFormats":["png"],"platform":"mock","platformDescription":"mock"});
            for (expected, data) in std::iter::once(("GetVersion", version)).chain(script) {
                let message = socket.next().await.unwrap().unwrap();
                let request: Value = serde_json::from_str(message.as_text().unwrap()).unwrap();
                assert_eq!(request["d"]["requestType"], expected);
                let failed = data
                    .get("__error")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let status = if failed {
                    json!({"result":false,"code":data.get("__code").and_then(Value::as_u64).unwrap_or(600),"comment":data.get("__comment").and_then(Value::as_str).unwrap_or("mock missing source")})
                } else {
                    json!({"result":true,"code":100})
                };
                let mut response = json!({"op":7,"d":{"requestId":request["d"]["requestId"],"requestType":expected,"requestStatus":status}});
                if data != json!({}) {
                    response["d"]["responseData"] = data;
                }
                socket
                    .send(Message::text(response.to_string()))
                    .await
                    .unwrap();
                requests.push(request);
            }
            let _ = ready_rx.await;
            requests
        });
        let (client, _events) = ObsClient::connect("127.0.0.1", port, None).await.unwrap();
        ready_tx.send(()).unwrap();
        (client, server)
    }

    #[tokio::test]
    async fn directional_actions_preserve_scene_item_owner_and_explicit_state() {
        let (client, server) = mock_client(vec![
            ("SetSceneItemEnabled", json!({})),
            ("PauseRecord", json!({})),
            ("ResumeRecord", json!({})),
            ("StartReplayBuffer", json!({})),
            ("TriggerMediaInputAction", json!({})),
        ])
        .await;
        client
            .execute_control(&ControlAction::SetItemEnabled {
                owner: "Nested group".into(),
                item_id: 42,
                enabled: false,
            })
            .await
            .unwrap();
        client
            .execute_control(&ControlAction::SetRecordingPaused(true))
            .await
            .unwrap();
        client
            .execute_control(&ControlAction::SetRecordingPaused(false))
            .await
            .unwrap();
        client
            .execute_control(&ControlAction::SetReplayActive(true))
            .await
            .unwrap();
        client
            .execute_control(&ControlAction::Media {
                input: "Clip".into(),
                action: MediaAction::Pause,
            })
            .await
            .unwrap();
        let requests = server.await.unwrap();
        assert_eq!(
            requests[1]["d"]["requestData"],
            json!({"sceneName":"Nested group","sceneItemId":42,"sceneItemEnabled":false})
        );
        assert_eq!(
            requests[5]["d"]["requestData"]["mediaAction"],
            "OBS_WEBSOCKET_MEDIA_INPUT_ACTION_PAUSE"
        );
    }

    #[tokio::test]
    async fn seeking_checks_fresh_duration_before_sending_cursor() {
        let (client, server) = mock_client(vec![(
            "GetMediaInputStatus",
            json!({"mediaState":"OBS_MEDIA_STATE_PLAYING","mediaDuration":5000,"mediaCursor":0}),
        )])
        .await;
        let error = client
            .execute_control(&ControlAction::SeekMedia {
                input: "Clip".into(),
                position_ms: 5001,
            })
            .await
            .unwrap_err();
        assert!(error.to_string().contains("outside current duration"));
        assert_eq!(server.await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn recursive_inventory_keeps_disabled_group_owner_and_input_identity() {
        let input = json!({"inputName":"Mic", "inputUuid":"00000000-0000-0000-0000-000000000001", "inputKind":"pulse_input_capture", "unversionedInputKind":"pulse_input_capture"});
        let (client, server) = mock_client(vec![
            ("GetInputList", json!({"inputs":[input]})),
            ("GetSceneItemList", json!({"sceneItems":[{"sceneItemId":7,"sceneItemIndex":0,"sourceName":"Group","sourceType":"OBS_SOURCE_TYPE_SCENE","isGroup":true}]})),
            ("GetSceneItemEnabled", json!({"sceneItemEnabled":false})),
            ("GetGroupSceneItemList", json!({"sceneItems":[{"sceneItemId":9,"sceneItemIndex":0,"sourceName":"Mic","sourceType":"OBS_SOURCE_TYPE_INPUT","inputKind":"pulse_input_capture"}]})),
            ("GetSceneItemEnabled", json!({"sceneItemEnabled":true})),
        ]).await;
        let (items, warnings) = client.scan_source_items("Program", 4096).await;
        assert!(warnings.is_empty());
        assert_eq!(items.len(), 2);
        assert_eq!(items[1].owner, "Group");
        assert_eq!(items[1].item_id, 9);
        assert_eq!(items[1].path, ["Program", "Group"]);
        assert!(items[1].enabled);
        assert!(!items[1].ancestor_enabled);
        assert_eq!(items[1].input_uuid, "00000000-0000-0000-0000-000000000001");
        assert_eq!(server.await.unwrap().len(), 6);
    }

    #[tokio::test]
    async fn source_inventory_budget_stops_extra_visibility_requests_and_reports_partial() {
        let (client, server) = mock_client(vec![
            ("GetInputList", json!({"inputs":[]})),
            ("GetSceneItemList", json!({"sceneItems":[
                {"sceneItemId":1,"sceneItemIndex":0,"sourceName":"First","sourceType":"OBS_SOURCE_TYPE_INPUT","inputKind":"image_source"},
                {"sceneItemId":2,"sceneItemIndex":1,"sourceName":"Second","sourceType":"OBS_SOURCE_TYPE_INPUT","inputKind":"image_source"}
            ]})),
            ("GetSceneItemEnabled", json!({"sceneItemEnabled":true})),
        ]).await;
        let (items, warnings) = client.scan_source_items("Program", 1).await;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].input, "First");
        assert!(warnings.iter().any(|warning| warning.contains("truncated")));
        assert_eq!(server.await.unwrap().len(), 4);
    }

    #[tokio::test]
    async fn missing_nested_scene_reports_partial_coverage_without_losing_other_items() {
        let (client, server) = mock_client(vec![
            ("GetInputList", json!({"inputs":[]})),
            ("GetSceneItemList", json!({"sceneItems":[
                {"sceneItemId":1,"sceneItemIndex":0,"sourceName":"Missing nested scene","sourceType":"OBS_SOURCE_TYPE_SCENE","isGroup":false},
                {"sceneItemId":2,"sceneItemIndex":1,"sourceName":"Sibling","sourceType":"OBS_SOURCE_TYPE_INPUT","inputKind":"image_source"}
            ]})),
            ("GetSceneItemEnabled",json!({"sceneItemEnabled":true})),
            ("GetSceneItemEnabled",json!({"sceneItemEnabled":true})),
            ("GetSceneItemList",json!({"__error":true})),
        ]).await;
        let (items, warnings) = client.scan_source_items("Program", 4096).await;
        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|item| item.input == "Sibling"));
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Missing nested scene"));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn text_edit_only_overlays_text_and_preserves_other_properties() {
        let (client, server) = mock_client(vec![
            ("GetInputSettings", json!({"inputKind":"text_ft2_source_v2", "inputSettings":{"text":"Old","color":123,"from_file":false}})),
            ("GetInputDefaultSettings", json!({"defaultInputSettings":{"from_file":false}})),
            ("SetInputSettings", json!({})),
        ]).await;
        client
            .execute_control(&ControlAction::SetText {
                input: "Caption".into(),
                text: "New".into(),
            })
            .await
            .unwrap();
        let requests = server.await.unwrap();
        assert_eq!(
            requests[3]["d"]["requestData"],
            json!({"inputName":"Caption","inputSettings":{"text":"New"},"overlay":true})
        );
    }

    #[tokio::test]
    async fn file_backed_text_never_sends_write() {
        let (client, server) = mock_client(vec![
            (
                "GetInputSettings",
                json!({"inputKind":"text_ft2_source_v2", "inputSettings":{}}),
            ),
            (
                "GetInputDefaultSettings",
                json!({"defaultInputSettings":{"from_file":true}}),
            ),
        ])
        .await;
        assert!(client
            .execute_control(&ControlAction::SetText {
                input: "Caption".into(),
                text: "New".into()
            })
            .await
            .is_err());
        assert_eq!(server.await.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn filter_enabled_action_retains_source_and_filter_identity() {
        let (client, server) = mock_client(vec![("SetSourceFilterEnabled", json!({}))]).await;
        client
            .execute_control(&ControlAction::SetFilterEnabled {
                input: "Mic".into(),
                filter: "Noise suppression".into(),
                enabled: false,
            })
            .await
            .unwrap();
        let requests = server.await.unwrap();
        assert_eq!(
            requests[1]["d"]["requestData"],
            json!({"sourceName":"Mic","filterName":"Noise suppression","filterEnabled":false})
        );
    }

    #[test]
    fn thumbnails_fit_canvas_without_stretching_or_unbounded_decode() {
        assert_eq!(thumbnail_dimensions(1920, 1080), (384, 216));
        assert_eq!(thumbnail_dimensions(1080, 1920), (122, 216));
        assert_eq!(thumbnail_dimensions(4000, 1000), (384, 96));
        assert_eq!(thumbnail_dimensions(0, 0), (1, 1));
        assert_eq!(thumbnail_dimensions(320, 180), (320, 180));
    }
    #[tokio::test]
    async fn cancelled_thumbnail_does_not_request_canvas_settings() {
        let (client, server) = mock_client(vec![]).await;
        assert!(client
            .get_scene_thumbnail("Portrait", &std::sync::atomic::AtomicBool::new(true))
            .await
            .is_err());
        assert_eq!(server.await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn thumbnail_request_is_bounded_and_preserves_portrait_canvas() {
        let (client,server)=mock_client(vec![
            ("GetVideoSettings",json!({"fpsNumerator":30,"fpsDenominator":1,"baseWidth":1080,"baseHeight":1920,"outputWidth":1080,"outputHeight":1920})),
            ("GetSourceScreenshot",json!({"imageData":"data:image/png;base64,fixture"})),
        ]).await;
        client
            .get_scene_thumbnail("Portrait", &std::sync::atomic::AtomicBool::new(false))
            .await
            .unwrap();
        let requests = server.await.unwrap();
        assert_eq!(
            requests[2]["d"]["requestData"],
            json!({"sourceName":"Portrait","imageFormat":"png","imageWidth":122,"imageHeight":216,"imageCompressionQuality":-1})
        );
    }

    #[tokio::test]
    async fn invalid_audio_values_never_reach_obs() {
        let (client, server) = mock_client(vec![]).await;
        assert!(client
            .execute_control(&ControlAction::SetAudioSync {
                input: "Mic".into(),
                offset_ms: -951
            })
            .await
            .is_err());
        assert!(client
            .execute_control(&ControlAction::SetAudioBalance {
                input: "Mic".into(),
                balance: f32::NAN
            })
            .await
            .is_err());
        assert_eq!(server.await.unwrap().len(), 1);
    }
}

#[cfg(test)]
#[path = "regression_tests.rs"]
mod regression_tests;
