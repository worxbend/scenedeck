//! Stateful loopback OBS: a two-scene show whose replies follow the events a
//! test pushes, and whose replies a test can hold back or refuse.
//!
//! Tests speak the typed vocabulary here ([`Request`], [`Event`], [`Scene`],
//! [`When`]); the wire JSON never leaves this module.

use std::collections::HashMap;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

use super::wire::{self, MockSocket, Reply};
use crate::obs::client::ObsClient;
use crate::obs::event::ObsEventStream;

/// How long a control or an awaited request may take. Only a failing test
/// waits this long.
const DEADLINE: Duration = Duration::from_secs(3);

/// The status code of a refused request (`RequestProcessingFailed`).
const REFUSED: u64 = 702;

/// UUID shared by scene A, the `Fade` transition and every input.
const FIRST_UUID: &str = "00000000-0000-0000-0000-000000000001";

macro_rules! requests {
    ($($name:ident),+ $(,)?) => {
        /// The requests [`StatefulObs`] answers. It refuses any other.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub(crate) enum Request {
            $($name),+
        }

        impl Request {
            /// The OBS `requestType` on the wire.
            fn wire_name(self) -> &'static str {
                match self {
                    $(Self::$name => stringify!($name)),+
                }
            }

            fn from_wire(name: &str) -> Option<Self> {
                match name {
                    $(stringify!($name) => Some(Self::$name),)+
                    _ => None,
                }
            }
        }
    };
}

requests!(
    GetVersion,
    GetStudioModeEnabled,
    SetCurrentProgramScene,
    SetCurrentPreviewScene,
    GetProfileList,
    GetSceneCollectionList,
    StartStream,
    StopStream,
    GetStreamStatus,
    StopRecord,
    GetRecordStatus,
    GetSceneList,
    GetSpecialInputs,
    GetSceneItemList,
    GetSceneItemEnabled,
    SetSceneItemEnabled,
    GetInputList,
    GetInputMute,
    SetInputMute,
    GetInputVolume,
    GetInputSettings,
    GetInputDefaultSettings,
    SetInputSettings,
    GetSourceFilterList,
    GetMediaInputStatus,
    SetMediaInputCursor,
);

/// The two scenes of the show. Each holds one input, `Mic<scene>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Scene {
    A,
    B,
}

impl Scene {
    fn name(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
        }
    }

    fn uuid(self) -> &'static str {
        match self {
            Self::A => FIRST_UUID,
            Self::B => "00000000-0000-0000-0000-000000000002",
        }
    }
}

/// An OBS event the test pushes to the client. Events that change what OBS
/// reports also change the replies to later reads.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Event {
    /// `CurrentProgramSceneChanged`; later scene reads report this Program.
    ProgramChanged(Scene),
    /// `CurrentPreviewSceneChanged`.
    PreviewChanged(Scene),
    /// `SceneNameChanged`. Scene reads keep reporting the old name.
    SceneRenamed { scene: Scene, to: &'static str },
    /// `SceneListChanged` with an empty list.
    SceneListChanged,
    /// `SceneItemEnableStateChanged` for item 1 of `scene`.
    ItemEnabledChanged { scene: Scene, enabled: bool },
    /// `StudioModeStateChanged`; later mode reads report it.
    StudioModeChanged(bool),
    /// `SceneTransitionEnded` for the `Fade` transition.
    TransitionEnded,
    /// `InputMuteStateChanged` for `MicA`; later mute reads report it.
    MicMuted(bool),
    /// `InputVolumeChanged` for `MicA`; later volume reads report `mul`.
    MicVolume { mul: f64, db: f64 },
    /// `StreamStateChanged` to started. Stream status reads do not change.
    StreamStarted,
}

