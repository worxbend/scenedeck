//! Application-lifetime controller receiver; shell consumers are owned separately.

use crate::controller::event::AppEvent;
use adw::prelude::*;
use std::{cell::RefCell, rc::Rc, sync::mpsc, time::Duration};

/// Install once, when the first shell is built. The callback must not strongly
/// own shell widgets. Last-window removal and application shutdown both release
/// the receiver and its state/controller captures; neither changes Close/Quit
/// admission or the persistence/shutdown barriers in app.rs.
pub(super) fn install(
    app: &adw::Application,
    receiver: mpsc::Receiver<AppEvent>,
    apply: impl Fn(AppEvent) + 'static,
) {
    let id = glib::timeout_add_local(Duration::from_millis(50), move || {
        drain_batch(&receiver, &apply)
    });
    // Source::destroy is idempotent, including after the channel terminates.
    let source = Rc::new(RefCell::new(
        glib::MainContext::default().find_source_by_id(&id),
    ));
    app.connect_window_removed({
        let source = source.clone();
        move |app, _| {
            if app.windows().is_empty() {
                if let Some(source) = source.borrow_mut().take() {
                    source.destroy();
                }
            }
        }
    });
    app.connect_shutdown(move |_| {
        if let Some(source) = source.borrow_mut().take() {
            source.destroy();
        }
    });
}

fn drain_batch(
    receiver: &mpsc::Receiver<AppEvent>,
    mut apply: impl FnMut(AppEvent),
) -> glib::ControlFlow {
    for _ in 0..128 {
        match receiver.try_recv() {
            Ok(event) => apply(event),
            Err(mpsc::TryRecvError::Empty) => break,
            Err(mpsc::TryRecvError::Disconnected) => return glib::ControlFlow::Break,
        }
    }
    glib::ControlFlow::Continue
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drain_is_bounded_and_keeps_queued_event_order() {
        let (tx, rx) = mpsc::channel();
        for index in 0..129 {
            tx.send(AppEvent::ReplaySaved(index.to_string())).unwrap();
        }
        let received = RefCell::new(Vec::new());
        let mut apply = |event| match event {
            AppEvent::ReplaySaved(path) => received.borrow_mut().push(path),
            _ => panic!("unexpected event"),
        };
        assert_eq!(drain_batch(&rx, &mut apply), glib::ControlFlow::Continue);
        assert_eq!(received.borrow().len(), 128);
        drop(tx);
        assert_eq!(drain_batch(&rx, &mut apply), glib::ControlFlow::Break);
        assert_eq!(
            received.into_inner(),
            (0..129).map(|index| index.to_string()).collect::<Vec<_>>()
        );
    }
}
