//! Connection identity, command cancellation, and client-slot ownership.

use std::collections::VecDeque;
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::runtime::Handle;
use tokio::sync::Notify;
use tokio::task::{AbortHandle, JoinHandle};

use crate::obs::client::ObsClient;

#[derive(Clone)]
pub(crate) struct SessionScope {
    generation: u64,
    owner: Arc<Mutex<u64>>,
    active: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    cancelled: Arc<Notify>,
    collection: Arc<AtomicU64>,
    graph_revision: Arc<AtomicU64>,
    scene_activation_revision: Arc<AtomicU64>,
    tasks: Arc<Mutex<Vec<AbortHandle>>>,
    slot: Arc<Mutex<Option<ObsClient>>>,
    evidence: Arc<Mutex<OperationJournal>>,
    terminal_reason: Arc<Mutex<Option<String>>>,
    terminals: Arc<TerminalQueue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OperationKind {
    Stream,
    Record,
    Workspace {
        epoch: u64,
        sequence: u64,
        result_id: Option<u64>,
    },
    Audio,
}

#[derive(Debug, Clone)]
pub(crate) struct OperationEvidence {
    pub id: u64,
    pub kind: OperationKind,
    pub endpoint: String,
    pub collection: u64,
    pub dispatched: bool,
    pub acknowledged: bool,
    pub uncertain: bool,
    pub completed: bool,
    pub failure: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct SessionRetirement {
    pub generation: u64,
    pub reason: String,
    pub operations: Vec<OperationEvidence>,
    pub omitted_sessions: u64,
}

#[derive(Default)]
struct OperationJournal {
    next: u64,
    entries: Vec<OperationEvidence>,
}

struct TerminalQueue {
    pending: Mutex<VecDeque<SessionRetirement>>,
    changed: tokio::sync::watch::Sender<u64>,
}

impl Default for TerminalQueue {
    fn default() -> Self {
        Self {
            pending: Mutex::new(VecDeque::new()),
            changed: tokio::sync::watch::channel(0).0,
        }
    }
}

pub(crate) struct OperationLease {
    id: u64,
    journal: Arc<Mutex<OperationJournal>>,
}

impl OperationLease {
    fn update(&self, update: impl FnOnce(&mut OperationEvidence)) {
        if let Some(entry) = self
            .journal
            .lock()
            .expect("operation evidence")
            .entries
            .iter_mut()
            .find(|entry| entry.id == self.id)
        {
            update(entry);
        }
    }
    pub(crate) fn dispatch(&self) {
        self.update(|entry| entry.dispatched = true);
    }
    pub(crate) fn acknowledge(&self) {
        self.update(|entry| entry.acknowledged = true);
    }
    pub(crate) fn unknown(&self, message: String) {
        self.update(|entry| {
            entry.uncertain = entry.dispatched && !entry.acknowledged;
            entry.completed = true;
            entry.failure = Some(message);
        });
    }
    pub(crate) fn finish(&self, result: Result<(), String>) {
        self.update(|entry| {
            entry.completed = true;
            entry.failure = result.err();
        });
    }
}

impl SessionScope {
    pub(crate) fn initial(slot: Arc<Mutex<Option<ObsClient>>>) -> Self {
        Self {
            generation: 0,
            owner: Arc::new(Mutex::new(0)),
            active: Arc::new(AtomicBool::new(true)),
            failed: Arc::new(AtomicBool::new(false)),
            cancelled: Arc::new(Notify::new()),
            collection: Arc::new(AtomicU64::new(0)),
            graph_revision: Arc::new(AtomicU64::new(0)),
            scene_activation_revision: Arc::new(AtomicU64::new(0)),
            tasks: Arc::new(Mutex::new(Vec::new())),
            slot,
            evidence: Arc::new(Mutex::new(OperationJournal::default())),
            terminal_reason: Arc::new(Mutex::new(None)),
            terminals: Arc::new(TerminalQueue::default()),
        }
    }

    pub(crate) fn next(&self) -> Self {
        let generation = {
            let mut owner = self.owner.lock().expect("session ownership");
            *owner = owner.checked_add(1).expect("session generation exhausted");
            *self.slot.lock().expect("OBS client slot") = None;
            *owner
        };
        self.retire();
        Self {
            generation,
            owner: self.owner.clone(),
            active: Arc::new(AtomicBool::new(true)),
            failed: Arc::new(AtomicBool::new(false)),
            cancelled: Arc::new(Notify::new()),
            collection: Arc::new(AtomicU64::new(0)),
            graph_revision: Arc::new(AtomicU64::new(0)),
            scene_activation_revision: Arc::new(AtomicU64::new(0)),
            tasks: Arc::new(Mutex::new(Vec::new())),
            slot: self.slot.clone(),
            evidence: Arc::new(Mutex::new(OperationJournal::default())),
            terminal_reason: Arc::new(Mutex::new(None)),
            terminals: self.terminals.clone(),
        }
    }

