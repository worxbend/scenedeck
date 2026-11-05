//! Local bookmark admission; recording evidence is sampled, never extrapolated.

use super::{AppState, ObsStatus};
use crate::domain::{
    bookmark::{valid_name, Bookmark, RecordingTime, BOOKMARK_LIMIT},
    output::OutputRunState,
};
use crate::storage::bookmarks::BookmarkArchive;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Default)]
pub(crate) struct BookmarksState {
    pub archive: BookmarkArchive,
    session: Option<(u64, u64, Instant)>,
    pub load_error: Option<String>,
    pub loading: bool,
    pub revision: u64,
    pub saved_revision: u64,
    pub pending: usize,
    pub save_error: Option<String>,
    pub failed_revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BookmarkError {
    Unavailable,
    InvalidName,
    Full,
}

impl BookmarksState {
    pub fn session_context(&self) -> Option<(u64, u64, Instant)> {
        self.session
    }

    pub fn begin_session(&mut self, now: Instant, unix_ms: u64) {
        self.session = Some((0, unix_ms, now));
        if self.load_error.is_some() {
            return;
        }
        let Some(next) = self.archive.next_session.checked_add(1) else {
            return;
        };
        self.session = Some((self.archive.next_session, unix_ms, now));
        self.archive.next_session = next;
    }

    pub fn end_session(&mut self) {
        self.session = None;
    }

    pub fn rename(&mut self, id: u64, name: &str) -> bool {
        if self.load_error.is_some() || !valid_name(name) {
            return false;
        }
        let Some(entry) = self.archive.entries.iter_mut().find(|entry| entry.id == id) else {
            return false;
        };
        entry.name = name.trim().to_owned();
        true
    }

    pub fn remove(&mut self, id: u64) -> bool {
        if self.load_error.is_some() {
            return false;
        }
        let before = self.archive.entries.len();
        self.archive.entries.retain(|entry| entry.id != id);
        self.archive.entries.len() != before
    }
}

impl AppState {
    /// Load failure leaves the original file untouched and local edits disabled.
    pub fn load_bookmarks(&mut self, result: Result<BookmarkArchive, String>) {
        self.bookmarks.loading = false;
        match result {
            Ok(archive) => {
                self.bookmarks.archive = archive;
                self.bookmarks.load_error = None;
                if matches!(self.obs_status, ObsStatus::Connected { .. }) {
                    let (_, unix_ms, at) = self.bookmarks.session.unwrap_or((
                        0,
                        crate::controller::workspace::unix_ms(),
                        Instant::now(),
                    ));
                    self.bookmarks.begin_session(at, unix_ms);
                }
            }
            Err(error) => self.bookmarks.load_error = Some(error),
        }
    }

    pub fn capture_bookmark(
        &mut self,
        name: &str,
        now: Instant,
        unix_ms: u64,
    ) -> Result<u64, BookmarkError> {
        if !valid_name(name) {
            return Err(BookmarkError::InvalidName);
        }
        let Some((session_id, session_started_unix_ms, start)) = self.bookmarks.session else {
            return Err(BookmarkError::Unavailable);
        };
        if session_id == 0
            || !matches!(self.obs_status, ObsStatus::Connected { .. })
            || self.bookmarks.load_error.is_some()
        {
            return Err(BookmarkError::Unavailable);
        }
        let (Some(connection_id), Some(endpoint)) =
            (&self.connected_connection_id, &self.connected_endpoint)
        else {
            return Err(BookmarkError::Unavailable);
        };
        if self.bookmarks.archive.entries.len() >= BOOKMARK_LIMIT {
            return Err(BookmarkError::Full);
        }
        let id = self.bookmarks.archive.next_id;
        let Some(next_id) = id.checked_add(1) else {
            return Err(BookmarkError::Full);
        };
        let recording = self
            .record_observed_at
            .and_then(|at| now.checked_duration_since(at))
            .filter(|age| *age <= Duration::from_secs(5) && !self.workspace.pending)
            .map(|age| {
                match (
                    self.record_status.active,
                    self.record_status.state,
                    self.record_status.duration_ms,
                ) {
                    (
                        true,
                        state @ (OutputRunState::Active | OutputRunState::Paused),
                        Some(duration_ms),
                    ) => RecordingTime::Observed {
                        duration_ms,
                        paused: state == OutputRunState::Paused,
                        age_ms: age.as_millis() as u64,
                    },
                    (false, OutputRunState::Inactive, _) => RecordingTime::Inactive,
                    _ => RecordingTime::Unavailable,
                }
            })
            .unwrap_or(RecordingTime::Unavailable);
        self.bookmarks.archive.entries.push(Bookmark {
            id,
            name: name.trim().to_owned(),
            created_unix_ms: unix_ms,
            session_id,
            session_started_unix_ms,
            session_elapsed_ms: now
                .saturating_duration_since(start)
                .as_millis()
                .min(u64::MAX as u128) as u64,
            connection_id: connection_id.clone(),
            endpoint: endpoint.clone(),
            collection: self.scene_collections.current.clone(),
            scene: self.scene_inventory.current_id.clone(),
            recording,
        });
        self.bookmarks.archive.next_id = next_id;
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        controller::event::AppEvent,
        domain::output::{OutputKind, OutputStatus},
    };

