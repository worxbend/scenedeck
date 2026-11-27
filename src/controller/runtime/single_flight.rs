//! One worker per queue, draining only the newest job.
//!
//! A caller offers a job on the GTK thread. The first offer hands back a
//! [`Worker`]; later offers replace the job that worker has not taken yet.
//! The queue counts as running exactly as long as that worker lives, so a
//! worker that is never spawned, or that session retirement aborts mid-job,
//! cannot leave later offers waiting on a task that no longer exists. After a
//! refused spawn, `take_pending` hands the job back so its owner can publish
//! a failure.
//!
//! The lock guards whole-value assignments and is never held across an
//! await, so it uses [`lock_or_recover`].

use std::sync::{Arc, Mutex};

use crate::infra::sync::lock_or_recover;

/// What an offer did to the queue.
pub(crate) enum Admission<J> {
    /// No worker was running. Move this worker into the task that drains
    /// the queue; dropping it, spawned or not, ends it.
    Started(Worker<J>),
    /// A worker is running and will take this job next.
    Coalesced,
}

pub(crate) struct SingleFlight<J> {
    slot: Arc<Mutex<Slot<J>>>,
}

struct Slot<J> {
    running: bool,
    pending: Option<J>,
}

impl<J> SingleFlight<J> {
    pub(crate) fn new() -> Self {
        Self {
            slot: Arc::new(Mutex::new(Slot {
                running: false,
                pending: None,
            })),
        }
    }

    /// Queue `job` in place of any job the worker has not taken.
    pub(crate) fn offer(&self, job: J) -> Admission<J> {
        let mut slot = lock_or_recover(self.slot.as_ref());
        slot.pending = Some(job);
        if slot.running {
            return Admission::Coalesced;
        }
        slot.running = true;
        Admission::Started(Worker {
            slot: Arc::clone(&self.slot),
            finished: false,
        })
    }

    /// Take the job no worker took, such as the one whose worker a retired
    /// scope refused to spawn.
    pub(crate) fn take_pending(&self) -> Option<J> {
        lock_or_recover(self.slot.as_ref()).pending.take()
    }

    /// Drop the queued job. A running worker keeps its current job.
    pub(crate) fn discard_pending(&self) {
        lock_or_recover(self.slot.as_ref()).pending = None;
    }
}

/// The queue's only worker, alive from the offer that started it until it
/// is dropped or its queue runs empty.
pub(crate) struct Worker<J> {
    slot: Arc<Mutex<Slot<J>>>,
    finished: bool,
}

impl<J> Worker<J> {
    /// The next job. `None` also ends the worker, under the same lock, so a
    /// concurrent offer starts a new one.
    pub(crate) fn next(&mut self) -> Option<J> {
        if self.finished {
            return None;
        }
        let mut slot = lock_or_recover(self.slot.as_ref());
        let job = slot.pending.take();
        if job.is_none() {
            slot.running = false;
            self.finished = true;
        }
        job
    }
}

impl<J> Drop for Worker<J> {
    fn drop(&mut self) {
        // A finished worker may already have a successor; leave its flag.
        if !self.finished {
            lock_or_recover(self.slot.as_ref()).running = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::lifecycle::SessionScope;
    use tokio::runtime::Handle;

    fn start<J>(queue: &SingleFlight<J>, job: J) -> Worker<J> {
        match queue.offer(job) {
            Admission::Started(worker) => worker,
            Admission::Coalesced => panic!("expected the offer to start a worker"),
        }
    }

    fn coalesces<J>(queue: &SingleFlight<J>, job: J) -> bool {
        matches!(queue.offer(job), Admission::Coalesced)
    }

    #[test]
    fn the_first_offer_starts_a_worker() {
        let queue = SingleFlight::new();

        let mut worker = start(&queue, 1);

        assert_eq!(worker.next(), Some(1));
    }

    #[test]
    fn an_offer_while_running_replaces_the_untaken_job() {
        let queue = SingleFlight::new();
        let mut worker = start(&queue, 1);

        assert!(coalesces(&queue, 2));
        assert!(coalesces(&queue, 3));

        assert_eq!(worker.next(), Some(3));
    }

    #[test]
    fn an_offer_after_the_worker_took_its_job_still_coalesces() {
        let queue = SingleFlight::new();
        let mut worker = start(&queue, 1);
        assert_eq!(worker.next(), Some(1));

        assert!(coalesces(&queue, 2));
        assert_eq!(worker.next(), Some(2));
    }

    #[test]
    fn an_empty_next_ends_the_worker_so_the_next_offer_starts_one() {
        let queue = SingleFlight::new();
        let mut worker = start(&queue, 1);
        assert_eq!(worker.next(), Some(1));

        assert_eq!(worker.next(), None);

        let mut successor = start(&queue, 2);
        assert_eq!(worker.next(), None, "a finished worker takes nothing");
        assert_eq!(successor.next(), Some(2));
    }

    #[test]
    fn dropping_a_finished_worker_leaves_its_successor_running() {
        let queue = SingleFlight::new();
        let mut worker = start(&queue, 1);
        assert_eq!(worker.next(), Some(1));
        assert_eq!(worker.next(), None);
        let _successor = start(&queue, 2);

        drop(worker);

        assert!(coalesces(&queue, 3));
    }

    #[test]
    fn an_unspawned_worker_ends_on_drop_and_hands_its_job_back() {
        let queue = SingleFlight::new();

        drop(start(&queue, 1));

        assert_eq!(queue.take_pending(), Some(1));
        assert_eq!(queue.take_pending(), None);
        drop(start(&queue, 2));
    }

    #[test]
    fn a_worker_dropped_mid_job_ends_so_the_next_offer_starts_one() {
        let queue = SingleFlight::new();
        let mut worker = start(&queue, 1);
        assert_eq!(worker.next(), Some(1));

        drop(worker);

        drop(start(&queue, 2));
    }

    #[test]
    fn discard_pending_keeps_the_worker_running() {
        let queue = SingleFlight::new();
        let mut worker = start(&queue, 1);

        queue.discard_pending();

        assert!(coalesces(&queue, 2));
        assert_eq!(worker.next(), Some(2));
    }

    #[tokio::test]
    async fn a_worker_the_scope_aborts_mid_job_releases_the_queue() {
        let scope = SessionScope::initial(Arc::new(Mutex::new(None)));
        let queue = SingleFlight::new();
        let mut worker = start(&queue, 1);
        let (took_job, job_taken) = tokio::sync::oneshot::channel();
        let task = scope
            .spawn_reserved(&Handle::current(), async move {
                worker.next();
                let _ = took_job.send(());
                std::future::pending::<()>().await;
            })
            .expect("an active scope spawns its reserved worker");
        job_taken.await.expect("the worker takes its job");

        scope.retire();

        // Abort or the scope's own cancellation branch: either drops the worker.
        let _ = task.await;
        drop(start(&queue, 2));
    }
}
