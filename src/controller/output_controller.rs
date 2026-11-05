//! Stream and recording output orchestration.
//!
//! Owns duplicate-command guards, output-client selection, command events,
//! status refreshes, and graceful output shutdown during disconnect.

use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};

use crate::controller::event_sender::EventSender;
use crate::controller::lifecycle::SessionScope;
use crate::controller::session_controller::REQUEST_BUDGET;
use futures_util::{future::BoxFuture, FutureExt};
use i18n_embed_fl::fl;
use tokio::runtime::Handle;

use crate::controller::event::{
    fallback_status_after_failed_output_command, AppEvent, OutputCommandFailureRecovery,
};
use crate::domain::output::{OutputKind, OutputRunState, OutputStatus};
use crate::infra::error::AppError;
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::obs::client::ObsClient;

#[derive(Debug, Default)]
struct OutputPending {
    stream: Option<u64>,
    record: Option<u64>,
    next: u64,
}

pub(crate) struct OutputController {
    runtime: Handle,
    event_tx: EventSender,
    scope: SessionScope,
    client_slot: Arc<Mutex<Option<ObsClient>>>,
    pending: Arc<Mutex<OutputPending>>,
    #[cfg(test)]
    client_override: Arc<Mutex<Option<OutputCommandClient>>>,
}

impl OutputController {
    pub(crate) fn new(
        runtime: Handle,
        event_tx: SyncSender<AppEvent>,
        client_slot: Arc<Mutex<Option<ObsClient>>>,
    ) -> Self {
        Self {
            runtime,
            event_tx: EventSender::new(event_tx),
            scope: SessionScope::initial(client_slot.clone()),
            client_slot,
            pending: Arc::new(Mutex::new(OutputPending::default())),
            #[cfg(test)]
            client_override: Arc::new(Mutex::new(None)),
        }
    }

    pub(crate) fn set_scope(&mut self, scope: SessionScope) {
        self.clear_pending();
        self.scope = scope;
    }

    pub(crate) fn set_output(&self, kind: OutputKind, active: bool) {
        let Some(operation) = mark_pending(&self.pending, kind) else {
            tracing::warn!(
                ?kind,
                active,
                "output command ignored while previous operation is pending"
            );
            return;
        };

        let Some(client) = self.client() else {
            clear_pending(&self.pending, kind, operation);
            let failure = OutputCommandFailureRecovery::with_failed_command_fallback_status(
                fl!(LANGUAGE_LOADER, "controller-not-connected"),
                OutputStatus::default(),
            );
            let tx = self.event_tx.scoped(&self.scope);
            self.scope.spawn(&self.runtime, async move {
                let _ = tx.send(AppEvent::OutputCommandFailed(kind, failure)).await;
            });
            return;
        };

        let tx = self.event_tx.scoped(&self.scope);
        let pending = Arc::clone(&self.pending);
        let Some(evidence) = self
            .scope
            .operation(operation_kind(kind), client.endpoint())
        else {
            clear_pending(&pending, kind, operation);
            return;
        };
        let admitted_at = std::time::Instant::now();
        let scope = self.scope.clone();
        let pending_status = pending_status(active);
        let fallback = fallback_status_after_failed_output_command(&pending_status);
        let completion = PendingOperation {
            pending: pending.clone(),
            kind,
            operation,
        };
        self.scope.spawn_reserved(&self.runtime, async move {
            let _completion = completion;
            let _ = tx
                .send(AppEvent::OutputCommandPending(
                    kind,
                    pending_status,
                    admitted_at,
                ))
                .await;
            let acknowledged = match client
                .set_output_if(kind, active, &|| {
                    if !scope.is_active() {
                        return false;
                    }
                    evidence.dispatch();
                    true
                })
                .await
            {
                Ok(path) => {
                    evidence.acknowledge();
                    let _ = tx.send(AppEvent::OutputCommandSucceeded(kind)).await;
                    // The response proves the earlier stop/file, not that the
                    // output is still inactive when a delayed response arrives.
                    if let (OutputKind::Record, false, Some(path)) = (kind, active, path) {
                        let _ = tx
                            .send(AppEvent::RecordingStopObserved {
                                path,
                                command_at: admitted_at,
                            })
                            .await;
                    }
                    true
                }
                Err(error) => {
                    tracing::warn!(%error, ?kind, active, "output command failed");
                    let uncertain = match &client {
                        OutputCommandClient::Obs(client) => {
                            client.wait_retired().now_or_never().is_some()
                        }
                        #[cfg(test)]
                        OutputCommandClient::Fake(_) => matches!(error, AppError::Connection(_)),
                    };
                    let failure = if uncertain {
                        OutputCommandFailureRecovery::outcome_unknown(error.user_message())
                    } else {
                        OutputCommandFailureRecovery::with_failed_command_fallback_status(
                            error.user_message(),
                            fallback,
                        )
                    };
                    if uncertain {
                        evidence.unknown(failure.message().to_owned());
                    } else {
                        evidence.finish(Err(failure.message().to_owned()));
                    }
                    let _ = tx.send(AppEvent::OutputCommandFailed(kind, failure)).await;
                    false
                }
            };
            let readback = refresh_output_statuses(&client, &tx).await;
            if acknowledged {
                evidence.finish(readback.result_for(kind));
                readback.publish(&tx).await;
            }
            clear_pending(&pending, kind, operation);
        });
    }

