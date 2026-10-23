//! Bridge blocking local I/O back to GTK without blocking the main loop.

use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::mpsc::{channel, sync_channel, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::Duration;
use std::time::Instant;

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
    Secret,
}

fn serial_queue(lane: SerialLane) -> &'static SerialQueue {
    static CONFIG: OnceLock<SerialQueue> = OnceLock::new();
    static REGISTRY: OnceLock<SerialQueue> = OnceLock::new();
    static SECRET: OnceLock<SerialQueue> = OnceLock::new();

    match lane {
        SerialLane::Secret => SECRET.get_or_init(|| SerialQueue::new("scenedeck-secret-writer")),
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
    // Explicit jobs are FIFO barriers: a later snapshot cannot replace work
    // admitted before this transaction (including callback-chained writes).
    if matches!(lane, SerialLane::Config) {
        CONFIG_TAIL.with(|tail| *tail.borrow_mut() = Weak::new());
    }
    let (sender, receiver) = sync_channel(1);
    serial_queue(lane).enqueue(Box::new(move || {
        let _ = sender.send(work());
    }));

    deliver_on_main_thread(receiver, complete);
}

type ConfigWork = Box<dyn FnOnce() -> std::io::Result<()> + Send>;

struct ConfigBatch {
    latest: Option<ConfigWork>,
    running: bool,
    // Sticky until GTK publication: later successful snapshots are not an
    // explicit recovery decision for an earlier failed operation.
    failed: bool,
    #[cfg(test)]
    on_idle: Option<Job>,
}

type ConfigSlot = Mutex<ConfigBatch>;

thread_local! {
    // A FIFO barrier seals this tail. A segment owns at most one executing
    // write, one replaceable snapshot and one GTK publication source, including
    // when the worker finishes faster than GTK can poll its completions.
    static CONFIG_TAIL: RefCell<Weak<ConfigSlot>> = const { RefCell::new(Weak::new()) };
}

fn run_config_batch(slot: Arc<ConfigSlot>) {
    serial_queue(SerialLane::Config).enqueue(Box::new(move || loop {
        let work = {
            let mut batch = slot.lock().expect("config snapshot lock");
            let Some(work) = batch.latest.take() else {
                batch.running = false;
                #[cfg(test)]
                {
                    let on_idle = batch.on_idle.take();
                    drop(batch);
                    if let Some(on_idle) = on_idle {
                        on_idle();
                    }
                }
                break;
            };
            work
        };
        match catch_unwind(AssertUnwindSafe(work)) {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                tracing::warn!(%error, "failed to save configuration");
                slot.lock().expect("config snapshot lock").failed = true;
            }
            Err(_) => {
                tracing::error!("config snapshot worker panicked");
                slot.lock().expect("config snapshot lock").failed = true;
            }
        }
    }));
}

/// Admit a replaceable whole Config snapshot now, never via a debounce timer.
/// Explicit `run_serialized(Config, ..)` jobs remain non-coalescible FIFO barriers.
pub(crate) fn run_config_snapshot(work: impl FnOnce() -> std::io::Result<()> + Send + 'static) {
    let slot = CONFIG_TAIL.with(|tail| tail.borrow().upgrade());
    if let Some(slot) = slot {
        let start_worker = {
            let mut batch = slot.lock().expect("config snapshot lock");
            batch.latest = Some(Box::new(work));
            !std::mem::replace(&mut batch.running, true)
        };
        if start_worker {
            run_config_batch(slot);
        }
        return;
    }
    let slot = Arc::new(Mutex::new(ConfigBatch {
        latest: Some(Box::new(work)),
        running: true,
        failed: false,
        #[cfg(test)]
        on_idle: None,
    }));
    CONFIG_TAIL.with(|tail| *tail.borrow_mut() = Arc::downgrade(&slot));
    // Register before returning to the signal caller: Close/Quit must include
    // this batch even if its first worker has not started yet.
    let mut pending = Some(PendingJob::new());
    let publication = slot.clone();
    glib::timeout_add_local(Duration::from_millis(20), move || {
        let batch = publication.lock().expect("config snapshot lock");
        if batch.running {
            return glib::ControlFlow::Continue;
        }
        if batch.failed {
            record_failure();
        }
        drop(batch);
        // The worker can still own its Arc after marking this batch idle.
        // Retire admission with the publication ticket, not with Arc lifetime;
        // otherwise a later edit can revive a batch with no poller or ticket.
        // A FIFO barrier may already have installed a different tail.
        CONFIG_TAIL.with(|tail| {
            let mut tail = tail.borrow_mut();
            if tail.ptr_eq(&Arc::downgrade(&publication)) {
                *tail = Weak::new();
            }
        });
        pending.take();
        glib::ControlFlow::Break
    });
    run_config_batch(slot);
}

