//! Deterministic loopback regressions for the actual adapter/controllers/state.
use super::*;
use crate::controller::lifecycle::{OperationKind, SessionRetirement};
use crate::controller::session_controller::SessionRunner;
use crate::controller::state::{
    ActiveAudioStatus, MixerVisibleAudioStatus, MixerVisibleRenderSource, ObsStatus,
};
use crate::controller::test_support::deps::dependencies;
use crate::controller::test_support::events::{
    graph_ready, peel, recv_event, recv_matching, TEST_DEADLINE,
};
use crate::controller::test_support::loopback::{consume_echo_pair, echo_pair, pump, switch_b};
use crate::controller::test_support::state::app_state;
use crate::controller::test_support::workspace::{binding, request};
use crate::controller::workspace::WorkspaceController;
use crate::domain::control::ControlAction;
use crate::domain::output::{OutputKind, OutputRunState, OutputStatus};
use crate::obs::event::ObsEventStream;
use crate::obs::testing::stateful::{Event, Request, Scene, StatefulObs, When, SCENE_WRITES};
use futures_util::future::BoxFuture;
use std::collections::VecDeque;
use std::sync::mpsc;
use std::time::Duration;

#[tokio::test]
async fn cancelled_or_collection_invalidated_text_preflight_never_begins_final_write() {
    for cancel_ui in [true, false] {
        let mut mock = StatefulObs::default()
            .hold(Request::GetInputDefaultSettings, When::Nth(2))
            .connect()
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
        mock.wait(Request::GetInputDefaultSettings, 2).await;
        if cancel_ui {
            workspace.cancel();
        } else {
            scope.advance_collection();
        }
        mock.release_all().await;
        tokio::time::timeout(TEST_DEADLINE, scope.quiesce())
            .await
            .unwrap();
        mock.client.get_version().await.unwrap(); // Drained socket remains reusable.
        scope.retire();
        let frames = mock.stop().await;
        assert_eq!(frames.count(Request::SetInputSettings), 0);
    }
}

#[tokio::test]
async fn cancelled_or_collection_invalidated_media_preflight_never_begins_cursor_write() {
    for cancel_ui in [true, false] {
        let mut mock = StatefulObs::default()
            .hold(Request::GetMediaInputStatus, When::Nth(1))
            .connect()
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
        mock.wait(Request::GetMediaInputStatus, 1).await;
        if cancel_ui {
            workspace.cancel();
        } else {
            scope.advance_collection();
        }
        mock.release_all().await;
        tokio::time::timeout(TEST_DEADLINE, scope.quiesce())
            .await
            .unwrap();
        mock.client.get_version().await.unwrap();
        scope.retire();
        assert_eq!(mock.stop().await.count(Request::SetMediaInputCursor), 0);
    }
}

#[tokio::test]
async fn queued_primary_scene_switch_cannot_cross_a_collection_boundary() {
    let mut mock = StatefulObs::default()
        .hold(Request::GetVersion, When::AfterFirst)
        .connect()
        .await;
    let (tx, _rx) = mpsc::sync_channel(16);
    let mut controller = AppController::with_dependencies(Handle::current(), tx, dependencies());
    let scope = controller.session_scope();
    assert!(scope.install(mock.client.clone()));
    let mut reads = Vec::new();
    for _ in 0..8 {
        let client = mock.client.clone();
        reads.push(tokio::spawn(async move { client.get_version().await }));
    }
    mock.wait(Request::GetVersion, 9).await;
    controller.handle(AppCommand::SwitchPrimaryScene("A".into()));
    scope.advance_collection();
    mock.release_all().await;
    for read in reads {
        read.await.unwrap().unwrap();
    }
    tokio::time::timeout(TEST_DEADLINE, scope.quiesce())
        .await
        .unwrap();
    assert!(mock.client.retirement_reason().is_none());
    controller.shutdown().await.unwrap();
    let frames = mock.stop().await;
    assert_eq!(
        frames.count(Request::SetCurrentProgramScene),
        0,
        "a queued scene switch must not target a replacement collection"
    );
}

#[tokio::test]
async fn scene_rename_publishes_identity_and_refreshes_topology() {
    let mut mock = StatefulObs::default().connect().await;
    let (tx, rx) = mpsc::sync_channel(128);
    let task = pump(&mut mock, tx);
    graph_ready(&rx).await;
    mock.emit(Event::SceneRenamed {
        scene: Scene::A,
        to: "Guest",
    })
    .await;
    let mut renamed = false;
    let mut refreshed = false;
    while !renamed || !refreshed {
        let event = match peel(recv_event(&rx).await) {
            AppEvent::Observation { event, .. } => *event,
            event => event,
        };
        match event {
            AppEvent::SceneRenamed { uuid, old, new } => {
                assert!(uuid.is_some());
                assert_eq!((old.as_str(), new.as_str()), ("A", "Guest"));
                renamed = true;
            }
            AppEvent::GraphUpdated(_) => refreshed = true,
            _ => {}
        }
    }
    task.abort();
    let _ = task.await;
    mock.stop().await;
}