    pub(crate) fn mark_failed(&self) {
        self.failed.store(true, Ordering::Release);
    }

    pub(crate) fn failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn collection(&self) -> u64 {
        self.collection.load(Ordering::Acquire)
    }

    pub(crate) fn advance_collection(&self) {
        self.collection.fetch_add(1, Ordering::AcqRel);
        self.invalidate_scene_activation();
    }

    pub(crate) fn scene_activation_revision(&self) -> u64 {
        self.scene_activation_revision.load(Ordering::Acquire)
    }

    pub(crate) fn invalidate_scene_activation(&self) {
        self.scene_activation_revision
            .fetch_add(1, Ordering::AcqRel);
    }

    pub(crate) fn graph_revision(&self) -> u64 {
        self.graph_revision.load(Ordering::Acquire)
    }

    pub(crate) fn observe_graph_revision(&self, revision: u64) {
        // Tokens come from the connection's observation clock. fetch_max keeps
        // concurrently starting manual/background scans ordered by that clock.
        self.graph_revision.fetch_max(revision, Ordering::AcqRel);
    }

    pub(crate) fn owns_generation(&self, generation: u64) -> bool {
        *self.owner.lock().expect("session ownership") == generation
    }

    pub(crate) fn is_current(&self) -> bool {
        self.owns_generation(self.generation)
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire) && self.is_current()
    }

    pub(crate) fn install(&self, client: ObsClient) -> bool {
        let owner = self.owner.lock().expect("session ownership");
        if *owner != self.generation || !self.active.load(Ordering::Acquire) {
            return false;
        }
        *self.slot.lock().expect("OBS client slot") = Some(client);
        true
    }

    pub(crate) fn retire(&self) {
        if self.active.swap(false, Ordering::AcqRel) {
            self.finish_evidence();
        }
        self.cancelled.notify_waiters();
        for task in self.tasks.lock().expect("session tasks").iter() {
            task.abort();
        }
        let owner = self.owner.lock().expect("session ownership");
        if *owner == self.generation {
            *self.slot.lock().expect("OBS client slot") = None;
        }
    }

    pub(crate) fn set_terminal_reason(&self, reason: String) {
        self.terminal_reason
            .lock()
            .expect("session terminal reason")
            .get_or_insert(reason);
    }

    pub(crate) fn has_terminal_reason(&self) -> bool {
        self.terminal_reason
            .lock()
            .expect("session terminal reason")
            .is_some()
    }

    pub(crate) fn operation(
        &self,
        kind: OperationKind,
        endpoint: String,
    ) -> Option<OperationLease> {
        let mut journal = self.evidence.lock().expect("operation evidence");
        if !self.is_active() {
            return None;
        }
        if journal.entries.len() == 64 {
            let completed = journal.entries.iter().position(|entry| entry.completed)?;
            journal.entries.remove(completed);
        }
        journal.next = journal
            .next
            .checked_add(1)
            .expect("operation IDs exhausted");
        let id = journal.next;
        journal.entries.push(OperationEvidence {
            id,
            kind,
            endpoint,
            collection: self.collection(),
            dispatched: false,
            acknowledged: false,
            uncertain: false,
            completed: false,
            failure: None,
        });
        Some(OperationLease {
            id,
            journal: self.evidence.clone(),
        })
    }

    fn finish_evidence(&self) {
        let reason = self
            .terminal_reason
            .lock()
            .expect("session terminal reason")
            .clone();
        let mut journal = self.evidence.lock().expect("operation evidence");
        if journal.entries.is_empty() && reason.is_none() {
            return;
        }
        for entry in &mut journal.entries {
            if !entry.completed {
                entry.uncertain = entry.dispatched && !entry.acknowledged;
                entry.failure = Some(if !entry.dispatched {
                    "Cancelled before dispatch; command was not sent"
                } else if entry.acknowledged {
                    "OBS acknowledged the command, but readback is unavailable; command was not retried"
                } else {
                    "OBS command outcome unknown; command was not retried"
                }.into());
                entry.completed = true;
            }
        }
        let terminal = SessionRetirement {
            generation: self.generation,
            reason: reason.unwrap_or_else(|| "OBS session retired".into()),
            operations: std::mem::take(&mut journal.entries),
            omitted_sessions: 0,
        };
        let mut queued = self
            .terminals
            .pending
            .lock()
            .expect("terminal evidence queue");
        let mut terminal = terminal;
        if queued.len() == 32 {
            if let Some(discarded) = queued.pop_front() {
                terminal.omitted_sessions = discarded.omitted_sessions + 1;
            }
        }
        queued.push_back(terminal);
        self.terminals
            .changed
            .send_modify(|revision| *revision += 1);
    }

