//! Shared connection scopes and bounded factual OBS activity.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeBinding {
    pub endpoint: String,
    pub collection: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityOrigin {
    Requested,
    Observed,
    Failed,
    Connection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityRecord {
    pub timestamp_unix_ms: u64,
    pub origin: ActivityOrigin,
    pub description: String,
}

#[derive(Debug, Clone, Default)]
pub struct ActivityLog {
    pub entries: VecDeque<ActivityRecord>,
}

impl ActivityLog {
    pub const LIMIT: usize = 200;
    pub fn push(&mut self, entry: ActivityRecord) {
        while self.entries.len() >= Self::LIMIT {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
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