    fn connected(start: Instant) -> AppState {
        let mut state = AppState::new(Default::default(), Default::default(), None, None);
        state.obs_status = ObsStatus::Connected {
            obs_version: "fixture".into(),
        };
        state.connected_connection_id = Some("podcast".into());
        state.connected_endpoint = Some("localhost:4455".into());
        state.scene_collections.current = Some("Interview".into());
        state.scene_inventory.current_id = Some("Camera".into());
        state.bookmarks.begin_session(start, 1_000);
        state
    }

    fn observe(state: &mut AppState, at: Instant, run: OutputRunState, duration_ms: Option<u64>) {
        state.apply_output_event(
            &AppEvent::OutputStatusUpdated(
                OutputKind::Record,
                OutputStatus {
                    active: true,
                    state: run,
                    duration_ms,
                    detail: None,
                },
                at,
            ),
            1_000,
        );
    }

    #[test]
    fn bookmarks_keep_session_time_separate_from_paused_stale_and_restarted_output_time() {
        let start = Instant::now();
        let mut state = connected(start);
        observe(&mut state, start, OutputRunState::Active, Some(10_000));
        state.workspace.dirty = false;
        let activity = state.workspace.activity.entries.len();
        observe(
            &mut state,
            start + Duration::from_secs(1),
            OutputRunState::Active,
            Some(11_000),
        );
        assert!(
            !state.workspace.dirty,
            "clock ticks are not output state changes"
        );
        assert_eq!(state.workspace.activity.entries.len(), activity);
        observe(
            &mut state,
            start + Duration::from_secs(10),
            OutputRunState::Paused,
            Some(12_000),
        );
        state
            .capture_bookmark(
                " Paused interview ",
                start + Duration::from_secs(11),
                12_000,
            )
            .unwrap();
        let note = &state.bookmarks.archive.entries[0];
        assert_eq!(note.name, "Paused interview");
        assert_eq!(note.session_elapsed_ms, 11_000);
        assert_eq!(
            note.recording,
            RecordingTime::Observed {
                duration_ms: 12_000,
                paused: true,
                age_ms: 1_000
            }
        );
        state
            .capture_bookmark("Old sample", start + Duration::from_secs(16), 17_000)
            .unwrap();
        assert_eq!(
            state.bookmarks.archive.entries[1].recording,
            RecordingTime::Unavailable
        );
        observe(
            &mut state,
            start + Duration::from_secs(20),
            OutputRunState::Active,
            None,
        );
        observe(
            &mut state,
            start + Duration::from_secs(19),
            OutputRunState::Paused,
            Some(12_000),
        );
        state
            .capture_bookmark("Transport restart", start + Duration::from_secs(21), 22_000)
            .unwrap();
        assert_eq!(
            state.bookmarks.archive.entries[2].recording,
            RecordingTime::Unavailable
        );
        observe(
            &mut state,
            start + Duration::from_secs(22),
            OutputRunState::Active,
            Some(500),
        );
        state.workspace.pending = true;
        state
            .capture_bookmark("Changing controls", start + Duration::from_secs(23), 24_000)
            .unwrap();
        assert_eq!(
            state.bookmarks.archive.entries[3].recording,
            RecordingTime::Unavailable
        );
        assert_eq!(
            state.bookmarks.archive.entries[0].recording,
            RecordingTime::Observed {
                duration_ms: 12_000,
                paused: true,
                age_ms: 1_000
            }
        );
    }

