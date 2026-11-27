//! Characterization tests for `run_event_loop`, one per `ObsEvent` arm.
//!
//! Each test sends a real OBS wire event through a scripted loopback OBS and
//! pins what the loop publishes, in which order, under which observation
//! revision, and which session fences and rescans it triggers.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};

use super::{run_event_loop, TRANSPORT_PUBLICATION_CAPACITY};
use crate::controller::event::AppEvent;
use crate::controller::event_sender::EventSender;
use crate::controller::lifecycle::SessionScope;
use crate::domain::output::{OutputKind, OutputRunState, OutputStatus};
use crate::obs::client::ObsClient;
use crate::obs::testing::scripted::{EventFeed, RequestCounts, ScriptedObs};
use crate::obs::testing::wire::{self, Reply};

const WAIT: Duration = Duration::from_secs(3);
const SCENE_A_UUID: &str = "00000000-0000-0000-0000-000000000001";
const SCENE_B_UUID: &str = "00000000-0000-0000-0000-000000000002";
const INPUT_UUID: &str = "00000000-0000-0000-0000-0000000000a1";
const TRANSITION_UUID: &str = "00000000-0000-0000-0000-0000000000f1";

/// `eventIntent` subscription bits from the obs-websocket protocol.
mod intent {
    pub(super) const GENERAL: u32 = 1;
    pub(super) const CONFIG: u32 = 1 << 1;
    pub(super) const SCENES: u32 = 1 << 2;
    pub(super) const INPUTS: u32 = 1 << 3;
    pub(super) const TRANSITIONS: u32 = 1 << 4;
    pub(super) const OUTPUTS: u32 = 1 << 6;
    pub(super) const SCENE_ITEMS: u32 = 1 << 7;
    pub(super) const MEDIA_INPUTS: u32 = 1 << 8;
    pub(super) const INPUT_VOLUME_METERS: u32 = 1 << 16;
}

/// Answers every request the bootstrap scan, rescans and stats poll send.
///
/// Program scene "A" holds no items, so scans stay short. `GetStats` fails,
/// so the once-a-second stats poll never publishes into the GTK queue.
fn quiet_obs() -> ScriptedObs {
    ScriptedObs::by_type([
        (
            "GetProfileList",
            Reply::Ok(json!({"currentProfileName":"Main","profiles":["Main"]})),
        ),
        (
            "GetSceneCollectionList",
            Reply::Ok(json!({"currentSceneCollectionName":"Show","sceneCollections":["Show"]})),
        ),
        (
            "GetStreamStatus",
            Reply::Ok(
                json!({"outputActive":false,"outputReconnecting":false,"outputTimecode":"00:00:00.000","outputDuration":0,"outputCongestion":0.0,"outputBytes":0,"outputSkippedFrames":0,"outputTotalFrames":0}),
            ),
        ),
        (
            "GetRecordStatus",
            Reply::Ok(
                json!({"outputActive":false,"outputPaused":false,"outputTimecode":"00:00:00.000","outputDuration":0,"outputBytes":0}),
            ),
        ),
        (
            "GetSceneList",
            Reply::Ok(json!({
                "currentProgramSceneName":"A",
                "currentProgramSceneUuid":SCENE_A_UUID,
                "scenes":[{"sceneName":"A","sceneUuid":SCENE_A_UUID,"sceneIndex":0}],
            })),
        ),
        (
            "GetSpecialInputs",
            Reply::Ok(
                json!({"desktop1":null,"desktop2":null,"mic1":null,"mic2":null,"mic3":null,"mic4":null}),
            ),
        ),
        ("GetSceneItemList", Reply::Ok(json!({"sceneItems":[]}))),
        (
            "GetStats",
            Reply::Err {
                code: 207,
                comment: "stats are not part of this fixture".into(),
            },
        ),
    ])
}

/// One GTK-queue entry with its session envelope and observation removed.
#[derive(Debug)]
struct Published {
    collection: Option<u64>,
    revision: Option<u64>,
    event: AppEvent,
}

