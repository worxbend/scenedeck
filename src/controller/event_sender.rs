//! Bounded, cancellation-aware publication into the GTK-owned event queue.

use std::sync::mpsc::{SyncSender, TrySendError};
use std::time::Duration;

use super::event::AppEvent;
use super::lifecycle::SessionScope;

fn is_collection_independent(event: &AppEvent) -> bool {
    matches!(
        event,
        AppEvent::Connecting
            | AppEvent::Connected(_)
            | AppEvent::Disconnected
            | AppEvent::Error(_)
            | AppEvent::StreamCommandSucceeded
            | AppEvent::RecordCommandSucceeded
            | AppEvent::StreamCommandFailed(_)
            | AppEvent::RecordCommandFailed(_)
            | AppEvent::OutputReadbackFailed { .. }
            | AppEvent::GraphRefreshRejected(_)
            | AppEvent::WorkspaceAcknowledged { .. }
    )
}

#[derive(Clone)]
pub(crate) struct EventSender {
    inner: SyncSender<AppEvent>,
    scope: Option<SessionScope>,
    collection: u64,
}

impl EventSender {
    pub(crate) fn new(inner: SyncSender<AppEvent>) -> Self {
        Self {
            inner,
            scope: None,
            collection: 0,
        }
    }

    pub(crate) fn scoped(&self, scope: &SessionScope) -> Self {
        Self {
            inner: self.inner.clone(),
            scope: Some(scope.clone()),
            collection: scope.collection(),
        }
    }

    pub(crate) fn fresh(&self) -> Self {
        self.scope
            .as_ref()
            .map_or_else(|| self.clone(), |scope| self.scoped(scope))
    }

    pub(crate) fn session_scope(&self) -> Option<&SessionScope> {
        self.scope.as_ref()
    }

    pub(crate) fn advance_collection(&self) {
        if let Some(scope) = &self.scope {
            scope.advance_collection();
        }
    }

    pub(crate) fn operation(
        &self,
        kind: super::lifecycle::OperationKind,
        endpoint: String,
    ) -> Option<super::lifecycle::OperationLease> {
        self.scope.as_ref()?.operation(kind, endpoint)
    }

    pub(crate) fn terminal_reason(&self, reason: String) {
        if let Some(scope) = &self.scope {
            scope.set_terminal_reason(reason);
        }
    }

    pub(crate) fn accepts_current_scope(&self) -> bool {
        self.scope
            .as_ref()
            .is_none_or(|scope| scope.is_active() && scope.collection() == self.collection)
    }

    fn envelope(&self, event: AppEvent) -> AppEvent {
        match &self.scope {
            Some(scope) if scope.generation() != 0 => AppEvent::SessionEvent {
                generation: scope.generation(),
                collection: if is_collection_independent(&event) {
                    None
                } else {
                    Some(self.collection)
                },
                event: Box::new(event),
            },
            _ => event,
        }
    }

    pub(crate) async fn send(&self, event: AppEvent) -> Result<(), ()> {
        self.send_if(event, &|| true).await
    }

