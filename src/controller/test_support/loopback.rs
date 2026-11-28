//! The real event loop over a stateful loopback OBS, and the OBS echoes the
//! regressions push through it.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use crate::controller::event::AppEvent;
use crate::controller::event_sender::EventSender;
use crate::controller::state::AppState;
use crate::controller::test_support::events::{peel, recv_event};
use crate::obs::testing::stateful::{Event, Scene, StatefulSession};

/// Runs `run_event_loop` over `mock`'s client and events, publishing to `tx`.
pub(crate) fn pump(
    mock: &mut StatefulSession,
    tx: mpsc::SyncSender<AppEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(crate::controller::refresh_controller::run_event_loop(
        mock.client.clone(),
        mock.take_events(),
        EventSender::new(tx),
        vec![],
        Arc::new(Mutex::new(None)),
    ))
}

/// Makes scene B the Program scene.
pub(crate) async fn switch_b(mock: &StatefulSession) {
    mock.emit(Event::ProgramChanged(Scene::B)).await;
}

/// Mutes `MicA` and sets it to -12 dB, as two transport echoes.
pub(crate) async fn echo_pair(mock: &StatefulSession) {
    mock.emit(Event::MicMuted(true)).await;
    mock.emit(Event::MicVolume {
        mul: 0.25,
        db: -12.0,
    })
    .await;
}

/// Applies both echoes of [`echo_pair`] to `state` through the reducers GTK
/// uses, dropping every other event.
pub(crate) async fn consume_echo_pair(rx: &mpsc::Receiver<AppEvent>, state: &mut AppState) {
    let mut echoes = 0;
    while echoes < 2 {
        if let AppEvent::Observation { revision, event } = peel(recv_event(rx).await) {
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
