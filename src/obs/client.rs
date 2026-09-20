//! Thin async wrapper around `obws::Client`.
//!
//! This is the only module that imports from `obws`.  All external OBS types
//! are converted to domain types here or in `mapper.rs` before returning.

use std::collections::HashSet;
use std::future::Future;
use std::sync::Arc;

use futures_util::{stream, StreamExt, TryStreamExt};
use obws::client::ConnectConfig;
use obws::requests::EventSubscription;
use obws::Client;

use crate::controller::event::ConnectionInfo;
use crate::domain::audio::{AudioInput, AudioSourceScope};
use crate::domain::graph::SceneGraph;
use crate::domain::obs::ObsNamedList;
use crate::domain::output::OutputStatus;
use crate::domain::scene::SceneInventory;
use crate::domain::stats::{ObsStats, StreamHealth};
use crate::infra::error::AppError;
use crate::obs::event::ObsEventStream;
use crate::obs::mapper;

const MAX_CONCURRENT_OBS_REQUESTS: usize = 8;

/// Run independent OBS requests concurrently while retaining input ordering.
///
/// Ordered buffering makes multiple failures deterministic: the earliest
/// input's error wins and stops the operation, just as it did when requests
/// were issued serially.
async fn collect_bounded_ordered<I, F, Fut, T, E>(
    inputs: I,
    concurrency: usize,
    request: F,
) -> Result<Vec<T>, E>
where
    I: IntoIterator,
    F: FnMut(I::Item) -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    stream::iter(inputs)
        .map(request)
        .buffered(concurrency.max(1))
        .try_collect()
        .await
}

/// Cheaply cloneable handle to an active OBS WebSocket session.
#[derive(Clone)]
pub struct ObsClient {
    inner: Arc<Client>,
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
        let client = Client::connect_with_config(ConnectConfig {
            host,
            port,
            password,
            event_subscriptions: Some(
                EventSubscription::ALL | EventSubscription::INPUT_VOLUME_METERS,
            ),
            broadcast_capacity: obws::client::DEFAULT_BROADCAST_CAPACITY,
            connect_timeout: obws::client::DEFAULT_CONNECT_TIMEOUT,
            dangerous: None,
        })
        .await
        .map_err(AppError::connection)?;

        let events = client.events().map_err(AppError::connection)?;

