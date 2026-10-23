//! Workspace reconciliation without widget ownership. Shell effects follow state publication.

use crate::controller::state::AppState;
use crate::controller::workspace::{unix_ms, WorkspaceObservation};
use crate::domain::control::SourceDetail;
use crate::infra::i18n::LANGUAGE_LOADER;
use i18n_embed_fl::fl;
use std::time::Instant;

pub(super) struct WorkspaceCompletion {
    pub epoch: u64,
    pub sequence: u64,
    pub result_id: Option<u64>,
    pub acknowledged: bool,
    pub uncertain: bool,
    pub result: Result<Box<WorkspaceObservation>, String>,
    pub action: bool,
}

pub(super) fn acknowledge(state: &mut AppState, epoch: u64, sequence: u64, result_id: Option<u64>) {
    if !state.workspace.accepts(epoch, sequence) {
        return;
    }
    state.workspace.pending = false;
    state.workspace.pending_scene = None;
    if let Some(id) = result_id {
        state.workspace.output_results.acknowledge(id, unix_ms());
    }
    state.workspace.result = fl!(LANGUAGE_LOADER, "workspace-action-acknowledged");
    state.workspace.revision = state.workspace.revision.wrapping_add(1);
}

pub(super) fn media_loaded(
    state: &mut AppState,
    epoch: u64,
    sequence: u64,
    input: String,
    result: Result<SourceDetail, String>,
) {
    if !state.workspace.accepts(epoch, sequence)
        || state.workspace.selected_input.as_deref() != Some(&input)
        || state.workspace.pending
        || state.workspace.dirty
    {
        return;
    }
    match result {
        Ok(detail) => state.workspace.detail = Some(detail),
        Err(error) => state.workspace.fail_read(error),
    }
    state.workspace.revision = state.workspace.revision.wrapping_add(1);
}

/// Returns true only when the shell should redraw scenes, Record, Stream, then Doctor.
/// Mutate the live state in place: never clone/replace AppState or its version fences.
pub(super) fn complete_workspace(state: &mut AppState, event: WorkspaceCompletion) -> bool {
    let WorkspaceCompletion {
        epoch,
        sequence,
        result_id,
        acknowledged,
        uncertain,
        result,
        action,
    } = event;
    if !state.workspace.accepts(epoch, sequence) {
        return false;
    }
    state.workspace.loading = false;
    state.workspace.pending = false;
    state.workspace.pending_scene = None;
    if acknowledged {
        if let Some(id) = result_id {
            state.workspace.output_results.acknowledge(id, unix_ms());
        }
    }
    match result {
        Ok(mut observed) => {
            // A workspace read may have begun before a toggle's transport echo.
            // Never animate a control back to that older readback value.
            if let Some((enabled, at)) = state.workspace.scene_control_observation {
                if at > observed.observed_at {
                    observed.snapshot.studio.enabled = Some(enabled);
                }
            }
            if let Some((active, at)) = state.workspace.virtual_camera_observation {
                if at > observed.observed_at {
                    observed.snapshot.virtual_camera_active = active;
                }
            }
            if let Some(id) = result_id {
                state.workspace.output_results.acknowledge(id, unix_ms());
            }
            if observed.screenshot.is_some() {
                state.workspace.screenshot_scene = observed.snapshot.scene.clone();
                state.workspace.screenshot_observed_at = Some(observed.observed_at);
            }
            if !state.workspace.dirty {
                if let Some(status) = &observed.record_status {
                    update_active_since(status.active, &mut state.record_active_since);
                    state.set_record_status(status.clone());
                }
                if let Some(status) = &observed.stream_status {
                    update_active_since(status.active, &mut state.stream_active_since);
                    state.set_stream_status(status.clone());
                }
            }
            state.workspace.snapshot = Some(observed.snapshot);
            state.workspace.detail = observed.detail;
            state.workspace.usage = observed.usage;
            state.workspace.audio = observed.audio;
            state.workspace.observed_at = Some(observed.observed_at);
            if !state.workspace.dirty {
                state.workspace.observe_global_controls();
            }
            state.workspace.warnings = observed.warnings;
            if observed.screenshot.is_some() {
                state.workspace.screenshot = observed.screenshot;
            }
            state.workspace.error = None;
            if action && result_id.is_none() {
                state.workspace.result = fl!(LANGUAGE_LOADER, "workspace-action-acknowledged");
            }
        }
        Err(error) => {
            if uncertain {
                state.workspace.output_results.invalidate(unix_ms());
            } else if !acknowledged {
                if let Some(id) = result_id {
                    state
                        .workspace
                        .output_results
                        .fail(id, error.clone(), unix_ms());
                }
            }
            state.workspace.log(
                crate::domain::workflow::ActivityOrigin::Failed,
                error.clone(),
            );
            state.workspace.fail_read(error);
        }
    }
    state.workspace.active_action = None;
    state.workspace.revision = state.workspace.revision.wrapping_add(1);
    true
}

