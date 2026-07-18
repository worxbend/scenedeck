//! Stream and recording output orchestration.
//!
//! Owns duplicate-command guards, output-client selection, command events,
//! status refreshes, and graceful output shutdown during disconnect.

use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};

use futures_util::future::BoxFuture;
use i18n_embed_fl::fl;
use tokio::runtime::Handle;

use crate::controller::event::{
    fallback_status_after_failed_output_command, AppEvent, OutputCommandFailureRecovery,
};
use crate::domain::output::{OutputRunState, OutputStatus};
use crate::infra::error::AppError;
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::obs::client::ObsClient;

#[derive(Debug, Default)]
struct OutputPending {
    stream: bool,
    record: bool,
}

pub(crate) struct OutputController {
    runtime: Handle,
    event_tx: SyncSender<AppEvent>,
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
            event_tx,
            client_slot,
            pending: Arc::new(Mutex::new(OutputPending::default())),
            #[cfg(test)]
            client_override: Arc::new(Mutex::new(None)),
        }
    }

    pub(crate) fn set_streaming(&self, active: bool) {
        if !mark_stream_pending(&self.pending) {
            tracing::warn!(
                active,
                "stream command ignored while previous operation is pending"
            );
            return;
        }

        let Some(client) = self.client() else {
            clear_stream_pending(&self.pending);
            let failure = OutputCommandFailureRecovery::with_failed_command_fallback_status(
                fl!(LANGUAGE_LOADER, "controller-not-connected"),
                OutputStatus::default(),
            );
            let _ = self.event_tx.send(AppEvent::StreamCommandFailed(failure));
            return;
        };

        let tx = self.event_tx.clone();
        let pending = Arc::clone(&self.pending);
        let pending_status = pending_status(active);
        let fallback = fallback_status_after_failed_output_command(&pending_status);
        let _ = tx.send(AppEvent::StreamCommandPending(pending_status));

        self.runtime.spawn(async move {
            match client.set_streaming(active).await {
                Ok(()) => {
                    let _ = tx.send(AppEvent::StreamCommandSucceeded);
                }
                Err(error) => {
                    tracing::warn!(%error, active, "stream command failed");
                    let failure = OutputCommandFailureRecovery::with_failed_command_fallback_status(
                        error.to_string(),
                        fallback,
                    );
                    let _ = tx.send(AppEvent::StreamCommandFailed(failure));
                }
            }
            refresh_output_statuses(&client, &tx).await;
            clear_stream_pending(&pending);
        });
    }

    pub(crate) fn set_recording(&self, active: bool) {
        if !mark_record_pending(&self.pending) {
            tracing::warn!(
                active,
                "record command ignored while previous operation is pending"
            );
            return;
        }

        let Some(client) = self.client() else {
            clear_record_pending(&self.pending);
            let failure = OutputCommandFailureRecovery::with_failed_command_fallback_status(
                fl!(LANGUAGE_LOADER, "controller-not-connected"),
                OutputStatus::default(),
            );
            let _ = self.event_tx.send(AppEvent::RecordCommandFailed(failure));
            return;
        };

        let tx = self.event_tx.clone();
        let pending = Arc::clone(&self.pending);
        let pending_status = pending_status(active);
        let fallback = fallback_status_after_failed_output_command(&pending_status);
        let _ = tx.send(AppEvent::RecordCommandPending(pending_status));

        self.runtime.spawn(async move {
            match client.set_recording(active).await {
                Ok(path) => {
                    let _ = tx.send(AppEvent::RecordCommandSucceeded);
                    if let Some(path) = path {
                        let _ = tx.send(AppEvent::RecordStatusUpdated(OutputStatus {
                            active: false,
                            state: OutputRunState::Inactive,
                            detail: Some(path),
                        }));
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, active, "record command failed");
                    let failure = OutputCommandFailureRecovery::with_failed_command_fallback_status(
                        error.to_string(),
                        fallback,
                    );
                    let _ = tx.send(AppEvent::RecordCommandFailed(failure));
                }
            }
            refresh_output_statuses(&client, &tx).await;
            clear_record_pending(&pending);
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
            *pending = OutputPending::default();
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
    async fn set_streaming(&self, active: bool) -> Result<(), AppError> {
        match self {
            Self::Obs(client) => client.set_streaming(active).await,
            #[cfg(test)]
            Self::Fake(client) => client.set_streaming(active).await,
        }
    }

    async fn set_recording(&self, active: bool) -> Result<Option<String>, AppError> {
        match self {
            Self::Obs(client) => client.set_recording(active).await,
            #[cfg(test)]
            Self::Fake(client) => client.set_recording(active).await,
        }
    }
}

pub(crate) trait OutputStatusReader {
    fn read_stream_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>>;
    fn read_record_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>>;
}

impl OutputStatusReader for ObsClient {
    fn read_stream_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>> {
        Box::pin(async move { self.get_stream_status().await })
    }

    fn read_record_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>> {
        Box::pin(async move { self.get_record_status().await })
    }
}

impl OutputStatusReader for OutputCommandClient {
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

fn mark_stream_pending(pending: &Arc<Mutex<OutputPending>>) -> bool {
    let Ok(mut pending) = pending.lock() else {
        return false;
    };
    if pending.stream {
        return false;
    }
    pending.stream = true;
    true
}

fn clear_stream_pending(pending: &Arc<Mutex<OutputPending>>) {
    if let Ok(mut pending) = pending.lock() {
        pending.stream = false;
    }
}

fn mark_record_pending(pending: &Arc<Mutex<OutputPending>>) -> bool {
    let Ok(mut pending) = pending.lock() else {
        return false;
    };
    if pending.record {
        return false;
    }
    pending.record = true;
    true
}

fn clear_record_pending(pending: &Arc<Mutex<OutputPending>>) {
    if let Ok(mut pending) = pending.lock() {
        pending.record = false;
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

pub(crate) async fn refresh_output_statuses(
    client: &impl OutputStatusReader,
    tx: &SyncSender<AppEvent>,
) {
    match client.read_stream_status().await {
        Ok(status) => {
            let _ = tx.send(AppEvent::StreamStatusUpdated(status));
        }
        Err(error) => tracing::warn!(%error, "stream status refresh failed"),
    }
    match client.read_record_status().await {
        Ok(status) => {
            let _ = tx.send(AppEvent::RecordStatusUpdated(status));
        }
        Err(error) => tracing::warn!(%error, "record status refresh failed"),
    }
}
