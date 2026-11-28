//! Scene activation against a changing Studio mode: an obsolete click is
//! fenced at every gate it waits at, and never rerouted or replayed.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::runtime::Handle;

use super::WorkspaceController;
use crate::controller::event::AppEvent;
use crate::controller::event_sender::EventSender;
use crate::controller::lifecycle::SessionScope;
use crate::controller::test_support::events::{peel, recv_matching, TEST_DEADLINE};
use crate::controller::test_support::workspace::request;
use crate::domain::control::ControlAction;
use crate::obs::testing::stateful::{Event, Request, StatefulObs, When, SCENE_WRITES};

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