#[tokio::test]
async fn visibility_and_transition_events_do_not_reload_topology_or_scene_mode() {
    let mut mock = StatefulObs::default().connect().await;
    let (tx, rx) = mpsc::sync_channel(128);
    let task = pump(&mut mock, tx);
    graph_ready(&rx).await;
    mock.emit(Event::TransitionEnded).await;
    mock.emit(Event::PreviewChanged(Scene::B)).await;
    mock.emit(Event::ItemEnabledChanged {
        scene: Scene::A,
        enabled: false,
    })
    .await;
    let mut values_changed = false;
    loop {
        let event = match peel(recv_event(&rx).await) {
            AppEvent::Observation { event, .. } => *event,
            event => event,
        };
        assert!(
            !matches!(
                event,
                AppEvent::WorkspaceInvalidated
                    | AppEvent::GraphInvalidated
                    | AppEvent::GraphUpdated(_)
            ),
            "value-only event invalidated unrelated state: {event:?}"
        );
        values_changed |= matches!(event, AppEvent::WorkspaceValuesChanged);
        if matches!(event, AppEvent::ActiveAudioObserved { result: Ok(_), .. }) {
            break;
        }
    }
    assert!(values_changed);
    task.abort();
    let _ = task.await;
    let requests = mock.stop().await;
    for request in [Request::GetProfileList, Request::GetSceneCollectionList] {
        assert_eq!(requests.count(request), 1);
    }
}

#[tokio::test]
async fn program_switch_refreshes_audio_without_reloading_topology_or_setup() {
    let mut mock = StatefulObs::default().connect().await;
    let (tx, rx) = mpsc::sync_channel(128);
    let task = pump(&mut mock, tx);
    graph_ready(&rx).await;
    switch_b(&mock).await;
    loop {
        let event = match peel(recv_event(&rx).await) {
            AppEvent::Observation { event, .. } => *event,
            event => event,
        };
        assert!(
            !matches!(
                event,
                AppEvent::GraphInvalidated | AppEvent::GraphUpdated(_)
            ),
            "a Program change must not blank Graph or Doctor"
        );
        if let AppEvent::ActiveAudioObserved { scene, result } = event {
            if scene.as_deref() == Some("B") {
                assert!(result.is_ok());
                break;
            }
        }
    }
    task.abort();
    let _ = task.await;
    let requests = mock.stop().await;
    for request in [
        Request::GetSceneList,
        Request::GetProfileList,
        Request::GetSceneCollectionList,
    ] {
        assert_eq!(
            requests.count(request),
            1,
            "Program switching repeated {request:?}"
        );
    }
}

#[tokio::test]
async fn queued_scene_activation_rejects_authoritative_mode_change_during_collection_read() {
    let mut mock = StatefulObs::default()
        .hold(Request::GetSceneCollectionList, When::Nth(1))
        .connect()
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
    mock.wait(Request::GetSceneCollectionList, 1).await;
    mock.emit(Event::StudioModeChanged(true)).await;
    mock.release_all().await;
    tokio::time::timeout(TEST_DEADLINE, scope.quiesce())
        .await
        .unwrap();
    mock.client.get_version().await.unwrap();
    scope.retire();
    let frames = mock.stop().await;
    assert!(
        frames.matching(&SCENE_WRITES).is_empty(),
        "an obsolete click must be rejected, not rerouted or replayed in the new mode"
    );
}

#[tokio::test]
async fn scene_activation_mode_echo_during_delayed_mode_reply_fences_old_click() {
    mode_echo_fences_activation(GatePoint::DelayedModeReply, ModeEcho::Once).await;
}

#[tokio::test]
async fn scene_activation_mode_echo_while_final_mutation_waits_for_gate_fences_old_click() {
    mode_echo_fences_activation(GatePoint::FinalMutationAdmission, ModeEcho::Once).await;
}

#[tokio::test]
async fn scene_activation_mode_aba_while_waiting_for_gate_does_not_revive_old_click() {
    mode_echo_fences_activation(GatePoint::FinalMutationAdmission, ModeEcho::ThereAndBack).await;
}

/// Where the scene click waits when the studio mode echo arrives.
#[derive(Clone, Copy, PartialEq)]
enum GatePoint {
    /// Its mode read is still unanswered.
    DelayedModeReply,
    /// Its final mutation queues behind saturated connection admission.
    FinalMutationAdmission,
}

