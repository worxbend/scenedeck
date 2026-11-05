//! Shared connection scopes and bounded factual OBS activity.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeBinding {
    pub endpoint: String,
    pub collection: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityOrigin {
    Requested,
    Observed,
    Failed,
    Connection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityRecord {
    pub timestamp_unix_ms: u64,
    pub origin: ActivityOrigin,
    pub description: String,
}

#[derive(Debug, Clone, Default)]
pub struct ActivityLog {
    pub entries: VecDeque<ActivityRecord>,
    sequence: u64,
}

impl ActivityLog {
    pub const LIMIT: usize = 200;
    pub fn push(&mut self, entry: ActivityRecord) {
        self.sequence = self.sequence.saturating_add(1);
        while self.entries.len() >= Self::LIMIT {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    /// Monotonic receipt cursor; lets longer-lived reports detect ring-buffer gaps.
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn numbered(&self) -> impl Iterator<Item = (u64, &ActivityRecord)> {
        let first = self.sequence.saturating_sub(self.entries.len() as u64) + 1;
        self.entries
            .iter()
            .enumerate()
            .map(move |(index, entry)| (first + index as u64, entry))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn activity_evicts_oldest_entry() {
        let mut log = ActivityLog::default();
        for timestamp in 0..205 {
            log.push(ActivityRecord {
                timestamp_unix_ms: timestamp,
                origin: ActivityOrigin::Observed,
                description: String::new(),
            });
        }
        assert_eq!(log.entries.len(), 200);
        assert_eq!(log.entries.front().unwrap().timestamp_unix_ms, 5);
    }
}