impl Event {
    /// The `op: 5` frame announcing this event.
    fn frame(self) -> Value {
        let (event_type, intent, data) = match self {
            Self::ProgramChanged(scene) => ("CurrentProgramSceneChanged", 4, scene_ref(scene)),
            Self::PreviewChanged(scene) => ("CurrentPreviewSceneChanged", 4, scene_ref(scene)),
            Self::SceneRenamed { scene, to } => (
                "SceneNameChanged",
                4,
                json!({"oldSceneName":scene.name(),"sceneName":to,"sceneUuid":scene.uuid()}),
            ),
            Self::SceneListChanged => ("SceneListChanged", 4, json!({"scenes":[]})),
            Self::ItemEnabledChanged { scene, enabled } => (
                "SceneItemEnableStateChanged",
                128,
                json!({"sceneName":scene.name(),"sceneUuid":scene.uuid(),"sceneItemId":1,"sceneItemEnabled":enabled}),
            ),
            Self::StudioModeChanged(enabled) => (
                "StudioModeStateChanged",
                1,
                json!({"studioModeEnabled":enabled}),
            ),
            Self::TransitionEnded => (
                "SceneTransitionEnded",
                16,
                json!({"transitionName":"Fade","transitionUuid":FIRST_UUID}),
            ),
            Self::MicMuted(muted) => (
                "InputMuteStateChanged",
                8,
                json!({"inputName":"MicA","inputUuid":FIRST_UUID,"inputMuted":muted}),
            ),
            Self::MicVolume { mul, db } => (
                "InputVolumeChanged",
                8,
                json!({"inputName":"MicA","inputUuid":FIRST_UUID,"inputVolumeMul":mul,"inputVolumeDb":db}),
            ),
            Self::StreamStarted => (
                "StreamStateChanged",
                64,
                json!({"outputActive":true,"outputState":"OBS_WEBSOCKET_OUTPUT_STARTED"}),
            ),
        };
        wire::event(event_type, intent, data)
    }
}

/// Which requests of one type a hold or refusal applies to.
#[derive(Clone, Copy, Debug)]
pub(crate) enum When {
    /// Only the nth request of the type, counting from 1.
    Nth(usize),
    /// Every request of the type after the first.
    AfterFirst,
    /// Every request of the type.
    Always,
    /// Every request of the type while `scene` is Program.
    WhileProgram(Scene),
}

impl When {
    fn matches(self, nth: usize, program: Scene) -> bool {
        match self {
            Self::Nth(wanted) => nth == wanted,
            Self::AfterFirst => nth > 1,
            Self::Always => true,
            Self::WhileProgram(scene) => program == scene,
        }
    }
}

/// Builds a stateful loopback OBS. Scene A starts as Program, studio mode is
/// off, and outputs are idle unless [`Self::with_active_outputs`] says so.
#[derive(Default)]
pub(crate) struct StatefulObs {
    holds: Vec<(Request, When)>,
    refusals: Vec<(Request, When)>,
    empty_scenes: Vec<Scene>,
    outputs_active: bool,
}

impl StatefulObs {
    /// Withhold matching replies until [`StatefulSession::release`].
    pub(crate) fn hold(mut self, request: Request, when: When) -> Self {
        self.holds.push((request, when));
        self
    }

    /// Answer matching requests with a failure status.
    pub(crate) fn refuse(mut self, request: Request, when: When) -> Self {
        self.refusals.push((request, when));
        self
    }

    /// Report `scene` without scene items.
    pub(crate) fn empty_scene(mut self, scene: Scene) -> Self {
        self.empty_scenes.push(scene);
        self
    }

    /// Start with the stream and the recording active.
    pub(crate) fn with_active_outputs(mut self) -> Self {
        self.outputs_active = true;
        self
    }

    /// Start the server and connect a real client to it.
    pub(crate) async fn connect(self) -> StatefulSession {
        let (listener, port) = wire::listen().await;
        let (controls, control_rx) = mpsc::unbounded_channel();
        let (seen_tx, seen) = mpsc::unbounded_channel();
        let (finish, finished) = oneshot::channel();
        let server = Server::new(self, seen_tx);
        let server = tokio::spawn(async move {
            let mut socket = wire::accept(listener).await;
            server.run(&mut socket, control_rx, finished).await
        });
        let (client, events) = ObsClient::connect("127.0.0.1", port, None).await.unwrap();
        StatefulSession {
            client,
            events: Some(events),
            controls,
            seen,
            finish,
            server,
        }
    }

