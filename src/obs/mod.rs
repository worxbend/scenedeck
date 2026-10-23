//! OBS WebSocket adapter layer.
//!
//! Controller code should use `ObsClient` and `ObsEventStream` instead of
//! importing raw `obws` response or event types outside this boundary.

mod audio;
mod requests;
mod scene_activation;
mod sources;

pub(crate) mod client;
pub(crate) mod event;
pub(crate) mod mapper;
