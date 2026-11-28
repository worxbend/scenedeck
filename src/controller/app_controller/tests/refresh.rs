//! Graph and Mixer refreshes after the session ended: each request is still
//! answered, and a refusal never strands the queue.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use futures_util::future::BoxFuture;
use tokio::runtime::Handle;

use crate::controller::app_controller::AppController;
use crate::controller::command::AppCommand;
use crate::controller::event::AppEvent;
use crate::controller::event_sender::EventSender;
use crate::controller::lifecycle::SessionScope;
use crate::controller::session_controller::SessionController;
use crate::controller::test_support::events::{recv_event, recv_matching};
use crate::controller::test_support::state::app_state;

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
