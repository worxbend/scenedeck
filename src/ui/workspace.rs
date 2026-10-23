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

fn request(nav: &NavigationContext, screenshot: bool, action: bool) -> Option<WorkspaceRequest> {
    if !nav.is_current() {
        return None;
    }
    let mut state = nav.state.borrow_mut();
    if !matches!(state.obs_status, ObsStatus::Connected { .. }) || state.workspace.pending {
        return None;
    }
    let scope = state.workflow_scope();
    if scope.collection.is_empty() {
        return None;
    }
    state.workspace.sequence = state.workspace.sequence.wrapping_add(1);
    state.workspace.loading = true;
    state.workspace.pending = action;
    state.workspace.revision = state.workspace.revision.wrapping_add(1);
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
    if let Some(request) = request(nav, false, false) {
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
        state.workspace.revision = state.workspace.revision.wrapping_add(1);
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
        let state = nav.state.borrow();
        if state.workspace.pending || state.workspace.loading {
            return;
        }
        if !state.workspace.can_control() {
            drop(state);
            refresh(nav);
            return;
        }
    }
    if let Some(mut request) = request(nav, false, true) {
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
