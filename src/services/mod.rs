//! Pure application services.
//!
//! Services consume domain types and contain validation or derivation logic that
//! should stay independent of GTK widgets and `obws` response structs.

pub(crate) mod audio_service;
pub(crate) mod doctor_service;
pub(crate) mod graph_service;
pub(crate) mod scene_service;
