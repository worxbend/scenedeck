//! Operator notes with separate local session time and observed OBS output time.

use serde::{Deserialize, Serialize};

/// Timing evidence captured with a bookmark. Never an encoded-file offset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum RecordingTime {
    Unavailable,
    Inactive,
    Observed {
        duration_ms: u64,
        paused: bool,
        age_ms: u64,
    },
}

/// An explicitly created local note; carries its original production context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Bookmark {
    pub id: u64,
    pub name: String,
    pub created_unix_ms: u64,
    pub session_id: u64,
    pub session_started_unix_ms: u64,
    pub session_elapsed_ms: u64,
    pub connection_id: String,
    pub endpoint: String,
    pub collection: Option<String>,
    pub scene: Option<String>,
    pub recording: RecordingTime,
}

pub(crate) const BOOKMARK_LIMIT: usize = 2_000;
pub(crate) const NAME_LIMIT: usize = 120;

/// Reject empty/oversized notes without silently changing their identity.
pub(crate) fn valid_name(name: &str) -> bool {
    !name.trim().is_empty()
        && name.chars().count() <= NAME_LIMIT
        && !name.chars().any(char::is_control)
}