thread_local! {
    static PENDING: Rc<Cell<usize>> = Rc::new(Cell::new(0));
    // A successful later save cannot establish that a failed operation persisted.
    static WORKER_FAILURES: Cell<u64> = const { Cell::new(0) };
    static UNRESOLVED_FAILURES: RefCell<std::collections::BTreeSet<u64>> = const { RefCell::new(std::collections::BTreeSet::new()) };
}

#[derive(Clone, Copy)]
pub(crate) struct FailureEvidence(u64);

/// Record a returned persistence error as well as worker panics. A ticket
/// permits an explicit successful retry to resolve only its own failure.
pub(crate) fn record_failure() -> FailureEvidence {
    let id = WORKER_FAILURES.with(|failures| {
        let id = failures.get().saturating_add(1);
        failures.set(id);
        id
    });
    UNRESOLVED_FAILURES.with(|failures| failures.borrow_mut().insert(id));
    FailureEvidence(id)
}

/// Resolve one failure only after an explicit retry has confirmed its data.
pub(crate) fn resolve_failure(evidence: FailureEvidence) {
    UNRESOLVED_FAILURES.with(|failures| failures.borrow_mut().remove(&evidence.0));
}

/// Snapshot only the failures described by an informed close decision.
pub(crate) fn failure_evidence() -> FailureEvidence {
    FailureEvidence(WORKER_FAILURES.with(Cell::get))
}

/// Call only after the user accepts that failed operations may be lost.
/// Evidence stays recorded; new work must still drain, and failures newer
/// than the decision's snapshot require another informed decision.
pub(crate) fn acknowledge_failures(evidence: FailureEvidence) {
    UNRESOLVED_FAILURES.with(|failures| failures.borrow_mut().retain(|id| *id > evidence.0));
    tracing::warn!(
        failures = evidence.0,
        "user acknowledged potentially unsaved background operations"
    );
}

struct PendingJob(Rc<Cell<usize>>);

impl PendingJob {
    fn new() -> Self {
        PENDING.with(|pending| {
            pending.set(pending.get() + 1);
            Self(Rc::clone(pending))
        })
    }
}