    /// One publisher survives reconnects; terminal audit is not a reusable
    /// collection snapshot and must not wait on a cancelled session task.
    pub(crate) async fn publish_retirements(&self, tx: super::event_sender::EventSender) {
        let mut changed = self.terminals.changed.subscribe();
        loop {
            let _ = *changed.borrow_and_update();
            loop {
                let terminal = self
                    .terminals
                    .pending
                    .lock()
                    .expect("terminal evidence queue")
                    .pop_front();
                let Some(terminal) = terminal else { break };
                if tx
                    .send(super::event::AppEvent::SessionRetired(terminal))
                    .await
                    .is_err()
                {
                    return;
                }
            }
            if changed.changed().await.is_err() {
                return;
            }
        }
    }

    pub(crate) async fn cancelled(&self) {
        loop {
            let notified = self.cancelled.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if !self.is_active() {
                return;
            }
            notified.await;
        }
    }

    pub(crate) async fn quiesce(&self) {
        while self
            .tasks
            .lock()
            .expect("session tasks")
            .iter()
            .any(|task| !task.is_finished())
        {
            tokio::task::yield_now().await;
        }
    }

    pub(crate) fn spawn<F>(&self, runtime: &Handle, future: F) -> Option<JoinHandle<()>>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        {
            let mut tasks = self.tasks.lock().expect("session tasks");
            tasks.retain(|task| !task.is_finished());
            if tasks.len() >= 64 {
                tracing::warn!("session command admission refused (busy)");
                return None;
            }
        }
        self.spawn_reserved(runtime, future)
    }

    /// Reserved for controllers that already own a bounded single worker or
    /// operation guard. Saturated optional commands must not strand UI loading.
    pub(crate) fn spawn_reserved<F>(&self, runtime: &Handle, future: F) -> Option<JoinHandle<()>>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let mut tasks = self.tasks.lock().expect("session tasks");
        tasks.retain(|task| !task.is_finished());
        if !self.is_active() {
            return None;
        }
        let scope = self.clone();
        let task = runtime.spawn(async move {
            tokio::select! {
                biased;
                () = scope.cancelled() => {}
                () = future => {}
            }
        });
        tasks.push(task.abort_handle());
        Some(task)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retirement_audit_is_bounded_and_reports_omitted_sessions() {
        let mut scope = SessionScope::initial(Arc::new(Mutex::new(None)));
        let sessions = 100;
        for _ in 0..sessions {
            scope.set_terminal_reason("fixture retirement".into());
            scope.retire();
            scope = scope.next();
        }
        let pending = scope.terminals.pending.lock().unwrap();
        assert_eq!(pending.len(), 32);
        assert_eq!(
            pending
                .iter()
                .map(|terminal| terminal.omitted_sessions + 1)
                .sum::<u64>(),
            sessions
        );
    }

    #[tokio::test]
    async fn optional_command_saturation_does_not_starve_a_bounded_reserved_worker() {
        let scope = SessionScope::initial(Arc::new(Mutex::new(None)));
        for _ in 0..64 {
            assert!(scope
                .spawn(&Handle::current(), std::future::pending())
                .is_some());
        }
        assert!(scope
            .spawn(&Handle::current(), std::future::pending())
            .is_none());
        let (started, observed) = tokio::sync::oneshot::channel();
        assert!(scope
            .spawn_reserved(&Handle::current(), async move {
                let _ = started.send(());
                std::future::pending::<()>().await;
            })
            .is_some());
        tokio::time::timeout(std::time::Duration::from_secs(1), observed)
            .await
            .unwrap()
            .unwrap();
        scope.retire();
        scope.quiesce().await;
    }

    #[tokio::test]
    async fn retirement_cancels_old_work_and_cannot_retire_a_new_generation() {
        let old = SessionScope::initial(Arc::new(Mutex::new(None)));
        let (release, blocked) = tokio::sync::oneshot::channel();
        let wrote = Arc::new(AtomicBool::new(false));
        let observed = wrote.clone();
        let task = old
            .spawn(&Handle::current(), async move {
                let _ = blocked.await;
                observed.store(true, Ordering::Release);
            })
            .unwrap();
        let current = old.next();
        old.retire();
        let _ = release.send(());
        let _ = task.await;
        assert!(!wrote.load(Ordering::Acquire));
        assert!(current.is_active());
        assert!(!old.is_current());
    }
}