    /// Revalidate freshness at each final queue-admission attempt, not merely
    /// before waiting for bounded GTK capacity. No await separates the check
    /// from try_send; an invalidated completion is discarded, never replayed.
    pub(crate) async fn send_if(
        &self,
        event: AppEvent,
        valid: &(impl Fn() -> bool + Sync),
    ) -> Result<(), ()> {
        let failed = matches!(event, AppEvent::Error(_));
        let lifecycle = is_collection_independent(&event);
        let mut event = self.envelope(event);
        loop {
            if !valid()
                || self.scope.as_ref().is_some_and(|scope| !scope.is_current())
                || (!lifecycle && !self.accepts_current_scope())
            {
                return Err(());
            }
            match self.inner.try_send(event) {
                Ok(()) => {
                    if failed {
                        if let Some(scope) = &self.scope {
                            scope.mark_failed();
                        }
                    }
                    return Ok(());
                }
                Err(TrySendError::Disconnected(_)) => return Err(()),
                Err(TrySendError::Full(pending)) => event = pending,
            }
            // SyncSender has no async capacity notification. Retrying yields the
            // executor and keeps the queue bounded without dropping a completion.
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    }

    pub(crate) fn try_send(&self, event: AppEvent) -> Result<(), ()> {
        if !self.accepts_current_scope() {
            return Err(());
        }
        self.inner.try_send(self.envelope(event)).map_err(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn lifecycle_envelope_and_collection_admission_follow_the_same_policy() {
        use crate::controller::event::{ConnectionInfo, OutputCommandFailureRecovery};
        use crate::domain::output::{OutputKind, OutputRunState, OutputStatus};
        use crate::infra::error::AppError;

        let failure = || {
            OutputCommandFailureRecovery::with_failed_command_fallback_status(
                "fixture failure".into(),
                OutputStatus::new(false, OutputRunState::Inactive),
            )
        };
        let cases = [
            (AppEvent::Connecting, true),
            (
                AppEvent::Connected(ConnectionInfo {
                    endpoint: String::new(),
                    obs_version: String::new(),
                    websocket_version: String::new(),
                    available_requests: Vec::new(),
                    supported_image_formats: Vec::new(),
                }),
                true,
            ),
            (AppEvent::Disconnected, true),
            (
                AppEvent::Error(AppError::Request("fixture failure".into())),
                true,
            ),
            (AppEvent::StreamCommandSucceeded, true),
            (AppEvent::RecordCommandSucceeded, true),
            (AppEvent::StreamCommandFailed(failure()), true),
            (AppEvent::RecordCommandFailed(failure()), true),
            (
                AppEvent::OutputReadbackFailed {
                    kind: OutputKind::Stream,
                    error: AppError::Request("fixture failure".into()),
                },
                true,
            ),
            (
                AppEvent::GraphRefreshRejected(AppError::Request("fixture failure".into())),
                true,
            ),
            (
                AppEvent::WorkspaceAcknowledged {
                    epoch: 1,
                    sequence: 1,
                    result_id: None,
                },
                true,
            ),
            (AppEvent::GraphInvalidated, false),
            (AppEvent::WorkspaceInvalidated, false),
            (AppEvent::CurrentSceneChanged("Fixture".into()), false),
        ];
        for (event, lifecycle) in cases {
            let (tx, rx) = std::sync::mpsc::sync_channel(1);
            let scope =
                SessionScope::initial(std::sync::Arc::new(std::sync::Mutex::new(None))).next();
            let sender = EventSender::new(tx).scoped(&scope);
            let AppEvent::SessionEvent { collection, .. } = sender.envelope(event.clone()) else {
                panic!("scoped event must have an envelope");
            };
            assert_eq!(collection.is_none(), lifecycle, "{event:?}");
            scope.advance_collection();
            assert_eq!(
                sender.send(event.clone()).await.is_ok(),
                lifecycle,
                "{event:?}"
            );
            assert_eq!(rx.try_recv().is_ok(), lifecycle, "{event:?}");
        }
    }

    #[tokio::test]
    async fn collection_change_rejects_prechange_snapshots_even_for_the_same_connection() {
        let (tx, rx) = std::sync::mpsc::sync_channel(4);
        let scope = SessionScope::initial(std::sync::Arc::new(std::sync::Mutex::new(None))).next();
        let sender = EventSender::new(tx).scoped(&scope);
        scope.advance_collection();
        assert!(sender.send(AppEvent::WorkspaceInvalidated).await.is_err());
        assert!(rx.try_recv().is_err());
        sender
            .fresh()
            .send(AppEvent::WorkspaceInvalidated)
            .await
            .unwrap();
        assert!(matches!(
            rx.try_recv(),
            Ok(AppEvent::SessionEvent {
                collection: Some(1),
                ..
            })
        ));
        scope.retire();
        assert!(sender
            .fresh()
            .send(AppEvent::WorkspaceInvalidated)
            .await
            .is_err());
        sender.send(AppEvent::Disconnected).await.unwrap();
    }

    #[tokio::test]
    async fn full_queue_yields_and_critical_event_is_delivered_after_capacity_returns() {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        tx.send(AppEvent::Connecting).unwrap();
        let sender = EventSender::new(tx);
        let task = tokio::spawn(async move { sender.send(AppEvent::Disconnected).await });
        tokio::task::yield_now().await;
        assert!(!task.is_finished());
        assert!(matches!(rx.try_recv(), Ok(AppEvent::Connecting)));
        tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(matches!(rx.try_recv(), Ok(AppEvent::Disconnected)));
    }

    #[tokio::test]
    async fn retired_publication_does_not_block_a_saturated_queue() {
        let (tx, _rx) = std::sync::mpsc::sync_channel(1);
        tx.send(AppEvent::Connecting).unwrap();
        let old = SessionScope::initial(std::sync::Arc::new(std::sync::Mutex::new(None)));
        let sender = EventSender::new(tx).scoped(&old);
        let task = tokio::spawn(async move { sender.send(AppEvent::Disconnected).await });
        let _new = old.next();
        assert!(tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .unwrap()
            .unwrap()
            .is_err());
    }
}