impl Drop for PendingJob {
    fn drop(&mut self) {
        self.0.set(self.0.get() - 1);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlushError {
    TimedOut,
    WorkerFailed,
}

/// Call before quitting, while the GTK main context is still being iterated.
/// Keep the application/window alive and disable new edits until this callback.
/// Waiting includes GTK publication and writes chained by accepted reads (YAML
/// import). A timeout does not cancel work and is NOT permission to claim it
/// reached disk. Returned Config errors and worker panics block this barrier
/// until explicitly resolved or acknowledged by an informed close decision.
/// Other callback-based operations must record their own returned save errors.
pub(crate) fn flush_pending(
    timeout: Duration,
    complete: impl FnOnce(Result<(), FlushError>) + 'static,
) {
    let deadline = Instant::now() + timeout;
    let mut complete = Some(complete);
    glib::timeout_add_local(Duration::from_millis(20), move || {
        let outcome = if PENDING.with(|pending| pending.get() == 0) {
            Some(
                if UNRESOLVED_FAILURES.with(|failures| !failures.borrow().is_empty()) {
                    Err(FlushError::WorkerFailed)
                } else {
                    Ok(())
                },
            )
        } else if Instant::now() >= deadline {
            Some(Err(FlushError::TimedOut))
        } else {
            None
        };
        if let Some(outcome) = outcome {
            complete.take().expect("one-shot flush callback")(outcome);
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
}

fn deliver_on_main_thread<T, Complete>(receiver: Receiver<T>, complete: Complete)
where
    T: Send + 'static,
    Complete: FnOnce(T) + 'static,
{
    // `timeout_add_local` wants an `FnMut`, but `complete` may only be called
    // once, so it is moved out of an `Option` on the tick that delivers.
    let mut complete = Some(complete);
    let mut pending = Some(PendingJob::new());
    glib::timeout_add_local(Duration::from_millis(20), move || {
        match receiver.try_recv() {
            Ok(value) => {
                if let Some(complete) = complete.take() {
                    complete(value);
                }
                pending.take();
                glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
            // The worker ended without sending: it panicked. Stop polling
            // rather than spinning forever over a result that is never coming.
            Err(TryRecvError::Disconnected) => {
                tracing::error!("background I/O worker ended without a result");
                record_failure();
                pending.take();
                glib::ControlFlow::Break
            }
        }
    });
}

#[cfg(test)]
pub(crate) fn isolated_test_child(name: &str) -> bool {
    const CHILD: &str = "SCENEDECK_IO_TEST_CHILD";
    if std::env::var(CHILD).as_deref() == Ok(name) {
        return true;
    }
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--include-ignored", "--test-threads=1"])
        .env(CHILD, name)
        .spawn()
        .unwrap();
    // This bounds a hung child, not storage latency: parallel suites can spend
    // several seconds in durable fsync before the actual assertions run.
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "isolated test failed: {name}");
            return false;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("isolated I/O test timed out: {name}");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    fn drain(expected: Result<(), super::FlushError>) {
        let main_loop = glib::MainLoop::new(None, false);
        let outcome = std::rc::Rc::new(std::cell::Cell::new(None));
        super::flush_pending(Duration::from_secs(2), {
            let main_loop = main_loop.clone();
            let outcome = outcome.clone();
            move |result| {
                outcome.set(Some(result));
                main_loop.quit();
            }
        });
        main_loop.run();
        assert_eq!(outcome.get(), Some(expected));
    }

    #[test]
    fn retired_config_batch_cannot_admit_untracked_work_through_worker_arc() {
        if !super::isolated_test_child(
            "ui::background_io::tests::retired_config_batch_cannot_admit_untracked_work_through_worker_arc",
        ) {
            return;
        }
        let (release_first, first_held) = std::sync::mpsc::channel();
        super::run_config_snapshot(move || {
            first_held.recv().unwrap();
            Ok(())
        });
        let (idle, observed_idle) = std::sync::mpsc::channel();
        let (release_worker, worker_held) = std::sync::mpsc::channel();
        super::CONFIG_TAIL.with(|tail| {
            let slot = tail.borrow().upgrade().unwrap();
            slot.lock().unwrap().on_idle = Some(Box::new(move || {
                idle.send(()).unwrap();
                worker_held.recv().unwrap();
            }));
        });
        release_first.send(()).unwrap();
        observed_idle.recv_timeout(Duration::from_secs(2)).unwrap();
        // The actual batch has finished, but its actual worker still owns its
        // Arc. Let the real GTK publication retire the ticket in that window.
        drain(Ok(()));
        super::PENDING.with(|pending| assert_eq!(pending.get(), 0));

        let (started, observed_start) = std::sync::mpsc::channel();
        let (release_second, second_held) = std::sync::mpsc::channel();
        let completed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        super::run_config_snapshot({
            let completed = completed.clone();
            move || {
                started.send(()).unwrap();
                second_held.recv().unwrap();
                completed.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            }
        });
        super::PENDING.with(|pending| {
            assert_eq!(pending.get(), 1, "every admitted batch needs a live ticket")
        });
        release_worker.send(()).unwrap();
        observed_start.recv_timeout(Duration::from_secs(2)).unwrap();
        let main_loop = glib::MainLoop::new(None, false);
        super::flush_pending(Duration::from_millis(60), {
            let main_loop = main_loop.clone();
            move |result| {
                assert_eq!(result, Err(super::FlushError::TimedOut));
                main_loop.quit();
            }
        });
        main_loop.run();
        assert!(!completed.load(std::sync::atomic::Ordering::SeqCst));
        release_second.send(()).unwrap();
        drain(Ok(()));
        assert!(completed.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn config_publication_keeps_the_newer_tail_and_callback_barrier() {
        if !super::isolated_test_child(
            "ui::background_io::tests::config_publication_keeps_the_newer_tail_and_callback_barrier",
        ) {
            return;
        }
        super::run_config_snapshot(|| Ok(()));
        let (release, held) = std::sync::mpsc::channel();
        let (started, observed_start) = std::sync::mpsc::channel();
        let writes = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        super::run_serialized(
            super::SerialLane::Config,
            move || {
                started.send(()).unwrap();
                held.recv().unwrap();
            },
            {
                let writes = writes.clone();
                move |()| {
                    // Callback-chained transactions still fence the tail.
                    super::run_serialized(
                        super::SerialLane::Config,
                        {
                            let writes = writes.clone();
                            move || writes.lock().unwrap().push(2)
                        },
                        move |()| {
                            super::run_config_snapshot(move || {
                                writes.lock().unwrap().push(3);
                                Ok(())
                            });
                        },
                    );
                }
            },
        );
        observed_start.recv_timeout(Duration::from_secs(2)).unwrap();
        super::run_config_snapshot({
            let writes = writes.clone();
            move || {
                writes.lock().unwrap().push(0);
                Ok(())
            }
        });
        // Retire only the completed first batch, while the newer tail is
        // queued behind the blocked transaction and still owns its ticket.
        let context = glib::MainContext::default();
        while super::PENDING.with(|pending| pending.get() == 3) {
            context.iteration(true);
        }
        for _ in 0..128 {
            let writes = writes.clone();
            super::run_config_snapshot(move || {
                writes.lock().unwrap().push(1);
                Ok(())
            });
        }
        super::PENDING.with(|pending| assert_eq!(pending.get(), 2));
        release.send(()).unwrap();
        drain(Ok(()));
        assert_eq!(*writes.lock().unwrap(), [1, 2, 3]);
    }

    #[test]
    fn config_snapshot_error_requires_an_explicit_close_decision() {
        if !super::isolated_test_child(
            "ui::background_io::tests::config_snapshot_error_requires_an_explicit_close_decision",
        ) {
            return;
        }
        super::run_config_snapshot(|| Err(std::io::Error::other("final snapshot failed")));
        drain(Err(super::FlushError::WorkerFailed));
        let decision = super::failure_evidence();

        // Neither a later Config save nor an unrelated lane proves the failed
        // operation was recovered. Cancel/rechecking the drain keeps evidence.
        super::run_config_snapshot(|| Ok(()));
        super::run_serialized(super::SerialLane::Registry, || (), |()| {});
        drain(Err(super::FlushError::WorkerFailed));
        super::run_config_snapshot(|| Err(std::io::Error::other("newer failure")));
        drain(Err(super::FlushError::WorkerFailed));
        super::acknowledge_failures(decision);
        drain(Err(super::FlushError::WorkerFailed));
        super::acknowledge_failures(super::failure_evidence());
        drain(Ok(()));
        super::WORKER_FAILURES.with(|failures| assert_eq!(failures.get(), 2));
    }

    #[test]
    fn config_snapshot_publication_is_bounded_when_gtk_has_not_polled() {
        if !super::isolated_test_child(
            "ui::background_io::tests::config_snapshot_publication_is_bounded_when_gtk_has_not_polled",
        ) { return; }
        let (written, observed) = std::sync::mpsc::channel();
        for _ in 0..128 {
            let written = written.clone();
            super::run_config_snapshot(move || {
                written.send(()).unwrap();
                Ok(())
            });
            observed.recv_timeout(Duration::from_secs(2)).unwrap();
        }
        // A fast writer must not turn a busy main context into an unbounded
        // collection of already-complete timers and completion closures.
        super::PENDING.with(|pending| assert_eq!(pending.get(), 1));
        let main_loop = glib::MainLoop::new(None, false);
        super::flush_pending(Duration::from_secs(2), {
            let main_loop = main_loop.clone();
            move |result| {
                assert_eq!(result, Ok(()));
                main_loop.quit();
            }
        });
        main_loop.run();
    }

    #[test]
    fn config_snapshots_are_bounded_and_latest_wins_between_fifo_barriers() {
        if !super::isolated_test_child(
            "ui::background_io::tests::config_snapshots_are_bounded_and_latest_wins_between_fifo_barriers",
        ) {
            return;
        }
        let (release, held) = std::sync::mpsc::channel();
        super::run_serialized(
            super::SerialLane::Config,
            move || held.recv().unwrap(),
            |()| {},
        );
        let writes = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        for value in 0..1000 {
            let writes = writes.clone();
            super::run_config_snapshot(move || {
                writes.lock().unwrap().push(value);
                Ok(())
            });
        }
        super::PENDING.with(|pending| assert_eq!(pending.get(), 2));
        super::run_serialized(
            super::SerialLane::Config,
            {
                let writes = writes.clone();
                move || writes.lock().unwrap().push(1000)
            },
            |()| {},
        );
        for value in 1001..2000 {
            let writes = writes.clone();
            super::run_config_snapshot(move || {
                writes.lock().unwrap().push(value);
                Ok(())
            });
        }
        // One held writer, two latest snapshots separated by a transactional barrier.
        super::PENDING.with(|pending| assert_eq!(pending.get(), 4));
        let main_loop = glib::MainLoop::new(None, false);
        super::flush_pending(Duration::from_millis(60), {
            let main_loop = main_loop.clone();
            move |result| {
                assert_eq!(result, Err(super::FlushError::TimedOut));
                main_loop.quit();
            }
        });
        main_loop.run();
        assert!(writes.lock().unwrap().is_empty());
        release.send(()).unwrap();
        super::flush_pending(Duration::from_secs(2), {
            let main_loop = main_loop.clone();
            move |result| {
                assert_eq!(result, Ok(()));
                main_loop.quit();
            }
        });
        main_loop.run();
        assert_eq!(*writes.lock().unwrap(), [999, 1000, 1999]);
        super::PENDING.with(|pending| assert_eq!(pending.get(), 0));
    }

    #[test]
    fn failure_acknowledgement_preserves_evidence_and_does_not_hide_new_failures() {
        if !super::isolated_test_child("ui::background_io::tests::failure_acknowledgement_preserves_evidence_and_does_not_hide_new_failures") { return; }
        fn drain(expected: Result<(), super::FlushError>) {
            let main_loop = glib::MainLoop::new(None, false);
            super::flush_pending(Duration::from_secs(2), {
                let main_loop = main_loop.clone();
                move |outcome| {
                    assert_eq!(outcome, expected);
                    main_loop.quit();
                }
            });
            main_loop.run();
        }
        super::run_serialized(
            super::SerialLane::Config,
            || panic!("first failed save"),
            |(): ()| {},
        );
        drain(Err(super::FlushError::WorkerFailed));
        let decision = super::failure_evidence();
        super::run_serialized(super::SerialLane::Config, || (), |()| {});
        drain(Err(super::FlushError::WorkerFailed));
        super::run_serialized(
            super::SerialLane::Config,
            || panic!("later failed save"),
            |(): ()| {},
        );
        drain(Err(super::FlushError::WorkerFailed));
        super::acknowledge_failures(decision);
        drain(Err(super::FlushError::WorkerFailed));
        super::acknowledge_failures(super::failure_evidence());
        drain(Ok(()));
        super::WORKER_FAILURES.with(|failures| {
            assert_eq!(failures.get(), 2, "acknowledgement must preserve evidence")
        });
    }

    #[test]
    fn flush_waits_for_workers_gtk_publication_and_chained_writes() {
        if !super::isolated_test_child(
            "ui::background_io::tests::flush_waits_for_workers_gtk_publication_and_chained_writes",
        ) {
            return;
        }
        let main_loop = glib::MainLoop::new(None, false);
        let published = std::rc::Rc::new(std::cell::Cell::new(false));
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        super::run(move || release_rx.recv().unwrap(), {
            let published = published.clone();
            move |()| {
                super::run_serialized(
                    super::SerialLane::Config,
                    || (),
                    move |()| published.set(true),
                )
            }
        });
        super::flush_pending(Duration::from_millis(40), {
            let main_loop = main_loop.clone();
            move |result| {
                assert_eq!(result, Err(super::FlushError::TimedOut));
                main_loop.quit();
            }
        });
        main_loop.run();
        assert!(!published.get());
        release_tx.send(()).unwrap();
        super::flush_pending(Duration::from_secs(2), {
            let main_loop = main_loop.clone();
            let published = published.clone();
            move |result| {
                assert_eq!(result, Ok(()));
                assert!(published.get());
                main_loop.quit();
            }
        });
        main_loop.run();
    }

    #[test]
    fn flush_reports_panicked_work_and_registry_lane_survives() {
        if !super::isolated_test_child(
            "ui::background_io::tests::flush_reports_panicked_work_and_registry_lane_survives",
        ) {
            return;
        }
        super::run_serialized(
            super::SerialLane::Registry,
            || panic!("injected failure"),
            |()| panic!("must not publish"),
        );
        let completed = std::rc::Rc::new(std::cell::Cell::new(false));
        super::run_serialized(super::SerialLane::Registry, || (), {
            let completed = completed.clone();
            move |()| completed.set(true)
        });
        let main_loop = glib::MainLoop::new(None, false);
        super::flush_pending(Duration::from_secs(2), {
            let main_loop = main_loop.clone();
            move |result| {
                assert_eq!(result, Err(super::FlushError::WorkerFailed));
                assert!(completed.get());
                main_loop.quit();
            }
        });
        main_loop.run();
    }

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
