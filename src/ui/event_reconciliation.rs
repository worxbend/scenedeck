//! Workspace reconciliation without widget ownership. Shell effects follow state publication.

use crate::controller::state::update_active_since;
use crate::controller::state::AppState;
use crate::controller::workspace::{unix_ms, WorkspaceObservation};
use crate::domain::control::SourceDetail;
use crate::infra::i18n::LANGUAGE_LOADER;
use i18n_embed_fl::fl;

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
    state.workspace.touch();
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
    state.workspace.touch();
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
            state.merge_workspace_audio(&mut observed);
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
                use crate::domain::output::{OutputKind, OutputRunState, OutputStatus};
                for (kind, status, at) in [
                    (
                        OutputKind::Stream,
                        observed.stream_status.as_ref(),
                        observed.stream_observed_at,
                    ),
                    (
                        OutputKind::Record,
                        observed.record_status.as_ref(),
                        observed.record_observed_at,
                    ),
                ] {
                    if !state.accept_output_observation(kind, at) {
                        continue;
                    }
                    if let Some(status) = status {
                        state.health.observe_output(kind, status, at);
                        match kind {
                            OutputKind::Stream => {
                                update_active_since(status.active, &mut state.stream_active_since)
                            }
                            OutputKind::Record => {
                                update_active_since(status.active, &mut state.record_active_since)
                            }
                        }
                    } else {
                        state.health.output_unobserved(kind);
                    }
                    let status = status
                        .cloned()
                        .unwrap_or_else(|| OutputStatus::new(false, OutputRunState::Unknown));
                    match kind {
                        OutputKind::Stream => state.set_stream_status(status),
                        OutputKind::Record => state.set_record_status(status),
                    }
                }
            }
            state.workspace.snapshot = Some(observed.snapshot);
            state.workspace.detail = observed.detail;
            state.workspace.usage = observed.usage;
            state.workspace.audio = observed.audio;
            state.workspace.input_catalog = Some(observed.input_catalog);
            state.workspace.operator_inputs = observed.operator_inputs;
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
    state.workspace.touch();
    true
}