pub(super) fn audio_failed(state: &mut AppState, error: String) {
    state
        .workspace
        .log(crate::domain::workflow::ActivityOrigin::Failed, error);
    state.workspace.dirty = true;
    state.workspace.revision = state.workspace.revision.wrapping_add(1);
}

pub(super) fn replay_saved(state: &mut AppState, path: String) {
    let scope = state.workflow_scope();
    state.workspace.output_results.observe(
        crate::domain::output_result::OutputResultKind::ReplaySave,
        scope,
        Some(path.clone()),
        unix_ms(),
    );
    state.workspace.result = fl!(
        LANGUAGE_LOADER,
        "workspace-replay-observed",
        path = path.clone()
    );
    state.workspace.log(
        crate::domain::workflow::ActivityOrigin::Observed,
        fl!(LANGUAGE_LOADER, "workspace-replay-observed", path = path),
    );
    state.workspace.dirty = true;
    state.workspace.revision = state.workspace.revision.wrapping_add(1);
}

pub(super) fn update_active_since(active: bool, active_since: &mut Option<Instant>) {
    match (active, active_since.is_some()) {
        (true, false) => *active_since = Some(Instant::now()),
        (false, true) => *active_since = None,
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::output::OutputStatus;
    use crate::domain::output_result::{OutputResultKind, OutputResultStatus};

    fn state() -> AppState {
        AppState::new(Default::default(), Default::default(), None, None)
    }

    fn observation() -> Box<crate::controller::workspace::WorkspaceObservation> {
        Box::new(crate::controller::workspace::WorkspaceObservation {
            snapshot: Default::default(),
            detail: None,
            usage: vec![],
            audio: vec![],
            stream_status: Some(OutputStatus::new(
                true,
                crate::domain::output::OutputRunState::Active,
            )),
            record_status: Some(OutputStatus::new(
                true,
                crate::domain::output::OutputRunState::Active,
            )),
            screenshot: None,
            warnings: vec!["read warning".into()],
            observed_at: Instant::now(),
        })
    }

    fn completion(state: &AppState) -> WorkspaceCompletion {
        WorkspaceCompletion {
            epoch: state.workspace.epoch,
            sequence: state.workspace.sequence,
            result_id: None,
            acknowledged: false,
            uncertain: false,
            result: Ok(observation()),
            action: false,
        }
    }

    #[test]
    fn older_readback_cannot_roll_back_global_toggle_events_or_their_timestamps() {
        use crate::domain::control::GlobalToggle;
        for dirty in [false, true] {
            for camera in [None, Some(true)] {
                let mut state = state();
                let mut event = completion(&state);
                let observed = event.result.as_mut().unwrap();
                observed.observed_at = Instant::now() - std::time::Duration::from_secs(1);
                observed.snapshot.studio.enabled = Some(false);
                observed.snapshot.virtual_camera_active = Some(false);
                let latest = Instant::now();
                state.workspace.scene_control_observation = Some((true, latest));
                state.workspace.virtual_camera_observation = Some((camera, latest));
                state.workspace.dirty = dirty;
                assert!(complete_workspace(&mut state, event));
                let snapshot = state.workspace.snapshot.as_ref().unwrap();
                assert_eq!(snapshot.studio.enabled, Some(true));
                assert_eq!(snapshot.virtual_camera_active, camera);
                assert_eq!(
                    state.workspace.scene_control_observation,
                    Some((true, latest))
                );
                assert_eq!(
                    state.workspace.virtual_camera_observation,
                    Some((camera, latest))
                );
                assert_eq!(
                    state.workspace.toggle_value(GlobalToggle::VirtualCamera),
                    camera
                );
            }
        }
    }

    #[test]
    fn stale_workspace_completion_cannot_publish_state_or_request_shell_effects() {
        let mut state = state();
        state.workspace.loading = true;
        state.workspace.pending = true;
        let mut event = completion(&state);
        event.sequence = event.sequence.wrapping_add(1);
        assert!(!complete_workspace(&mut state, event));
        assert!(state.workspace.loading && state.workspace.pending);
        assert!(state.workspace.snapshot.is_none());
        assert_eq!(state.workspace.revision, 0);
    }

    #[test]
    fn dirty_completion_keeps_newer_output_observations_but_publishes_readback() {
        let mut state = state();
        state.workspace.dirty = true;
        state.workspace.loading = true;
        state.workspace.pending = true;
        let event = completion(&state);
        assert!(complete_workspace(&mut state, event));
        assert!(!state.stream_status.active && !state.record_status.active);
        assert!(state.stream_active_since.is_none() && state.record_active_since.is_none());
        assert!(state.workspace.snapshot.is_some());
        assert_eq!(state.workspace.warnings, ["read warning"]);
        assert!(!state.workspace.loading && !state.workspace.pending);
        assert!(state.workspace.dirty);
        assert_eq!(state.workspace.revision, 1);
    }

    #[test]
    fn acknowledgement_and_failed_readback_remain_distinct_from_observed_file_evidence() {
        for (acknowledged, uncertain, expected) in [
            (false, false, OutputResultStatus::Failed),
            (true, false, OutputResultStatus::Acknowledged),
            (false, true, OutputResultStatus::Unconfirmed),
        ] {
            let mut state = state();
            let scope = state.workflow_scope();
            let id = state
                .workspace
                .output_results
                .request(OutputResultKind::ReplaySave, scope, 1);
            let mut event = completion(&state);
            event.result_id = Some(id);
            event.acknowledged = acknowledged;
            event.uncertain = uncertain;
            event.result = Err("readback failed".into());
            assert!(complete_workspace(&mut state, event));
            let result = state.workspace.output_results.entries().next().unwrap();
            assert_eq!(result.status, expected);
            assert!(result.path.is_none());
            assert_eq!(state.workspace.error.as_deref(), Some("readback failed"));
            assert!(state.workspace.observed_at.is_none());
        }
    }

    #[test]
    fn acknowledgement_is_scope_fenced_and_replay_file_remains_independent() {
        let mut state = state();
        state.workspace.pending = true;
        let scope = state.workflow_scope();
        let id = state
            .workspace
            .output_results
            .request(OutputResultKind::ReplaySave, scope, 1);
        let epoch = state.workspace.epoch;
        let sequence = state.workspace.sequence;
        acknowledge(&mut state, epoch.wrapping_add(1), sequence, Some(id));
        assert!(state.workspace.pending);
        assert_eq!(state.workspace.revision, 0);
        acknowledge(&mut state, epoch, sequence, Some(id));
        assert!(!state.workspace.pending);
        replay_saved(&mut state, "/obs/replay.mkv".into());
        let entries: Vec<_> = state.workspace.output_results.entries().collect();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].status, OutputResultStatus::Acknowledged);
        assert!(entries[0].path.is_none());
        assert_eq!(entries[1].status, OutputResultStatus::Observed);
        assert_eq!(entries[1].path.as_deref(), Some("/obs/replay.mkv"));
        assert!(state.workspace.dirty);
        assert_eq!(state.workspace.revision, 2);
    }

    #[test]
    fn clean_completion_publishes_output_clocks_and_preserves_prior_screenshot() {
        let mut state = state();
        let mut event = completion(&state);
        let observed = event.result.as_mut().unwrap();
        observed.screenshot = Some("image".into());
        observed.snapshot.scene = Some("Scene".into());
        let observed_at = observed.observed_at;
        assert!(complete_workspace(&mut state, event));
        assert!(state.stream_status.active && state.record_status.active);
        assert!(state.stream_active_since.is_some() && state.record_active_since.is_some());
        assert_eq!(state.workspace.screenshot.as_deref(), Some("image"));
        assert_eq!(state.workspace.screenshot_scene.as_deref(), Some("Scene"));
        assert_eq!(state.workspace.screenshot_observed_at, Some(observed_at));
        let event = completion(&state);
        assert!(complete_workspace(&mut state, event));
        assert_eq!(state.workspace.screenshot.as_deref(), Some("image"));
        assert_eq!(state.workspace.screenshot_observed_at, Some(observed_at));
        audio_failed(&mut state, "audio failed".into());
        assert!(state.workspace.dirty);
        assert_eq!(state.workspace.revision, 3);
    }

    #[test]
    fn media_reply_requires_current_selection_and_clean_idle_workspace() {
        for (input, pending, dirty, accepted) in [
            ("Mic", false, false, true),
            ("Other", false, false, false),
            ("Mic", true, false, false),
            ("Mic", false, true, false),
        ] {
            let mut state = state();
            state.workspace.selected_input = Some("Mic".into());
            state.workspace.pending = pending;
            state.workspace.dirty = dirty;
            let epoch = state.workspace.epoch;
            let sequence = state.workspace.sequence;
            media_loaded(
                &mut state,
                epoch,
                sequence,
                input.into(),
                Ok(Default::default()),
            );
            assert_eq!(state.workspace.detail.is_some(), accepted);
            assert_eq!(state.workspace.revision, u64::from(accepted));
        }
    }
}
