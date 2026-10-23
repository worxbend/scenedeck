//! Session-scoped reads and explicit native operations for the operating workspace.

use crate::controller::event::AppEvent;
use crate::domain::audio::AudioInput;
use crate::domain::control::{
    ControlAction, ControlSnapshot, SourceDetail, SourceItem, WorkspaceWarning,
};
use crate::domain::workflow::ScopeBinding;
use crate::obs::client::ObsClient;
use futures_util::FutureExt;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::{runtime::Handle, task::JoinHandle};

#[derive(Debug, Clone)]
pub struct WorkspaceRequest {
    pub epoch: u64,
    pub sequence: u64,
    pub scope: ScopeBinding,
    pub scene: Option<String>,
    pub input: Option<String>,
    pub include_sources: bool,
    pub screenshot: bool,
    pub result_id: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct WorkspaceObservation {
    pub snapshot: ControlSnapshot,
    pub detail: Option<SourceDetail>,
    pub usage: Vec<SourceItem>,
    pub audio: Vec<AudioInput>,
    pub stream_status: Option<crate::domain::output::OutputStatus>,
    pub record_status: Option<crate::domain::output::OutputStatus>,
    pub screenshot: Option<String>,
    pub warnings: Vec<WorkspaceWarning>,
    pub observed_at: Instant,
}

struct WorkspaceJob {
    request: WorkspaceRequest,
    action: Option<ControlAction>,
    expected_text: Option<String>,
    revision: u64,
    ticket: u64,
    activation_revision: u64,
    tx: crate::controller::event_sender::EventSender,
}

#[derive(Default)]
struct WorkspaceQueue {
    running: bool,
    pending: Option<WorkspaceJob>,
}

pub(crate) struct WorkspaceController {
    runtime: Handle,
    tx: crate::controller::event_sender::EventSender,
    scope: crate::controller::lifecycle::SessionScope,
    slot: Arc<Mutex<Option<ObsClient>>>,
    task: Option<JoinHandle<()>>,
    media_task: Option<JoinHandle<()>>,
    audio_tasks: Vec<JoinHandle<()>>,
    queued: Arc<Mutex<WorkspaceQueue>>,
    cancellation: Arc<std::sync::atomic::AtomicU64>,
    revision: Arc<std::sync::atomic::AtomicU64>,
}

impl WorkspaceController {
    pub fn new(
        runtime: Handle,
        tx: SyncSender<AppEvent>,
        slot: Arc<Mutex<Option<ObsClient>>>,
    ) -> Self {
        Self {
            runtime,
            tx: crate::controller::event_sender::EventSender::new(tx),
            scope: crate::controller::lifecycle::SessionScope::initial(slot.clone()),
            slot,
            task: None,
            media_task: None,
            audio_tasks: Vec::new(),
            queued: Arc::new(Mutex::new(WorkspaceQueue::default())),
            cancellation: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            revision: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub fn set_scope(&mut self, scope: crate::controller::lifecycle::SessionScope) {
        self.cancel();
        self.task = None;
        self.media_task = None;
        self.audio_tasks.clear();
        self.scope = scope;
        self.queued = Arc::new(Mutex::new(WorkspaceQueue::default()));
    }

    pub fn cancel(&mut self) {
        self.cancellation
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        self.revision
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        self.queued.lock().expect("workspace queue").pending = None;
        // Keep already-dispatched reads alive to drain their protocol entries.
        // Session retirement, unlike UI cancellation, cancels the whole socket.
        self.audio_tasks.clear();
    }

    pub fn audio(&mut self, scope: ScopeBinding, action: ControlAction) {
        if scope.collection.is_empty() {
            return;
        }
        let Some(client) = self.slot.lock().ok().and_then(|slot| slot.clone()) else {
            return;
        };
        self.audio_tasks.retain(|task| !task.is_finished());
        if self.audio_tasks.len() >= 16 {
            tracing::warn!("audio command admission full");
            return;
        }
        let cancellation = self.cancellation.clone();
        let ticket = cancellation.load(std::sync::atomic::Ordering::Acquire);
        let tx = self.tx.scoped(&self.scope);
        if let Some(task) = self.scope.spawn(&self.runtime, async move {
            let Some(evidence) = tx.operation(
                super::lifecycle::OperationKind::Audio,
                client.endpoint().into(),
            ) else {
                return;
            };
            let operation = async {
                validate_scope(&client, &scope).await?;
                if cancellation.load(std::sync::atomic::Ordering::Acquire) != ticket
                    || !tx.accepts_current_scope()
                {
                    return Ok(());
                }
                client
                    .execute_control_if(&action, &|| {
                        let valid = cancellation.load(std::sync::atomic::Ordering::Acquire)
                            == ticket
                            && tx.accepts_current_scope();
                        if valid {
                            evidence.dispatch();
                        }
                        valid
                    })
                    .await
                    .map_err(|error| error.user_message())
            };
            let result = bounded_workspace_read(&client, operation).await;
            if result.is_ok() {
                evidence.acknowledge();
            }
            evidence.finish(result.clone());
            if let Err(error) = result {
                if cancellation.load(std::sync::atomic::Ordering::Acquire) == ticket {
                    let _ = tx.send(AppEvent::WorkspaceAudioFailed(error)).await;
                }
            }
        }) {
            self.audio_tasks.push(task);
        }
    }

    pub fn media(&mut self, request: WorkspaceRequest) {
        if self
            .media_task
            .as_ref()
            .is_some_and(|task| !task.is_finished())
        {
            return;
        }
        let Some(input) = request.input.clone() else {
            return;
        };
        let Some(client) = self.slot.lock().ok().and_then(|slot| slot.clone()) else {
            return;
        };
        let cancellation = self.cancellation.clone();
        let ticket = cancellation.load(std::sync::atomic::Ordering::Acquire);
        let tx = self.tx.scoped(&self.scope);
        self.media_task = self.scope.spawn_reserved(&self.runtime, async move {
            let operation = async {
                validate_scope(&client, &request.scope).await?;
                client
                    .get_source_detail(&input)
                    .await
                    .map_err(|error| error.user_message())
            };
            let result = bounded_workspace_read(&client, operation).await;
            if cancellation.load(std::sync::atomic::Ordering::Acquire) == ticket {
                let _ = tx
                    .send(AppEvent::WorkspaceMediaLoaded {
                        epoch: request.epoch,
                        sequence: request.sequence,
                        input,
                        result,
                    })
                    .await;
            }
        });
    }

    pub fn request(
        &mut self,
        request: WorkspaceRequest,
        action: Option<ControlAction>,
        expected_text: Option<String>,
    ) {
        let revision = self
            .revision
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel)
            + 1;
        let ticket = self.cancellation.load(std::sync::atomic::Ordering::Acquire);
        {
            let mut queued = self.queued.lock().expect("workspace queue");
            queued.pending = Some(WorkspaceJob {
                request,
                action,
                expected_text,
                revision,
                ticket,
                activation_revision: self.scope.scene_activation_revision(),
                tx: self.tx.scoped(&self.scope),
            });
            if queued.running {
                return;
            }
            queued.running = true;
        }
        let client = self.slot.lock().ok().and_then(|slot| slot.clone());
        let queued = self.queued.clone();
        let current_revision = self.revision.clone();
        let cancellation = self.cancellation.clone();
        let scope = self.scope.clone();
        self.task = self.scope.spawn_reserved(&self.runtime, async move {
            loop {
                let job = {
                    let mut queued = queued.lock().expect("workspace queue");
                    let job = queued.pending.take();
                    if job.is_none() {
                        queued.running = false;
                    }
                    job
                };
                let Some(WorkspaceJob {
                    request,
                    action,
                    expected_text,
                    revision,
                    ticket,
                    activation_revision,
                    tx,
                }) = job
                else {
                    break;
                };
                let evidence = action.as_ref().and_then(|_| {
                    tx.operation(
                        super::lifecycle::OperationKind::Workspace {
                            epoch: request.epoch,
                            sequence: request.sequence,
                            result_id: request.result_id,
                        },
                        request.scope.endpoint.clone(),
                    )
                });
                let mut acknowledged = false;
                let result = if let Some(client) = &client {
                    let operation = async {
                        let mutation = async {
                            validate_scope(client, &request.scope).await?;
                            if cancellation.load(std::sync::atomic::Ordering::Acquire) != ticket
                                || current_revision.load(std::sync::atomic::Ordering::Acquire)
                                    != revision
                            {
                                return Err("Workspace request superseded before mutation".into());
                            }
                            if let Some(action) = &action {
                                let evidence = evidence.as_ref().ok_or_else(|| {
                                    "Workspace operation admission full or retired".to_owned()
                                })?;
                                if let ControlAction::SetText { input, .. } = action {
                                    let detail = client
                                        .get_source_detail(input)
                                        .await
                                        .map_err(|error| error.user_message())?;
                                    if detail.text != expected_text {
                                        return Err(
                                            "Text changed in OBS. Refresh before saving".into()
                                        );
                                    }
                                }
                                if cancellation.load(std::sync::atomic::Ordering::Acquire) != ticket
                                    || !tx.accepts_current_scope()
                                {
                                    return Err("Workspace action cancelled before mutation".into());
                                }
                                client
                                    .execute_control_if(action, &|| {
                                        let valid = cancellation
                                            .load(std::sync::atomic::Ordering::Acquire)
                                            == ticket
                                            && current_revision
                                                .load(std::sync::atomic::Ordering::Acquire)
                                                == revision
                                            && tx.accepts_current_scope()
                                            && (!matches!(
                                                action,
                                                ControlAction::ActivateScene { .. }
                                            ) || scope.scene_activation_revision()
                                                == activation_revision);
                                        if valid {
                                            evidence.dispatch();
                                        }
                                        valid
                                    })
                                    .await
                                    .map_err(|error| error.user_message())?;
                            }
                            Ok(())
                        };
                        bounded_workspace_read(client, mutation).await?;
                        if action.is_some() {
                            acknowledged = true;
                            if let Some(evidence) = &evidence {
                                evidence.acknowledge();
                            }
                            let _ = tx
                                .send(AppEvent::WorkspaceAcknowledged {
                                    epoch: request.epoch,
                                    sequence: request.sequence,
                                    result_id: request.result_id,
                                })
                                .await;
                        }
                        bounded_workspace_read(client, read_observation(client, &request)).await
                    };
                    operation.await
                } else {
                    Err("OBS is disconnected".into())
                };
                if let Some(evidence) = &evidence {
                    if !acknowledged
                        && client
                            .as_ref()
                            .is_some_and(|client| client.retirement_reason().is_some())
                    {
                        evidence.unknown(result.as_ref().err().cloned().unwrap_or_default());
                    } else {
                        evidence.finish(result.as_ref().map(|_| ()).map_err(Clone::clone));
                    }
                }
                if cancellation.load(std::sync::atomic::Ordering::Acquire) == ticket
                    && current_revision.load(std::sync::atomic::Ordering::Acquire) == revision
                {
                    let _ = tx
                        .send(AppEvent::WorkspaceLoaded {
                            epoch: request.epoch,
                            sequence: request.sequence,
                            result_id: request.result_id,
                            acknowledged,
                            uncertain: !acknowledged
                                && action.is_some()
                                && client.as_ref().is_some_and(|client| {
                                    client.wait_retired().now_or_never().is_some()
                                }),
                            result: result.map(Box::new),
                            action: action.is_some(),
                        })
                        .await;
                }
            }
        });
    }
}

async fn validate_scope(client: &ObsClient, scope: &ScopeBinding) -> Result<(), String> {
    if client.endpoint() != scope.endpoint {
        return Err("The OBS endpoint changed; reconnect and refresh".into());
    }
    let collection = client
        .get_scene_collections()
        .await
        .map_err(|error| error.user_message())?
        .current;
    if collection.as_deref() != Some(scope.collection.as_str()) {
        return Err("The OBS collection changed; refresh before continuing".into());
    }
    Ok(())
}

async fn bounded_workspace_read<T>(
    client: &ObsClient,
    operation: impl std::future::Future<Output = Result<T, String>>,
) -> Result<T, String> {
    match tokio::time::timeout(
        crate::controller::session_controller::REQUEST_BUDGET,
        operation,
    )
    .await
    {
        Ok(result) => result,
        Err(_) => {
            client.retire_after_deadline();
            Err("OBS deadline exceeded; outcome/readback is unknown. Reconnect and refresh; the action was not retried".into())
        }
    }
}

async fn read_observation(
    client: &ObsClient,
    request: &WorkspaceRequest,
) -> Result<WorkspaceObservation, String> {
    let observed_at = Instant::now();
    let mut snapshot = client
        .get_control_snapshot(request.scene.as_deref(), request.include_sources)
        .await
        .map_err(|e| e.to_string())?;
    let mut warnings = Vec::new();
    let input = request
        .include_sources
        .then_some(request.input.as_deref())
        .flatten();
    let detail = if let Some(input) = input {
        match client.get_source_detail(input).await {
            Ok(value) => Some(value),
            Err(e) => {
                warnings.push(WorkspaceWarning::ReadFailed {
                    detail: e.to_string(),
                });
                None
            }
        }
    } else {
        None
    };
    let usage = if let Some(input) = input {
        match client.get_source_usage(input).await {
            Ok((items, partial)) => {
                warnings.extend(partial);
                items
            }
            Err(e) => {
                warnings.push(WorkspaceWarning::ReadFailed {
                    detail: e.to_string(),
                });
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };
    let audio_read = client.get_all_audio_inputs().await;
    let audio = match audio_read {
        Ok(value) => value,
        Err(e) => {
            warnings.push(WorkspaceWarning::ReadFailed {
                detail: e.to_string(),
            });
            Vec::new()
        }
    };
    let screenshot = if request.screenshot {
        let target = snapshot
            .scene
            .as_deref()
            .or(snapshot.studio.program.as_deref());
        if let Some(target) = target {
            match client.get_source_screenshot(target).await {
                Ok(image) => Some(image),
                Err(e) => {
                    warnings.push(WorkspaceWarning::ReadFailed {
                        detail: e.to_string(),
                    });
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };
    let stream_status = client.get_stream_status().await.ok();
    let record_status = client.get_record_status().await.ok();
    let collection = client
        .get_scene_collections()
        .await
        .map_err(|e| e.to_string())?
        .current;
    if collection.as_deref() != Some(request.scope.collection.as_str()) {
        return Err("The OBS collection changed during the read; results discarded".into());
    }
    warnings.append(&mut snapshot.warnings);
    Ok(WorkspaceObservation {
        snapshot,
        detail,
        usage,
        audio,
        stream_status,
        record_status,
        screenshot,
        warnings,
        observed_at,
    })
}

#[cfg(test)]
#[path = "workspace_lifecycle_tests.rs"]
mod lifecycle_tests;

pub fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use serde_json::{json, Value};
    use std::sync::mpsc;
    use std::time::Duration;
    use tokio::{net::TcpListener, sync::oneshot, time::timeout};
    use tokio_websockets::{Message, ServerBuilder};

    struct MockObs {
        client: ObsClient,
        server: JoinHandle<Vec<Value>>,
        collection_started: oneshot::Receiver<()>,
        release_collection: oneshot::Sender<()>,
        finish: oneshot::Sender<()>,
    }

    async fn mock_obs(collection: &str, expect_audio_write: bool) -> MockObs {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        let collection = collection.to_string();
        let (started_tx, collection_started) = oneshot::channel();
        let (release_collection, release_rx) = oneshot::channel();
        let (finish, finish_rx) = oneshot::channel();
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
            let version = json!({"obsStudioVersion":"31.0.0","obsWebSocketVersion":"5.5.0","rpcVersion":1,"availableRequests":[],"supportedImageFormats":["png"],"platform":"mock","platformDescription":"mock"});
            let mut script = vec![
                ("GetVersion", version.clone()),
                (
                    "GetSceneCollectionList",
                    json!({"currentSceneCollectionName":collection,"sceneCollections":[collection]}),
                ),
            ];
            if expect_audio_write {
                script.push(("SetInputMute", json!({})));
            }
            // A request sent by the test after the operation forms a protocol barrier.
            script.push(("GetVersion", version));
            let mut started_tx = Some(started_tx);
            let mut release_rx = Some(release_rx);
            let mut requests = Vec::new();
            for (expected, data) in script {
                let message = timeout(Duration::from_secs(2), socket.next())
                    .await
                    .unwrap()
                    .unwrap()
                    .unwrap();
                let request: Value = serde_json::from_str(message.as_text().unwrap()).unwrap();
                assert_eq!(
                    request["d"]["requestType"], expected,
                    "unexpected OBS mutation or read"
                );
                if expected == "GetSceneCollectionList" {
                    started_tx.take().unwrap().send(()).unwrap();
                    release_rx.take().unwrap().await.unwrap();
                }
                let mut response = json!({"op":7,"d":{"requestId":request["d"]["requestId"],"requestType":expected,"requestStatus":{"result":true,"code":100}}});
                if data != json!({}) {
                    response["d"]["responseData"] = data;
                }
                socket
                    .send(Message::text(response.to_string()))
                    .await
                    .unwrap();
                requests.push(request);
            }
            finish_rx.await.unwrap();
            requests
        });
        let (client, _events) = timeout(
            Duration::from_secs(2),
            ObsClient::connect("127.0.0.1", port, None),
        )
        .await
        .unwrap()
        .unwrap();
        MockObs {
            client,
            server,
            collection_started,
            release_collection,
            finish,
        }
    }

    fn controller(client: &ObsClient) -> (WorkspaceController, mpsc::Receiver<AppEvent>) {
        let (tx, rx) = mpsc::sync_channel(8);
        (
            WorkspaceController::new(
                Handle::current(),
                tx,
                Arc::new(Mutex::new(Some(client.clone()))),
            ),
            rx,
        )
    }

    fn scope(client: &ObsClient) -> ScopeBinding {
        ScopeBinding {
            endpoint: client.endpoint().into(),
            collection: "Expected collection".into(),
        }
    }

    #[tokio::test]
    async fn live_read_skips_retired_workflows_and_selected_source_inspection() {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();
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
            while let Some(message) = socket.next().await {
                let request: Value =
                    serde_json::from_str(message.unwrap().as_text().unwrap()).unwrap();
                let name = request["d"]["requestType"].as_str().unwrap();
                let data = match name {
                    "GetVersion" => Some(
                        json!({"obsStudioVersion":"31.0.0","obsWebSocketVersion":"5.5.0","rpcVersion":1,"availableRequests":[],"supportedImageFormats":["png"],"platform":"mock","platformDescription":"mock"}),
                    ),
                    "GetSceneList" => {
                        Some(json!({"currentProgramSceneName":"Program", "scenes":[]}))
                    }
                    "GetInputList" => Some(json!({"inputs":[]})),
                    "GetSceneCollectionList" => Some(
                        json!({"currentSceneCollectionName":"Expected collection","sceneCollections":["Expected collection"]}),
                    ),
                    _ => None,
                };
                let mut response = json!({"op":7,"d":{"requestId":request["d"]["requestId"],"requestType":name,"requestStatus":{"result":data.is_some(),"code":if data.is_some() {100} else {204}}}});
                if let Some(data) = data {
                    response["d"]["responseData"] = data;
                }
                socket
                    .send(Message::text(response.to_string()))
                    .await
                    .unwrap();
                requests.push(name.to_owned());
                if name == "GetSceneCollectionList" {
                    break;
                }
            }
            requests
        });
        let (client, _events) = ObsClient::connect("127.0.0.1", port, None).await.unwrap();
        let request = WorkspaceRequest {
            epoch: 1,
            sequence: 1,
            scope: scope(&client),
            scene: Some("Previously inspected scene".into()),
            input: Some("Previously inspected input".into()),
            include_sources: false,
            screenshot: false,
            result_id: None,
        };
        let observation = timeout(Duration::from_secs(3), read_observation(&client, &request))
            .await
            .unwrap()
            .unwrap();
        assert!(observation.detail.is_none());
        assert!(observation.usage.is_empty());
        assert!(observation.snapshot.items.is_empty());
        let requests = server.await.unwrap();
        assert_eq!(
            requests
                .iter()
                .filter(|name| name.as_str() == "GetInputList")
                .count(),
            1,
            "audio is read once without baseline duplication"
        );
        for retired in [
            "GetStats",
            "GetProfileList",
            "GetSceneItemList",
            "GetSourceFilterList",
            "GetInputSettings",
        ] {
            assert!(
                !requests.iter().any(|name| name == retired),
                "unexpected idle read: {retired}"
            );
        }
    }

    #[tokio::test]
    async fn workspace_mutation_rejects_a_changed_collection_before_writing() {
        let mock = mock_obs("Other collection", false).await;
        let (mut controller, events) = controller(&mock.client);
        controller.request(
            WorkspaceRequest {
                result_id: None,
                epoch: 4,
                sequence: 9,
                scope: scope(&mock.client),
                scene: None,
                input: None,
                screenshot: false,
                include_sources: false,
            },
            Some(ControlAction::SetItemEnabled {
                owner: "Scene".into(),
                item_id: 7,
                enabled: false,
            }),
            None,
        );
        mock.collection_started.await.unwrap();
        mock.release_collection.send(()).unwrap();
        timeout(Duration::from_secs(2), controller.task.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        match events.try_recv().unwrap() {
            AppEvent::WorkspaceLoaded {
                epoch,
                sequence,
                result,
                action,
                ..
            } => {
                assert_eq!((epoch, sequence, action), (4, 9, true));
                assert!(result.unwrap_err().contains("collection changed"));
            }
            event => panic!("unexpected workspace result: {event:?}"),
        }
        timeout(Duration::from_secs(2), mock.client.get_version())
            .await
            .unwrap()
            .unwrap();
        mock.finish.send(()).unwrap();
        let requests = mock.server.await.unwrap();
        assert_eq!(requests.len(), 3);
    }

    #[tokio::test]
    async fn cancelling_audio_during_scope_read_prevents_delayed_mutation() {
        let mock = mock_obs("Expected collection", false).await;
        let (mut controller, _) = controller(&mock.client);
        controller.audio(
            scope(&mock.client),
            ControlAction::SetInputMuted {
                input: "Mic".into(),
                muted: true,
            },
        );
        mock.collection_started.await.unwrap();
        controller.cancel();
        assert!(controller.audio_tasks.is_empty());
        mock.release_collection.send(()).unwrap();
        timeout(Duration::from_secs(2), mock.client.get_version())
            .await
            .unwrap()
            .unwrap();
        mock.finish.send(()).unwrap();
        let requests = mock.server.await.unwrap();
        assert!(requests
            .iter()
            .all(|request| request["d"]["requestType"] != "SetInputMute"));
    }

    #[tokio::test]
    async fn audio_mutation_works_without_a_workspace_observation() {
        let mock = mock_obs("Expected collection", true).await;
        let (mut controller, events) = controller(&mock.client);
        assert!(controller.task.is_none());
        controller.audio(
            scope(&mock.client),
            ControlAction::SetInputMuted {
                input: "Mic".into(),
                muted: true,
            },
        );
        mock.collection_started.await.unwrap();
        mock.release_collection.send(()).unwrap();
        timeout(
            Duration::from_secs(2),
            controller.audio_tasks.pop().unwrap(),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(
            events.try_recv().is_err(),
            "audio operation unexpectedly failed"
        );
        timeout(Duration::from_secs(2), mock.client.get_version())
            .await
            .unwrap()
            .unwrap();
        mock.finish.send(()).unwrap();
        let requests = mock.server.await.unwrap();
        assert_eq!(
            requests[2]["d"]["requestData"],
            json!({"inputName":"Mic","inputMuted":true})
        );
    }

    #[tokio::test]
    async fn a_workspace_read_does_not_cancel_in_flight_audio_work() {
        let (tx, events) = mpsc::sync_channel(8);
        let mut controller =
            WorkspaceController::new(Handle::current(), tx, Arc::new(Mutex::new(None)));
        let (release, blocked) = oneshot::channel();
        controller.audio_tasks.push(tokio::spawn(async move {
            blocked.await.unwrap();
        }));
        controller.request(
            WorkspaceRequest {
                result_id: None,
                epoch: 1,
                sequence: 1,
                scope: ScopeBinding::default(),
                scene: None,
                input: None,
                screenshot: false,
                include_sources: false,
            },
            None,
            None,
        );
        timeout(Duration::from_secs(2), controller.task.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(
            events.try_recv().unwrap(),
            AppEvent::WorkspaceLoaded { action: false, .. }
        ));
        assert_eq!(controller.audio_tasks.len(), 1);
        release.send(()).unwrap();
        timeout(
            Duration::from_secs(2),
            controller.audio_tasks.pop().unwrap(),
        )
        .await
        .unwrap()
        .unwrap();
    }
}