pub(super) fn audio_failed(state: &mut AppState, error: String) {
    state
        .workspace
        .log(crate::domain::workflow::ActivityOrigin::Failed, error);
    state.workspace.mark_dirty();
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
    state.workspace.mark_dirty();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::output::OutputStatus;
    use crate::domain::output_result::{OutputResultKind, OutputResultStatus};
    use std::time::Instant;

    fn state() -> AppState {
        AppState::new(Default::default(), Default::default(), None, None)
    }

    fn observation() -> Box<crate::controller::workspace::WorkspaceObservation> {
        Box::new(crate::controller::workspace::WorkspaceObservation {
            audio_revision: 0,
            input_catalog: Ok(Vec::new()),
            operator_inputs: Vec::new(),
            snapshot: Default::default(),
            detail: None,
            usage: vec![],
            audio: vec![],
            stream_observed_at: Instant::now(),
            record_observed_at: Instant::now(),
            stream_status: Some(OutputStatus::new(
                true,
                crate::domain::output::OutputRunState::Active,
            )),
            record_status: Some(OutputStatus::new(
                true,
                crate::domain::output::OutputRunState::Active,
            )),
            screenshot: None,
            warnings: vec![crate::domain::control::WorkspaceWarning::InventoryTruncated],
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
    fn workspace_output_reads_share_event_fences_and_unknown_is_not_a_stop() {
        use crate::{
            controller::event::AppEvent,
            domain::health::AlertKind,
            domain::output::{OutputKind, OutputRunState},
        };
        let start = Instant::now() - std::time::Duration::from_secs(60);
        let mut state = state();
        state.health.connected();
        state.apply_output_event(
            &AppEvent::OutputStatusUpdated(
                OutputKind::Record,
                OutputStatus::active(),
                start + std::time::Duration::from_secs(20),
            ),
            0,
        );
        for (second, status) in [
            (10, Some(OutputStatus::inactive())),
            (11, None),
            (30, None),
            (25, Some(OutputStatus::inactive())),
            (40, Some(OutputStatus::inactive())),
        ] {
            state.workspace.dirty = false;
            let mut event = completion(&state);
            let read = event.result.as_mut().unwrap();
            read.record_status = status;
            read.record_observed_at = start + std::time::Duration::from_secs(second);
            assert!(complete_workspace(&mut state, event));
            match second {
                10 | 11 => {
                    assert!(state.record_status.active);
                    assert_eq!(state.health.output_activity(OutputKind::Record), Some(true));
                }
                30 | 25 => {
                    assert_eq!(state.record_status.state, OutputRunState::Unknown);
                    assert_eq!(state.health.output_activity(OutputKind::Record), None);
                }
                40 => {
                    assert!(!state.record_status.active);
                    assert_eq!(
                        state
                            .health
                            .alerts(Instant::now())
                            .iter()
                            .map(|alert| alert.kind)
                            .collect::<Vec<_>>(),
                        [AlertKind::RecordStopped]
                    );
                }
                _ => unreachable!(),
            }
            if second != 40 {
                assert!(state.health.alerts(Instant::now()).is_empty());
            }
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
        assert_eq!(state.workspace.revision(), 0);
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
        assert_eq!(
            state.workspace.warnings,
            [crate::domain::control::WorkspaceWarning::InventoryTruncated]
        );
        assert!(!state.workspace.loading && !state.workspace.pending);
        assert!(state.workspace.dirty);
        assert_eq!(state.workspace.revision(), 1);
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
        assert_eq!(state.workspace.revision(), 0);
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
        assert_eq!(state.workspace.revision(), 3, "logged failures render");
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
        assert_eq!(state.workspace.revision(), 4, "logged failures render");
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
            assert_eq!(state.workspace.revision(), u64::from(accepted));
        }
    }

    #[test]
    fn operator_observations_follow_workspace_fences_and_preserve_inspection_intent() {
        use crate::domain::{
            operator_action::ActionTarget,
            operator_input::{InputIdentity, InputObservation},
        };
        let mut state = state();
        state.workspace.selected_input = Some("Editor".into());
        let target = ActionTarget {
            name: "Clip".into(),
            uuid: Some("clip-1".into()),
        };
        let token = state.workspace.inspect_operator_input(target.clone());
        let mut event = completion(&state);
        let observed = event.result.as_mut().unwrap();
        observed.input_catalog = Ok(vec![InputIdentity {
            target: target.clone(),
            kind: "media".into(),
        }]);
        observed.operator_inputs = vec![InputObservation {
            target: target.clone(),
            media: Some(Err("unavailable".into())),
            filters: Some(Ok(Vec::new())),
        }];
        let mut stale = completion(&state);
        stale.epoch += 1;
        assert!(!complete_workspace(&mut state, stale));
        assert!(state.workspace.input_catalog.is_none());
        assert!(complete_workspace(&mut state, event));
        assert!(state.workspace.input_catalog.as_ref().unwrap().is_ok());
        assert!(state.workspace.operator_inputs[0]
            .media
            .as_ref()
            .unwrap()
            .is_err());
        assert_eq!(state.workspace.selected_input.as_deref(), Some("Editor"));
        assert!(
            state.workspace.dirty && !state.workspace.can_control(),
            "inspection added during a read still needs a fresh read"
        );
        state.workspace.invalidate();
        assert!(
            state.workspace.input_catalog.is_none() && state.workspace.operator_inputs.is_empty()
        );
        assert!(state.workspace.operator_inspections.is_empty());
        let next = state.workspace.inspect_operator_input(target);
        state.workspace.release_operator_inspection(token);
        assert_eq!(
            state.workspace.operator_inspections[0].0, next,
            "retired picker cleanup cannot release a new inspection"
        );
    }

    #[test]
    fn workspace_readback_retains_newer_mute_and_volume_echoes_for_pins_and_sources() {
        use crate::{
            controller::event::AppEvent,
            domain::{audio::AudioInput, control::SourceDetail},
        };
        let mut state = state();
        state.reconcile_observation(
            3,
            AppEvent::InputMuteChanged {
                input: "Mic".into(),
                muted: true,
            },
        );
        state.reconcile_observation(
            4,
            AppEvent::InputVolumeChanged {
                input: "Mic".into(),
                volume_mul: 0.5,
                volume_db: -6.0,
            },
        );
        for (revision, expected_muted, expected_db) in [(2, true, -6.0), (5, false, 0.0)] {
            let mut event = completion(&state);
            let observed = event.result.as_mut().unwrap();
            observed.audio_revision = revision;
            observed.audio = vec![AudioInput::new("Mic".into(), false, 1.0, 0.0)];
            observed.detail = Some(SourceDetail {
                input: "Mic".into(),
                muted: Some(false),
                volume_db: Some(0.0),
                ..Default::default()
            });
            assert!(complete_workspace(&mut state, event));
            assert_eq!(state.workspace.audio[0].muted, expected_muted);
            assert_eq!(state.workspace.audio[0].volume_db, expected_db);
            assert_eq!(
                state.workspace.detail.as_ref().unwrap().muted,
                Some(expected_muted)
            );
            assert_eq!(
                state.workspace.detail.as_ref().unwrap().volume_db,
                Some(expected_db)
            );
        }
    }

    #[test]
    fn lost_echo_coverage_leaves_workspace_audio_unknown_and_other_controls_intact() {
        use crate::{
            controller::event::AppEvent,
            domain::{audio::AudioInput, control::SourceDetail},
        };
        let mut state = state();
        for index in 0..=4096 {
            state.reconcile_observation(
                index + 2,
                AppEvent::InputMuteChanged {
                    input: format!("Mic{index}"),
                    muted: true,
                },
            );
        }
        let mut event = completion(&state);
        let observed = event.result.as_mut().unwrap();
        observed.audio_revision = 1;
        observed.audio = vec![AudioInput::new("Mic0".into(), false, 1.0, 0.0)];
        observed.detail = Some(SourceDetail {
            input: "Mic0".into(),
            muted: Some(false),
            volume_db: Some(0.0),
            ..Default::default()
        });
        observed.snapshot.replay_active = Some(true);
        assert!(complete_workspace(&mut state, event));
        assert!(state.workspace.audio.is_empty());
        assert!(state.workspace.detail.as_ref().unwrap().muted.is_none());
        assert!(state.workspace.detail.as_ref().unwrap().volume_db.is_none());
        assert_eq!(
            state.workspace.snapshot.as_ref().unwrap().replay_active,
            Some(true)
        );
        assert!(state.workspace.warnings.iter().any(|warning| matches!(
            warning,
            crate::domain::control::WorkspaceWarning::ReadFailed { .. }
        )));
    }
}