impl Published {
    fn from_queued(queued: AppEvent) -> Self {
        let AppEvent::SessionEvent {
            collection, event, ..
        } = queued
        else {
            panic!("scoped publication without a session envelope: {queued:?}");
        };
        match *event {
            AppEvent::Observation { revision, event } => Self {
                collection,
                revision: Some(revision),
                event: *event,
            },
            event => Self {
                collection,
                revision: None,
                event,
            },
        }
    }
}

/// The session fences the loop moves synchronously while it handles an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Fences {
    collection: u64,
    graph_revision: u64,
    scene_activation: u64,
}

/// `run_event_loop` on a scoped session against [`quiet_obs`].
struct EventLoop {
    obs: EventFeed,
    requests: RequestCounts,
    client: ObsClient,
    scope: SessionScope,
    gtk: mpsc::Receiver<AppEvent>,
    task: tokio::task::JoinHandle<()>,
}

impl EventLoop {
    /// Start with room in the GTK queue and wait until the bootstrap scan has
    /// published its graph, so the refresh worker is idle.
    async fn bootstrapped() -> Self {
        let event_loop = Self::start(mpsc::sync_channel(256)).await;
        while !matches!(event_loop.next().await.event, AppEvent::GraphUpdated(_)) {}
        event_loop
    }

    async fn start((tx, gtk): (mpsc::SyncSender<AppEvent>, mpsc::Receiver<AppEvent>)) -> Self {
        let obs = quiet_obs();
        let requests = obs.counts();
        let session = obs.connect_with_events().await;
        let scope = SessionScope::initial(Arc::new(Mutex::new(None))).next();
        let task = tokio::spawn(run_event_loop(
            session.client.clone(),
            session.events,
            EventSender::new(tx).scoped(&scope),
            Vec::new(),
            Arc::new(Mutex::new(None)),
        ));
        Self {
            obs: session.feed,
            requests,
            client: session.client,
            scope,
            gtk,
            task,
        }
    }

    fn push(&self, event_type: &str, intent: u32, data: Value) {
        self.obs.push(wire::event(event_type, intent, data));
    }

    fn fences(&self) -> Fences {
        Fences {
            collection: self.scope.collection(),
            graph_revision: self.scope.graph_revision(),
            scene_activation: self.scope.scene_activation_revision(),
        }
    }

    /// Take one token from the connection's observation clock. The next
    /// token the loop takes is this one plus one.
    fn take_observation(&self) -> u64 {
        self.client.next_observation()
    }

    fn requests(&self, request_type: &str) -> usize {
        self.requests
            .snapshot()
            .get(request_type)
            .copied()
            .unwrap_or_default()
    }