    pub(crate) fn client(&self) -> Option<OutputCommandClient> {
        #[cfg(test)]
        if let Some(client) = self
            .client_override
            .lock()
            .ok()
            .and_then(|slot| slot.clone())
        {
            return Some(client);
        }

        self.client_slot
            .lock()
            .ok()
            .and_then(|slot| slot.clone())
            .map(OutputCommandClient::Obs)
    }

    pub(crate) fn clear_pending(&self) {
        if let Ok(mut pending) = self.pending.lock() {
            pending.stream = None;
            pending.record = None;
        }
    }

    #[cfg(test)]
    pub(crate) fn set_client_override(&self, client: OutputCommandClient) {
        if let Ok(mut slot) = self.client_override.lock() {
            *slot = Some(client);
        }
    }
}

fn pending_status(active: bool) -> OutputStatus {
    OutputStatus {
        duration_ms: None,
        active: !active,
        state: if active {
            OutputRunState::Starting
        } else {
            OutputRunState::Stopping
        },
        detail: None,
    }
}

#[derive(Clone)]
pub(crate) enum OutputCommandClient {
    Obs(ObsClient),
    #[cfg(test)]
    Fake(Arc<dyn FakeOutputCommandClient>),
}

impl OutputCommandClient {
    fn endpoint(&self) -> String {
        match self {
            Self::Obs(client) => client.endpoint().to_owned(),
            #[cfg(test)]
            Self::Fake(_) => "fixture".into(),
        }
    }

    /// Stream commands report no path; a recording stop may.
    async fn set_output_if(
        &self,
        kind: OutputKind,
        active: bool,
        valid: &(impl Fn() -> bool + Sync),
    ) -> Result<Option<String>, AppError> {
        match kind {
            OutputKind::Stream => self.set_streaming_if(active, valid).await.map(|()| None),
            OutputKind::Record => self.set_recording_if(active, valid).await,
        }
    }

    async fn set_streaming(&self, active: bool) -> Result<(), AppError> {
        self.set_streaming_if(active, &|| true).await
    }

    async fn set_streaming_if(
        &self,
        active: bool,
        valid: &(impl Fn() -> bool + Sync),
    ) -> Result<(), AppError> {
        let operation = async {
            match self {
                Self::Obs(client) => client.set_streaming_if(active, valid).await,
                #[cfg(test)]
                Self::Fake(client) => {
                    if !valid() {
                        return Err(AppError::Connection(
                            "Command cancelled before dispatch".into(),
                        ));
                    }
                    client.set_streaming(active).await
                }
            }
        };
        self.with_deadline(operation).await
    }

    async fn set_recording(&self, active: bool) -> Result<Option<String>, AppError> {
        self.set_recording_if(active, &|| true).await
    }