/// How many studio mode echoes reach the event loop.
#[derive(Clone, Copy, PartialEq)]
enum ModeEcho {
    Once,
    /// The mode flips and flips back (A-B-A).
    ThereAndBack,
}

/// An OBS holding the click's mode read, and every later `GetVersion` when
/// the click must also wait at the final gate.
fn mode_read_held(gate: GatePoint) -> StatefulObs {
    let obs = StatefulObs::default().hold(Request::GetStudioModeEnabled, When::Nth(1));
    match gate {
        GatePoint::DelayedModeReply => obs,
        GatePoint::FinalMutationAdmission => obs.hold(Request::GetVersion, When::AfterFirst),
    }
}

async fn mode_echo_fences_activation(gate: GatePoint, echo: ModeEcho) {
    let at_final_gate = gate == GatePoint::FinalMutationAdmission;
    let mut mock = mode_read_held(gate).connect().await;
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
    mock.wait(Request::GetStudioModeEnabled, 1).await;
    let mut reads = Vec::new();
    if at_final_gate {
        // The held mode read occupies one permit; seven reads fill the rest.
        // An eighth read queues ahead of the final mutation (FIFO admission).
        for _ in 0..8 {
            let client = mock.client.clone();
            reads.push(tokio::spawn(async move { client.get_version().await }));
        }
        mock.wait(Request::GetVersion, 8).await;
        mock.release(Request::GetStudioModeEnabled).await;
        mock.wait(Request::GetVersion, 9).await;
    }
    // Bootstrap/stats requests may queue, but event pumping remains independent.
    let task = tokio::spawn(crate::controller::refresh_controller::run_event_loop(
        mock.client.clone(),
        mock.take_events(),
        EventSender::new(tx).scoped(&scope),
        vec![],
        Arc::new(Mutex::new(None)),
    ));
    mock.emit(Event::StudioModeChanged(true)).await;
    recv_matching(&rx, |event| {
        matches!(peel(event.clone()), AppEvent::StudioModeChanged(_))
    })
    .await;
    if echo == ModeEcho::ThereAndBack {
        mock.emit(Event::StudioModeChanged(false)).await;
        recv_matching(&rx, |event| {
            matches!(peel(event.clone()), AppEvent::StudioModeChanged(_))
        })
        .await;
    }
    mock.release_all().await;
    for read in reads {
        read.await.unwrap().unwrap();
    }
    tokio::time::timeout(TEST_DEADLINE, scope.quiesce())
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
        frames.matching(&SCENE_WRITES).is_empty(),
        "mode invalidation must fence the original click at final gate admission, without replay"
    );
}

#[tokio::test]
async fn full_gtk_queue_does_not_hide_mode_echo_during_delayed_mode_reply() {
    mode_echo_with_full_gtk_queue(GatePoint::DelayedModeReply).await;
}

#[tokio::test]
async fn full_gtk_queue_does_not_hide_mode_echo_at_final_mutation_admission() {
    mode_echo_with_full_gtk_queue(GatePoint::FinalMutationAdmission).await;
}

