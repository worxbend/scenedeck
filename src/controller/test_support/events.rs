//! Bounded waits for events and conditions in controller tests.
//!
//! `AppEvent`s cross a `std::sync::mpsc` channel, so an async test cannot
//! block on the receiver without stalling the runtime that produces them.
//! `recv_event` polls and yields instead.

use std::{
    sync::mpsc::{self, TryRecvError},
    time::{Duration, Instant},
};

use crate::controller::event::AppEvent;

/// How long one awaited event or condition may take. Only a failing test
/// waits this long.
pub(crate) const TEST_DEADLINE: Duration = Duration::from_secs(3);

/// The next event, yielding to the runtime between polls.
pub(crate) async fn recv_event(rx: &mpsc::Receiver<AppEvent>) -> AppEvent {
    let poll = async {
        loop {
            match rx.try_recv() {
                Ok(event) => return event,
                Err(TryRecvError::Empty) => tokio::task::yield_now().await,
                Err(TryRecvError::Disconnected) => panic!("every AppEvent sender was dropped"),
            }
        }
    };
    tokio::time::timeout(TEST_DEADLINE, poll)
        .await
        .unwrap_or_else(|_| panic!("no AppEvent within {TEST_DEADLINE:?}"))
}

/// The next event that `matches`, dropping the events before it. Each
/// received event gets its own `TEST_DEADLINE`.
pub(crate) async fn recv_matching(
    rx: &mpsc::Receiver<AppEvent>,
    matches: impl Fn(&AppEvent) -> bool,
) -> AppEvent {
    loop {
        let event = recv_event(rx).await;
        if matches(&event) {
            return event;
        }
    }
}

/// Polls `ready` until it holds or `TEST_DEADLINE` passes; true when ready.
/// It sleeps the calling thread, so only synchronous tests may call it.
pub(crate) fn wait_until(mut ready: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + TEST_DEADLINE;
    while !ready() {
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn recv_matching_skips_events_until_one_matches() {
        let (tx, rx) = mpsc::sync_channel(4);
        tx.send(AppEvent::GraphInvalidated).unwrap();
        tx.send(AppEvent::WorkspaceInvalidated).unwrap();

        let event =
            recv_matching(&rx, |event| matches!(event, AppEvent::WorkspaceInvalidated)).await;

        assert!(matches!(event, AppEvent::WorkspaceInvalidated));
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    #[should_panic(expected = "every AppEvent sender was dropped")]
    async fn recv_event_fails_at_once_when_no_sender_is_left() {
        let (tx, rx) = mpsc::sync_channel::<AppEvent>(1);
        drop(tx);

        recv_event(&rx).await;
    }
}