    fn holds(&self, request: Request, nth: usize, program: Scene) -> bool {
        applies(&self.holds, request, nth, program)
    }

    fn refuses(&self, request: Request, nth: usize, program: Scene) -> bool {
        applies(&self.refusals, request, nth, program)
    }
}

fn applies(rules: &[(Request, When)], request: Request, nth: usize, program: Scene) -> bool {
    rules
        .iter()
        .any(|(kind, when)| *kind == request && when.matches(nth, program))
}

/// Every request the server received, in arrival order.
#[derive(Debug)]
pub(crate) struct RequestLog(Vec<String>);

impl RequestLog {
    pub(crate) fn count(&self, request: Request) -> usize {
        let name = request.wire_name();
        self.0.iter().filter(|received| *received == name).count()
    }

    /// The received requests of the given types, in arrival order.
    pub(crate) fn matching(&self, kinds: &[Request]) -> Vec<Request> {
        self.0
            .iter()
            .filter_map(|received| Request::from_wire(received))
            .filter(|request| kinds.contains(request))
            .collect()
    }
}

enum Control {
    ReleaseAll,
    Release(Request),
    Emit(Event),
}

/// A client connected to a [`StatefulObs`], and the handles that drive it.
pub(crate) struct StatefulSession {
    pub(crate) client: ObsClient,
    events: Option<ObsEventStream>,
    controls: mpsc::UnboundedSender<(Control, oneshot::Sender<()>)>,
    seen: mpsc::UnboundedReceiver<(String, usize)>,
    finish: oneshot::Sender<()>,
    server: JoinHandle<RequestLog>,
}

impl StatefulSession {
    /// The client's OBS event stream; a test takes it once.
    pub(crate) fn take_events(&mut self) -> ObsEventStream {
        self.events
            .take()
            .expect("the OBS event stream was already taken")
    }

    /// Push `event` to the client and wait until it was sent.
    pub(crate) async fn emit(&self, event: Event) {
        self.control(Control::Emit(event)).await;
    }

    /// Send every held reply of type `request`.
    pub(crate) async fn release(&self, request: Request) {
        self.control(Control::Release(request)).await;
    }

    /// Send every held reply.
    pub(crate) async fn release_all(&self) {
        self.control(Control::ReleaseAll).await;
    }

    /// Wait until the server received the nth request of type `request`.
    pub(crate) async fn wait(&mut self, request: Request, nth: usize) {
        let name = request.wire_name();
        tokio::time::timeout(DEADLINE, async {
            while let Some((received, count)) = self.seen.recv().await {
                if received == name && count == nth {
                    return;
                }
            }
            panic!("mock closed before {name} #{nth}");
        })
        .await
        .unwrap();
    }

    /// Stop the server and return every request it received.
    pub(crate) async fn stop(self) -> RequestLog {
        self.finish.send(()).unwrap();
        self.server.await.unwrap()
    }

    async fn control(&self, control: Control) {
        let (done, applied) = oneshot::channel();
        self.controls.send((control, done)).unwrap();
        tokio::time::timeout(DEADLINE, applied)
            .await
            .unwrap()
            .unwrap();
    }
}

/// What the mock OBS currently reports.
struct Show {
    program: Scene,
    studio_enabled: bool,
    mic_muted: bool,
    mic_volume_mul: f64,
    stream_active: bool,
    record_active: bool,
}

impl Show {
    fn new(outputs_active: bool) -> Self {
        Self {
            program: Scene::A,
            studio_enabled: false,
            mic_muted: false,
            mic_volume_mul: 0.5,
            stream_active: outputs_active,
            record_active: outputs_active,
        }
    }

