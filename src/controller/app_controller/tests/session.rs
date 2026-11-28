//! Connect, Disconnect and session retirement routed through
//! `AppController`: disconnect drains the refresh before shutting outputs
//! down, and a retired session's operation evidence survives a delayed or
//! full GTK queue and a reconnect.

use std::collections::VecDeque;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::future::BoxFuture;
use tokio::runtime::Handle;

use crate::controller::app_controller::AppController;
use crate::controller::command::AppCommand;
use crate::controller::event::AppEvent;
use crate::controller::event_sender::EventSender;
use crate::controller::lifecycle::{OperationKind, SessionRetirement, SessionScope};
use crate::controller::session_controller::SessionRunner;
use crate::controller::state::ObsStatus;
use crate::controller::test_support::deps::dependencies;
use crate::controller::test_support::events::{graph_ready, peel, recv_event, TEST_DEADLINE};
use crate::controller::test_support::state::app_state;
use crate::controller::test_support::workspace::request;
use crate::domain::control::ControlAction;
use crate::domain::output::{OutputKind, OutputRunState, OutputStatus};
use crate::infra::error::AppError;
use crate::obs::client::ObsClient;
use crate::obs::event::ObsEventStream;
use crate::obs::testing::stateful::{Request, StatefulObs, When};

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
