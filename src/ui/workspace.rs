//! GTK-side scoped OBS workspace commands.

use crate::controller::{command::AppCommand, state::ObsStatus, workspace::WorkspaceRequest};
use crate::domain::{control::ControlAction, workflow::ActivityOrigin};
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::ui::navigation::NavigationContext;
use i18n_embed_fl::fl;

fn action_description(action: &ControlAction) -> String {
    match action {
        ControlAction::ActivateScene {
            scene,
            studio_enabled: true,
        } => {
            fl!(LANGUAGE_LOADER, "control-preview", scene = scene.clone())
        }
        ControlAction::SetProgram(scene)
        | ControlAction::ActivateScene {
            scene,
            studio_enabled: false,
        } => {
            fl!(LANGUAGE_LOADER, "control-program", scene = scene.clone())
        }
        ControlAction::Take => fl!(LANGUAGE_LOADER, "control-take"),
        ControlAction::SetRecordingPaused(true) => fl!(LANGUAGE_LOADER, "control-pause"),
        ControlAction::SetRecordingPaused(false) => fl!(LANGUAGE_LOADER, "control-resume"),
        ControlAction::SplitRecording => fl!(LANGUAGE_LOADER, "control-split"),
        ControlAction::RecordChapter(_) => fl!(LANGUAGE_LOADER, "control-chapter"),
        ControlAction::SetReplayActive(true) => fl!(LANGUAGE_LOADER, "control-replay-start"),
        ControlAction::SetReplayActive(false) => fl!(LANGUAGE_LOADER, "control-replay-stop"),
        ControlAction::SaveReplay => fl!(LANGUAGE_LOADER, "control-replay-save"),
        ControlAction::SetVirtualCamera(true) => fl!(LANGUAGE_LOADER, "control-camera-start"),
        ControlAction::SetVirtualCamera(false) => fl!(LANGUAGE_LOADER, "control-camera-stop"),
        ControlAction::SetText { input, .. } => {
            format!("{} · {input}", fl!(LANGUAGE_LOADER, "control-text-save"))
        }
        ControlAction::SetFilterEnabled { input, filter, .. } => format!(
            "{} · {input} / {filter}",
            fl!(LANGUAGE_LOADER, "control-filters")
        ),
        ControlAction::Media { input, .. } => {
            format!("{} · {input}", fl!(LANGUAGE_LOADER, "control-media"))
        }
        ControlAction::SeekMedia { input, .. } => {
            format!("{} · {input}", fl!(LANGUAGE_LOADER, "control-media-seek"))
        }
        ControlAction::SetItemEnabled { owner, item_id, .. } => format!(
            "{} · {owner} #{item_id}",
            fl!(LANGUAGE_LOADER, "workflow-check-item")
        ),
        ControlAction::SetStudioMode(_) => fl!(LANGUAGE_LOADER, "control-studio"),
        ControlAction::SetTransition(name) => {
            format!("{} · {name}", fl!(LANGUAGE_LOADER, "control-transition"))
        }
        ControlAction::SetTransitionDuration(ms) => {
            format!("{} · {ms}", fl!(LANGUAGE_LOADER, "control-duration"))
        }
        ControlAction::SetInputMuted { input, .. } => {
            fl!(LANGUAGE_LOADER, "audio-source-mute", source = input.clone())
        }
        ControlAction::SetInputVolumeDb { input, db } => format!(
            "{} · {db:.1} dB",
            fl!(
                LANGUAGE_LOADER,
                "audio-source-volume",
                source = input.clone()
            )
        ),
        ControlAction::SetAudioMonitor { input, .. }
        | ControlAction::SetAudioSync { input, .. }
        | ControlAction::SetAudioBalance { input, .. }
        | ControlAction::SetAudioTracks { input, .. } => format!(
            "{} · {input}",
            fl!(LANGUAGE_LOADER, "control-advanced-audio")
        ),
    }
}

fn request(
    nav: &NavigationContext,
    screenshot: bool,
    action: Option<&ControlAction>,
) -> Option<WorkspaceRequest> {
    if !nav.is_current() {
        return None;
    }
    let mut state = nav.state.borrow_mut();
    let scene = match action {
        Some(ControlAction::ActivateScene { scene, .. }) => Some(scene.clone()),
        _ => None,
    };
    if !matches!(state.obs_status, ObsStatus::Connected { .. })
        || (state.workspace.pending
            && !(scene.is_some() && state.workspace.pending_scene.is_some()))
        || (action.is_none() && state.workspace.active_action.is_some())
    {
        return None;
    }
    let scope = state.workflow_scope();
    if scope.collection.is_empty() {
        return None;
    }
    state.workspace.sequence = state.workspace.sequence.wrapping_add(1);
    if state.workspace.can_control() {
        state.workspace.observe_global_controls();
    }
    // Clearing the refresh request flag must not make invalidated data fresh.
    // Mutations also need a new readback before value-dependent controls resume.
    if state.workspace.dirty || action.is_some() {
        state.workspace.observed_at = None;
    }
    if matches!(action, Some(ControlAction::SetStudioMode(_))) {
        state.workspace.scene_control_observation = None;
    }
    if matches!(action, Some(ControlAction::SetVirtualCamera(_))) {
        state.workspace.virtual_camera_observation = None;
    }
    state.workspace.loading = true;
    state.workspace.pending = action.is_some();
    state.workspace.pending_scene = scene;
    state.workspace.active_action = action.cloned();
    state.workspace.touch();
    state.workspace.dirty = false;
    state.workspace.error = None;
    let workspace = &state.workspace;
    Some(WorkspaceRequest {
        epoch: workspace.epoch,
        sequence: workspace.sequence,
        scope,
        scene: workspace.selected_scene.clone(),
        input: workspace.selected_input.clone(),
        include_sources: state.current_page == crate::controller::state::Page::Sources,
        screenshot,
        result_id: None,
    })
}

