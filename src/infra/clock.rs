//! Wall-clock reads shared by GTK-side state and the UI.

use std::time::{SystemTime, UNIX_EPOCH};

/// Milliseconds since the Unix epoch.
///
/// A clock set before the epoch reads as 0 and a value past `u64::MAX`
/// saturates, so callers always get a timestamp to store.
pub(crate) fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}