    async fn set_recording_if(
        &self,
        active: bool,
        valid: &(impl Fn() -> bool + Sync),
    ) -> Result<Option<String>, AppError> {
        let operation = async {
            match self {
                Self::Obs(client) => client.set_recording_if(active, valid).await,
                #[cfg(test)]
                Self::Fake(client) => {
                    if !valid() {
                        return Err(AppError::Connection(
                            "Command cancelled before dispatch".into(),
                        ));
                    }
                    client.set_recording(active).await
                }
            }
        };
        self.with_deadline(operation).await
    }

    pub(crate) fn retire(&self) {
        match self {
            Self::Obs(client) => client.retire(),
            #[cfg(test)]
            Self::Fake(_) => {}
        }
    }

    async fn with_deadline<T>(
        &self,
        operation: impl std::future::Future<Output = Result<T, AppError>>,
    ) -> Result<T, AppError> {
        match tokio::time::timeout(REQUEST_BUDGET, operation).await {
            Ok(result) => result,
            Err(_) => {
                match self {
                    Self::Obs(client) => client.retire_after_deadline(),
                    #[cfg(test)]
                    Self::Fake(_) => {}
                }
                Err(AppError::Connection(
                    "OBS deadline exceeded; outcome unknown; command was not retried".into(),
                ))
            }
        }
    }
}

pub(crate) trait OutputStatusReader {
    fn next_observation(&self) -> Option<u64> {
        None
    }
    fn retire_after_timeout(&self);
    fn read_stream_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>>;
    fn read_record_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>>;
}

impl OutputStatusReader for ObsClient {
    fn next_observation(&self) -> Option<u64> {
        Some(ObsClient::next_observation(self))
    }
    fn retire_after_timeout(&self) {
        self.retire_after_deadline();
    }
    fn read_stream_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>> {
        Box::pin(async move { self.get_stream_status().await })
    }

    fn read_record_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>> {
        Box::pin(async move { self.get_record_status().await })
    }
}

impl OutputStatusReader for OutputCommandClient {
    fn next_observation(&self) -> Option<u64> {
        match self {
            Self::Obs(client) => Some(client.next_observation()),
            #[cfg(test)]
            Self::Fake(_) => None,
        }
    }
    fn retire_after_timeout(&self) {
        match self {
            Self::Obs(client) => client.retire_after_deadline(),
            #[cfg(test)]
            Self::Fake(_) => {}
        }
    }
    fn read_stream_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>> {
        Box::pin(async move {
            match self {
                Self::Obs(client) => client.get_stream_status().await,
                #[cfg(test)]
                Self::Fake(client) => client.get_stream_status().await,
            }
        })
    }

    fn read_record_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>> {
        Box::pin(async move {
            match self {
                Self::Obs(client) => client.get_record_status().await,
                #[cfg(test)]
                Self::Fake(client) => client.get_record_status().await,
            }
        })
    }
}

#[cfg(test)]
pub(crate) trait FakeOutputCommandClient: Send + Sync {
    fn set_streaming(&self, active: bool) -> BoxFuture<'_, Result<(), AppError>>;
    fn set_recording(&self, active: bool) -> BoxFuture<'_, Result<Option<String>, AppError>>;
    fn get_stream_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>>;
    fn get_record_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>>;
}

struct PendingOperation {
    pending: Arc<Mutex<OutputPending>>,
    kind: OutputKind,
    operation: u64,
}

impl Drop for PendingOperation {
    fn drop(&mut self) {
        clear_pending(&self.pending, self.kind, self.operation);
    }
}

impl OutputPending {
    fn slot(&mut self, kind: OutputKind) -> &mut Option<u64> {
        match kind {
            OutputKind::Stream => &mut self.stream,
            OutputKind::Record => &mut self.record,
        }
    }
}

fn mark_pending(pending: &Arc<Mutex<OutputPending>>, kind: OutputKind) -> Option<u64> {
    let mut pending = pending.lock().ok()?;
    if pending.slot(kind).is_some() {
        return None;
    }
    pending.next = pending
        .next
        .checked_add(1)
        .expect("output operation exhausted");
    let operation = pending.next;
    *pending.slot(kind) = Some(operation);
    Some(operation)
}