    #[test]
    fn bookmarks_preserve_context_across_collection_changes_disconnect_and_archive_reload() {
        let start = Instant::now();
        let mut state = connected(start);
        state.capture_bookmark("Interview", start, 1_000).unwrap();
        state.invalidate_collection_runtime();
        state.scene_collections.current = Some("Gaming".into());
        state
            .capture_bookmark("Game", start + Duration::from_secs(1), 2_000)
            .unwrap();
        assert_eq!(
            state.bookmarks.archive.entries[0].collection.as_deref(),
            Some("Interview")
        );
        assert_eq!(
            state.bookmarks.archive.entries[1].collection.as_deref(),
            Some("Gaming")
        );
        assert_eq!(
            state.bookmarks.archive.entries[0].session_id,
            state.bookmarks.archive.entries[1].session_id
        );
        let archive = state.bookmarks.archive.clone();
        state.reset_obs_session(ObsStatus::Disconnected);
        assert_eq!(
            state.capture_bookmark("Disconnected", start, 2_000),
            Err(BookmarkError::Unavailable)
        );
        state = connected(start);
        state.load_bookmarks(Ok(archive));
        state.capture_bookmark("Reconnected", start, 3_000).unwrap();
        assert_ne!(
            state.bookmarks.archive.entries[0].session_id,
            state.bookmarks.archive.entries[2].session_id
        );
        assert_eq!(state.bookmarks.archive.entries[2].id, 3);
        assert!(state.bookmarks.rename(1, "Edited"));
        assert!(!state.bookmarks.rename(1, "bad\nname"));
        assert!(state.bookmarks.remove(2));
        assert!(!state.bookmarks.remove(2));
        state.load_bookmarks(Err("corrupt".into()));
        assert!(!state.bookmarks.rename(1, "Must not overwrite"));
        assert_eq!(
            state.capture_bookmark("Blocked", start, 4_000),
            Err(BookmarkError::Unavailable)
        );
    }

    #[test]
    fn bookmarks_load_retry_keeps_connection_clock_and_capacity_never_evicts_notes() {
        let start = Instant::now();
        let mut state = connected(start);
        state.bookmarks.load_error = Some("unreadable".into());
        state.bookmarks.begin_session(start, 1_000);
        state.load_bookmarks(Ok(BookmarkArchive::default()));
        state
            .capture_bookmark("Recovered", start + Duration::from_secs(10), 11_000)
            .unwrap();
        assert_eq!(
            state.bookmarks.archive.entries[0].session_elapsed_ms,
            10_000
        );
        assert_eq!(
            state.bookmarks.archive.entries[0].session_started_unix_ms,
            1_000
        );
        let first = state.bookmarks.archive.entries[0].clone();
        state
            .bookmarks
            .archive
            .entries
            .resize(BOOKMARK_LIMIT, first.clone());
        let next = state.bookmarks.archive.next_id;
        assert_eq!(
            state.capture_bookmark("Full", start, 1_000),
            Err(BookmarkError::Full)
        );
        assert_eq!(state.bookmarks.archive.next_id, next);
        assert_eq!(state.bookmarks.archive.entries[0], first);
        assert_eq!(
            state.capture_bookmark(" ", start, 1_000),
            Err(BookmarkError::InvalidName)
        );
    }
}
