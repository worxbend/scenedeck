//! Desired and observed disk history stay separate; reload preserves local intent.

use super::AppState;
use crate::{domain::session_report::SessionReport, storage::report_history::HistoryArchive};
use std::{collections::BTreeSet, sync::Arc, time::Instant};

#[derive(Debug, Clone)]
pub(crate) struct ReportHistoryState {
    pub archive: HistoryArchive,
    pub stored: Arc<HistoryArchive>,
    pub load_error: Option<String>,
    pub save_error: Option<String>,
    pub settings_error: Option<String>,
    pub settings_pending: bool,
    pub exit_abandoned: bool,
    pub busy: bool,
    pub revision: u64,
    pub saved_revision: u64,
    pub force_after_pending: bool,
    pub last_attempt: Option<Instant>,
    pub urgent: bool,
    pub clear_pending: bool,
    pub eligible: BTreeSet<String>,
    pub owned: BTreeSet<String>,
    collected_revision: u64,
}

impl Default for ReportHistoryState {
    fn default() -> Self {
        Self {
            archive: HistoryArchive::default(),
            stored: Arc::new(HistoryArchive::default()),
            load_error: None,
            save_error: None,
            settings_error: None,
            settings_pending: false,
            exit_abandoned: false,
            busy: false,
            revision: 0,
            saved_revision: 0,
            force_after_pending: false,
            last_attempt: None,
            urgent: false,
            clear_pending: false,
            eligible: BTreeSet::new(),
            owned: BTreeSet::new(),
            collected_revision: 0,
        }
    }
}

impl AppState {
    pub(crate) fn load_report_history(&mut self, result: Result<HistoryArchive, String>) {
        let history = &mut self.report_history;
        let loaded = match result {
            Ok(archive) => archive,
            Err(error) => {
                history.load_error = Some(error);
                return;
            }
        };
        let mut merged = loaded.clone();
        for report in &mut merged.entries {
            if report.ended_unix_ms.is_none() {
                report.interrupted = true;
            }
        }
        for report in history
            .archive
            .entries
            .iter()
            .filter(|report| history.owned.contains(&report.id))
        {
            if let Err(error) = merged.upsert(report.clone()) {
                history.load_error = Some(error.to_string());
                return;
            }
        }
        if history.clear_pending {
            merged = history.archive.clone();
        }
        if self.config.session_history.enabled {
            merged.retain(self.config.session_history.limit());
        }
        history.stored = Arc::new(loaded);
        history.archive = merged;
        history.load_error = None;
        history.revision += 1;
        if history.archive == *history.stored {
            history.saved_revision = history.revision;
        }
        history.urgent = true;
    }

    pub(crate) fn stage_report_history(&mut self) {
        if !self.config.session_history.enabled {
            return;
        }
        let history = &mut self.report_history;
        if history.collected_revision == self.reports.revision {
            return;
        }
        let mut next = history.archive.clone();
        for report in self
            .reports
            .entries
            .iter()
            .filter(|report| history.eligible.contains(&report.id))
        {
            if let Err(error) = next.upsert(report.clone()) {
                history.save_error = Some(error.to_string());
                return;
            }
        }
        next.retain(self.config.session_history.limit());
        history.collected_revision = self.reports.revision;
        if next != history.archive {
            history.archive = next;
            history.revision += 1;
        }
        history.owned.retain(|id| {
            history
                .archive
                .entries
                .iter()
                .any(|report| &report.id == id)
                || self.reports.entries.iter().any(|report| &report.id == id)
        });
        history.eligible.retain(|id| {
            history
                .archive
                .entries
                .iter()
                .any(|report| &report.id == id)
                || self
                    .reports
                    .entries
                    .iter()
                    .any(|report| &report.id == id && report.ended_unix_ms.is_none())
        });
    }

    pub(crate) fn configure_report_history(&mut self, enabled: bool, retain: usize) {
        self.config.session_history.enabled = enabled;
        self.config.session_history.retain_sessions =
            retain.clamp(1, crate::domain::session_report::HISTORY_LIMIT);
        self.report_history.eligible.clear();
        if enabled {
            if let Some(report) = self
                .reports
                .entries
                .back()
                .filter(|report| report.ended_unix_ms.is_none())
            {
                self.report_history.eligible.insert(report.id.clone());
                self.report_history.owned.insert(report.id.clone());
            }
            self.report_history.collected_revision = u64::MAX;
            self.stage_report_history();
            let before = self.report_history.archive.entries.len();
            self.report_history
                .archive
                .retain(self.config.session_history.limit());
            if before != self.report_history.archive.entries.len() {
                self.report_history.revision += 1;
            }
        }
        self.report_history.urgent = true;
    }

    pub(crate) fn clear_report_history(&mut self) {
        let history = &mut self.report_history;
        history.archive = HistoryArchive::default();
        history.clear_pending = true;
        history.eligible.clear();
        history.owned.clear();
        history.collected_revision = self.reports.revision;
        history.revision += 1;
        history.urgent = true;
    }

