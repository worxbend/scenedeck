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
    Bookmarks,
    ReportExports,
    ReportHistory,
    Secret,
}

fn serial_queue(lane: SerialLane) -> &'static SerialQueue {
    static CONFIG: OnceLock<SerialQueue> = OnceLock::new();
    static REGISTRY: OnceLock<SerialQueue> = OnceLock::new();
    static SECRET: OnceLock<SerialQueue> = OnceLock::new();
    static BOOKMARKS: OnceLock<SerialQueue> = OnceLock::new();
    static REPORT_EXPORTS: OnceLock<SerialQueue> = OnceLock::new();
    static REPORT_HISTORY: OnceLock<SerialQueue> = OnceLock::new();

    match lane {
        SerialLane::ReportHistory => {
            REPORT_HISTORY.get_or_init(|| SerialQueue::new("scenedeck-history-writer"))
        }
        SerialLane::ReportExports => {
            REPORT_EXPORTS.get_or_init(|| SerialQueue::new("scenedeck-report-exporter"))
        }
        SerialLane::Bookmarks => {
            BOOKMARKS.get_or_init(|| SerialQueue::new("scenedeck-bookmark-writer"))
        }
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
            let mut batch = crate::infra::sync::lock_or_recover(slot.as_ref());
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
                crate::infra::sync::lock_or_recover(slot.as_ref()).failed = true;
            }
            Err(_) => {
                tracing::error!("config snapshot worker panicked");
                crate::infra::sync::lock_or_recover(slot.as_ref()).failed = true;
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
            let mut batch = crate::infra::sync::lock_or_recover(slot.as_ref());
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
        let batch = crate::infra::sync::lock_or_recover(publication.as_ref());
        if batch.running {
            return glib::ControlFlow::Continue;
        }
        if batch.failed {
            let evidence = record_failure();
            CONFIG_SNAPSHOT_FAILURES.with(|failures| failures.borrow_mut().push(evidence));
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

/// Re-admit the current Config as a FIFO barrier after a failed snapshot.
/// Snapshots are whole files, so a successful retry resolves the snapshot
/// failures recorded before it; other failures stay. Returns false, and
/// admits nothing, when no snapshot failure is waiting for a retry.
pub(crate) fn retry_config_snapshot(
    work: impl FnOnce() -> std::io::Result<()> + Send + 'static,
) -> bool {
    let retried = CONFIG_SNAPSHOT_FAILURES.with(|failures| failures.borrow().clone());
    if retried.is_empty() {
        return false;
    }
    run_serialized(SerialLane::Config, work, move |result| match result {
        Ok(()) => {
            CONFIG_SNAPSHOT_FAILURES.with(|failures| {
                failures
                    .borrow_mut()
                    .retain(|kept| !retried.iter().any(|done| done.0 == kept.0))
            });
            retried.into_iter().for_each(resolve_failure);
        }
        // The unresolved tickets keep the close barrier failing.
        Err(error) => tracing::warn!(%error, "retrying the configuration save failed"),
    });
    true
}

thread_local! {
    static PENDING: Rc<Cell<usize>> = Rc::new(Cell::new(0));
    // A successful later save cannot establish that a failed operation persisted.
    static WORKER_FAILURES: Cell<u64> = const { Cell::new(0) };
    static UNRESOLVED_FAILURES: RefCell<std::collections::BTreeSet<u64>> = const { RefCell::new(std::collections::BTreeSet::new()) };
    static CONFIG_SNAPSHOT_FAILURES: RefCell<Vec<FailureEvidence>> = const { RefCell::new(Vec::new()) };
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
mod tests;
