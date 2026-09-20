//! Bridge blocking local I/O back to GTK without blocking the main loop.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc::{channel, sync_channel, Receiver, Sender, TryRecvError};
use std::sync::OnceLock;
use std::time::Duration;

type Job = Box<dyn FnOnce() + Send + 'static>;

/// A dedicated FIFO worker for writes that target the same persisted file.
///
/// Merely putting a mutex around independently spawned threads is not enough:
/// the operating system may let a later thread acquire it first. A channel
/// records submission order before the worker runs anything, so an older
/// snapshot cannot overwrite a newer one.
struct SerialQueue {
    sender: Sender<Job>,
}

impl SerialQueue {
    fn new(thread_name: &'static str) -> Self {
        let (sender, receiver) = channel::<Job>();
        std::thread::Builder::new()
            .name(thread_name.to_string())
            .spawn(move || {
                while let Ok(job) = receiver.recv() {
                    // One bad job must not permanently disable persistence for
                    // this file. The job owns its one-shot result sender, so
                    // unwinding also tells the GTK poller to stop waiting.
                    if catch_unwind(AssertUnwindSafe(job)).is_err() {
                        tracing::error!(thread_name, "serialized background I/O job panicked");
                    }
                }
            })
            .expect("failed to start serialized background I/O worker");
        Self { sender }
    }

    fn enqueue(&self, job: Job) {
        if self.sender.send(job).is_err() {
            tracing::error!("serialized background I/O worker is unavailable");
        }
    }
}

/// Persistence lanes correspond to files whose snapshots must reach disk in
/// the same order in which GTK callbacks submitted them.
#[derive(Clone, Copy)]
pub(crate) enum SerialLane {
    Config,
    Registry,
}

fn serial_queue(lane: SerialLane) -> &'static SerialQueue {
    static CONFIG: OnceLock<SerialQueue> = OnceLock::new();
    static REGISTRY: OnceLock<SerialQueue> = OnceLock::new();

    match lane {
        SerialLane::Config => CONFIG.get_or_init(|| SerialQueue::new("scenedeck-config-writer")),
        SerialLane::Registry => {
            REGISTRY.get_or_init(|| SerialQueue::new("scenedeck-registry-writer"))
        }
    }
}

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

    deliver_on_main_thread(receiver, complete);
}

/// Run `work` after every previously submitted job for `lane`, without
/// blocking GTK, then invoke `complete` on GTK's main thread.
///
/// Use this for whole-file writes. A newer snapshot can be submitted while an
/// older one is still writing, but the lane's FIFO worker guarantees the newer
/// snapshot reaches disk last.
pub(crate) fn run_serialized<T, Work, Complete>(lane: SerialLane, work: Work, complete: Complete)
where
    T: Send + 'static,
    Work: FnOnce() -> T + Send + 'static,
    Complete: FnOnce(T) + 'static,
{
    let (sender, receiver) = sync_channel(1);
    serial_queue(lane).enqueue(Box::new(move || {
        let _ = sender.send(work());
    }));

    deliver_on_main_thread(receiver, complete);
}

fn deliver_on_main_thread<T, Complete>(receiver: Receiver<T>, complete: Complete)
where
    T: Send + 'static,
    Complete: FnOnce(T) + 'static,
{
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
    use std::time::Duration;

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

    #[test]
    fn serialized_jobs_do_not_overtake_a_blocked_predecessor() {
        let queue = super::SerialQueue::new("scenedeck-test-serialized-io");
        let (first_started_tx, first_started_rx) = std::sync::mpsc::channel();
        let (release_first_tx, release_first_rx) = std::sync::mpsc::channel();
        let (finished_tx, finished_rx) = std::sync::mpsc::channel();

        queue.enqueue(Box::new({
            let finished_tx = finished_tx.clone();
            move || {
                first_started_tx.send(()).expect("report first start");
                release_first_rx.recv().expect("release first job");
                finished_tx.send("first").expect("report first finish");
            }
        }));
        first_started_rx.recv().expect("first job started");

        queue.enqueue(Box::new(move || {
            finished_tx.send("second").expect("report second finish");
        }));

        assert_eq!(
            finished_rx.recv_timeout(Duration::from_millis(50)),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout),
            "the second job ran while its predecessor was still blocked"
        );

        release_first_tx.send(()).expect("release first job");
        assert_eq!(finished_rx.recv().expect("first result"), "first");
        assert_eq!(finished_rx.recv().expect("second result"), "second");
    }

    #[test]
    fn latest_serialized_file_snapshot_wins() {
        let queue = super::SerialQueue::new("scenedeck-test-latest-snapshot");
        let dir = std::env::temp_dir().join(format!(
            "scenedeck-background-io-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("config.json");
        let (first_started_tx, first_started_rx) = std::sync::mpsc::channel();
        let (release_first_tx, release_first_rx) = std::sync::mpsc::channel();
        let (finished_tx, finished_rx) = std::sync::mpsc::channel();

        queue.enqueue(Box::new({
            let path = path.clone();
            let finished_tx = finished_tx.clone();
            move || {
                first_started_tx.send(()).expect("report first start");
                release_first_rx.recv().expect("release first snapshot");
                crate::storage::atomic::write(&path, b"older").expect("write older snapshot");
                finished_tx.send(()).expect("report first finish");
            }
        }));
        first_started_rx.recv().expect("first snapshot started");

        queue.enqueue(Box::new({
            let path = path.clone();
            move || {
                crate::storage::atomic::write(&path, b"newer").expect("write newer snapshot");
                finished_tx.send(()).expect("report second finish");
            }
        }));

        release_first_tx.send(()).expect("release first snapshot");
        finished_rx.recv().expect("first snapshot finished");
        finished_rx.recv().expect("second snapshot finished");

        assert_eq!(std::fs::read(&path).expect("read final snapshot"), b"newer");
        std::fs::remove_dir_all(dir).expect("remove temp dir");
    }
}