    fn observe(&mut self, event: Event) {
        match event {
            Event::ProgramChanged(scene) => self.program = scene,
            Event::StudioModeChanged(enabled) => self.studio_enabled = enabled,
            Event::MicMuted(muted) => self.mic_muted = muted,
            Event::MicVolume { mul, .. } => self.mic_volume_mul = mul,
            Event::PreviewChanged(_)
            | Event::SceneRenamed { .. }
            | Event::SceneListChanged
            | Event::ItemEnabledChanged { .. }
            | Event::TransitionEnded
            | Event::StreamStarted => {}
        }
    }

    /// The response data for `request`; `{}` sends none, as for setters.
    fn answer(&mut self, request: Request, frame: &Value, empty_scenes: &[Scene]) -> Value {
        match request {
            Request::GetVersion => wire::version(&[]),
            Request::GetStudioModeEnabled => json!({"studioModeEnabled":self.studio_enabled}),
            Request::GetProfileList => json!({"currentProfileName":"Main","profiles":["Main"]}),
            Request::GetSceneCollectionList => {
                json!({"currentSceneCollectionName":"Show","sceneCollections":["Show"]})
            }
            Request::StopStream => {
                self.stream_active = false;
                json!({})
            }
            Request::GetStreamStatus => stream_status(self.stream_active),
            Request::StopRecord => {
                self.record_active = false;
                json!({"outputPath":"fixture.mkv"})
            }
            Request::GetRecordStatus => record_status(self.record_active),
            Request::GetSceneList => scene_list(self.program),
            Request::GetSpecialInputs => {
                json!({"desktop1":null,"desktop2":null,"mic1":null,"mic2":null,"mic3":null,"mic4":null})
            }
            Request::GetSceneItemList => scene_items(frame, empty_scenes),
            Request::GetSceneItemEnabled => json!({"sceneItemEnabled":true}),
            Request::GetInputList => json!({"inputs":[]}),
            Request::GetInputMute => json!({"inputMuted":self.mic_muted}),
            Request::GetInputVolume => {
                json!({"inputVolumeMul":self.mic_volume_mul,"inputVolumeDb":-6.0})
            }
            Request::GetInputSettings => json!({
                "inputKind":"text_ft2_source_v2",
                "inputSettings":{"text":"before","from_file":false},
            }),
            Request::GetInputDefaultSettings => {
                json!({"defaultInputSettings":{"text":"","from_file":false}})
            }
            Request::GetSourceFilterList => json!({"filters":[]}),
            Request::GetMediaInputStatus => json!({
                "mediaState":"OBS_MEDIA_STATE_PLAYING",
                "mediaDuration":10000,
                "mediaCursor":0,
            }),
            Request::SetCurrentProgramScene
            | Request::SetCurrentPreviewScene
            | Request::StartStream
            | Request::SetSceneItemEnabled
            | Request::SetInputMute
            | Request::SetInputSettings
            | Request::SetMediaInputCursor => json!({}),
        }
    }
}

fn scene_ref(scene: Scene) -> Value {
    json!({"sceneName":scene.name(),"sceneUuid":scene.uuid()})
}

fn stream_status(active: bool) -> Value {
    json!({
        "outputActive":active,
        "outputReconnecting":false,
        "outputTimecode":"00:00:00.000",
        "outputDuration":0,
        "outputCongestion":0.0,
        "outputBytes":0,
        "outputSkippedFrames":0,
        "outputTotalFrames":0,
    })
}

fn record_status(active: bool) -> Value {
    json!({
        "outputActive":active,
        "outputPaused":false,
        "outputTimecode":"00:00:00.000",
        "outputDuration":0,
        "outputBytes":0,
    })
}

fn scene_list(program: Scene) -> Value {
    let scenes: Vec<Value> = [Scene::A, Scene::B]
        .into_iter()
        .enumerate()
        .map(|(index, scene)| {
            json!({"sceneName":scene.name(),"sceneUuid":scene.uuid(),"sceneIndex":index})
        })
        .collect();
    json!({
        "currentProgramSceneName":program.name(),
        "currentProgramSceneUuid":program.uuid(),
        "scenes":scenes,
    })
}

