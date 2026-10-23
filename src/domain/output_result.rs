//! Bounded output evidence. An OBS observation is never assigned to a requester.

use crate::domain::workflow::ScopeBinding;
use std::collections::VecDeque;

const LIMIT: usize = 24;
const CONFIRMATION_TIMEOUT_MS: u64 = 30_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputResultKind {
    ReplaySave,
    RecordingStop,
    RecordingSplit,
    RecordingChapter,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputResultStatus {
    Requested,
    Acknowledged,
    Observed,
    Failed,
    Unconfirmed,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputResult {
    pub id: u64,
    pub kind: OutputResultKind,
    pub scope: ScopeBinding,
    pub status: OutputResultStatus,
    pub requested_unix_ms: u64,
    pub updated_unix_ms: u64,
    pub path: Option<String>,
    pub detail: Option<String>,
}
#[derive(Debug, Clone, Default)]
pub struct OutputResults {
    next_id: u64,
    entries: VecDeque<OutputResult>,
}
impl OutputResults {
    pub fn entries(&self) -> impl DoubleEndedIterator<Item = &OutputResult> {
        self.entries.iter()
    }
    pub fn request(&mut self, kind: OutputResultKind, scope: ScopeBinding, now_ms: u64) -> u64 {
        self.push(kind, scope, OutputResultStatus::Requested, None, now_ms)
    }
    pub fn acknowledge(&mut self, id: u64, now_ms: u64) {
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == id) {
            if entry.status == OutputResultStatus::Requested {
                entry.status = OutputResultStatus::Acknowledged;
                entry.updated_unix_ms = now_ms;
            }
        }
    }
    pub fn fail(&mut self, id: u64, detail: String, now_ms: u64) {
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == id) {
            if matches!(
                entry.status,
                OutputResultStatus::Requested | OutputResultStatus::Acknowledged
            ) {
                entry.status = OutputResultStatus::Failed;
                entry.detail = Some(detail);
                entry.updated_unix_ms = now_ms;
            }
        }
    }
    pub fn observe(
        &mut self,
        kind: OutputResultKind,
        scope: ScopeBinding,
        path: Option<String>,
        now_ms: u64,
    ) -> u64 {
        self.push(kind, scope, OutputResultStatus::Observed, path, now_ms)
    }
    pub fn expire(&mut self, now_ms: u64) -> bool {
        let mut changed = false;
        for entry in &mut self.entries {
            if matches!(
                entry.status,
                OutputResultStatus::Requested | OutputResultStatus::Acknowledged
            ) && now_ms.saturating_sub(entry.requested_unix_ms) >= CONFIRMATION_TIMEOUT_MS
            {
                changed = true;
                entry.status = OutputResultStatus::Unconfirmed;
                entry.updated_unix_ms = now_ms;
            }
        }
        changed
    }
    pub fn invalidate(&mut self, now_ms: u64) {
        for entry in &mut self.entries {
            if matches!(
                entry.status,
                OutputResultStatus::Requested | OutputResultStatus::Acknowledged
            ) {
                entry.status = OutputResultStatus::Unconfirmed;
                entry.updated_unix_ms = now_ms;
            }
        }
    }
    fn push(
        &mut self,
        kind: OutputResultKind,
        scope: ScopeBinding,
        status: OutputResultStatus,
        path: Option<String>,
        now_ms: u64,
    ) -> u64 {
        self.next_id = self.next_id.wrapping_add(1);
        let id = self.next_id;
        self.entries.push_back(OutputResult {
            id,
            kind,
            scope,
            status,
            requested_unix_ms: now_ms,
            updated_unix_ms: now_ms,
            path,
            detail: None,
        });
        while self.entries.len() > LIMIT {
            self.entries.pop_front();
        }
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn observation_before_ack_is_preserved_without_false_request_correlation() {
        let mut results = OutputResults::default();
        let request = results.request(OutputResultKind::ReplaySave, ScopeBinding::default(), 1);
        let observed = results.observe(
            OutputResultKind::ReplaySave,
            ScopeBinding::default(),
            Some("/obs/replay.mkv".into()),
            2,
        );
        results.acknowledge(request, 3);
        let entries: Vec<_> = results.entries().collect();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].status, OutputResultStatus::Acknowledged);
        assert_eq!(entries[1].id, observed);
        assert_eq!(entries[1].status, OutputResultStatus::Observed);
        assert_eq!(entries[1].path.as_deref(), Some("/obs/replay.mkv"));
    }
    #[test]
    fn external_observations_and_same_paths_remain_independent_evidence() {
        let mut results = OutputResults::default();
        for time in 1..=2 {
            results.observe(
                OutputResultKind::ReplaySave,
                ScopeBinding::default(),
                Some("same.mkv".into()),
                time,
            );
        }
        assert_eq!(results.entries().count(), 2);
    }
    #[test]
    fn timeout_and_scope_loss_never_invent_saved_results() {
        let mut results = OutputResults::default();
        let first = results.request(OutputResultKind::ReplaySave, ScopeBinding::default(), 10);
        results.acknowledge(first, 20);
        results.expire(30_010);
        let second = results.request(
            OutputResultKind::RecordingChapter,
            ScopeBinding::default(),
            30_011,
        );
        results.invalidate(30_012);
        results.acknowledge(second, 30_013);
        assert!(results
            .entries()
            .all(|entry| entry.status == OutputResultStatus::Unconfirmed && entry.path.is_none()));
    }
    #[test]
    fn failed_request_retains_error_and_history_is_bounded() {
        let mut results = OutputResults::default();
        let id = results.request(OutputResultKind::RecordingSplit, ScopeBinding::default(), 1);
        results.fail(id, "Unsupported format".into(), 2);
        assert_eq!(
            results.entries().next().unwrap().detail.as_deref(),
            Some("Unsupported format")
        );
        for time in 3..100 {
            results.observe(
                OutputResultKind::RecordingStop,
                ScopeBinding::default(),
                None,
                time,
            );
        }
        assert_eq!(results.entries().count(), LIMIT);
    }
}