fn clear_pending(pending: &Arc<Mutex<OutputPending>>, kind: OutputKind, operation: u64) {
    if let Ok(mut pending) = pending.lock() {
        let slot = pending.slot(kind);
        if *slot == Some(operation) {
            *slot = None;
        }
    }
}

fn operation_kind(kind: OutputKind) -> super::lifecycle::OperationKind {
    match kind {
        OutputKind::Stream => super::lifecycle::OperationKind::Stream,
        OutputKind::Record => super::lifecycle::OperationKind::Record,
    }
}

pub(crate) async fn stop_active_outputs_before_disconnect(client: &OutputCommandClient) {
    match client.read_stream_status().await {
        Ok(status) if status.active => {
            if let Err(error) = client.set_streaming(false).await {
                tracing::warn!(%error, "failed to stop active stream before disconnect");
            }
        }
        Ok(_) => {}
        Err(error) => tracing::warn!(%error, "could not read stream status before disconnect"),
    }
    match client.read_record_status().await {
        Ok(status) if status.active => {
            if let Err(error) = client.set_recording(false).await {
                tracing::warn!(%error, "failed to stop active recording before disconnect");
            }
        }
        Ok(_) => {}
        Err(error) => tracing::warn!(%error, "could not read recording status before disconnect"),
    }
}

pub(crate) struct OutputReadbackErrors {
    stream: Option<(AppError, std::time::Instant)>,
    record: Option<(AppError, std::time::Instant)>,
}

impl OutputReadbackErrors {
    fn result_for(&self, kind: OutputKind) -> Result<(), String> {
        let error = match kind {
            OutputKind::Stream => &self.stream,
            OutputKind::Record => &self.record,
        };
        error
            .as_ref()
            .map_or(Ok(()), |(error, _)| Err(error.user_message()))
    }

    pub(crate) async fn publish(self, tx: &EventSender) {
        for (kind, error) in [
            (OutputKind::Stream, self.stream),
            (OutputKind::Record, self.record),
        ] {
            if let Some((error, observed_at)) = error {
                let _ = tx
                    .send(AppEvent::OutputReadbackFailed {
                        kind,
                        error,
                        observed_at,
                    })
                    .await;
            }
        }
    }
}