    pub(crate) fn report_list(&self) -> Vec<&SessionReport> {
        self.report_history
            .archive
            .entries
            .iter()
            .filter(|old| {
                !self
                    .reports
                    .entries
                    .iter()
                    .any(|recent| old.id == recent.id)
            })
            .chain(self.reports.entries.iter())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        controller::{event::ConnectionInfo, state::ObsStatus},
        domain::{session_report::HistoryPolicy, workflow::ActivityOrigin},
    };

    fn state() -> AppState {
        AppState::new(Default::default(), Default::default(), None, None)
    }
    fn connect(state: &mut AppState, name: &str) {
        state.apply_connected(ConnectionInfo {
            connection_id: name.into(),
            endpoint: "fixture:4455".into(),
            obs_version: "fixture".into(),
            websocket_version: "5".into(),
            available_requests: Vec::new(),
            supported_image_formats: Vec::new(),
        });
    }

    #[test]
    fn policy_defaults_off_and_limits_external_retention_values() {
        let config: crate::storage::config::AppConfig = serde_json::from_str("{}").unwrap();
        assert!(!config.session_history.enabled);
        assert_eq!(config.session_history.limit(), 8);
        assert_eq!(
            HistoryPolicy {
                enabled: true,
                retain_sessions: 0
            }
            .limit(),
            1
        );
        assert_eq!(
            HistoryPolicy {
                enabled: true,
                retain_sessions: usize::MAX
            }
            .limit(),
            32
        );
    }

    #[test]
    fn enabling_saves_current_and_future_sessions_and_disabling_keeps_saved_snapshots() {
        let mut state = state();
        connect(&mut state, "earlier");
        state.reset_obs_session(ObsStatus::Disconnected);
        connect(&mut state, "current");
        state.stage_report_history();
        assert!(state.report_history.archive.entries.is_empty());
        state.configure_report_history(true, 2);
        assert_eq!(state.report_history.archive.entries.len(), 1);
        assert_eq!(
            state.report_history.archive.entries[0].connection_id,
            "current"
        );
        let captured = state.report_history.archive.clone();
        state.configure_report_history(false, 2);
        state
            .workspace
            .log(ActivityOrigin::Observed, "Not saved while disabled".into());
        state.collect_report(Instant::now(), 10);
        state.stage_report_history();
        assert_eq!(state.report_history.archive, captured);
        state.reset_obs_session(ObsStatus::Disconnected);
        connect(&mut state, "next");
        state.stage_report_history();
        assert_eq!(state.report_history.archive, captured);
        state.configure_report_history(true, 1);
        assert_eq!(state.report_history.archive.entries.len(), 1);
        assert_eq!(
            state.report_history.archive.entries[0].connection_id,
            "next"
        );
        state.reports.revision += 1;
        state.stage_report_history();
        assert_eq!(
            state.report_history.archive.entries[0].connection_id,
            "next"
        );
    }

    #[test]
    fn reload_marks_incomplete_sessions_without_inventing_end_time_and_preserves_local_intent() {
        let mut old = state();
        old.configure_report_history(true, 3);
        connect(&mut old, "old");
        old.stage_report_history();
        let disk = old.report_history.archive.clone();
        let mut new = state();
        new.load_report_history(Ok(disk.clone()));
        assert!(new.report_history.archive.entries[0].interrupted);
        assert_eq!(new.report_history.archive.entries[0].ended_unix_ms, None);
        assert_eq!(*new.report_history.stored, disk);
        new.configure_report_history(true, 3);
        connect(&mut new, "new");
        new.stage_report_history();
        new.load_report_history(Err("broken".into()));
        assert!(new.report_history.load_error.is_some());
        assert_eq!(new.report_list().len(), 2);
        new.load_report_history(Ok(disk));
        assert!(new.report_history.load_error.is_none());
        assert_eq!(new.report_history.archive.entries.len(), 2);
        assert_ne!(
            new.report_history.archive.entries[0].id,
            new.report_history.archive.entries[1].id
        );
    }

    #[test]
    fn clear_survives_reload_and_does_not_resave_the_current_connection() {
        let mut state = state();
        state.configure_report_history(true, 3);
        connect(&mut state, "current");
        state.stage_report_history();
        let disk = state.report_history.archive.clone();
        state.clear_report_history();
        state.load_report_history(Ok(disk));
        assert!(state.report_history.archive.entries.is_empty());
        state.reports.revision += 1;
        state.stage_report_history();
        assert!(state.report_history.archive.entries.is_empty());
        assert_eq!(
            state.report_list().len(),
            1,
            "explicit clear keeps in-memory review available"
        );
        state.reset_obs_session(ObsStatus::Disconnected);
        connect(&mut state, "future");
        state.stage_report_history();
        assert_eq!(state.report_history.archive.entries.len(), 1);
        assert_eq!(
            state.report_history.archive.entries[0].connection_id,
            "future"
        );
    }
}
