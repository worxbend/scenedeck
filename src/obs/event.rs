//! OBS event stream boundary types.
//!
//! The controller consumes `ObsEvent` instead of raw `obws` event variants so
//! protocol-specific payloads remain contained in the OBS adapter layer.

use futures_util::StreamExt;

use crate::domain::audio::InputId;
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

    /// Wait for the next normalized OBS event.
    pub(crate) async fn next(&mut self) -> Option<ObsEvent> {
        self.inner.next().await.map(mapper::map_event)
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
    /// Input inventory changed and the active scene audio list should refresh.
    InputsChanged,
    /// Scene item membership changed and scene-derived audio should refresh.
    SceneItemsChanged,
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
    /// Event not relevant to the current UI state.
    Ignored,
}
