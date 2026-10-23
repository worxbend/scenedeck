//! Deterministic loopback regressions for the actual adapter/controllers/state.
use super::*;
use crate::controller::lifecycle::{OperationKind, SessionRetirement};
use crate::controller::session_controller::SessionRunner;
use crate::controller::state::{
    ActiveAudioStatus, AppState, MixerVisibleAudioStatus, MixerVisibleRenderSource, ObsStatus,
};
use crate::controller::workspace::{WorkspaceController, WorkspaceRequest};
use crate::domain::control::ControlAction;
use crate::domain::workflow::ScopeBinding;
use crate::obs::event::ObsEventStream;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;
use tokio::sync::{mpsc as async_mpsc, oneshot};
use tokio_websockets::{Message, ServerBuilder};

#[derive(Default)]
struct Plan {
    hold: Option<(&'static str, usize)>,
    second_hold: Option<(&'static str, usize)>,
    saturated_versions: bool,
    fail_b: Option<&'static str>,
    empty_b: bool,
    mode_unavailable: bool,
    active_outputs: bool,
    fail_at: Option<(&'static str, usize)>,
}

enum Signal {
    Release,
    ReleaseKind(&'static str),
    Event(&'static str, u64, Value),
}
struct Mock {
    client: ObsClient,
    events: Option<ObsEventStream>,
    signals: async_mpsc::UnboundedSender<(Signal, oneshot::Sender<()>)>,
    entered: async_mpsc::UnboundedReceiver<(String, usize)>,
    finish: oneshot::Sender<()>,
    server: tokio::task::JoinHandle<Vec<String>>,
}

fn version() -> Value {
    json!({"obsStudioVersion":"31.0.0","obsWebSocketVersion":"5.5.0","rpcVersion":1,"availableRequests":[],"supportedImageFormats":[],"platform":"mock","platformDescription":"mock"})
}
async fn mock(plan: Plan) -> Mock {
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    let (signals, mut commands) = async_mpsc::unbounded_channel::<(Signal, oneshot::Sender<()>)>();
    let (entered_tx, entered) = async_mpsc::unbounded_channel();
    let (finish, mut finished) = oneshot::channel();
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
        let mut counts = HashMap::<String, usize>::new();
        let mut held = Vec::new();
        let mut names = Vec::new();
        let mut scene = "A".to_owned();
        let mut muted = false;
        let mut volume = 0.5;
        let mut studio = false;
        let mut stream_active = plan.active_outputs;
        let mut record_active = plan.active_outputs;
        loop {
            let message = tokio::select! {
                _ = &mut finished => break,
                command = commands.recv() => {
                    let Some((signal, acknowledged)) = command else { break };
                    match signal {
                        Signal::Release => {
                            for (_, response) in held.drain(..) { socket.send(Message::text(response)).await.unwrap(); }
                        }
                        Signal::ReleaseKind(kind) => {
                            let mut remaining = Vec::new();
                            for (name, response) in held.drain(..) {
                                if name == kind { socket.send(Message::text(response)).await.unwrap(); }
                                else { remaining.push((name, response)); }
                            }
                            held = remaining;
                        }
                        Signal::Event(name, intent, data) => {
                            if name == "CurrentProgramSceneChanged" { scene = data["sceneName"].as_str().unwrap().to_owned(); }
                            if name == "StudioModeStateChanged" { studio = data["studioModeEnabled"].as_bool().unwrap(); }
                            if name == "InputMuteStateChanged" { muted = data["inputMuted"].as_bool().unwrap(); }
                            if name == "InputVolumeChanged" { volume = data["inputVolumeMul"].as_f64().unwrap(); }
                            socket.send(Message::text(json!({"op":5,"d":{"eventType":name,"eventIntent":intent,"eventData":data}}).to_string())).await.unwrap();
                        }
                    }
                    let _ = acknowledged.send(());
                    continue;
                }
                message = socket.next() => match message {
                    Some(Ok(message)) if message.is_text() => message,
                    _ => break,
                },
            };
            let request: Value = serde_json::from_str(message.as_text().unwrap()).unwrap();
            let name = request["d"]["requestType"].as_str().unwrap();
            let count = counts.entry(name.to_owned()).or_default();
            *count += 1;
            let count = *count;
            names.push(name.to_owned());
            let mut success = plan.fail_at != Some((name, count))
                && !(scene == "B" && plan.fail_b == Some(name))
                && !(name == "GetStudioModeEnabled" && plan.mode_unavailable);
            let data = match name {
                "GetVersion" => Some(version()),
                "GetStudioModeEnabled" => Some(json!({"studioModeEnabled":studio})),
                "SetCurrentProgramScene" | "SetCurrentPreviewScene" => None,
                "StopStream" => {
                    stream_active = false;
                    None
                }
                "StopRecord" => {
                    record_active = false;
                    Some(json!({"outputPath":"fixture.mkv"}))
                }
                "GetProfileList" => Some(json!({"currentProfileName":"Main","profiles":["Main"]})),
                "GetSceneCollectionList" => {
                    Some(json!({"currentSceneCollectionName":"Show","sceneCollections":["Show"]}))
                }
                "GetStreamStatus" => Some(
                    json!({"outputActive":stream_active,"outputReconnecting":false,"outputTimecode":"00:00:00.000","outputDuration":0,"outputCongestion":0.0,"outputBytes":0,"outputSkippedFrames":0,"outputTotalFrames":0}),
                ),
                "GetRecordStatus" => Some(
                    json!({"outputActive":record_active,"outputPaused":false,"outputTimecode":"00:00:00.000","outputDuration":0,"outputBytes":0}),
                ),
                "GetSceneList" => Some(
                    json!({"currentProgramSceneName":scene,"currentProgramSceneUuid":"00000000-0000-0000-0000-000000000001","scenes":[{"sceneName":"A","sceneUuid":"00000000-0000-0000-0000-000000000001","sceneIndex":0},{"sceneName":"B","sceneUuid":"00000000-0000-0000-0000-000000000002","sceneIndex":1}]}),
                ),
                "GetSpecialInputs" => Some(
                    json!({"desktop1":null,"desktop2":null,"mic1":null,"mic2":null,"mic3":null,"mic4":null}),
                ),
                "GetSceneItemList" => {
                    let owner = request["d"]["requestData"]["sceneName"].as_str().unwrap();
                    if owner == "B" && plan.empty_b {
                        Some(json!({"sceneItems":[]}))
                    } else {
                        Some(
                            json!({"sceneItems":[{"sceneItemId":1,"sceneItemIndex":0,"sourceName":format!("Mic{owner}"),"sourceType":"OBS_SOURCE_TYPE_INPUT","inputKind":"mock"}]}),
                        )
                    }
                }
                "GetSceneItemEnabled" => Some(json!({"sceneItemEnabled":true})),
                "GetInputMute" => Some(json!({"inputMuted":muted})),
                "GetInputVolume" => Some(json!({"inputVolumeMul":volume,"inputVolumeDb":-6.0})),
                "GetInputList" => Some(json!({"inputs":[]})),
                "GetInputSettings" => Some(
                    json!({"inputKind":"text_ft2_source_v2","inputSettings":{"text":"before","from_file":false}}),
                ),
                "GetInputDefaultSettings" => {
                    Some(json!({"defaultInputSettings":{"text":"","from_file":false}}))
                }
                "GetSourceFilterList" => Some(json!({"filters":[]})),
                "GetMediaInputStatus" => Some(
                    json!({"mediaState":"OBS_MEDIA_STATE_PLAYING","mediaDuration":10000,"mediaCursor":0}),
                ),
                "SetInputSettings"
                | "SetMediaInputCursor"
                | "SetInputMute"
                | "SetSceneItemEnabled"
                | "StartStream" => None,
                _ => {
                    success = false;
                    None
                }
            };
            let mut response = json!({"op":7,"d":{"requestId":request["d"]["requestId"],"requestType":name,"requestStatus":{"result":success,"code":if success {100} else {702}}}});
            if let Some(data) = data {
                response["d"]["responseData"] = data;
            }
            if plan.hold == Some((name, count))
                || plan.second_hold == Some((name, count))
                || (plan.saturated_versions && name == "GetVersion" && count > 1)
            {
                held.push((name.to_owned(), response.to_string()));
            } else {
                socket
                    .send(Message::text(response.to_string()))
                    .await
                    .unwrap();
            }
            let _ = entered_tx.send((name.to_owned(), count));
        }
        names
    });
    let (client, events) = ObsClient::connect("127.0.0.1", port, None).await.unwrap();
    Mock {
        client,
        events: Some(events),
        signals,
        entered,
        finish,
        server,
    }
}
impl Mock {
    async fn signal(&self, signal: Signal) {
        let (done, acknowledged) = oneshot::channel();
        self.signals.send((signal, done)).unwrap();
        tokio::time::timeout(Duration::from_secs(2), acknowledged)
            .await
            .unwrap()
            .unwrap();
    }
    async fn wait(&mut self, name: &str, count: usize) {
        tokio::time::timeout(Duration::from_secs(3), async {
            while let Some(observed) = self.entered.recv().await {
                if observed == (name.into(), count) {
                    return;
                }
            }
            panic!("mock closed before {name} #{count}");
        })
        .await
        .unwrap();
    }
    async fn stop(self) -> Vec<String> {
        self.finish.send(()).unwrap();
        self.server.await.unwrap()
    }
}
async fn receive(rx: &mpsc::Receiver<AppEvent>) -> AppEvent {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Ok(event) = rx.try_recv() {
                return event;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap()
}
fn peel(event: AppEvent) -> AppEvent {
    match event {
        AppEvent::SessionEvent { event, .. } => peel(*event),
        event => event,
    }
}
fn state() -> AppState {
    AppState::new(Default::default(), Default::default(), None, None)
}
fn binding(client: &ObsClient) -> ScopeBinding {
    ScopeBinding {
        endpoint: client.endpoint().into(),
        collection: "Show".into(),
    }
}
fn request(client: &ObsClient) -> WorkspaceRequest {
    WorkspaceRequest {
        epoch: 1,
        sequence: 1,
        scope: binding(client),
        scene: None,
        input: Some("Text".into()),
        include_sources: false,
        screenshot: false,
        result_id: None,
    }
}

#[tokio::test]
async fn cancelled_or_collection_invalidated_text_preflight_never_begins_final_write() {
    for cancel_ui in [true, false] {
        let mut mock = mock(Plan {
            hold: Some(("GetInputDefaultSettings", 2)),
            ..Default::default()
        })
        .await;
        let slot = Arc::new(Mutex::new(None));
        let scope = SessionScope::initial(slot.clone()).next();
        assert!(scope.install(mock.client.clone()));
        let (tx, _rx) = mpsc::sync_channel(32);
        let mut workspace = WorkspaceController::new(Handle::current(), tx, slot);
        workspace.set_scope(scope.clone());
        workspace.request(
            request(&mock.client),
            Some(ControlAction::SetText {
                input: "Text".into(),
                text: "after".into(),
            }),
            Some("before".into()),
        );
        mock.wait("GetInputDefaultSettings", 2).await;
        if cancel_ui {
            workspace.cancel();
        } else {
            scope.advance_collection();
        }
        mock.signal(Signal::Release).await;
        tokio::time::timeout(Duration::from_secs(2), scope.quiesce())
            .await
            .unwrap();
        mock.client.get_version().await.unwrap(); // Drained socket remains reusable.
        scope.retire();
        let frames = mock.stop().await;
        assert_eq!(
            frames
                .iter()
                .filter(|name| *name == "SetInputSettings")
                .count(),
            0
        );
    }
}

#[tokio::test]
async fn cancelled_or_collection_invalidated_media_preflight_never_begins_cursor_write() {
    for cancel_ui in [true, false] {
        let mut mock = mock(Plan {
            hold: Some(("GetMediaInputStatus", 1)),
            ..Default::default()
        })
        .await;
        let slot = Arc::new(Mutex::new(None));
        let scope = SessionScope::initial(slot.clone()).next();
        scope.install(mock.client.clone());
        let (tx, _rx) = mpsc::sync_channel(32);
        let mut workspace = WorkspaceController::new(Handle::current(), tx, slot);
        workspace.set_scope(scope.clone());
        workspace.audio(
            binding(&mock.client),
            ControlAction::SeekMedia {
                input: "Media".into(),
                position_ms: 500,
            },
        );
        mock.wait("GetMediaInputStatus", 1).await;
        if cancel_ui {
            workspace.cancel();
        } else {
            scope.advance_collection();
        }
        mock.signal(Signal::Release).await;
        tokio::time::timeout(Duration::from_secs(2), scope.quiesce())
            .await
            .unwrap();
        mock.client.get_version().await.unwrap();
        scope.retire();
        assert!(!mock
            .stop()
            .await
            .iter()
            .any(|name| name == "SetMediaInputCursor"));
    }
}

#[tokio::test]
async fn cancellation_while_waiting_for_connection_admission_does_not_poll_a_mutation() {
    let mut mock = mock(Plan {
        saturated_versions: true,
        ..Default::default()
    })
    .await;
    let mut reads = Vec::new();
    for _ in 0..8 {
        let client = mock.client.clone();
        reads.push(tokio::spawn(async move { client.get_version().await }));
    }
    mock.wait("GetVersion", 9).await;
    let valid = Arc::new(AtomicBool::new(true));
    let lease = valid.clone();
    let client = mock.client.clone();
    let (polled, queued) = oneshot::channel();
    let mutation = tokio::spawn(async move {
        let action = ControlAction::SetInputMuted {
            input: "MicA".into(),
            muted: true,
        };
        let check = || lease.load(Ordering::Acquire);
        let write = client.execute_control_if(&action, &check);
        tokio::pin!(write);
        let mut polled = Some(polled);
        std::future::poll_fn(|context| {
            if let Some(polled) = polled.take() {
                let _ = polled.send(());
            }
            write.as_mut().poll(context)
        })
        .await
    });
    queued.await.unwrap();
    valid.store(false, Ordering::Release);
    mock.signal(Signal::Release).await;
    for read in reads {
        read.await.unwrap().unwrap();
    }
    assert!(mutation.await.unwrap().is_err());
    assert!(
        mock.client.retirement_reason().is_none(),
        "a skipped, unpolled request does not poison admission"
    );
    let frames = mock.stop().await;
    assert_eq!(
        frames.iter().filter(|name| *name == "GetVersion").count(),
        9
    );
    assert!(!frames.iter().any(|name| name == "SetInputMute"));
}

#[tokio::test]
async fn queued_primary_scene_switch_cannot_cross_a_collection_boundary() {
    let mut mock = mock(Plan {
        saturated_versions: true,
        ..Default::default()
    })
    .await;
    let (tx, _rx) = mpsc::sync_channel(16);
    let mut controller = AppController::with_dependencies(Handle::current(), tx, dependencies());
    let scope = controller.session.scope();
    assert!(scope.install(mock.client.clone()));
    let mut reads = Vec::new();
    for _ in 0..8 {
        let client = mock.client.clone();
        reads.push(tokio::spawn(async move { client.get_version().await }));
    }
    mock.wait("GetVersion", 9).await;
    controller.handle(AppCommand::SwitchPrimaryScene("A".into()));
    scope.advance_collection();
    mock.signal(Signal::Release).await;
    for read in reads {
        read.await.unwrap().unwrap();
    }
    tokio::time::timeout(Duration::from_secs(3), scope.quiesce())
        .await
        .unwrap();
    assert!(mock.client.retirement_reason().is_none());
    controller.shutdown().await.unwrap();
    let frames = mock.stop().await;
    assert!(
        !frames.iter().any(|name| name == "SetCurrentProgramScene"),
        "a queued scene switch must not target a replacement collection"
    );
}

fn pump(mock: &mut Mock, tx: mpsc::SyncSender<AppEvent>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(crate::controller::refresh_controller::run_event_loop(
        mock.client.clone(),
        mock.events.take().unwrap(),
        EventSender::new(tx),
        vec![],
        Arc::new(Mutex::new(None)),
    ))
}
async fn graph_ready(rx: &mpsc::Receiver<AppEvent>) {
    loop {
        let event = peel(receive(rx).await);
        let event = match event {
            AppEvent::Observation { event, .. } => *event,
            event => event,
        };
        if matches!(event, AppEvent::GraphUpdated(_)) {
            return;
        }
    }
}

#[tokio::test]
async fn queued_scene_activation_rejects_authoritative_mode_change_during_collection_read() {
    let mut mock = mock(Plan {
        hold: Some(("GetSceneCollectionList", 1)),
        ..Default::default()
    })
    .await;
    let slot = Arc::new(Mutex::new(None));
    let scope = SessionScope::initial(slot.clone()).next();
    scope.install(mock.client.clone());
    let (tx, _rx) = mpsc::sync_channel(32);
    let mut workspace = WorkspaceController::new(Handle::current(), tx, slot);
    workspace.set_scope(scope.clone());
    let mut cached = crate::controller::state::WorkspaceState::default();
    let mut snapshot = crate::domain::control::ControlSnapshot::default();
    snapshot.studio.enabled = Some(false);
    cached.snapshot = Some(snapshot);
    cached.observed_at = Some(std::time::Instant::now());
    let action = cached.scene_action("B").unwrap();
    workspace.request(request(&mock.client), Some(action), None);
    mock.wait("GetSceneCollectionList", 1).await;
    mock.signal(Signal::Event(
        "StudioModeStateChanged",
        1,
        json!({"studioModeEnabled":true}),
    ))
    .await;
    mock.signal(Signal::Release).await;
    tokio::time::timeout(Duration::from_secs(3), scope.quiesce())
        .await
        .unwrap();
    mock.client.get_version().await.unwrap();
    scope.retire();
    let frames = mock.stop().await;
    assert!(
        !frames
            .iter()
            .any(|name| name == "SetCurrentProgramScene" || name == "SetCurrentPreviewScene"),
        "an obsolete click must be rejected, not rerouted or replayed in the new mode"
    );
}

#[tokio::test]
async fn scene_activation_mode_echo_during_delayed_mode_reply_fences_old_click() {
    mode_echo_fences_activation(false, false).await;
}

#[tokio::test]
async fn scene_activation_mode_echo_while_final_mutation_waits_for_gate_fences_old_click() {
    mode_echo_fences_activation(true, false).await;
}

#[tokio::test]
async fn scene_activation_mode_aba_while_waiting_for_gate_does_not_revive_old_click() {
    mode_echo_fences_activation(true, true).await;
}

async fn mode_echo_fences_activation(at_final_gate: bool, aba: bool) {
    let mut mock = mock(Plan {
        hold: Some(("GetStudioModeEnabled", 1)),
        saturated_versions: at_final_gate,
        ..Default::default()
    })
    .await;
    let slot = Arc::new(Mutex::new(None));
    let scope = SessionScope::initial(slot.clone()).next();
    scope.install(mock.client.clone());
    let (tx, rx) = mpsc::sync_channel(128);
    let mut workspace = WorkspaceController::new(Handle::current(), tx.clone(), slot);
    workspace.set_scope(scope.clone());
    workspace.request(
        request(&mock.client),
        Some(ControlAction::ActivateScene {
            scene: "B".into(),
            studio_enabled: false,
        }),
        None,
    );
    mock.wait("GetStudioModeEnabled", 1).await;
    let mut reads = Vec::new();
    if at_final_gate {
        // The held mode read occupies one permit; seven reads fill the rest.
        // An eighth read queues ahead of the final mutation (FIFO admission).
        for _ in 0..8 {
            let client = mock.client.clone();
            reads.push(tokio::spawn(async move { client.get_version().await }));
        }
        mock.wait("GetVersion", 8).await;
        mock.signal(Signal::ReleaseKind("GetStudioModeEnabled"))
            .await;
        mock.wait("GetVersion", 9).await;
    }
    // Bootstrap/stats requests may queue, but event pumping remains independent.
    let task = tokio::spawn(crate::controller::refresh_controller::run_event_loop(
        mock.client.clone(),
        mock.events.take().unwrap(),
        EventSender::new(tx).scoped(&scope),
        vec![],
        Arc::new(Mutex::new(None)),
    ));
    mock.signal(Signal::Event(
        "StudioModeStateChanged",
        1,
        json!({"studioModeEnabled":true}),
    ))
    .await;
    loop {
        if matches!(peel(receive(&rx).await), AppEvent::WorkspaceInvalidated) {
            break;
        }
    }
    if aba {
        mock.signal(Signal::Event(
            "StudioModeStateChanged",
            1,
            json!({"studioModeEnabled":false}),
        ))
        .await;
        loop {
            if matches!(peel(receive(&rx).await), AppEvent::WorkspaceInvalidated) {
                break;
            }
        }
    }
    mock.signal(Signal::Release).await;
    for read in reads {
        read.await.unwrap().unwrap();
    }
    tokio::time::timeout(Duration::from_secs(3), scope.quiesce())
        .await
        .unwrap();
    assert!(
        mock.client.retirement_reason().is_none(),
        "a rejected unpolled mutation must not retire the socket"
    );
    mock.client.retire();
    task.await.unwrap();
    scope.retire();
    let frames = mock.stop().await;
    assert!(
        !frames
            .iter()
            .any(|name| name == "SetCurrentProgramScene" || name == "SetCurrentPreviewScene"),
        "mode invalidation must fence the original click at final gate admission, without replay"
    );
}

#[tokio::test]
async fn full_gtk_queue_does_not_hide_mode_echo_during_delayed_mode_reply() {
    mode_echo_with_full_gtk_queue(false).await;
}

#[tokio::test]
async fn full_gtk_queue_does_not_hide_mode_echo_at_final_mutation_admission() {
    mode_echo_with_full_gtk_queue(true).await;
}

async fn mode_echo_with_full_gtk_queue(at_final_gate: bool) {
    let mut mock = mock(Plan {
        hold: Some(("GetStudioModeEnabled", 1)),
        saturated_versions: at_final_gate,
        ..Default::default()
    })
    .await;
    let slot = Arc::new(Mutex::new(None));
    let scope = SessionScope::initial(slot.clone()).next();
    scope.install(mock.client.clone());
    let (tx, rx) = mpsc::sync_channel(1);
    tx.send(AppEvent::Connecting).unwrap();
    let mut workspace = WorkspaceController::new(Handle::current(), tx.clone(), slot);
    workspace.set_scope(scope.clone());
    let task = tokio::spawn(crate::controller::refresh_controller::run_event_loop(
        mock.client.clone(),
        mock.events.take().unwrap(),
        EventSender::new(tx).scoped(&scope),
        vec![],
        Arc::new(Mutex::new(None)),
    ));
    // First event reaches ingress, but its publication cannot enter the full
    // GTK queue. Accept the click only AFTER this event's revision advances.
    mock.signal(Signal::Event(
        "StudioModeStateChanged",
        1,
        json!({"studioModeEnabled":false}),
    ))
    .await;
    tokio::time::timeout(Duration::from_secs(2), async {
        while scope.scene_activation_revision() == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let accepted = scope.scene_activation_revision();
    workspace.request(
        request(&mock.client),
        Some(ControlAction::ActivateScene {
            scene: "B".into(),
            studio_enabled: false,
        }),
        None,
    );
    mock.wait("GetStudioModeEnabled", 1).await;
    let mut reads = Vec::new();
    if at_final_gate {
        // Bootstrap's held GetVersion already owns one of the eight slots.
        for _ in 0..7 {
            let client = mock.client.clone();
            reads.push(tokio::spawn(async move { client.get_version().await }));
        }
        mock.wait("GetVersion", 8).await;
        mock.signal(Signal::ReleaseKind("GetStudioModeEnabled"))
            .await;
        mock.wait("GetVersion", 9).await;
    }
    mock.signal(Signal::Event(
        "StudioModeStateChanged",
        1,
        json!({"studioModeEnabled":true}),
    ))
    .await;
    let ingested_before_gtk_drain = tokio::time::timeout(Duration::from_millis(250), async {
        while scope.scene_activation_revision() == accepted {
            tokio::task::yield_now().await;
        }
    })
    .await
    .is_ok();
    mock.signal(Signal::Release).await;
    for read in reads {
        read.await.unwrap().unwrap();
    }
    loop {
        if matches!(peel(receive(&rx).await), AppEvent::WorkspaceLoaded { .. }) {
            break;
        }
    }
    tokio::time::timeout(Duration::from_secs(3), scope.quiesce())
        .await
        .unwrap();
    mock.client.retire();
    task.await.unwrap();
    scope.retire();
    let frames = mock.stop().await;
    assert!(
        ingested_before_gtk_drain,
        "mode ingress was blocked by unrelated UI publication; {frames:?}"
    );
    assert!(
        !frames
            .iter()
            .any(|name| name == "SetCurrentProgramScene" || name == "SetCurrentPreviewScene"),
        "a full GTK queue must not let an obsolete scene click mutate OBS: {frames:?}"
    );
}

#[tokio::test]
async fn bootstrap_failure_fences_mutations_before_waiting_for_full_gtk_queue() {
    let mut mock = mock(Plan {
        fail_at: Some(("GetVersion", 2)),
        ..Default::default()
    })
    .await;
    let scope = SessionScope::initial(Arc::new(Mutex::new(None))).next();
    scope.install(mock.client.clone());
    let (tx, rx) = mpsc::sync_channel(1);
    tx.send(AppEvent::Connecting).unwrap();
    let task = tokio::spawn(crate::controller::refresh_controller::run_event_loop(
        mock.client.clone(),
        mock.events.take().unwrap(),
        EventSender::new(tx).scoped(&scope),
        vec![],
        Arc::new(Mutex::new(None)),
    ));
    mock.wait("GetVersion", 2).await;
    let fenced_without_ui =
        tokio::time::timeout(Duration::from_millis(250), mock.client.wait_retired())
            .await
            .is_ok();
    rx.try_recv().unwrap();
    while !matches!(peel(receive(&rx).await), AppEvent::Error(_)) {}
    task.await.unwrap();
    scope.retire();
    mock.stop().await;
    assert!(
        fenced_without_ui,
        "failed ingress owner left mutations admitted while its Error publication waited for GTK"
    );
}

#[tokio::test]
async fn transport_publication_overflow_retires_instead_of_stalling_mode_ingress() {
    let mut mock = mock(Plan::default()).await;
    let scope = SessionScope::initial(Arc::new(Mutex::new(None))).next();
    scope.install(mock.client.clone());
    let (tx, _rx) = mpsc::sync_channel(1);
    tx.send(AppEvent::Connecting).unwrap();
    let task = tokio::spawn(crate::controller::refresh_controller::run_event_loop(
        mock.client.clone(),
        mock.events.take().unwrap(),
        EventSender::new(tx).scoped(&scope),
        vec![],
        Arc::new(Mutex::new(None)),
    ));
    // One publication may be selected by the publisher and another 64 queued.
    // There is no GTK drain: exceeding that bound must fail the connection closed.
    for _ in 0..crate::controller::refresh_controller::TRANSPORT_PUBLICATION_CAPACITY + 2 {
        mock.signal(Signal::Event(
            "StudioModeStateChanged",
            1,
            json!({"studioModeEnabled":false}),
        ))
        .await;
    }
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
    assert!(mock.client.retirement_reason().is_some());
    assert!(
        scope.has_terminal_reason(),
        "notification loss must remain explicit terminal evidence"
    );
    assert!(mock
        .client
        .activate_scene_if("B", false, &|| true)
        .await
        .is_err());
    scope.retire();
    let frames = mock.stop().await;
    assert!(!frames
        .iter()
        .any(|name| name == "SetCurrentProgramScene" || name == "SetCurrentPreviewScene"));
}

#[tokio::test]
async fn scene_activation_preserves_current_mode_and_rejects_an_unavailable_mode() {
    for (studio_enabled, unavailable) in [(false, false), (true, false), (false, true)] {
        let mock = mock(Plan {
            mode_unavailable: unavailable,
            ..Default::default()
        })
        .await;
        mock.signal(Signal::Event(
            "StudioModeStateChanged",
            1,
            json!({"studioModeEnabled":studio_enabled}),
        ))
        .await;
        let result = mock
            .client
            .activate_scene_if("B", studio_enabled, &|| true)
            .await;
        assert_eq!(result.is_err(), unavailable);
        let frames = mock.stop().await;
        let writes: Vec<_> = frames
            .iter()
            .filter(|name| name.starts_with("SetCurrent"))
            .collect();
        if unavailable {
            assert!(writes.is_empty(), "unknown mode must fail closed");
        } else {
            assert_eq!(
                writes,
                vec![if studio_enabled {
                    "SetCurrentPreviewScene"
                } else {
                    "SetCurrentProgramScene"
                }]
            );
        }
    }
}

#[test]
fn graph_reducer_rejects_superseded_success_failure_and_invalidation() {
    use crate::controller::state::TopologyStatus;
    let mut state = state();
    state
        .reconcile_observation(10, AppEvent::GraphInvalidated)
        .unwrap();
    state.set_topology_loading();
    state
        .reconcile_observation(12, AppEvent::GraphInvalidated)
        .unwrap();
    assert!(state
        .reconcile_observation(10, AppEvent::GraphUpdated(Default::default()))
        .is_none());
    assert!(state
        .reconcile_observation(
            10,
            AppEvent::GraphFailed(AppError::Request("old failure".into()))
        )
        .is_none());
    assert!(state
        .reconcile_observation(11, AppEvent::GraphInvalidated)
        .is_none());
    assert_eq!(state.topology_status, TopologyStatus::Loading);
    let graph = match state
        .reconcile_observation(12, AppEvent::GraphUpdated(Default::default()))
        .unwrap()
    {
        AppEvent::GraphUpdated(graph) => graph,
        _ => unreachable!(),
    };
    state.set_topology_ready(graph);
    assert_eq!(
        state.topology_status,
        TopologyStatus::Ready,
        "valid empty topology remains Ready"
    );
}

#[tokio::test]
async fn manual_graph_scan_invalidated_midscan_cannot_publish_ready() {
    let mut mock = mock(Plan {
        hold: Some(("GetSceneItemList", 5)),
        second_hold: Some(("GetSceneList", 3)),
        ..Default::default()
    })
    .await;
    let scope = SessionScope::initial(Arc::new(Mutex::new(None))).next();
    scope.install(mock.client.clone());
    let (tx, rx) = mpsc::sync_channel(128);
    let sender = EventSender::new(tx).scoped(&scope);
    let task = tokio::spawn(crate::controller::refresh_controller::run_event_loop(
        mock.client.clone(),
        mock.events.take().unwrap(),
        sender.clone(),
        vec![],
        Arc::new(Mutex::new(None)),
    ));
    graph_ready(&rx).await;
    let client = mock.client.clone();
    let manual = tokio::spawn(async move {
        crate::controller::refresh_controller::refresh_live_data(&client, &sender.fresh(), &[])
            .await;
    });
    mock.wait("GetSceneItemList", 5).await;
    // Ignore earlier loading/audio publications; establish the actual structural
    // invalidation before releasing the old graph's held scene response.
    while rx.try_recv().is_ok() {}
    mock.signal(Signal::Event("SceneListChanged", 4, json!({"scenes":[]})))
        .await;
    loop {
        let event = peel(receive(&rx).await);
        if matches!(event, AppEvent::Observation { event, .. } if matches!(*event, AppEvent::GraphInvalidated))
        {
            break;
        }
    }
    mock.signal(Signal::ReleaseKind("GetSceneItemList")).await;
    tokio::time::timeout(Duration::from_secs(3), manual)
        .await
        .unwrap()
        .unwrap();
    mock.wait("GetSceneList", 3).await;
    while let Ok(event) = rx.try_recv() {
        let event = peel(event);
        assert!(
            !matches!(event, AppEvent::Observation { event, .. } if matches!(*event, AppEvent::GraphUpdated(_) | AppEvent::GraphFailed(_))),
            "a manual scan invalidated mid-read must not publish a graph completion"
        );
    }
    mock.signal(Signal::Release).await;
    graph_ready(&rx).await;
    mock.client.retire();
    task.await.unwrap();
    scope.retire();
    mock.stop().await;
}

#[tokio::test]
async fn manual_graph_completion_waiting_for_gtk_capacity_rechecks_ingress_revision() {
    graph_completion_with_full_gtk_queue(false).await;
}

#[tokio::test]
async fn manual_graph_failure_waiting_for_gtk_capacity_rechecks_ingress_revision() {
    graph_completion_with_full_gtk_queue(true).await;
}

async fn graph_completion_with_full_gtk_queue(fail: bool) {
    let mut mock = mock(Plan {
        fail_at: fail.then_some(("GetSceneItemList", 3)),
        ..Default::default()
    })
    .await;
    let scope = SessionScope::initial(Arc::new(Mutex::new(None))).next();
    scope.install(mock.client.clone());
    // Exactly the four invalidation/inventory/audio publications fit; the
    // completed graph must wait for final GTK admission.
    let (tx, rx) = mpsc::sync_channel(4);
    let sender = EventSender::new(tx).scoped(&scope);
    let client = mock.client.clone();
    let manual = crate::controller::refresh_controller::refresh_live_data(&client, &sender, &[]);
    tokio::pin!(manual);
    tokio::select! {
        () = &mut manual => panic!("graph completion must be blocked by the full queue"),
        () = mock.wait("GetSceneItemList", 3) => {}
    }
    // One later wire reply proves the graph responses passed through obws's
    // receiver. Poll the actual scan through graph_is_current and into send.
    mock.client.get_version().await.unwrap();
    assert!(futures_util::poll!(manual.as_mut()).is_pending());
    let scan_revision = scope.graph_revision();
    let task = tokio::spawn(crate::controller::refresh_controller::run_event_loop(
        mock.client.clone(),
        mock.events.take().unwrap(),
        sender.clone(),
        vec![],
        Arc::new(Mutex::new(None)),
    ));
    mock.signal(Signal::Event("SceneListChanged", 4, json!({"scenes":[]})))
        .await;
    tokio::time::timeout(Duration::from_secs(2), async {
        while scope.graph_revision() == scan_revision {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    // Release one slot and poll this independent worker FIRST. The obsolete
    // completion must not enter ahead of the queued session invalidation.
    // EventSender retries SyncSender capacity on a two-millisecond timer.
    // Let that timer mature while capacity is still unavailable, then poll
    // synchronously so a competing publisher cannot win the released slot.
    tokio::time::sleep(Duration::from_millis(5)).await;
    rx.try_recv().unwrap();
    assert!(futures_util::poll!(manual.as_mut()).is_ready());
    let admitted: Vec<_> = rx.try_iter().collect();
    assert!(!admitted.iter().cloned().map(peel).any(|event|
        matches!(event, AppEvent::Observation { event, .. } if matches!(*event, AppEvent::GraphUpdated(_) | AppEvent::GraphFailed(_)))
    ), "obsolete graph completion overtook ingress invalidation: {admitted:?}");
    mock.client.retire();
    task.await.unwrap();
    scope.retire();
    mock.stop().await;
}

#[tokio::test]
async fn graph_invalidated_midscan_never_publishes_ready_before_followup_completes() {
    let mut mock = mock(Plan {
        hold: Some(("GetSceneItemList", 2)),
        second_hold: Some(("GetSceneList", 2)),
        ..Default::default()
    })
    .await;
    let (tx, rx) = mpsc::sync_channel(128);
    let task = pump(&mut mock, tx);
    let mut state = state();
    mock.wait("GetSceneItemList", 2).await;
    while let Ok(event) = rx.try_recv() {
        if matches!(peel(event), AppEvent::GraphInvalidated) {
            state.set_topology_loading();
        }
    }
    mock.signal(Signal::Event("SceneListChanged", 4, json!({"scenes":[]})))
        .await;
    loop {
        let event = peel(receive(&rx).await);
        let event = match event {
            AppEvent::Observation { revision, event } => {
                state.reconcile_observation(revision, *event)
            }
            event => Some(event),
        };
        if matches!(event, Some(AppEvent::GraphInvalidated)) {
            state.set_topology_loading();
            break;
        }
    }
    mock.signal(Signal::Release).await;
    mock.wait("GetSceneList", 2).await;
    while let Ok(event) = rx.try_recv() {
        let event = peel(event);
        let event = match event {
            AppEvent::Observation { revision, event } => {
                state.reconcile_observation(revision, *event)
            }
            event => Some(event),
        };
        assert!(
            !matches!(event, Some(AppEvent::GraphUpdated(_))),
            "superseded graph must not mark topology Ready while follow-up is blocked"
        );
    }
    assert_ne!(
        state.topology_status,
        crate::controller::state::TopologyStatus::Ready
    );
    mock.signal(Signal::Release).await;
    graph_ready(&rx).await;
    mock.client.retire();
    task.await.unwrap();
    mock.stop().await;
}
async fn echo_pair(mock: &Mock) {
    let id = "00000000-0000-0000-0000-000000000001";
    mock.signal(Signal::Event(
        "InputMuteStateChanged",
        8,
        json!({"inputName":"MicA","inputUuid":id,"inputMuted":true}),
    ))
    .await;
    mock.signal(Signal::Event(
        "InputVolumeChanged",
        8,
        json!({"inputName":"MicA","inputUuid":id,"inputVolumeMul":0.25,"inputVolumeDb":-12.0}),
    ))
    .await;
}
async fn consume_echo_pair(rx: &mpsc::Receiver<AppEvent>, state: &mut AppState) {
    let mut echoes = 0;
    while echoes < 2 {
        if let AppEvent::Observation { revision, event } = peel(receive(rx).await) {
            let Some(event) = state.reconcile_observation(revision, *event) else {
                continue;
            };
            match event {
                AppEvent::InputMuteChanged { input, muted } => {
                    state.apply_input_mute(&input, muted);
                    echoes += 1;
                }
                AppEvent::InputVolumeChanged {
                    input,
                    volume_mul,
                    volume_db,
                } => {
                    state.apply_input_volume(&input, volume_mul, volume_db);
                    echoes += 1;
                }
                _ => {}
            }
        }
    }
}

#[tokio::test]
async fn bootstrap_active_audio_snapshot_merges_newer_mute_and_volume_echoes() {
    let mut mock = mock(Plan {
        hold: Some(("GetInputVolume", 1)),
        ..Default::default()
    })
    .await;
    let (tx, rx) = mpsc::sync_channel(128);
    let task = pump(&mut mock, tx);
    let mut state = state();
    state.set_current_program("A".into());
    mock.wait("GetInputVolume", 1).await;
    // Consume loading, then use the exact production reconciliation API used by GTK.
    loop {
        if let AppEvent::Observation { revision, event } = receive(&rx).await {
            if matches!(*event, AppEvent::ActiveAudioLoading { .. }) {
                state.reconcile_observation(revision, *event).unwrap();
                break;
            }
        }
    }
    echo_pair(&mock).await;
    consume_echo_pair(&rx, &mut state).await;
    mock.signal(Signal::Release).await;
    loop {
        if let AppEvent::Observation { revision, event } = receive(&rx).await {
            if let Some(AppEvent::ActiveAudioObserved { scene, result }) =
                state.reconcile_observation(revision, *event)
            {
                let inputs = result.unwrap();
                assert!(inputs[0].muted);
                assert_eq!(inputs[0].volume_mul, 0.25);
                state.apply_active_audio(scene, Ok(inputs));
                break;
            }
        }
    }
    assert!(state.audio_inputs[0].muted);
    assert_eq!(state.audio_inputs[0].volume_mul, 0.25);
    assert!(
        matches!(state.visible_mixer_render_source(), MixerVisibleRenderSource::ActiveScene(inputs) if inputs[0].muted)
    );
    mock.client.retire();
    task.await.unwrap();
    mock.stop().await;
}

struct ConfigProvider;
impl crate::controller::dependencies::AppConfigProvider for ConfigProvider {
    fn load_config(&self) -> crate::storage::config::AppConfig {
        Default::default()
    }
}
struct NoPassword;
impl crate::controller::dependencies::ObsPasswordProvider for NoPassword {
    fn obs_password(&self) -> Result<Option<String>, AppError> {
        panic!("wire fixtures must never access keyring")
    }
}
fn dependencies() -> ControllerDependencies {
    ControllerDependencies::new(Arc::new(ConfigProvider), Arc::new(NoPassword))
}

#[tokio::test]
async fn selected_audio_real_command_snapshot_merges_newer_echoes() {
    let mut mock = mock(Plan {
        hold: Some(("GetInputVolume", 2)),
        ..Default::default()
    })
    .await;
    let (tx, rx) = mpsc::sync_channel(128);
    let task = pump(&mut mock, tx.clone());
    graph_ready(&rx).await;
    let mut controller = AppController::with_dependencies(Handle::current(), tx, dependencies());
    *controller.client_slot.lock().unwrap() = Some(mock.client.clone());
    let mut state = state();
    let request = state.set_mixer_audio_loading("A".into());
    controller.handle(AppCommand::RefreshMixerSceneAudio(request.clone()));
    mock.wait("GetInputVolume", 2).await;
    echo_pair(&mock).await;
    consume_echo_pair(&rx, &mut state).await;
    mock.signal(Signal::Release).await;
    loop {
        if let AppEvent::Observation { revision, event } = peel(receive(&rx).await) {
            if let Some(AppEvent::MixerAudioInputsUpdated {
                request: completed,
                inputs,
            }) = state.reconcile_observation(revision, *event)
            {
                assert_eq!(completed, request);
                state.set_mixer_audio_success(completed, inputs);
                break;
            }
        }
    }
    assert!(
        matches!(state.visible_mixer_audio_status("A"), MixerVisibleAudioStatus::Loaded(inputs) if inputs[0].muted && inputs[0].volume_mul == 0.25)
    );
    controller.shutdown().await.unwrap();
    task.await.unwrap();
    mock.stop().await;
}

async fn switch_b(mock: &Mock) {
    mock.signal(Signal::Event(
        "CurrentProgramSceneChanged",
        4,
        json!({"sceneName":"B","sceneUuid":"00000000-0000-0000-0000-000000000002"}),
    ))
    .await;
}

#[tokio::test]
async fn failed_program_audio_clears_previous_controls_and_is_not_successful_emptiness() {
    for failure in [
        Some("GetSceneItemList"),
        Some("GetSceneItemEnabled"),
        Some("GetInputMute"),
        Some("GetInputVolume"),
        None,
    ] {
        let mut mock = mock(Plan {
            fail_b: failure,
            empty_b: failure.is_none(),
            ..Default::default()
        })
        .await;
        let (tx, rx) = mpsc::sync_channel(128);
        let task = pump(&mut mock, tx);
        let mut state = state();
        state.set_current_program("A".into());
        // Complete the first real observation using the same state APIs as GTK.
        loop {
            if let AppEvent::Observation { revision, event } = receive(&rx).await {
                if let Some(AppEvent::ActiveAudioObserved { scene, result }) =
                    state.reconcile_observation(revision, *event)
                {
                    state.apply_active_audio(scene, result);
                    break;
                }
            }
        }
        assert_eq!(state.audio_inputs[0].id, "MicA");
        graph_ready(&rx).await;
        switch_b(&mock).await;
        loop {
            if let AppEvent::Observation { revision, event } = receive(&rx).await {
                if let Some(AppEvent::CurrentSceneChanged(scene)) =
                    state.reconcile_observation(revision, *event)
                {
                    state.set_current_program(scene);
                    break;
                }
            }
        }
        assert!(state.audio_inputs.is_empty());
        assert!(state.active_audio_notice().is_some());
        assert!(matches!(
            state.visible_mixer_render_source(),
            MixerVisibleRenderSource::Scene {
                status: MixerVisibleAudioStatus::Loading,
                ..
            }
        ));
        loop {
            if let AppEvent::Observation { revision, event } = receive(&rx).await {
                if let Some(AppEvent::ActiveAudioObserved { scene, result }) =
                    state.reconcile_observation(revision, *event)
                {
                    state.apply_active_audio(scene, result);
                    break;
                }
            }
        }
        assert!(state.audio_inputs.is_empty());
        if failure.is_some() {
            assert!(matches!(
                state.active_audio_status,
                ActiveAudioStatus::Failed(_)
            ));
            assert!(matches!(
                state.visible_mixer_render_source(),
                MixerVisibleRenderSource::Scene {
                    status: MixerVisibleAudioStatus::Error(_),
                    ..
                }
            ));
            assert!(state.active_audio_notice().is_some());
        } else {
            assert!(matches!(
                state.active_audio_status,
                ActiveAudioStatus::Ready
            ));
            assert!(state.active_audio_notice().is_none());
            assert!(
                matches!(state.visible_mixer_render_source(), MixerVisibleRenderSource::ActiveScene(inputs) if inputs.is_empty())
            );
        }
        mock.client.retire();
        task.await.unwrap();
        mock.stop().await;
    }
}

#[tokio::test]
async fn app_disconnect_drains_controller_owned_refresh_before_output_shutdown() {
    let mut mock = mock(Plan {
        hold: Some(("GetProfileList", 2)),
        active_outputs: true,
        ..Default::default()
    })
    .await;
    let (tx, rx) = mpsc::sync_channel(128);
    let mut controller =
        AppController::with_dependencies(Handle::current(), tx.clone(), dependencies());
    controller.session = SessionController::with_runner(
        Handle::current(),
        EventSender::new(tx),
        Arc::new(Runner {
            clients: Mutex::new(VecDeque::from([(
                mock.client.clone(),
                mock.events.take().unwrap(),
            )])),
        }),
        SessionScope::initial(controller.client_slot.clone()),
        Arc::new(Mutex::new(None)),
    );
    controller.sync_session_scope();
    controller.handle(AppCommand::Connect);
    graph_ready(&rx).await;
    controller.handle(AppCommand::RefreshData);
    mock.wait("GetProfileList", 2).await;
    controller.handle(AppCommand::Disconnect);
    // Give the command's synchronous scope synchronization and teardown a turn
    // while the admitted refresh read is still held by the loopback server.
    tokio::task::yield_now().await;
    mock.signal(Signal::Release).await;
    loop {
        match peel(receive(&rx).await) {
            AppEvent::Disconnected => break,
            AppEvent::Error(error) => panic!("graceful output shutdown failed: {error}"),
            _ => {}
        }
    }
    assert!(controller.connected_client().is_none());
    let frames = mock.stop().await;
    assert_eq!(
        frames.iter().filter(|name| *name == "StopStream").count(),
        1,
        "{frames:?}"
    );
    assert_eq!(
        frames.iter().filter(|name| *name == "StopRecord").count(),
        1,
        "{frames:?}"
    );
    controller.shutdown().await.unwrap();
}

struct Runner {
    clients: Mutex<VecDeque<(ObsClient, ObsEventStream)>>,
}
impl SessionRunner for Runner {
    fn run(&self, tx: EventSender, scope: SessionScope) -> BoxFuture<'static, ()> {
        let (client, events) = self.clients.lock().unwrap().pop_front().unwrap();
        Box::pin(async move {
            assert!(scope.install(client.clone()));
            crate::controller::refresh_controller::run_event_loop(
                client,
                events,
                tx,
                vec![],
                Arc::new(Mutex::new(None)),
            )
            .await;
        })
    }
}
async fn terminal_case(acknowledged: bool, full: bool, reconnect: bool) {
    let held = if acknowledged {
        ("GetStreamStatus", 2)
    } else {
        ("StartStream", 1)
    };
    let mut first = mock(Plan {
        hold: Some(held),
        ..Default::default()
    })
    .await;
    let mut second = if reconnect {
        Some(mock(Plan::default()).await)
    } else {
        None
    };
    let (tx, rx) = mpsc::sync_channel(if full { 1 } else { 128 });
    let mut controller =
        AppController::with_dependencies(Handle::current(), tx.clone(), dependencies());
    let mut clients = VecDeque::from([(first.client.clone(), first.events.take().unwrap())]);
    if let Some(second) = &mut second {
        clients.push_back((second.client.clone(), second.events.take().unwrap()));
    }
    controller.session = SessionController::with_runner(
        Handle::current(),
        EventSender::new(tx),
        Arc::new(Runner {
            clients: Mutex::new(clients),
        }),
        SessionScope::initial(controller.client_slot.clone()),
        Arc::new(Mutex::new(None)),
    );
    controller.sync_session_scope();
    controller.handle(AppCommand::Connect);
    graph_ready(&rx).await;
    let generation = controller.session.scope().generation();
    controller.handle(AppCommand::StartStreaming);
    first.wait(held.0, held.1).await;
    tokio::time::timeout(Duration::from_secs(12), first.client.wait_retired())
        .await
        .unwrap();
    tokio::time::timeout(
        Duration::from_secs(2),
        controller.session.scope().cancelled(),
    )
    .await
    .unwrap();
    // Deliberately no GTK drain until after the real owner retires the scope.
    if reconnect {
        controller.handle(AppCommand::Connect);
    }
    let terminal: SessionRetirement = loop {
        if let AppEvent::SessionRetired(terminal) = receive(&rx).await {
            if terminal.generation == generation {
                break terminal;
            }
        }
    };
    assert!(terminal.reason.contains("deadline"), "{}", terminal.reason);
    assert_eq!(terminal.operations.len(), 1);
    let operation = &terminal.operations[0];
    assert!(matches!(operation.kind, OperationKind::Stream));
    assert!(operation.dispatched);
    assert_eq!(operation.acknowledged, acknowledged);
    assert_eq!(operation.uncertain, !acknowledged);
    assert!(operation.failure.is_some());
    let current = controller.accepts_scope(terminal.generation, None);
    assert_eq!(current, !reconnect);
    assert!(
        !controller.accepts_scope(terminal.generation, Some(0)),
        "retired snapshots remain rejected"
    );
    let mut state = state();
    state.obs_status = ObsStatus::Connected {
        obs_version: "replacement sentinel".into(),
    };
    state.stream_status = OutputStatus::new(true, OutputRunState::Active);
    if current {
        state.reset_obs_session(ObsStatus::Error(terminal.reason.clone()));
    }
    state.apply_terminal_evidence(terminal, current);
    assert_eq!(state.session_retirements.len(), 1);
    assert!(!state.accepts_session_publication(generation, &AppEvent::StreamCommandSucceeded));
    assert!(!state.accepts_session_publication(generation, &AppEvent::Disconnected));
    assert!(state.accepts_session_publication(
        generation,
        &AppEvent::GraphRefreshRejected(AppError::Connection("fixture".into()))
    ));
    if reconnect {
        assert_eq!(state.stream_status.state, OutputRunState::Active);
        assert!(matches!(state.obs_status, ObsStatus::Connected { .. }));
    } else {
        assert_eq!(state.stream_status.state, OutputRunState::Unknown);
        assert!(state.last_stream_command_error.is_some());
    }
    controller.shutdown().await.unwrap();
    assert_eq!(
        first
            .stop()
            .await
            .iter()
            .filter(|name| *name == "StartStream")
            .count(),
        1
    );
    if let Some(second) = second {
        assert!(!second.stop().await.iter().any(|name| name == "StartStream"));
    }
}
#[tokio::test]
async fn delayed_gtk_drain_preserves_unknown_mutation_evidence_after_real_session_timeout() {
    terminal_case(false, false, false).await;
}
#[tokio::test]
async fn delayed_gtk_drain_preserves_acknowledgement_when_readback_retires_the_session() {
    terminal_case(true, false, false).await;
}
#[tokio::test]
async fn full_gtk_channel_does_not_erase_owner_completed_terminal_evidence() {
    terminal_case(false, true, false).await;
}
#[tokio::test]
async fn reconnect_before_terminal_delivery_keeps_old_evidence_audit_only() {
    terminal_case(false, true, true).await;
}

#[tokio::test]
async fn bootstrap_scene_snapshot_cannot_relabel_stale_program_audio_as_current() {
    let mut mock = mock(Plan {
        hold: Some(("GetSceneList", 1)),
        ..Default::default()
    })
    .await;
    let (tx, rx) = mpsc::sync_channel(128);
    let task = pump(&mut mock, tx);
    mock.wait("GetSceneList", 1).await;
    let mut state = state();
    state.set_current_program("A".into());
    switch_b(&mock).await;
    loop {
        if let AppEvent::Observation { revision, event } = receive(&rx).await {
            if let Some(AppEvent::CurrentSceneChanged(scene)) =
                state.reconcile_observation(revision, *event)
            {
                state.set_current_program(scene);
                break;
            }
        }
    }
    mock.signal(Signal::Release).await;
    let mut stale_audio_discarded = false;
    loop {
        if let AppEvent::Observation { revision, event } = receive(&rx).await {
            let old_audio = matches!(&*event, AppEvent::ActiveAudioObserved { scene: Some(scene), .. } if scene == "A");
            let reconciled = state.reconcile_observation(revision, *event);
            if old_audio {
                assert!(reconciled.is_none());
                stale_audio_discarded = true;
            }
            match reconciled {
                Some(AppEvent::SceneInventoryUpdated(inventory)) => {
                    assert_eq!(inventory.current_id.as_deref(), Some("B"));
                    state.scene_inventory = inventory;
                }
                Some(AppEvent::ActiveAudioObserved { scene, result }) => {
                    state.apply_active_audio(scene, result);
                    break;
                }
                _ => {}
            }
        }
    }
    assert!(stale_audio_discarded);
    assert_eq!(state.audio_inputs[0].id, "MicB");
    mock.client.retire();
    task.await.unwrap();
    mock.stop().await;
}

#[tokio::test]
async fn program_aba_during_audio_scan_rejects_the_old_same_name_completion() {
    let mut mock = mock(Plan {
        hold: Some(("GetInputVolume", 1)),
        ..Default::default()
    })
    .await;
    let (tx, rx) = mpsc::sync_channel(128);
    let task = pump(&mut mock, tx);
    let mut state = state();
    state.set_current_program("A".into());
    mock.wait("GetInputVolume", 1).await;
    // Establish the actual request identity before invalidating its Program scope.
    loop {
        if let AppEvent::Observation { revision, event } = receive(&rx).await {
            if matches!(*event, AppEvent::ActiveAudioLoading { .. }) {
                state.reconcile_observation(revision, *event).unwrap();
                break;
            }
        }
    }
    switch_b(&mock).await;
    mock.signal(Signal::Event(
        "CurrentProgramSceneChanged",
        4,
        json!({"sceneName":"A","sceneUuid":"00000000-0000-0000-0000-000000000001"}),
    ))
    .await;
    let mut changes = 0;
    while changes < 2 {
        if let AppEvent::Observation { revision, event } = receive(&rx).await {
            if let Some(AppEvent::CurrentSceneChanged(scene)) =
                state.reconcile_observation(revision, *event)
            {
                state.set_current_program(scene);
                changes += 1;
            }
        }
    }
    mock.signal(Signal::Release).await;
    loop {
        if let AppEvent::Observation { revision, event } = receive(&rx).await {
            if matches!(*event, AppEvent::ActiveAudioObserved { .. }) {
                assert!(state.reconcile_observation(revision, *event).is_none());
                break;
            }
        }
    }
    assert!(state.audio_inputs.is_empty());
    loop {
        if let AppEvent::Observation { revision, event } = receive(&rx).await {
            if let Some(AppEvent::ActiveAudioObserved { scene, result }) =
                state.reconcile_observation(revision, *event)
            {
                state.apply_active_audio(scene, result);
                break;
            }
        }
    }
    assert_eq!(state.audio_inputs[0].id, "MicA");
    mock.client.retire();
    task.await.unwrap();
    assert_eq!(
        mock.stop()
            .await
            .iter()
            .filter(|name| *name == "GetInputMute")
            .count(),
        2
    );
}

#[tokio::test]
async fn real_session_retirement_preserves_workspace_ack_and_unknown_outcomes() {
    for acknowledged in [true, false] {
        let held = if acknowledged {
            ("GetSceneList", 2)
        } else {
            ("SetSceneItemEnabled", 1)
        };
        let mut mock = mock(Plan {
            hold: Some(held),
            ..Default::default()
        })
        .await;
        let (tx, rx) = mpsc::sync_channel(128);
        let mut controller =
            AppController::with_dependencies(Handle::current(), tx.clone(), dependencies());
        let runner = Runner {
            clients: Mutex::new(VecDeque::from([(
                mock.client.clone(),
                mock.events.take().unwrap(),
            )])),
        };
        controller.session = SessionController::with_runner(
            Handle::current(),
            EventSender::new(tx),
            Arc::new(runner),
            SessionScope::initial(controller.client_slot.clone()),
            Arc::new(Mutex::new(None)),
        );
        controller.sync_session_scope();
        controller.handle(AppCommand::Connect);
        graph_ready(&rx).await;
        let mut request = request(&mock.client);
        request.result_id = Some(7);
        controller.handle(AppCommand::WorkspaceAction {
            request,
            action: ControlAction::SetItemEnabled {
                owner: "A".into(),
                item_id: 1,
                enabled: true,
            },
            expected_text: None,
        });
        mock.wait(held.0, held.1).await;
        tokio::time::timeout(Duration::from_secs(12), mock.client.wait_retired())
            .await
            .unwrap();
        tokio::time::timeout(
            Duration::from_secs(2),
            controller.session.scope().cancelled(),
        )
        .await
        .unwrap();
        let terminal = loop {
            if let AppEvent::SessionRetired(terminal) = receive(&rx).await {
                break terminal;
            }
        };
        let operation = terminal
            .operations
            .iter()
            .find(|operation| {
                matches!(
                    operation.kind,
                    OperationKind::Workspace {
                        result_id: Some(7),
                        ..
                    }
                )
            })
            .unwrap();
        assert_eq!(operation.acknowledged, acknowledged);
        assert_eq!(operation.uncertain, !acknowledged);
        assert!(operation.dispatched);
        assert!(operation.failure.is_some());
        controller.shutdown().await.unwrap();
        assert_eq!(
            mock.stop()
                .await
                .iter()
                .filter(|name| *name == "SetSceneItemEnabled")
                .count(),
            1
        );
    }
}

#[tokio::test]
async fn output_status_snapshot_cannot_undo_a_newer_transport_echo() {
    let mut mock = mock(Plan {
        hold: Some(("GetStreamStatus", 1)),
        ..Default::default()
    })
    .await;
    let (tx, rx) = mpsc::sync_channel(128);
    let task = pump(&mut mock, tx);
    mock.wait("GetStreamStatus", 1).await;
    mock.signal(Signal::Event(
        "StreamStateChanged",
        64,
        json!({"outputActive":true,"outputState":"OBS_WEBSOCKET_OUTPUT_STARTED"}),
    ))
    .await;
    let mut state = state();
    loop {
        if let AppEvent::Observation { revision, event } = receive(&rx).await {
            if let Some(AppEvent::StreamStatusUpdated(status)) =
                state.reconcile_observation(revision, *event)
            {
                assert!(status.active);
                state.set_stream_status(status);
                break;
            }
        }
    }
    mock.signal(Signal::Release).await;
    loop {
        if let AppEvent::Observation { revision, event } = receive(&rx).await {
            if matches!(*event, AppEvent::StreamStatusUpdated(_)) {
                assert!(state.reconcile_observation(revision, *event).is_none());
                break;
            }
        }
    }
    assert!(state.stream_status.active);
    mock.client.retire();
    task.await.unwrap();
    mock.stop().await;
}

#[tokio::test]
async fn doctor_refresh_without_client_completes_loading_with_failure_even_after_retirement() {
    let (tx, rx) = mpsc::sync_channel(1);
    let mut controller = AppController::with_dependencies(Handle::current(), tx, dependencies());
    for retired in [false, true] {
        if retired {
            controller.session.scope().retire();
        }
        controller.handle(AppCommand::RefreshData);
        assert!(matches!(
            peel(receive(&rx).await),
            AppEvent::GraphRefreshRejected(_)
        ));
        controller.refresh_task.take().unwrap().await.unwrap();
    }
    controller.shutdown().await.unwrap();
}

#[tokio::test]
async fn doctor_refresh_is_one_coalesced_worker_even_at_optional_command_saturation() {
    let mut mock = mock(Plan {
        hold: Some(("GetSceneList", 1)),
        ..Default::default()
    })
    .await;
    let (tx, rx) = mpsc::sync_channel(128);
    let mut controller = AppController::with_dependencies(Handle::current(), tx, dependencies());
    assert!(controller.session.scope().install(mock.client.clone()));
    for _ in 0..64 {
        controller
            .session
            .scope()
            .spawn(&Handle::current(), std::future::pending())
            .unwrap();
    }
    assert!(controller
        .session
        .scope()
        .spawn(&Handle::current(), std::future::pending())
        .is_none());
    controller.handle(AppCommand::RefreshData);
    mock.wait("GetSceneList", 1).await;
    for _ in 0..256 {
        controller.handle(AppCommand::RefreshData);
    }
    mock.signal(Signal::Release).await;
    graph_ready(&rx).await;
    graph_ready(&rx).await;
    tokio::time::timeout(
        Duration::from_secs(2),
        controller.refresh_task.take().unwrap(),
    )
    .await
    .unwrap()
    .unwrap();
    controller.shutdown().await.unwrap();
    assert_eq!(
        mock.stop()
            .await
            .iter()
            .filter(|name| *name == "GetSceneList")
            .count(),
        2
    );
}