    async fn next(&self) -> Published {
        tokio::time::timeout(WAIT, async {
            loop {
                if let Ok(queued) = self.gtk.try_recv() {
                    return Published::from_queued(queued);
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("no publication reached the GTK queue")
    }

    async fn take(&self, count: usize) -> Vec<Published> {
        let mut published = Vec::with_capacity(count);
        for _ in 0..count {
            published.push(self.next().await);
        }
        published
    }

    /// Wait for a full rescan, whose first publication is the profile list.
    async fn rescan(&self) -> Published {
        let profiles = self.next().await;
        assert!(
            matches!(profiles.event, AppEvent::ProfilesUpdated(_)),
            "expected a full rescan, got {profiles:?}"
        );
        assert_eq!(self.requests("GetProfileList"), 2);
        profiles
    }

    async fn stop(self) {
        self.client.retire();
        tokio::time::timeout(WAIT, self.task)
            .await
            .expect("event loop ignored retirement")
            .expect("event loop panicked");
        self.scope.retire();
    }
}

/// The one publication matching `wanted`. The refresh worker publishes
/// directly, so its reaction may overtake the loop's bounded handoff.
fn find(published: &[Published], wanted: impl Fn(&AppEvent) -> bool) -> &Published {
    let mut matching = published
        .iter()
        .filter(|candidate| wanted(&candidate.event));
    let found = matching
        .next()
        .unwrap_or_else(|| panic!("no matching publication in {published:?}"));
    assert!(matching.next().is_none(), "two matches in {published:?}");
    found
}

#[tokio::test]
async fn studio_mode_change_fences_scene_activation_and_publishes_the_mode() {
    let event_loop = EventLoop::bootstrapped().await;
    let before = event_loop.fences();

    event_loop.push(
        "StudioModeStateChanged",
        intent::GENERAL,
        json!({"studioModeEnabled":true}),
    );

    let published = event_loop.next().await;
    assert!(matches!(published.event, AppEvent::StudioModeChanged(true)));
    assert_eq!((published.collection, published.revision), (Some(0), None));
    assert_eq!(
        event_loop.fences(),
        Fences {
            scene_activation: before.scene_activation + 1,
            ..before
        }
    );
    event_loop.stop().await;
}

#[tokio::test]
async fn virtual_camera_change_publishes_the_settled_state() {
    let event_loop = EventLoop::bootstrapped().await;

    event_loop.push(
        "VirtualcamStateChanged",
        intent::OUTPUTS,
        json!({"outputActive":true,"outputState":"OBS_WEBSOCKET_OUTPUT_STARTED"}),
    );

    let published = event_loop.next().await;
    assert!(matches!(
        published.event,
        AppEvent::VirtualCameraChanged(Some(true))
    ));
    assert_eq!(published.revision, None);
    event_loop.stop().await;
}

#[tokio::test]
async fn controls_change_publishes_workspace_values_changed() {
    let event_loop = EventLoop::bootstrapped().await;
    let before = event_loop.fences();

    event_loop.push(
        "MediaInputPlaybackStarted",
        intent::MEDIA_INPUTS,
        json!({"inputName":"Clip","inputUuid":INPUT_UUID}),
    );

    let published = event_loop.next().await;
    assert!(matches!(published.event, AppEvent::WorkspaceValuesChanged));
    assert_eq!(published.revision, None);
    assert_eq!(event_loop.fences(), before);
    event_loop.stop().await;
}

#[tokio::test]
async fn scene_item_visibility_change_rereads_program_audio_without_a_full_rescan() {
    let event_loop = EventLoop::bootstrapped().await;

    event_loop.push(
        "SceneItemEnableStateChanged",
        intent::SCENE_ITEMS,
        json!({"sceneName":"A","sceneUuid":SCENE_A_UUID,"sceneItemId":1,"sceneItemEnabled":false}),
    );

    let published = event_loop.take(2).await;
    find(&published, |event| {
        matches!(event, AppEvent::WorkspaceValuesChanged)
    });
    // The worker re-reads Program from the inventory, then only its audio.
    find(
        &published,
        |event| matches!(event, AppEvent::ActiveAudioLoading { scene } if scene.as_deref() == Some("A")),
    );
    assert_eq!(event_loop.requests("GetSceneList"), 2);
    assert_eq!(event_loop.requests("GetProfileList"), 1);
    event_loop.stop().await;
}

#[tokio::test]
async fn collection_changing_advances_the_collection_and_invalidates_the_graph_first() {
    let event_loop = EventLoop::bootstrapped().await;
    let before = event_loop.fences();
    let clock = event_loop.take_observation();

    event_loop.push(
        "CurrentSceneCollectionChanging",
        intent::CONFIG,
        json!({"sceneCollectionName":"Show"}),
    );

    let invalidated = event_loop.next().await;
    assert!(matches!(invalidated.event, AppEvent::GraphInvalidated));
    assert_eq!(invalidated.revision, Some(clock + 1));
    assert_eq!(invalidated.collection, Some(1));
    let changing = event_loop.next().await;
    assert!(matches!(changing.event, AppEvent::WorkspaceScopeChanging));
    assert_eq!((changing.collection, changing.revision), (Some(1), None));
    assert_eq!(
        event_loop.fences(),
        Fences {
            collection: before.collection + 1,
            graph_revision: clock + 1,
            scene_activation: before.scene_activation + 1,
        }
    );
    event_loop.stop().await;
}

#[tokio::test]
async fn replay_saved_publishes_the_saved_path() {
    let event_loop = EventLoop::bootstrapped().await;

    event_loop.push(
        "ReplayBufferSaved",
        intent::OUTPUTS,
        json!({"savedReplayPath":"/videos/replay.mkv"}),
    );

    let published = event_loop.next().await;
    assert!(
        matches!(&published.event, AppEvent::ReplaySaved(path) if path == "/videos/replay.mkv"),
        "{published:?}"
    );
    assert_eq!(published.revision, None);
    event_loop.stop().await;
}

#[tokio::test]
async fn stream_status_update_publishes_an_observed_stream_status() {
    let event_loop = EventLoop::bootstrapped().await;
    let clock = event_loop.take_observation();

    event_loop.push(
        "StreamStateChanged",
        intent::OUTPUTS,
        json!({"outputActive":true,"outputState":"OBS_WEBSOCKET_OUTPUT_STARTED"}),
    );

    let published = event_loop.next().await;
    assert_eq!(published.revision, Some(clock + 1));
    assert!(
        matches!(
            &published.event,
            AppEvent::OutputStatusUpdated(OutputKind::Stream, status, _)
                if *status == OutputStatus::new(true, OutputRunState::Active)
        ),
        "{published:?}"
    );
    event_loop.stop().await;
}

#[tokio::test]
async fn record_status_update_publishes_an_observed_record_status() {
    let event_loop = EventLoop::bootstrapped().await;
    let clock = event_loop.take_observation();

    event_loop.push(
        "RecordStateChanged",
        intent::OUTPUTS,
        json!({"outputActive":false,"outputState":"OBS_WEBSOCKET_OUTPUT_STOPPED","outputPath":null}),
    );

    let published = event_loop.next().await;
    assert_eq!(published.revision, Some(clock + 1));
    assert!(
        matches!(
            &published.event,
            AppEvent::OutputStatusUpdated(OutputKind::Record, status, _)
                if *status == OutputStatus::new(false, OutputRunState::Inactive)
        ),
        "{published:?}"
    );
    event_loop.stop().await;
}

#[tokio::test]
async fn program_scene_change_publishes_the_scene_and_refreshes_only_its_audio() {
    let event_loop = EventLoop::bootstrapped().await;
    let clock = event_loop.take_observation();

    event_loop.push(
        "CurrentProgramSceneChanged",
        intent::SCENES,
        json!({"sceneName":"B","sceneUuid":SCENE_B_UUID}),
    );

    let published = event_loop.take(2).await;
    let program = find(
        &published,
        |event| matches!(event, AppEvent::CurrentSceneChanged(scene) if scene == "B"),
    );
    assert_eq!(program.revision, Some(clock + 1));
    // The loop wakes the worker, then takes its token before it yields, so on
    // this single-threaded runtime the worker's token is the next one. The new
    // scene is known, so the worker skips the inventory read.
    let audio = find(
        &published,
        |event| matches!(event, AppEvent::ActiveAudioLoading { scene } if scene.as_deref() == Some("B")),
    );
    assert_eq!(audio.revision, Some(clock + 2));
    assert_eq!(event_loop.requests("GetSceneList"), 1);
    assert_eq!(event_loop.requests("GetProfileList"), 1);
    event_loop.stop().await;
}

#[tokio::test]
async fn input_mute_change_publishes_an_observed_mute_state() {
    let event_loop = EventLoop::bootstrapped().await;
    let clock = event_loop.take_observation();

    event_loop.push(
        "InputMuteStateChanged",
        intent::INPUTS,
        json!({"inputName":"Mic","inputUuid":INPUT_UUID,"inputMuted":true}),
    );

    let published = event_loop.next().await;
    assert_eq!(published.revision, Some(clock + 1));
    assert!(
        matches!(&published.event, AppEvent::InputMuteChanged { input, muted: true } if input == "Mic"),
        "{published:?}"
    );
    event_loop.stop().await;
}

#[tokio::test]
async fn input_volume_change_publishes_an_observed_volume() {
    let event_loop = EventLoop::bootstrapped().await;
    let clock = event_loop.take_observation();

    event_loop.push(
        "InputVolumeChanged",
        intent::INPUTS,
        json!({"inputName":"Mic","inputUuid":INPUT_UUID,"inputVolumeMul":0.5,"inputVolumeDb":-6.0}),
    );

    let published = event_loop.next().await;
    assert_eq!(published.revision, Some(clock + 1));
    assert!(
        matches!(
            &published.event,
            AppEvent::InputVolumeChanged { input, volume_mul, volume_db }
                if input == "Mic" && *volume_mul == 0.5 && *volume_db == -6.0
        ),
        "{published:?}"
    );
    event_loop.stop().await;
}

#[tokio::test]
async fn input_levels_publish_directly_without_taking_an_observation() {
    let event_loop = EventLoop::bootstrapped().await;
    let clock = event_loop.take_observation();

    event_loop.push(
        "InputVolumeMeters",
        intent::INPUT_VOLUME_METERS,
        json!({"inputs":[]}),
    );
    event_loop.push(
        "StreamStateChanged",
        intent::OUTPUTS,
        json!({"outputActive":true,"outputState":"OBS_WEBSOCKET_OUTPUT_STARTED"}),
    );

    let levels = event_loop.next().await;
    assert!(
        matches!(&levels.event, AppEvent::InputLevelsUpdated { levels, .. } if levels.is_empty()),
        "{levels:?}"
    );
    assert_eq!((levels.collection, levels.revision), (Some(0), None));
    let stream = event_loop.next().await;
    assert_eq!(
        stream.revision,
        Some(clock + 1),
        "meter frames must not consume observation tokens"
    );
    event_loop.stop().await;
}

#[tokio::test]
async fn input_levels_skip_the_bounded_handoff_and_drop_while_gtk_is_full() {
    let (tx, gtk) = mpsc::sync_channel(1);
    tx.send(AppEvent::Connecting).unwrap();
    let event_loop = EventLoop::start((tx, gtk)).await;
    let before = event_loop.fences();

    // Through the bounded handoff this many publications would retire the
    // connection; see final_regression_tests::
    // transport_publication_overflow_retires_instead_of_stalling_mode_ingress.
    for _ in 0..TRANSPORT_PUBLICATION_CAPACITY + 2 {
        event_loop.push(
            "InputVolumeMeters",
            intent::INPUT_VOLUME_METERS,
            json!({"inputs":[]}),
        );
    }
    // Ingress is sequential: once this later event fences, every meter frame
    // has been handled.
    event_loop.push(
        "StudioModeStateChanged",
        intent::GENERAL,
        json!({"studioModeEnabled":true}),
    );
    tokio::time::timeout(WAIT, async {
        while event_loop.fences() == before {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("studio mode event never reached ingress");

    assert_eq!(
        event_loop.fences(),
        Fences {
            scene_activation: before.scene_activation + 1,
            ..before
        }
    );
    assert!(event_loop.client.retirement_reason().is_none());
    assert!(!event_loop.scope.has_terminal_reason());
    assert!(matches!(
        event_loop.gtk.try_recv(),
        Ok(AppEvent::Connecting)
    ));
    event_loop.stop().await;
}

#[tokio::test]
async fn scene_rename_takes_a_scope_revision_before_invalidating_the_graph() {
    let event_loop = EventLoop::bootstrapped().await;
    let before = event_loop.fences();
    let clock = event_loop.take_observation();

    event_loop.push(
        "SceneNameChanged",
        intent::SCENES,
        json!({"oldSceneName":"A","sceneName":"Guest","sceneUuid":SCENE_A_UUID}),
    );

    // clock + 1 went to the scope before the graph invalidation took clock + 2,
    // so the published invalidation is the scope's current graph revision.
    let invalidated = event_loop.next().await;
    assert!(matches!(invalidated.event, AppEvent::GraphInvalidated));
    assert_eq!(invalidated.revision, Some(clock + 2));
    let renamed = event_loop.next().await;
    assert!(
        matches!(
            &renamed.event,
            AppEvent::SceneRenamed { uuid, old, new }
                if uuid.as_deref() == Some(SCENE_A_UUID) && old == "A" && new == "Guest"
        ),
        "{renamed:?}"
    );
    assert_eq!(renamed.revision, None);
    assert_eq!(
        event_loop.fences(),
        Fences {
            graph_revision: clock + 2,
            scene_activation: before.scene_activation + 1,
            ..before
        }
    );
    event_loop.rescan().await;
    event_loop.stop().await;
}

/// A topology event takes a scope revision, invalidates the graph under the
/// next one, announces the invalidation and wakes a full rescan.
async fn assert_topology_invalidation(event_type: &str, intent: u32, data: Value) {
    let event_loop = EventLoop::bootstrapped().await;
    let before = event_loop.fences();
    let clock = event_loop.take_observation();

    event_loop.push(event_type, intent, data);

    let invalidated = event_loop.next().await;
    assert!(matches!(invalidated.event, AppEvent::GraphInvalidated));
    assert_eq!(invalidated.revision, Some(clock + 2));
    let announced = event_loop.next().await;
    assert!(matches!(announced.event, AppEvent::WorkspaceInvalidated));
    assert_eq!(announced.revision, None);
    assert_eq!(
        event_loop.fences(),
        Fences {
            graph_revision: clock + 2,
            ..before
        }
    );
    event_loop.rescan().await;
    event_loop.stop().await;
}

#[tokio::test]
async fn inputs_change_invalidates_topology_and_rescans() {
    assert_topology_invalidation(
        "InputRemoved",
        intent::INPUTS,
        json!({"inputName":"Mic","inputUuid":INPUT_UUID}),
    )
    .await;
}

#[tokio::test]
async fn scene_items_change_invalidates_topology_and_rescans() {
    assert_topology_invalidation(
        "SceneItemRemoved",
        intent::SCENE_ITEMS,
        json!({"sceneName":"A","sceneUuid":SCENE_A_UUID,"sourceName":"Mic","sourceUuid":INPUT_UUID,"sceneItemId":1}),
    )
    .await;
}

#[tokio::test]
async fn scene_list_change_invalidates_topology_and_rescans() {
    assert_topology_invalidation("SceneListChanged", intent::SCENES, json!({"scenes":[]})).await;
}

#[tokio::test]
async fn current_collection_change_advances_the_collection_and_rescans_it() {
    let event_loop = EventLoop::bootstrapped().await;
    let before = event_loop.fences();

    event_loop.push(
        "CurrentSceneCollectionChanged",
        intent::CONFIG,
        json!({"sceneCollectionName":"Next"}),
    );

    let changing = event_loop.next().await;
    assert!(matches!(changing.event, AppEvent::WorkspaceScopeChanging));
    assert_eq!((changing.collection, changing.revision), (Some(1), None));
    assert_eq!(
        event_loop.fences(),
        Fences {
            collection: before.collection + 1,
            scene_activation: before.scene_activation + 1,
            ..before
        }
    );
    let rescan = event_loop.rescan().await;
    assert_eq!(rescan.collection, Some(1));
    event_loop.stop().await;
}

/// A profile or list event publishes nothing itself; it only wakes a full
/// rescan, whose profile list is the next publication.
async fn assert_silent_rescan(event_type: &str, data: Value) {
    let event_loop = EventLoop::bootstrapped().await;
    let before = event_loop.fences();

    event_loop.push(event_type, intent::CONFIG, data);

    let rescan = event_loop.rescan().await;
    assert_eq!(rescan.collection, Some(0));
    assert_eq!(event_loop.fences(), before);
    event_loop.stop().await;
}

#[tokio::test]
async fn current_profile_change_only_rescans() {
    assert_silent_rescan("CurrentProfileChanged", json!({"profileName":"Alt"})).await;
}

#[tokio::test]
async fn profile_list_change_only_rescans() {
    assert_silent_rescan("ProfileListChanged", json!({"profiles":["Main","Alt"]})).await;
}

#[tokio::test]
async fn scene_collection_list_change_only_rescans() {
    assert_silent_rescan(
        "SceneCollectionListChanged",
        json!({"sceneCollections":["Show","Next"]}),
    )
    .await;
}

#[tokio::test]
async fn ignored_event_publishes_nothing_and_moves_no_fence() {
    let event_loop = EventLoop::bootstrapped().await;
    let before = event_loop.fences();

    event_loop.push(
        "SceneTransitionEnded",
        intent::TRANSITIONS,
        json!({"transitionName":"Fade","transitionUuid":TRANSITION_UUID}),
    );
    event_loop.push(
        "ReplayBufferSaved",
        intent::OUTPUTS,
        json!({"savedReplayPath":"/videos/marker.mkv"}),
    );

    let published = event_loop.next().await;
    assert!(
        matches!(&published.event, AppEvent::ReplaySaved(path) if path == "/videos/marker.mkv"),
        "the ignored event published {published:?}"
    );
    assert_eq!(event_loop.fences(), before);
    event_loop.stop().await;
}
