//! Deterministic loopback regressions for the actual adapter/controllers/state.
use super::*;
use crate::controller::lifecycle::{OperationKind, SessionRetirement};
use crate::controller::session_controller::SessionRunner;
use crate::controller::state::{MixerVisibleAudioStatus, ObsStatus};
use crate::controller::test_support::deps::dependencies;
use crate::controller::test_support::events::{graph_ready, peel, recv_event, TEST_DEADLINE};
use crate::controller::test_support::loopback::{consume_echo_pair, echo_pair, pump};
use crate::controller::test_support::state::app_state;
use crate::controller::test_support::workspace::request;
use crate::domain::control::ControlAction;
use crate::domain::output::{OutputKind, OutputRunState, OutputStatus};
use crate::obs::event::ObsEventStream;
use crate::obs::testing::stateful::{Request, StatefulObs, When};
use futures_util::future::BoxFuture;
use std::collections::VecDeque;
use std::sync::mpsc;
use std::time::Duration;

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