pub fn refresh(nav: &NavigationContext) {
    if !nav.is_current() {
        return;
    }
    if let Some(request) = request(nav, false, None) {
        nav.dispatch(AppCommand::ReadWorkspace(request));
    }
}
pub fn inspect_scene(nav: &NavigationContext, scene: Option<String>) {
    if !nav.is_current() {
        return;
    }
    {
        let mut state = nav.state.borrow_mut();
        if state.workspace.pending {
            return;
        }
        state.workspace.selected_scene = scene.clone();
        state.workspace.observed_at = None;
        if let Some(snapshot) = &mut state.workspace.snapshot {
            snapshot.scene = scene;
            snapshot.items.clear();
        }
        state.workspace.selected_input = None;
        state.workspace.detail = None;
        state.workspace.usage.clear();
        state.workspace.screenshot = None;
        state.workspace.touch();
    }
    refresh(nav);
}
pub fn inspect_input(nav: &NavigationContext, input: Option<String>) {
    if !nav.is_current() {
        return;
    }
    {
        let mut state = nav.state.borrow_mut();
        if state.workspace.pending {
            return;
        }
        state.workspace.select_input(input);
    }
    refresh(nav);
}
pub fn act(nav: &NavigationContext, action: ControlAction) {
    if !nav.is_current() {
        return;
    }
    let expected_text = nav
        .state
        .borrow()
        .workspace
        .detail
        .as_ref()
        .and_then(|detail| detail.text.clone());
    act_expected(nav, action, expected_text);
}

pub fn save_text(nav: &NavigationContext, input: String, text: String, expected: String) {
    if !nav.is_current() {
        return;
    }
    act_expected(nav, ControlAction::SetText { input, text }, Some(expected));
}

fn act_expected(nav: &NavigationContext, action: ControlAction, expected_text: Option<String>) {
    if !nav.is_current() {
        return;
    }
    {
        let mut state = nav.state.borrow_mut();
        let toggle_ready = if let Some(toggle) = action.global_toggle() {
            if state.workspace.toggle_pending(toggle) {
                return;
            }
            let ready = state.workspace.toggle_value(toggle).is_some();
            if ready && (state.workspace.pending || state.workspace.active_action.is_some()) {
                // Do not supersede an unrelated admitted write. Retain one
                // explicit intent per toggle and dispatch after its readback.
                state.workspace.queued_toggles.push_back(action);
                state.workspace.touch();
                return;
            }
            ready
        } else {
            false
        };
        let scene_ready = match &action {
            ControlAction::ActivateScene { scene, .. } => {
                state.workspace.scene_action(scene).as_ref() == Some(&action)
            }
            _ => false,
        };
        if state.workspace.pending && !scene_ready {
            return;
        }
        if !scene_ready && !toggle_ready && !state.workspace.can_control() {
            drop(state);
            refresh(nav);
            return;
        }
    }
    if let Some(mut request) = request(nav, false, Some(&action)) {
        use crate::domain::output_result::OutputResultKind;
        let kind = match &action {
            ControlAction::SaveReplay => Some(OutputResultKind::ReplaySave),
            ControlAction::SplitRecording => Some(OutputResultKind::RecordingSplit),
            ControlAction::RecordChapter(_) => Some(OutputResultKind::RecordingChapter),
            _ => None,
        };
        if let Some(kind) = kind {
            request.result_id = Some(nav.state.borrow_mut().workspace.output_results.request(
                kind,
                request.scope.clone(),
                crate::controller::workspace::unix_ms(),
            ));
        }
        nav.state
            .borrow_mut()
            .workspace
            .log(ActivityOrigin::Requested, action_description(&action));
        nav.dispatch(AppCommand::WorkspaceAction {
            request,
            action,
            expected_text,
        });
    }
}

/// Finish an unrelated command before submitting a queued global toggle.
pub(super) fn dispatch_queued_toggle(nav: &NavigationContext) {
    let action = {
        let mut state = nav.state.borrow_mut();
        if state.workspace.pending || state.workspace.loading {
            return;
        }
        if let Some(toggle) = state
            .workspace
            .queued_toggles
            .front()
            .and_then(|action| action.global_toggle())
        {
            if state.workspace.toggle_value(toggle).is_none() {
                if state.workspace.dirty {
                    drop(state);
                    refresh(nav);
                    return;
                }
                // A failed/unsupported observation cannot authorize a queued
                // toggle. Report it instead of leaving an endless busy state.
                state.workspace.queued_toggles.clear();
                let message = fl!(LANGUAGE_LOADER, "control-stale");
                state.workspace.result.clone_from(&message);
                state.workspace.log(ActivityOrigin::Failed, message);
                return;
            }
        }
        state.workspace.queued_toggles.pop_front()
    };
    if let Some(action) = action {
        act(nav, action);
    }
}
