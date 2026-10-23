//! OBS event stream boundary types.
//!
//! The controller consumes `ObsEvent` instead of raw `obws` event variants so
//! protocol-specific payloads remain contained in the OBS adapter layer.

use futures_util::StreamExt;

use crate::domain::audio::InputId;
use crate::domain::meter::InputLevels;
use crate::domain::output::OutputStatus;
use crate::domain::scene::SceneId;
use crate::obs::mapper;

/// Owned OBS event stream that yields normalized SceneDeck events.
pub(crate) struct ObsEventStream {
    inner: obws::events::EventStream,
}

impl ObsEventStream {
    /// Wrap a raw `obws` event stream.
    pub(crate) const fn new(inner: obws::events::EventStream) -> Self {
        Self { inner }
    }

    /// Re-subscribe only after a request proves that a stream gap was not a
    /// transport close. obws 0.15 reports broadcast lag as `None` too.
    pub(crate) async fn recover(
        &mut self,
        client: &crate::obs::client::ObsClient,
    ) -> Result<(), crate::infra::error::AppError> {
        client.get_version().await?;
        self.inner = client
            .inner
            .events()
            .map_err(crate::infra::error::AppError::connection)?;
        Ok(())
    }

    /// Wait for the next event. `None` requires liveness probing, not an
    /// unconditional disconnect, because the dependency also uses it for lag.
    pub(crate) async fn next(&mut self) -> Option<ObsEvent> {
        self.inner.next().await.map(mapper::map_event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use serde_json::{json, Value};
    use std::time::Duration;
    use tokio::sync::oneshot;
    use tokio_websockets::Message;

    #[tokio::test]
    async fn broadcast_lag_recovers_after_liveness_without_disconnect() {
        let (listener, port) = crate::obs::obs_fixture::listen().await;
        let (burst, begin_burst) = oneshot::channel();
        let (fresh, begin_fresh) = oneshot::channel();
        let (finish, finished) = oneshot::channel();
        let server = tokio::spawn(async move {
            let mut socket = crate::obs::obs_fixture::accept(listener).await;
            let mut begin_burst = Some(begin_burst);
            for index in 0..3 {
                let request: Value =
                    serde_json::from_str(socket.next().await.unwrap().unwrap().as_text().unwrap())
                        .unwrap();
                assert_eq!(request["d"]["requestType"], "GetVersion");
                socket.send(Message::text(json!({"op":7,"d":{"requestId":request["d"]["requestId"],"requestType":"GetVersion","requestStatus":{"result":true,"code":100},"responseData":crate::obs::obs_fixture::version(&[])}}).to_string())).await.unwrap();
                if index == 0 {
                    begin_burst.take().unwrap().await.unwrap();
                    for _ in 0..256 {
                        socket.send(Message::text(json!({"op":5,"d":{"eventType":"InputVolumeMeters","eventIntent":65536,"eventData":{"inputs":[]}}}).to_string())).await.unwrap();
                    }
                }
            }
            begin_fresh.await.unwrap();
            socket.send(Message::text(json!({"op":5,"d":{"eventType":"InputVolumeMeters","eventIntent":65536,"eventData":{"inputs":[]}}}).to_string())).await.unwrap();
            finished.await.unwrap();
        });
        let (client, mut events) = crate::obs::client::ObsClient::connect("127.0.0.1", port, None)
            .await
            .unwrap();
        burst.send(()).unwrap();
        client.get_version().await.unwrap();
        assert!(
            events.next().await.is_none(),
            "fixture must exercise actual pinned-library lag"
        );
        tokio::time::timeout(Duration::from_secs(2), events.recover(&client))
            .await
            .unwrap()
            .unwrap();
        fresh.send(()).unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), events.next())
                .await
                .unwrap(),
            Some(ObsEvent::InputLevelsUpdated(Vec::new()))
        );
        finish.send(()).unwrap();
        server.await.unwrap();
    }
}

/// OBS-originated event normalized before it reaches the controller.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ObsEvent {
    /// Streaming output state changed.
    StreamStatusUpdated(OutputStatus),
    /// Recording output state changed.
    RecordStatusUpdated(OutputStatus),
    /// OBS program scene changed.
    CurrentProgramSceneChanged(SceneId),
    /// An input's mute state changed.
    InputMuteChanged {
        /// OBS input id.
        input: InputId,
        /// New mute state.
        muted: bool,
    },
    /// An input's volume changed.
    InputVolumeChanged {
        /// OBS input id.
        input: InputId,
        /// Linear volume multiplier.
        volume_mul: f64,
        /// Volume in decibels.
        volume_db: f64,
    },
    /// Volume-meter readings for every input OBS is currently metering.
    ///
    /// High volume: OBS sends this about twenty times a second while
    /// connected, and every one is disposable — the next reading replaces it.
    InputLevelsUpdated(Vec<InputLevels>),
    /// Input inventory changed and the active scene audio list should refresh.
    InputsChanged,
    /// Scene item membership changed and scene-derived audio should refresh.
    SceneItemsChanged,
    /// Visibility affects audio membership, but does not change graph edges.
    SceneItemVisibilityChanged,
    /// Active OBS profile changed.
    CurrentProfileChanged(String),
    /// OBS profile list changed.
    ProfileListChanged(Vec<String>),
    /// Active OBS scene collection changed.
    CurrentSceneCollectionChanged(String),
    /// OBS scene collection list changed.
    SceneCollectionListChanged(Vec<String>),
    /// OBS scene inventory changed.
    SceneListChanged,
    /// Native operating controls changed; refresh the current scope.
    ControlsChanged,
    /// Studio mode changes invalidate the meaning of scene activation.
    StudioModeChanged(bool),
    VirtualCameraChanged(Option<bool>),
    /// OBS confirmed an actual replay file save.
    ReplaySaved(String),
    /// Collection is changing; requests must pause until change completes.
    CollectionChanging,
    /// Event not relevant to the current UI state.
    Ignored,
}
