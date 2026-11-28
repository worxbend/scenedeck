//! Graph and Mixer refresh commands: each request is still answered after
//! the session ended, a refusal never strands the queue, Doctor refreshes
//! coalesce into one worker, and the snapshots they read keep newer echoes
//! and report a failed readback.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use futures_util::future::BoxFuture;
use tokio::runtime::Handle;

use crate::controller::app_controller::AppController;
use crate::controller::command::AppCommand;
use crate::controller::event::AppEvent;
use crate::controller::event_sender::EventSender;
use crate::controller::lifecycle::SessionScope;
use crate::controller::session_controller::{SessionController, SessionRunner};
use crate::controller::state::MixerVisibleAudioStatus;
use crate::controller::test_support::deps::dependencies;
use crate::controller::test_support::events::{
    graph_ready, peel, recv_event, recv_matching, TEST_DEADLINE,
};
use crate::controller::test_support::loopback::{consume_echo_pair, echo_pair, pump};
use crate::controller::test_support::state::app_state;
use crate::domain::output::OutputKind;
use crate::obs::client::ObsClient;
use crate::obs::testing::stateful::{Request, StatefulObs, When};

struct EndedSessionRunner;

impl crate::controller::session_controller::SessionRunner for EndedSessionRunner {
    fn run(&self, _tx: EventSender, _scope: SessionScope) -> BoxFuture<'static, ()> {
        Box::pin(async {})
    }
}

/// A controller whose session runner already returned. Its scope is
/// retired while no Connect or Disconnect has rebuilt the queues.
async fn controller_after_session_ended() -> (AppController, mpsc::Receiver<AppEvent>) {
    let (tx, rx) = mpsc::sync_channel(16);
    let mut controller = AppController::new(Handle::current(), tx.clone());
    controller.session = SessionController::with_runner(
        Handle::current(),
        EventSender::new(tx),
        Arc::new(EndedSessionRunner),
        SessionScope::initial(controller.client_slot.clone()),
        Arc::new(Mutex::new(None)),
    );
    controller.sync_session_scope();
    controller.handle(AppCommand::Connect);
    recv_matching(&rx, |event| {
        matches!(event, AppEvent::SessionEvent { event, .. } if matches!(**event, AppEvent::Disconnected))
    })
    .await;
    (controller, rx)
}

#[tokio::test]
async fn graph_refresh_after_the_session_ended_is_rejected_every_time() {
    let (mut controller, rx) = controller_after_session_ended().await;

    for attempt in 1..=2 {
        controller.handle(AppCommand::RefreshData);

        let event = recv_event(&rx).await;
        assert!(
            matches!(&event, AppEvent::SessionEvent { event, .. } if matches!(**event, AppEvent::GraphRefreshRejected(_))),
            "attempt {attempt}: expected a rejected refresh, got {event:?}"
        );
    }
}

#[tokio::test]
async fn mixer_refresh_after_the_session_ended_fails_instead_of_loading_forever() {
    let (mut controller, rx) = controller_after_session_ended().await;
    let mut state = app_state();

    // The second request proves the first refusal did not strand the queue.
    for scene in ["Scene A", "Scene B"] {
        let request = state.set_mixer_audio_loading(scene.to_string());
        controller.handle(AppCommand::RefreshMixerSceneAudio(request.clone()));

        let event = recv_event(&rx).await;
        let AppEvent::MixerAudioInputsFailed {
            request: failed,
            message,
        } = event
        else {
            panic!("expected a mixer failure for {scene}, got {event:?}");
        };
        assert_eq!(failed, request);
        assert_eq!(message, "Not connected to OBS");
    }
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