async fn mode_echo_with_full_gtk_queue(gate: GatePoint) {
    let at_final_gate = gate == GatePoint::FinalMutationAdmission;
    let mut mock = mode_read_held(gate).connect().await;
    let slot = Arc::new(Mutex::new(None));
    let scope = SessionScope::initial(slot.clone()).next();
    scope.install(mock.client.clone());
    let (tx, rx) = mpsc::sync_channel(1);
    tx.send(AppEvent::Connecting).unwrap();
    let mut workspace = WorkspaceController::new(Handle::current(), tx.clone(), slot);
    workspace.set_scope(scope.clone());
    let task = tokio::spawn(crate::controller::refresh_controller::run_event_loop(
        mock.client.clone(),
        mock.take_events(),
        EventSender::new(tx).scoped(&scope),
        vec![],
        Arc::new(Mutex::new(None)),
    ));
    // First event reaches ingress, but its publication cannot enter the full
    // GTK queue. Accept the click only AFTER this event's revision advances.
    mock.emit(Event::StudioModeChanged(false)).await;
    tokio::time::timeout(TEST_DEADLINE, async {
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
    mock.wait(Request::GetStudioModeEnabled, 1).await;
    let mut reads = Vec::new();
    if at_final_gate {
        // Bootstrap's held GetVersion already owns one of the eight slots.
        for _ in 0..7 {
            let client = mock.client.clone();
            reads.push(tokio::spawn(async move { client.get_version().await }));
        }
        mock.wait(Request::GetVersion, 8).await;
        mock.release(Request::GetStudioModeEnabled).await;
        mock.wait(Request::GetVersion, 9).await;
    }
    mock.emit(Event::StudioModeChanged(true)).await;
    let ingested_before_gtk_drain = tokio::time::timeout(Duration::from_millis(250), async {
        while scope.scene_activation_revision() == accepted {
            tokio::task::yield_now().await;
        }
    })
    .await
    .is_ok();
    mock.release_all().await;
    for read in reads {
        read.await.unwrap().unwrap();
    }
    recv_matching(&rx, |event| {
        matches!(peel(event.clone()), AppEvent::WorkspaceLoaded { .. })
    })
    .await;
    tokio::time::timeout(TEST_DEADLINE, scope.quiesce())
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
        frames.matching(&SCENE_WRITES).is_empty(),
        "a full GTK queue must not let an obsolete scene click mutate OBS: {frames:?}"
    );
}

#[tokio::test]
async fn bootstrap_failure_fences_mutations_before_waiting_for_full_gtk_queue() {
    let mut mock = StatefulObs::default()
        .refuse(Request::GetVersion, When::Nth(2))
        .connect()
        .await;
    let scope = SessionScope::initial(Arc::new(Mutex::new(None))).next();
    scope.install(mock.client.clone());
    let (tx, rx) = mpsc::sync_channel(1);
    tx.send(AppEvent::Connecting).unwrap();
    let task = tokio::spawn(crate::controller::refresh_controller::run_event_loop(
        mock.client.clone(),
        mock.take_events(),
        EventSender::new(tx).scoped(&scope),
        vec![],
        Arc::new(Mutex::new(None)),
    ));
    mock.wait(Request::GetVersion, 2).await;
    let fenced_without_ui =
        tokio::time::timeout(Duration::from_millis(250), mock.client.wait_retired())
            .await
            .is_ok();
    rx.try_recv().unwrap();
    recv_matching(&rx, |event| {
        matches!(peel(event.clone()), AppEvent::Error(_))
    })
    .await;
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
    let mut mock = StatefulObs::default().connect().await;
    let scope = SessionScope::initial(Arc::new(Mutex::new(None))).next();
    scope.install(mock.client.clone());
    let (tx, _rx) = mpsc::sync_channel(1);
    tx.send(AppEvent::Connecting).unwrap();
    let task = tokio::spawn(crate::controller::refresh_controller::run_event_loop(
        mock.client.clone(),
        mock.take_events(),
        EventSender::new(tx).scoped(&scope),
        vec![],
        Arc::new(Mutex::new(None)),
    ));
    // One publication may be selected by the publisher and another 64 queued.
    // There is no GTK drain: exceeding that bound must fail the connection closed.
    for _ in 0..crate::controller::refresh_controller::TRANSPORT_PUBLICATION_CAPACITY + 2 {
        mock.emit(Event::StudioModeChanged(false)).await;
    }
    tokio::time::timeout(TEST_DEADLINE, task)
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
    assert!(frames.matching(&SCENE_WRITES).is_empty());
}

#[tokio::test]
async fn manual_graph_scan_invalidated_midscan_cannot_publish_ready() {
    let mut mock = StatefulObs::default()
        .hold(Request::GetSceneItemList, When::Nth(5))
        .hold(Request::GetSceneList, When::Nth(3))
        .connect()
        .await;
    let scope = SessionScope::initial(Arc::new(Mutex::new(None))).next();
    scope.install(mock.client.clone());
    let (tx, rx) = mpsc::sync_channel(128);
    let sender = EventSender::new(tx).scoped(&scope);
    let task = tokio::spawn(crate::controller::refresh_controller::run_event_loop(
        mock.client.clone(),
        mock.take_events(),
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
    mock.wait(Request::GetSceneItemList, 5).await;
    // Ignore earlier loading/audio publications; establish the actual structural
    // invalidation before releasing the old graph's held scene response.
    while rx.try_recv().is_ok() {}
    mock.emit(Event::SceneListChanged).await;
    recv_matching(&rx, |event| {
        matches!(peel(event.clone()), AppEvent::Observation { event, .. } if matches!(*event, AppEvent::GraphInvalidated))
    })
    .await;
    mock.release(Request::GetSceneItemList).await;
    tokio::time::timeout(TEST_DEADLINE, manual)
        .await
        .unwrap()
        .unwrap();
    mock.wait(Request::GetSceneList, 3).await;
    while let Ok(event) = rx.try_recv() {
        let event = peel(event);
        assert!(
            !matches!(event, AppEvent::Observation { event, .. } if matches!(*event, AppEvent::GraphUpdated(_) | AppEvent::GraphFailed(_))),
            "a manual scan invalidated mid-read must not publish a graph completion"
        );
    }
    mock.release_all().await;
    graph_ready(&rx).await;
    mock.client.retire();
    task.await.unwrap();
    scope.retire();
    mock.stop().await;
}

#[tokio::test]
async fn manual_graph_completion_waiting_for_gtk_capacity_rechecks_ingress_revision() {
    graph_completion_with_full_gtk_queue(GraphScan::Completes).await;
}

#[tokio::test]
async fn manual_graph_failure_waiting_for_gtk_capacity_rechecks_ingress_revision() {
    graph_completion_with_full_gtk_queue(GraphScan::Fails).await;
}

/// How the manual graph scan ends.
#[derive(Clone, Copy)]
enum GraphScan {
    Completes,
    /// Its scene item read is refused.
    Fails,
}

async fn graph_completion_with_full_gtk_queue(scan: GraphScan) {
    let obs = StatefulObs::default();
    let obs = match scan {
        GraphScan::Completes => obs,
        GraphScan::Fails => obs.refuse(Request::GetSceneItemList, When::Nth(3)),
    };
    let mut mock = obs.connect().await;
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
        () = mock.wait(Request::GetSceneItemList, 3) => {}
    }
    // One later wire reply proves the graph responses passed through obws's
    // receiver. Poll the actual scan through graph_is_current and into send.
    mock.client.get_version().await.unwrap();
    assert!(futures_util::poll!(manual.as_mut()).is_pending());
    let scan_revision = scope.graph_revision();
    let task = tokio::spawn(crate::controller::refresh_controller::run_event_loop(
        mock.client.clone(),
        mock.take_events(),
        sender.clone(),
        vec![],
        Arc::new(Mutex::new(None)),
    ));
    mock.emit(Event::SceneListChanged).await;
    tokio::time::timeout(TEST_DEADLINE, async {
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
    let mut mock = StatefulObs::default()
        .hold(Request::GetSceneItemList, When::Nth(2))
        .hold(Request::GetSceneList, When::Nth(2))
        .connect()
        .await;
    let (tx, rx) = mpsc::sync_channel(128);
    let task = pump(&mut mock, tx);
    let mut state = app_state();
    mock.wait(Request::GetSceneItemList, 2).await;
    while let Ok(event) = rx.try_recv() {
        if matches!(peel(event), AppEvent::GraphInvalidated) {
            state.set_topology_loading();
        }
    }
    mock.emit(Event::SceneListChanged).await;
    loop {
        let event = peel(recv_event(&rx).await);
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
    mock.release_all().await;
    mock.wait(Request::GetSceneList, 2).await;
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
    mock.release_all().await;
    graph_ready(&rx).await;
    mock.client.retire();
    task.await.unwrap();
    mock.stop().await;
}
#[tokio::test]
async fn bootstrap_active_audio_snapshot_merges_newer_mute_and_volume_echoes() {
    let mut mock = StatefulObs::default()
        .hold(Request::GetInputVolume, When::Nth(1))
        .connect()
        .await;
    let (tx, rx) = mpsc::sync_channel(128);
    let task = pump(&mut mock, tx);
    let mut state = app_state();
    state.set_current_program("A".into());
    mock.wait(Request::GetInputVolume, 1).await;
    // Consume loading, then use the exact production reconciliation API used by GTK.
    loop {
        if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
            if matches!(*event, AppEvent::ActiveAudioLoading { .. }) {
                state.reconcile_observation(revision, *event).unwrap();
                break;
            }
        }
    }
    echo_pair(&mock).await;
    consume_echo_pair(&rx, &mut state).await;
    mock.release_all().await;
    loop {
        if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
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

#[tokio::test]
async fn selected_audio_real_command_snapshot_merges_newer_echoes() {
    let mut mock = StatefulObs::default()
        .hold(Request::GetInputVolume, When::Nth(2))
        .connect()
        .await;
    let (tx, rx) = mpsc::sync_channel(128);
    let task = pump(&mut mock, tx.clone());
    graph_ready(&rx).await;
    let mut controller = AppController::with_dependencies(Handle::current(), tx, dependencies());
    *controller.client_slot.lock().unwrap() = Some(mock.client.clone());
    let mut state = app_state();
    let request = state.set_mixer_audio_loading("A".into());
    controller.handle(AppCommand::RefreshMixerSceneAudio(request.clone()));
    mock.wait(Request::GetInputVolume, 2).await;
    echo_pair(&mock).await;
    consume_echo_pair(&rx, &mut state).await;
    mock.release_all().await;
    loop {
        if let AppEvent::Observation { revision, event } = peel(recv_event(&rx).await) {
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

#[tokio::test]
async fn failed_program_audio_clears_previous_controls_and_is_not_successful_emptiness() {
    for failure in [
        Some(Request::GetSceneItemList),
        Some(Request::GetSceneItemEnabled),
        Some(Request::GetInputMute),
        Some(Request::GetInputVolume),
        None,
    ] {
        let obs = StatefulObs::default();
        let obs = match failure {
            Some(request) => obs.refuse(request, When::WhileProgram(Scene::B)),
            None => obs.empty_scene(Scene::B),
        };
        let mut mock = obs.connect().await;
        let (tx, rx) = mpsc::sync_channel(128);
        let task = pump(&mut mock, tx);
        let mut state = app_state();
        state.set_current_program("A".into());
        // Complete the first real observation using the same state APIs as GTK.
        loop {
            if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
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
            if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
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
            if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
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
    let mut mock = StatefulObs::default()
        .hold(Request::GetProfileList, When::Nth(2))
        .with_active_outputs()
        .connect()
        .await;
    let (tx, rx) = mpsc::sync_channel(128);
    let runner = Runner {
        clients: Mutex::new(VecDeque::from([(mock.client.clone(), mock.take_events())])),
    };
    let mut controller =
        AppController::with_session_runner(Handle::current(), tx, dependencies(), Arc::new(runner));
    controller.handle(AppCommand::Connect);
    graph_ready(&rx).await;
    controller.handle(AppCommand::RefreshData);
    mock.wait(Request::GetProfileList, 2).await;
    controller.handle(AppCommand::Disconnect);
    // Give the command's synchronous scope synchronization and teardown a turn
    // while the admitted refresh read is still held by the loopback server.
    tokio::task::yield_now().await;
    mock.release_all().await;
    loop {
        match peel(recv_event(&rx).await) {
            AppEvent::Disconnected => break,
            AppEvent::Error(error) => panic!("graceful output shutdown failed: {error}"),
            _ => {}
        }
    }
    assert!(controller.connected_client().is_none());
    let frames = mock.stop().await;
    assert_eq!(frames.count(Request::StopStream), 1, "{frames:?}");
    assert_eq!(frames.count(Request::StopRecord), 1, "{frames:?}");
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

/// Installs one client and holds the session open without an event loop,
/// so only the command under test reads from OBS.
struct InstallOnlyRunner(ObsClient);
impl SessionRunner for InstallOnlyRunner {
    fn run(&self, _: EventSender, scope: SessionScope) -> BoxFuture<'static, ()> {
        let client = self.0.clone();
        Box::pin(async move {
            assert!(scope.install(client));
            std::future::pending::<()>().await;
        })
    }
}

#[tokio::test]
async fn manual_refresh_publishes_a_failed_stream_readback() {
    let mock = StatefulObs::default()
        .refuse(Request::GetStreamStatus, When::Always)
        .connect()
        .await;
    let (tx, rx) = mpsc::sync_channel(128);
    let mut controller = AppController::with_session_runner(
        Handle::current(),
        tx,
        dependencies(),
        Arc::new(InstallOnlyRunner(mock.client.clone())),
    );
    controller.handle(AppCommand::Connect);
    tokio::time::timeout(TEST_DEADLINE, async {
        while controller.connected_client().is_none() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the session installs its client");

    controller.handle(AppCommand::RefreshData);

    let mut failed_readbacks = Vec::new();
    loop {
        match peel(recv_event(&rx).await) {
            AppEvent::OutputReadbackFailed { kind, .. } => failed_readbacks.push(kind),
            AppEvent::Observation { event, .. } if matches!(*event, AppEvent::GraphUpdated(_)) => {
                break
            }
            _ => {}
        }
    }
    assert_eq!(failed_readbacks, [OutputKind::Stream]);
}

/// Which reply of the stream command OBS withholds until the session times out.
#[derive(Clone, Copy, PartialEq)]
enum Delivery {
    /// `StartStream` itself: OBS may or may not have started streaming.
    Unacknowledged,
    /// The status readback after OBS acknowledged `StartStream`.
    Acknowledged,
}

/// Room in the GTK event channel while nothing drains it.
#[derive(Clone, Copy, PartialEq)]
enum Channel {
    Roomy,
    Full,
}

/// What the operator does after the session retired, before GTK drains.
#[derive(Clone, Copy, PartialEq)]
enum Followup {
    Nothing,
    Reconnect,
}

async fn terminal_case(delivery: Delivery, channel: Channel, followup: Followup) {
    let acknowledged = delivery == Delivery::Acknowledged;
    let reconnect = followup == Followup::Reconnect;
    let held = match delivery {
        Delivery::Acknowledged => (Request::GetStreamStatus, 2),
        Delivery::Unacknowledged => (Request::StartStream, 1),
    };
    let mut first = StatefulObs::default()
        .hold(held.0, When::Nth(held.1))
        .connect()
        .await;
    let mut second = if reconnect {
        Some(StatefulObs::default().connect().await)
    } else {
        None
    };
    let capacity = match channel {
        Channel::Roomy => 128,
        Channel::Full => 1,
    };
    let (tx, rx) = mpsc::sync_channel(capacity);
    let mut clients = VecDeque::from([(first.client.clone(), first.take_events())]);
    if let Some(second) = &mut second {
        clients.push_back((second.client.clone(), second.take_events()));
    }
    let runner = Runner {
        clients: Mutex::new(clients),
    };
    let mut controller =
        AppController::with_session_runner(Handle::current(), tx, dependencies(), Arc::new(runner));
    controller.handle(AppCommand::Connect);
    graph_ready(&rx).await;
    let generation = controller.session_scope().generation();
    controller.handle(AppCommand::SetOutput {
        kind: OutputKind::Stream,
        active: true,
    });
    first.wait(held.0, held.1).await;
    tokio::time::timeout(Duration::from_secs(12), first.client.wait_retired())
        .await
        .unwrap();
    tokio::time::timeout(TEST_DEADLINE, controller.session_scope().cancelled())
        .await
        .unwrap();
    // Deliberately no GTK drain until after the real owner retires the scope.
    if reconnect {
        controller.handle(AppCommand::Connect);
    }
    let terminal: SessionRetirement = loop {
        if let AppEvent::SessionRetired(terminal) = recv_event(&rx).await {
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
    let mut state = app_state();
    state.obs_status = ObsStatus::Connected {
        obs_version: "replacement sentinel".into(),
    };
    state.stream_status = OutputStatus::new(true, OutputRunState::Active);
    if current {
        state.reset_obs_session(ObsStatus::Error(terminal.reason.clone()));
    }
    state.apply_terminal_evidence(terminal, current);
    assert_eq!(state.session_retirements.len(), 1);
    assert!(!state.accepts_session_publication(
        generation,
        &AppEvent::OutputCommandSucceeded(OutputKind::Stream)
    ));
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
    assert_eq!(first.stop().await.count(Request::StartStream), 1);
    if let Some(second) = second {
        assert_eq!(second.stop().await.count(Request::StartStream), 0);
    }
}
#[tokio::test]
async fn delayed_gtk_drain_preserves_unknown_mutation_evidence_after_real_session_timeout() {
    terminal_case(Delivery::Unacknowledged, Channel::Roomy, Followup::Nothing).await;
}
#[tokio::test]
async fn delayed_gtk_drain_preserves_acknowledgement_when_readback_retires_the_session() {
    terminal_case(Delivery::Acknowledged, Channel::Roomy, Followup::Nothing).await;
}
#[tokio::test]
async fn full_gtk_channel_does_not_erase_owner_completed_terminal_evidence() {
    terminal_case(Delivery::Unacknowledged, Channel::Full, Followup::Nothing).await;
}
#[tokio::test]
async fn reconnect_before_terminal_delivery_keeps_old_evidence_audit_only() {
    terminal_case(Delivery::Unacknowledged, Channel::Full, Followup::Reconnect).await;
}

#[tokio::test]
async fn bootstrap_scene_snapshot_cannot_relabel_stale_program_audio_as_current() {
    let mut mock = StatefulObs::default()
        .hold(Request::GetSceneList, When::Nth(1))
        .connect()
        .await;
    let (tx, rx) = mpsc::sync_channel(128);
    let task = pump(&mut mock, tx);
    mock.wait(Request::GetSceneList, 1).await;
    let mut state = app_state();
    state.set_current_program("A".into());
    switch_b(&mock).await;
    loop {
        if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
            if let Some(AppEvent::CurrentSceneChanged(scene)) =
                state.reconcile_observation(revision, *event)
            {
                state.set_current_program(scene);
                break;
            }
        }
    }
    mock.release_all().await;
    let mut stale_audio_discarded = false;
    loop {
        if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
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
    let mut mock = StatefulObs::default()
        .hold(Request::GetInputVolume, When::Nth(1))
        .connect()
        .await;
    let (tx, rx) = mpsc::sync_channel(128);
    let task = pump(&mut mock, tx);
    let mut state = app_state();
    state.set_current_program("A".into());
    mock.wait(Request::GetInputVolume, 1).await;
    // Establish the actual request identity before invalidating its Program scope.
    loop {
        if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
            if matches!(*event, AppEvent::ActiveAudioLoading { .. }) {
                state.reconcile_observation(revision, *event).unwrap();
                break;
            }
        }
    }
    switch_b(&mock).await;
    mock.emit(Event::ProgramChanged(Scene::A)).await;
    let mut changes = 0;
    while changes < 2 {
        if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
            if let Some(AppEvent::CurrentSceneChanged(scene)) =
                state.reconcile_observation(revision, *event)
            {
                state.set_current_program(scene);
                changes += 1;
            }
        }
    }
    mock.release_all().await;
    loop {
        if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
            if matches!(*event, AppEvent::ActiveAudioObserved { .. }) {
                assert!(state.reconcile_observation(revision, *event).is_none());
                break;
            }
        }
    }
    assert!(state.audio_inputs.is_empty());
    loop {
        if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
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
    assert_eq!(mock.stop().await.count(Request::GetInputMute), 2);
}

#[tokio::test]
async fn real_session_retirement_preserves_workspace_ack_and_unknown_outcomes() {
    for acknowledged in [true, false] {
        let held = if acknowledged {
            (Request::GetSceneList, 2)
        } else {
            (Request::SetSceneItemEnabled, 1)
        };
        let mut mock = StatefulObs::default()
            .hold(held.0, When::Nth(held.1))
            .connect()
            .await;
        let (tx, rx) = mpsc::sync_channel(128);
        let runner = Runner {
            clients: Mutex::new(VecDeque::from([(mock.client.clone(), mock.take_events())])),
        };
        let mut controller = AppController::with_session_runner(
            Handle::current(),
            tx,
            dependencies(),
            Arc::new(runner),
        );
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
        tokio::time::timeout(TEST_DEADLINE, controller.session_scope().cancelled())
            .await
            .unwrap();
        let terminal = loop {
            if let AppEvent::SessionRetired(terminal) = recv_event(&rx).await {
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
        assert_eq!(mock.stop().await.count(Request::SetSceneItemEnabled), 1);
    }
}

#[tokio::test]
async fn output_status_snapshot_cannot_undo_a_newer_transport_echo() {
    let mut mock = StatefulObs::default()
        .hold(Request::GetStreamStatus, When::Nth(1))
        .connect()
        .await;
    let (tx, rx) = mpsc::sync_channel(128);
    let task = pump(&mut mock, tx);
    mock.wait(Request::GetStreamStatus, 1).await;
    let request_seen = std::time::Instant::now();
    mock.emit(Event::StreamStarted).await;
    let mut state = app_state();
    state.health.connected();
    loop {
        if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
            if let Some(event @ AppEvent::OutputStatusUpdated(OutputKind::Stream, _, _)) =
                state.reconcile_observation(revision, *event)
            {
                let AppEvent::OutputStatusUpdated(_, status, observed_at) = &event else {
                    unreachable!()
                };
                assert!(status.active);
                assert!(*observed_at >= request_seen);
                state.apply_output_event(&event, 0);
                break;
            }
        }
    }
    mock.release_all().await;
    loop {
        if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
            if matches!(
                *event,
                AppEvent::OutputStatusUpdated(OutputKind::Stream, _, _)
            ) {
                let AppEvent::OutputStatusUpdated(_, _, observed_at) = &*event else {
                    unreachable!()
                };
                assert!(
                    *observed_at <= request_seen,
                    "readback time must precede its wait and publication"
                );
                assert!(state.reconcile_observation(revision, *event).is_none());
                break;
            }
        }
    }
    assert!(state.stream_status.active);
    assert_eq!(state.health.output_activity(OutputKind::Stream), Some(true));
    assert!(state.health.alerts(std::time::Instant::now()).is_empty());
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
            controller.session_scope().retire();
        }
        controller.handle(AppCommand::RefreshData);
        assert!(matches!(
            peel(recv_event(&rx).await),
            AppEvent::GraphRefreshRejected(_)
        ));
        controller.take_refresh_task().unwrap().await.unwrap();
    }
    controller.shutdown().await.unwrap();
}

#[tokio::test]
async fn doctor_refresh_is_one_coalesced_worker_even_at_optional_command_saturation() {
    let mut mock = StatefulObs::default()
        .hold(Request::GetSceneList, When::Nth(1))
        .connect()
        .await;
    let (tx, rx) = mpsc::sync_channel(128);
    let mut controller = AppController::with_dependencies(Handle::current(), tx, dependencies());
    assert!(controller.session_scope().install(mock.client.clone()));
    for _ in 0..64 {
        controller
            .session_scope()
            .spawn(&Handle::current(), std::future::pending())
            .unwrap();
    }
    assert!(controller
        .session_scope()
        .spawn(&Handle::current(), std::future::pending())
        .is_none());
    controller.handle(AppCommand::RefreshData);
    mock.wait(Request::GetSceneList, 1).await;
    for _ in 0..256 {
        controller.handle(AppCommand::RefreshData);
    }
    mock.release_all().await;
    graph_ready(&rx).await;
    graph_ready(&rx).await;
    tokio::time::timeout(TEST_DEADLINE, controller.take_refresh_task().unwrap())
        .await
        .unwrap()
        .unwrap();
    controller.shutdown().await.unwrap();
    assert_eq!(mock.stop().await.count(Request::GetSceneList), 2);
}
