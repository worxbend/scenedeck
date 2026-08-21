//! Bridge blocking local I/O back to GTK without blocking the main loop.

use std::sync::mpsc::{sync_channel, TryRecvError};
use std::time::Duration;

/// Run `work` on a dedicated worker thread and invoke `complete` on GTK's main
/// thread. GTK objects may safely be captured by `complete` because it never
/// crosses the thread boundary.
///
/// The result travels over a one-shot channel rather than a shared slot so
/// that a worker which panics is detectable. When the worker thread unwinds it
/// drops its sender, the receiver reports `Disconnected`, and the polling
/// timer stops. A shared `Option` slot cannot distinguish "not finished yet"
/// from "will never finish", so a panicking worker left the timer running for
/// the lifetime of the process, holding `complete` and every GTK widget it had
/// captured. That is reachable code: `work` performs keyring access, reads
/// user-supplied CSS, and parses user-supplied YAML.
pub(crate) fn run<T, Work, Complete>(work: Work, complete: Complete)
where
    T: Send + 'static,
    Work: FnOnce() -> T + Send + 'static,
    Complete: FnOnce(T) + 'static,
{
    let (sender, receiver) = sync_channel(1);
    std::thread::spawn(move || {
        // The receiver is dropped once the timer has taken the value, so a
        // send failure only means nobody is listening any more.
        let _ = sender.send(work());
    });

    // `timeout_add_local` wants an `FnMut`, but `complete` may only be called
    // once, so it is moved out of an `Option` on the tick that delivers.
    let mut complete = Some(complete);
    glib::timeout_add_local(Duration::from_millis(20), move || {
        match receiver.try_recv() {
            Ok(value) => {
                if let Some(complete) = complete.take() {
                    complete(value);
                }
                glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
            // The worker ended without sending: it panicked. Stop polling
            // rather than spinning forever over a result that is never coming.
            Err(TryRecvError::Disconnected) => {
                tracing::error!("background I/O worker ended without a result");
                glib::ControlFlow::Break
            }
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn work_runs_off_the_calling_thread() {
        let caller = std::thread::current().id();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            tx.send(std::thread::current().id()).expect("worker id");
        });
        assert_ne!(rx.recv().expect("worker id"), caller);
    }

    /// A worker that panics must be observable, or the GTK timer polls forever.
    ///
    /// `run` cannot be called here — it needs a GTK main loop — so this pins
    /// the channel behaviour `run` relies on instead: a worker that unwinds
    /// drops its sender, and the receiver reports `Disconnected` rather than
    /// staying `Empty`.
    #[test]
    fn a_panicking_worker_disconnects_rather_than_staying_silent() {
        let (sender, receiver) = std::sync::mpsc::sync_channel::<()>(1);
        let worker = std::thread::spawn(move || {
            let _sender = sender;
            panic!("worker failed");
        });
        assert!(worker.join().is_err(), "the worker should have panicked");

        assert!(matches!(
            receiver.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Disconnected)
        ));
    }

    /// A value sent before a panic still arrives.
    ///
    /// This is what keeps the happy path identical to the shared-slot version
    /// it replaced: `try_recv` yields buffered data before it reports that the
    /// sender is gone, so a worker that panics *after* sending still delivers.
    #[test]
    fn a_value_sent_before_a_panic_is_still_delivered() {
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || {
            sender.send("done").expect("send");
            panic!("worker failed after sending");
        });
        assert!(worker.join().is_err(), "the worker should have panicked");

        assert_eq!(receiver.try_recv(), Ok("done"));
    }
}
