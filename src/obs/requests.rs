//! Bounded request admission and cancellation containment for obws 0.15.
//!
//! obws retains an unanswered registration when its request future is dropped.
//! Retire admission on cancellation/timeout instead of reusing that socket and
//! accumulating abandoned registrations. Retirement never retries a mutation.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{watch, OwnedSemaphorePermit, Semaphore};

pub(super) const MAX_CONCURRENT_REQUESTS: usize = 8;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RetirementReason {
    Cancelled,
    Deadline,
}

impl RetirementReason {
    fn message(self) -> &'static str {
        match self {
            Self::Cancelled => "OBS request cancelled; outstanding outcomes/readback are unknown; commands were not retried",
            Self::Deadline => "OBS request deadline exceeded; outstanding outcomes/readback are unknown; commands were not retried",
        }
    }
}

pub(super) struct RequestGate {
    slots: Arc<Semaphore>,
    capacity: usize,
    drained: watch::Sender<u64>,
    shutdown_claimed: std::sync::atomic::AtomicBool,
    timeout: Option<Duration>,
    retired: watch::Sender<bool>,
    reason: std::sync::Mutex<Option<RetirementReason>>,
}

impl Default for RequestGate {
    fn default() -> Self {
        Self::new(MAX_CONCURRENT_REQUESTS, Some(REQUEST_TIMEOUT))
    }
}

impl RequestGate {
    pub(super) fn optional_image() -> Self {
        // Keep one unanswered optional registration until its reply or final
        // socket teardown. A UI deadline must not retire responsive controls.
        Self::new(1, None)
    }

    pub(super) fn shutdown() -> Self {
        Self::new(1, Some(REQUEST_TIMEOUT))
    }

    pub(super) fn close(&self) {
        self.slots.close();
    }

    pub(super) async fn drain_for_shutdown(&self) -> obws::error::Result<()> {
        use std::sync::atomic::Ordering;
        if !self.slots.is_closed() || self.shutdown_claimed.swap(true, Ordering::AcqRel) {
            return Err(obws::error::Error::Disconnected);
        }
        let mut changed = self.drained.subscribe();
        let mut retirement = self.retired.subscribe();
        loop {
            changed.borrow_and_update();
            if *retirement.borrow() {
                return Err(obws::error::Error::Disconnected);
            }
            if self.slots.available_permits() == self.capacity {
                return Ok(());
            }
            tokio::select! {
                _ = retirement.changed() => return Err(obws::error::Error::Disconnected),
                _ = changed.changed() => {}
            }
        }
    }

    fn new(capacity: usize, timeout: Option<Duration>) -> Self {
        let (retired, _) = watch::channel(false);
        Self {
            slots: Arc::new(Semaphore::new(capacity)),
            capacity,
            drained: watch::channel(0).0,
            shutdown_claimed: std::sync::atomic::AtomicBool::new(false),
            timeout,
            retired,
            reason: std::sync::Mutex::new(None),
        }
    }

    pub(super) fn retire(&self) {
        self.retire_with_reason(RetirementReason::Cancelled);
    }

    pub(super) fn retire_with_reason(&self, reason: RetirementReason) {
        let mut current = self.reason.lock().expect("request retirement reason");
        if current.is_none() || reason == RetirementReason::Deadline {
            *current = Some(reason);
        }
        drop(current);
        self.retired.send_replace(true);
    }

    pub(super) fn retirement_reason(&self) -> Option<String> {
        self.reason
            .lock()
            .expect("request retirement reason")
            .map(|reason| reason.message().to_owned())
    }

    pub(super) async fn wait_retired(&self) {
        let mut retirement = self.retired.subscribe();
        while !*retirement.borrow_and_update() {
            if retirement.changed().await.is_err() {
                return;
            }
        }
    }

    pub(super) async fn run<T>(
        &self,
        request: impl Future<Output = obws::error::Result<T>>,
    ) -> obws::error::Result<T> {
        self.run_if_valid(request, &|| true).await
    }

    pub(super) async fn run_if_valid<T>(
        &self,
        request: impl Future<Output = obws::error::Result<T>>,
        valid: &(impl Fn() -> bool + Sync),
    ) -> obws::error::Result<T> {
        let mut retirement = self.retired.subscribe();
        if *retirement.borrow() {
            return Err(obws::error::Error::Disconnected);
        }
        let permit = tokio::select! {
            biased;
            _ = retirement.changed() => return Err(obws::error::Error::Disconnected),
            permit = Arc::clone(&self.slots).acquire_owned() => match permit {
                Ok(permit) => permit,
                Err(_) => {
                    // Park refresh pipelines until final retirement instead of
                    // making admission closure look like a transport failure.
                    self.wait_retired().await;
                    return Err(obws::error::Error::Disconnected);
                }
            },
        };
        let mut lease = RequestLease {
            gate: self,
            permit: Some(permit),
            completed: false,
        };
        let response = tokio::select! {
            biased;
            _ = retirement.changed() => return Err(obws::error::Error::Disconnected),
            response = async {
                // Check only in the selected request branch, immediately before
                // the first upstream poll can register/transmit a mutation.
                if self.slots.is_closed() || !valid() { return Ok(None); }
                match self.timeout {
                    Some(timeout) => tokio::time::timeout(timeout, request).await.map(Some),
                    None => Ok(Some(request.await)),
                }
            } => response,
        };
        match response {
            Ok(response) => {
                // Skipping an unpolled request creates no abandoned registration.
                lease.completed = true;
                drop(lease);
                if response.is_none() && self.slots.is_closed() {
                    self.wait_retired().await;
                }
                response.unwrap_or(Err(obws::error::Error::Disconnected))
            }
            Err(_) => {
                self.retire_with_reason(RetirementReason::Deadline);
                Err(obws::error::Error::Timeout)
            }
        }
    }
}

struct RequestLease<'a> {
    gate: &'a RequestGate,
    permit: Option<OwnedSemaphorePermit>,
    completed: bool,
}

impl Drop for RequestLease<'_> {
    fn drop(&mut self) {
        if !self.completed {
            self.gate.retire();
        }
        self.permit.take();
        self.gate.drained.send_modify(|revision| *revision += 1);
    }
}