pub(crate) async fn refresh_output_statuses(
    client: &impl OutputStatusReader,
    tx: &EventSender,
) -> OutputReadbackErrors {
    let stream_at = std::time::Instant::now();
    let stream_revision = client.next_observation();
    let stream = match tokio::time::timeout(REQUEST_BUDGET, client.read_stream_status()).await {
        Ok(Ok(status)) => {
            let status = AppEvent::OutputStatusUpdated(OutputKind::Stream, status, stream_at);
            let event = match stream_revision {
                Some(revision) => AppEvent::Observation {
                    revision,
                    event: Box::new(status),
                },
                None => status,
            };
            let _ = tx.send(event).await;
            None
        }
        Ok(Err(error)) => {
            tracing::warn!(%error, "stream status refresh failed");
            Some(error)
        }
        Err(error) => {
            client.retire_after_timeout();
            Some(AppError::request(error))
        }
    };
    let record_at = std::time::Instant::now();
    let record_revision = client.next_observation();
    let record = match tokio::time::timeout(REQUEST_BUDGET, client.read_record_status()).await {
        Ok(Ok(status)) => {
            let status = AppEvent::OutputStatusUpdated(OutputKind::Record, status, record_at);
            let event = match record_revision {
                Some(revision) => AppEvent::Observation {
                    revision,
                    event: Box::new(status),
                },
                None => status,
            };
            let _ = tx.send(event).await;
            None
        }
        Ok(Err(error)) => {
            tracing::warn!(%error, "record status refresh failed");
            Some(error)
        }
        Err(error) => {
            client.retire_after_timeout();
            Some(AppError::request(error))
        }
    };
    OutputReadbackErrors {
        stream: stream.map(|error| (error, stream_at)),
        record: record.map(|error| (error, record_at)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::Notify;

    struct GatedOutput {
        writes: AtomicUsize,
        reading: Notify,
        release: Notify,
    }

    impl FakeOutputCommandClient for GatedOutput {
        fn set_streaming(&self, _: bool) -> BoxFuture<'_, Result<(), AppError>> {
            Box::pin(async move {
                self.writes.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        }
        fn set_recording(&self, _: bool) -> BoxFuture<'_, Result<Option<String>, AppError>> {
            Box::pin(async { Ok(None) })
        }
        fn get_stream_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>> {
            Box::pin(async move {
                self.reading.notify_one();
                self.release.notified().await;
                Err(AppError::Request("readback unavailable".into()))
            })
        }
        fn get_record_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>> {
            Box::pin(async { Ok(OutputStatus::default()) })
        }
    }

    async fn receive(rx: &std::sync::mpsc::Receiver<AppEvent>) -> AppEvent {
        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            loop {
                if let Ok(event) = rx.try_recv() {
                    return event;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn late_recording_stop_response_keeps_file_evidence_after_an_observed_restart() {
        struct DelayedStop {
            requested: Notify,
            release: Notify,
        }
        impl FakeOutputCommandClient for DelayedStop {
            fn set_streaming(&self, _: bool) -> BoxFuture<'_, Result<(), AppError>> {
                Box::pin(async { Ok(()) })
            }
            fn set_recording(
                &self,
                active: bool,
            ) -> BoxFuture<'_, Result<Option<String>, AppError>> {
                assert!(!active);
                Box::pin(async move {
                    self.requested.notify_one();
                    self.release.notified().await;
                    Ok(Some("closed.mkv".into()))
                })
            }
            fn get_stream_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>> {
                Box::pin(async { Ok(OutputStatus::inactive()) })
            }
            fn get_record_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>> {
                Box::pin(async { Ok(OutputStatus::active()) })
            }
        }
        let (tx, rx) = std::sync::mpsc::sync_channel(8);
        let outputs = OutputController::new(Handle::current(), tx, Arc::new(Mutex::new(None)));
        let fake = Arc::new(DelayedStop {
            requested: Notify::new(),
            release: Notify::new(),
        });
        outputs.set_client_override(OutputCommandClient::Fake(fake.clone()));
        let mut state = crate::controller::state::AppState::new(
            Default::default(),
            Default::default(),
            None,
            None,
        );
        state.health.connected();
        state.apply_output_event(
            &AppEvent::OutputStatusUpdated(
                OutputKind::Record,
                OutputStatus::active(),
                std::time::Instant::now(),
            ),
            0,
        );
        outputs.set_output(OutputKind::Record, false);
        state.apply_output_event(&receive(&rx).await, 1);
        fake.requested.notified().await;
        // Another controller restarts recording while the completed stop's response is held.
        state.apply_output_event(
            &AppEvent::OutputStatusUpdated(
                OutputKind::Record,
                OutputStatus::active_with_detail("new.mkv"),
                std::time::Instant::now(),
            ),
            2,
        );
        fake.release.notify_one();
        let ack = receive(&rx).await;
        assert!(matches!(
            ack,
            AppEvent::OutputCommandSucceeded(OutputKind::Record)
        ));
        state.apply_output_event(&ack, 3);
        let file = receive(&rx).await;
        assert!(matches!(file, AppEvent::RecordingStopObserved { .. }));
        state.apply_output_event(&file, 4);
        assert!(
            state.record_status.active,
            "a response to the earlier stop is not current inactive state"
        );
        assert_eq!(state.health.output_activity(OutputKind::Record), Some(true));
        assert!(state.health.alerts(std::time::Instant::now()).is_empty());
        assert_eq!(state.last_recording_path.as_deref(), Some("new.mkv"));
        assert!(state
            .workspace
            .output_results
            .entries()
            .any(|result| result.path.as_deref() == Some("closed.mkv")));
        for now_ms in 5..=6 {
            state.apply_output_event(&receive(&rx).await, now_ms);
        }
        assert!(state.record_status.active);
        outputs.scope.retire();
        outputs.scope.quiesce().await;
    }

    #[tokio::test]
    async fn output_admission_timestamp_precedes_executor_and_publication_backlog() {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        tx.send(AppEvent::GraphInvalidated).unwrap();
        let outputs = OutputController::new(Handle::current(), tx, Arc::new(Mutex::new(None)));
        let fake = Arc::new(GatedOutput {
            writes: AtomicUsize::new(0),
            reading: Notify::new(),
            release: Notify::new(),
        });
        outputs.set_client_override(OutputCommandClient::Fake(fake));
        outputs.set_output(OutputKind::Record, false);
        let admitted_before = std::time::Instant::now();
        assert!(matches!(receive(&rx).await, AppEvent::GraphInvalidated));
        let AppEvent::OutputCommandPending(OutputKind::Record, status, admitted_at) =
            receive(&rx).await
        else {
            panic!("missing admitted stop")
        };
        assert_eq!(status.state, OutputRunState::Stopping);
        assert!(
            admitted_at <= admitted_before,
            "waiting for the executor or UI must not refresh admission time"
        );
        outputs.scope.retire();
        outputs.scope.quiesce().await;
    }

    #[test]
    fn retired_output_completion_cannot_release_a_new_operation() {
        let pending = Arc::new(Mutex::new(OutputPending::default()));
        let old = mark_pending(&pending, OutputKind::Stream).unwrap();
        pending.lock().unwrap().stream = None;
        let new = mark_pending(&pending, OutputKind::Stream).unwrap();
        drop(PendingOperation {
            pending: pending.clone(),
            kind: OutputKind::Stream,
            operation: old,
        });
        assert_eq!(pending.lock().unwrap().stream, Some(new));
        clear_pending(&pending, OutputKind::Stream, new);
        assert!(pending.lock().unwrap().stream.is_none());
    }

    #[tokio::test]
    async fn output_acknowledgement_precedes_readback_and_failed_readback_is_not_replayed() {
        let (tx, rx) = std::sync::mpsc::sync_channel(8);
        let outputs = OutputController::new(Handle::current(), tx, Arc::new(Mutex::new(None)));
        let fake = Arc::new(GatedOutput {
            writes: AtomicUsize::new(0),
            reading: Notify::new(),
            release: Notify::new(),
        });
        outputs.set_client_override(OutputCommandClient::Fake(fake.clone()));
        outputs.set_output(OutputKind::Stream, true);
        assert!(matches!(
            receive(&rx).await,
            AppEvent::OutputCommandPending(OutputKind::Stream, _, _)
        ));
        assert!(matches!(
            receive(&rx).await,
            AppEvent::OutputCommandSucceeded(OutputKind::Stream)
        ));
        fake.reading.notified().await;
        let read_started_before = std::time::Instant::now();
        assert!(
            rx.try_recv().is_err(),
            "ack arrived while readback remains gated"
        );
        fake.release.notify_one();
        assert!(matches!(
            receive(&rx).await,
            AppEvent::OutputStatusUpdated(OutputKind::Record, _, _)
        ));
        let AppEvent::OutputReadbackFailed {
            kind: OutputKind::Stream,
            observed_at,
            ..
        } = receive(&rx).await
        else {
            panic!("missing failed readback")
        };
        assert!(
            observed_at <= read_started_before,
            "failed read preserves request-start time"
        );
        assert_eq!(fake.writes.load(Ordering::SeqCst), 1);
        outputs.scope.retire();
        outputs.scope.quiesce().await;
        assert!(outputs.pending.lock().unwrap().stream.is_none());
    }

    #[tokio::test]
    async fn retirement_drops_pending_lease_even_while_readback_is_gated() {
        let (tx, rx) = std::sync::mpsc::sync_channel(8);
        let outputs = OutputController::new(Handle::current(), tx, Arc::new(Mutex::new(None)));
        let fake = Arc::new(GatedOutput {
            writes: AtomicUsize::new(0),
            reading: Notify::new(),
            release: Notify::new(),
        });
        outputs.set_client_override(OutputCommandClient::Fake(fake.clone()));
        outputs.set_output(OutputKind::Stream, true);
        let _ = receive(&rx).await;
        let _ = receive(&rx).await;
        fake.reading.notified().await;
        outputs.scope.retire();
        outputs.scope.quiesce().await;
        assert!(outputs.pending.lock().unwrap().stream.is_none());
        fake.release.notify_one();
        assert!(rx.try_recv().is_err());
    }
}