        Ok((
            Self {
                inner: Arc::new(client),
            },
            // Wrapped here so the raw `obws` stream never leaves this module.
            ObsEventStream::new(events),
        ))
    }

    pub async fn get_version(&self) -> Result<ConnectionInfo, AppError> {
        self.inner
            .general()
            .version()
            .await
            .map(|v| mapper::map_version(&v))
            .map_err(AppError::request)
    }

    pub async fn get_scene_inventory(&self) -> Result<SceneInventory, AppError> {
        self.inner
            .scenes()
            .list()
            .await
            .map(mapper::map_scenes)
            .map_err(AppError::request)
    }

    /// Build the scene dependency graph by listing each scene's items and
    /// keeping only nested-scene sources (`OBS_SOURCE_TYPE_SCENE`).
    pub async fn get_scene_graph(&self, scene_names: &[String]) -> Result<SceneGraph, AppError> {
        use obws::responses::scene_items::SourceType;

        let mut graph = SceneGraph::default();

        let scenes = collect_bounded_ordered(
            scene_names.iter().cloned(),
            MAX_CONCURRENT_OBS_REQUESTS,
            |name| async move {
                let items = self
                    .inner
                    .scene_items()
                    .list(obws::requests::scenes::SceneId::Name(&name))
                    .await
                    .map_err(AppError::request)?;

                let children: Vec<String> = items
                    .into_iter()
                    .filter(|item| item.source_type == SourceType::Scene)
                    .map(|item| item.source_name)
                    .collect();

                Ok((name, children))
            },
        )
        .await?;

        for (name, children) in scenes {
            // Only record scenes that actually nest other scenes.
            if !children.is_empty() {
                graph.edges.insert(name, children);
            }
        }

        Ok(graph)
    }

    pub async fn set_current_program_scene(&self, name: &str) -> Result<(), AppError> {
        self.inner
            .scenes()
            .set_current_program_scene(obws::requests::scenes::SceneId::Name(name))
            .await
            .map_err(AppError::request)
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
            .map_err(AppError::request)
    }

    pub async fn set_current_profile(&self, name: &str) -> Result<(), AppError> {
        self.inner
            .profiles()
            .set_current(name)
            .await
            .map_err(AppError::request)
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
            .map_err(AppError::request)
    }

    pub async fn set_current_scene_collection(&self, name: &str) -> Result<(), AppError> {
        self.inner
            .scene_collections()
            .set_current(name)
            .await
            .map_err(AppError::request)
    }

    pub async fn get_stream_status(&self) -> Result<OutputStatus, AppError> {
        self.inner
            .streaming()
            .status()
            .await
            .map(mapper::map_stream_status)
            .map_err(AppError::request)
    }

    /// OBS process performance counters (`GetStats`) — CPU, memory, FPS,
    /// render/output frame timing. Available regardless of streaming state.
    pub async fn get_obs_stats(&self) -> Result<ObsStats, AppError> {
        self.inner
            .general()
            .stats()
            .await
            .map(mapper::map_stats)
            .map_err(AppError::request)
    }

    /// Stream output health (`GetStreamStatus`) — congestion, dropped frames,
    /// and the cumulative byte counter used to derive a rolling bitrate. OBS
    /// reports zeroed counters while the stream is inactive.
    pub async fn get_stream_health(&self) -> Result<StreamHealth, AppError> {
        self.inner
            .streaming()
            .status()
            .await
            .map(mapper::map_stream_health)
            .map_err(AppError::request)
    }

    pub async fn set_streaming(&self, active: bool) -> Result<(), AppError> {
        let streaming = self.inner.streaming();
        if active {
            streaming.start().await
        } else {
            streaming.stop().await
        }
        .map_err(AppError::request)
    }

    pub async fn get_record_status(&self) -> Result<OutputStatus, AppError> {
        self.inner
            .recording()
            .status()
            .await
            .map(mapper::map_record_status)
            .map_err(AppError::request)
    }

    pub async fn set_recording(&self, active: bool) -> Result<Option<String>, AppError> {
        let recording = self.inner.recording();
        if active {
            recording
                .start()
                .await
                .map(|()| None)
                .map_err(AppError::request)
        } else {
            recording.stop().await.map(Some).map_err(AppError::request)
        }
    }

    pub async fn set_input_mute(&self, name: &str, muted: bool) -> Result<(), AppError> {
        self.inner
            .inputs()
            .set_muted(obws::requests::inputs::InputId::Name(name), muted)
            .await
            .map_err(AppError::request)
    }

    pub async fn set_input_volume(&self, name: &str, volume_mul: f64) -> Result<(), AppError> {
        self.inner
            .inputs()
            .set_volume(
                obws::requests::inputs::InputId::Name(name),
                obws::requests::inputs::Volume::Mul(volume_mul as f32),
            )
            .await
            .map_err(AppError::request)
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
                Err(e) if container.required => return Err(AppError::request(e)),
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

            let items = collect_bounded_ordered(items, MAX_CONCURRENT_OBS_REQUESTS, |item| async {
                let enabled = self
                    .inner
                    .scene_items()
                    .enabled(
                        obws::requests::scenes::SceneId::Name(&container.name),
                        item.id,
                    )
                    .await
                    .unwrap_or(true);
                Ok::<_, std::convert::Infallible>((item, enabled))
            })
            .await
            .unwrap_or_else(|never| match never {});

            for (item, enabled) in items {
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
            .map_err(AppError::request)?;

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
        let inputs =
            collect_bounded_ordered(sources, MAX_CONCURRENT_OBS_REQUESTS, |source| async move {
                let muted = match self
                    .inner
                    .inputs()
                    .muted(obws::requests::inputs::InputId::Name(&source.name))
                    .await
                {
                    Ok(muted) => muted,
                    Err(_) => return Ok::<_, AppError>(None),
                };

                let volume = match self
                    .inner
                    .inputs()
                    .volume(obws::requests::inputs::InputId::Name(&source.name))
                    .await
                {
                    Ok(volume) => volume,
                    Err(_) => return Ok::<_, AppError>(None),
                };

                Ok::<_, AppError>(Some(
                    AudioInput::new(source.name, muted, volume.mul as f64, volume.db as f64)
                        .with_source_context(source.scope, source.parent_scene_path),
                ))
            })
            .await?;

        Ok(inputs.into_iter().flatten().collect())
    }
}

struct AudioInputSource {
    name: String,
    scope: AudioSourceScope,
    parent_scene_path: Vec<String>,
}

struct SceneAudioContainer {
    name: String,
    is_group: bool,
    required: bool,
    path: Vec<String>,
    scope: AudioSourceScope,
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