/// One input, `Mic<owner>`, per scene, except for scenes the test emptied.
fn scene_items(frame: &Value, empty_scenes: &[Scene]) -> Value {
    let owner = wire::request_data(frame)["sceneName"].as_str().unwrap();
    if empty_scenes.iter().any(|scene| scene.name() == owner) {
        return json!({"sceneItems":[]});
    }
    json!({"sceneItems":[{
        "sceneItemId":1,
        "sceneItemIndex":0,
        "sourceName":format!("Mic{owner}"),
        "sourceType":"OBS_SOURCE_TYPE_INPUT",
        "inputKind":"mock",
    }]})
}

/// The server half: answers from [`Show`], applies [`StatefulObs`] rules and
/// test controls, and records every request.
struct Server {
    rules: StatefulObs,
    show: Show,
    counts: HashMap<String, usize>,
    held: Vec<(Request, Value)>,
    log: Vec<String>,
    seen: mpsc::UnboundedSender<(String, usize)>,
}

impl Server {
    fn new(rules: StatefulObs, seen: mpsc::UnboundedSender<(String, usize)>) -> Self {
        Self {
            show: Show::new(rules.outputs_active),
            rules,
            counts: HashMap::new(),
            held: Vec::new(),
            log: Vec::new(),
            seen,
        }
    }

    async fn run(
        mut self,
        socket: &mut MockSocket,
        mut controls: mpsc::UnboundedReceiver<(Control, oneshot::Sender<()>)>,
        mut finished: oneshot::Receiver<()>,
    ) -> RequestLog {
        loop {
            tokio::select! {
                _ = &mut finished => break,
                control = controls.recv() => {
                    let Some((control, done)) = control else { break };
                    self.apply(socket, control).await;
                    // The test may have timed out and dropped the receiver.
                    let _ = done.send(());
                }
                request = wire::next_request(socket) => {
                    let Some(request) = request else { break };
                    self.receive(socket, &request).await;
                }
            }
        }
        RequestLog(self.log)
    }

    async fn apply(&mut self, socket: &mut MockSocket, control: Control) {
        match control {
            Control::ReleaseAll => {
                for (_, response) in self.held.drain(..) {
                    wire::send_json(socket, &response).await;
                }
            }
            Control::Release(request) => {
                let (released, kept) = std::mem::take(&mut self.held)
                    .into_iter()
                    .partition(|(held, _)| *held == request);
                self.held = kept;
                for (_, response) in released {
                    wire::send_json(socket, &response).await;
                }
            }
            Control::Emit(event) => {
                self.show.observe(event);
                wire::send_json(socket, &event.frame()).await;
            }
        }
    }

    async fn receive(&mut self, socket: &mut MockSocket, frame: &Value) {
        let name = frame["d"]["requestType"].as_str().unwrap().to_owned();
        let nth = self.number_request(&name);
        self.log.push(name.clone());
        let request = Request::from_wire(&name);
        let response = wire::response(frame, &self.reply(request, nth, frame));
        let program = self.show.program;
        match request.filter(|request| self.rules.holds(*request, nth, program)) {
            Some(request) => self.held.push((request, response)),
            None => wire::send_json(socket, &response).await,
        }
        // The test may not be waiting for requests.
        let _ = self.seen.send((name, nth));
    }

    /// Numbers a new request within its type, counting from 1.
    fn number_request(&mut self, name: &str) -> usize {
        let count = self.counts.entry(name.to_owned()).or_default();
        *count += 1;
        *count
    }

    fn reply(&mut self, request: Option<Request>, nth: usize, frame: &Value) -> Reply {
        let Some(request) = request else {
            return refused();
        };
        // A refused request leaves the show unchanged: a refused StopStream
        // keeps the stream active.
        if self.rules.refuses(request, nth, self.show.program) {
            return refused();
        }
        Reply::Ok(self.show.answer(request, frame, &self.rules.empty_scenes))
    }
}

fn refused() -> Reply {
    Reply::Err {
        code: REFUSED,
        comment: "stateful mock refused the request".into(),
    }
}
