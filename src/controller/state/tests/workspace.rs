//! Workspace generations, revisions and control availability.

use std::time::Instant;

use crate::controller::state::WorkspaceState;

fn controllable_workspace(studio: Option<bool>) -> WorkspaceState {
    WorkspaceState {
        snapshot: Some(crate::domain::control::ControlSnapshot {
            scene: Some("Inspected scene".into()),
            studio: crate::domain::control::StudioState {
                enabled: studio,
                ..Default::default()
            },
            ..Default::default()
        }),
        observed_at: Some(Instant::now()),
        ..Default::default()
    }
}

#[test]
fn studio_and_virtual_camera_echoes_patch_the_snapshot_and_mark_dirty() {
    let mut workspace = WorkspaceState {
        snapshot: Some(Default::default()),
        ..Default::default()
    };
    let at = Instant::now();
    workspace.observe_studio_mode(true, at);
    assert_eq!(workspace.scene_control_observation, Some((true, at)));
    assert_eq!(
        workspace.snapshot.as_ref().unwrap().studio.enabled,
        Some(true)
    );
    workspace.dirty = false;
    workspace.observe_virtual_camera(Some(false), at);
    assert_eq!(
        workspace.virtual_camera_observation,
        Some((Some(false), at))
    );
    assert_eq!(
        workspace.snapshot.as_ref().unwrap().virtual_camera_active,
        Some(false)
    );
    assert!(workspace.dirty);
}

#[test]
fn every_visible_workspace_mutation_advances_the_revision() {
    let mut workspace = WorkspaceState::default();
    workspace.log(
        crate::domain::workflow::ActivityOrigin::Failed,
        "stream failed".into(),
    );
    assert_eq!(workspace.revision(), 1, "log entries are rendered");

    workspace.mark_dirty();
    assert!(workspace.dirty);
    assert_eq!(workspace.revision(), 2);

    workspace.dirty = false;
    workspace.invalidate_controls();
    assert!(workspace.dirty);
    assert_eq!(workspace.revision(), 3);
}

#[test]
fn workspace_generation_rejects_old_and_out_of_order_results() {
    let mut workspace = WorkspaceState {
        epoch: 7,
        sequence: 12,
        loading: true,
        pending: true,
        selected_scene: Some("Old scene".into()),
        selected_input: Some("Old input".into()),
        snapshot: Some(crate::domain::control::ControlSnapshot::default()),
        observed_at: Some(std::time::Instant::now()),
        screenshot: Some("old image".into()),
        ..Default::default()
    };
    assert!(workspace.accepts(7, 12));
    assert!(!workspace.accepts(7, 11));
    assert!(!workspace.accepts(6, 12));
    workspace.invalidate();
    assert!(!workspace.accepts(7, 12));
    assert!(workspace.accepts(8, 13));
    assert!(!workspace.loading);
    assert!(!workspace.pending);
    assert!(workspace.snapshot.is_none());
    assert!(workspace.observed_at.is_none());
    assert!(workspace.screenshot.is_none());
    assert!(workspace.selected_scene.is_none());
    assert!(workspace.selected_input.is_none());
}

#[test]
fn source_inspection_retains_its_scene_when_program_changes() {
    let mut workspace = controllable_workspace(Some(false));
    workspace.select_input(Some("Caption".into()));
    assert_eq!(workspace.selected_scene.as_deref(), Some("Inspected scene"));
    workspace.snapshot.as_mut().unwrap().studio.program = Some("New Program".into());
    workspace.select_input(Some("Camera".into()));
    assert_eq!(workspace.selected_scene.as_deref(), Some("Inspected scene"));
    assert_eq!(workspace.selected_input.as_deref(), Some("Camera"));
    workspace.invalidate();
    assert!(workspace.selected_scene.is_none());
    assert!(workspace.selected_input.is_none());
}

#[test]
fn workspace_failed_read_disables_recent_cached_controls() {
    let mut workspace = controllable_workspace(Some(false));
    assert!(workspace.can_control());
    assert!(workspace.scene_action("Target").is_some());
    workspace.fail_read("Target read failed".into());
    assert!(
        workspace.snapshot.is_some(),
        "last-known display data may be retained"
    );
    assert!(!workspace.can_control());
    assert!(workspace.scene_action("Target").is_none());
    assert!(workspace.observed_at.is_none());
    assert_eq!(workspace.error.as_deref(), Some("Target read failed"));
    workspace.error = None;
    assert!(
        !workspace.can_control(),
        "clearing an error is not a fresh observation"
    );
}

#[test]
fn global_toggle_availability_is_independent_and_retires_with_its_scope() {
    use crate::domain::control::{ControlAction, GlobalToggle};
    let mut workspace = controllable_workspace(Some(false));
    workspace.snapshot.as_mut().unwrap().virtual_camera_active = Some(false);
    workspace.observe_global_controls();
    workspace.pending = true;
    workspace.loading = true;
    workspace.dirty = true;
    workspace.observed_at = None;
    workspace.active_action = Some(ControlAction::SetText {
        input: "Caption".into(),
        text: "Edit".into(),
    });
    for toggle in [GlobalToggle::StudioMode, GlobalToggle::VirtualCamera] {
        assert_eq!(workspace.toggle_value(toggle), Some(false));
        assert!(!workspace.toggle_pending(toggle));
    }
    workspace
        .queued_toggles
        .push_back(ControlAction::SetVirtualCamera(true));
    assert!(workspace.toggle_pending(GlobalToggle::VirtualCamera));
    assert!(!workspace.toggle_pending(GlobalToggle::StudioMode));
    workspace.virtual_camera_observation = Some((
        Some(false),
        Instant::now() - std::time::Duration::from_secs(31),
    ));
    assert_eq!(workspace.toggle_value(GlobalToggle::VirtualCamera), None);
    assert_eq!(
        workspace.toggle_value(GlobalToggle::StudioMode),
        Some(false)
    );
    workspace.invalidate();
    assert!(workspace.active_action.is_none() && workspace.queued_toggles.is_empty());
    assert_eq!(workspace.toggle_value(GlobalToggle::StudioMode), None);
    assert_eq!(workspace.toggle_value(GlobalToggle::VirtualCamera), None);
}

#[test]
fn workspace_scene_action_requires_known_studio_mode() {
    use crate::domain::control::ControlAction;
    let unknown = controllable_workspace(None);
    assert!(unknown.scene_action("Target").is_none());
    assert!(WorkspaceState::default().scene_action("Target").is_none());
    assert_eq!(
        controllable_workspace(Some(false)).scene_action("Target"),
        Some(ControlAction::ActivateScene {
            scene: "Target".into(),
            studio_enabled: false
        })
    );
    assert_eq!(
        controllable_workspace(Some(true)).scene_action("Target"),
        Some(ControlAction::ActivateScene {
            scene: "Target".into(),
            studio_enabled: true
        })
    );
}

#[test]
fn scene_controls_survive_scene_readback_but_reject_mode_changes_failures_and_other_actions() {
    for studio in [false, true] {
        let mut workspace = controllable_workspace(Some(studio));
        workspace.observe_scene_controls();
        workspace.loading = true;
        workspace.pending = true;
        workspace.pending_scene = Some("First".into());
        workspace.observed_at = None;
        workspace.dirty = true; // Program echo requires source membership readback.
        assert_eq!(
            workspace.scene_action("Next"),
            Some(crate::domain::control::ControlAction::ActivateScene {
                scene: "Next".into(),
                studio_enabled: studio,
            })
        );
        workspace.pending_scene = None;
        assert!(
            workspace.scene_action("Next").is_none(),
            "another mutation owns the workspace"
        );
        workspace.pending = false;
        workspace.invalidate_controls();
        assert!(
            workspace.scene_action("Next").is_none(),
            "an OBS mode change must invalidate cached scene controls"
        );
        workspace.dirty = false;
        workspace.observed_at = Some(Instant::now());
        workspace.observe_scene_controls();
        workspace.fail_read("read failed".into());
        assert!(workspace.scene_action("Next").is_none());
        workspace.error = None;
        workspace.scene_control_observation =
            Some((studio, Instant::now() - std::time::Duration::from_secs(31)));
        assert!(workspace.scene_action("Next").is_none());
        workspace.invalidate();
        assert!(workspace.scene_control_observation.is_none());
    }
}

#[test]
fn background_read_keeps_recent_controls_available_but_never_revives_invalid_data() {
    let mut workspace = controllable_workspace(Some(false));
    workspace.loading = true;
    assert!(workspace.can_control());
    assert!(workspace.scene_action("Target").is_some());
    workspace.dirty = true;
    assert!(!workspace.can_control());
    workspace.dirty = false;
    workspace.observed_at = None;
    assert!(!workspace.can_control());
    workspace.observed_at = Some(Instant::now());
    workspace.pending = true;
    assert!(!workspace.can_control());
}

#[test]
fn workspace_scene_actions_reject_busy_stale_and_failed_observations_in_both_modes() {
    type Invalidate = fn(&mut WorkspaceState);
    let cases: [(&str, Invalidate); 5] = [
        ("dirty", |state| state.dirty = true),
        ("pending", |state| state.pending = true),
        ("failed", |state| state.error = Some("Read failed".into())),
        ("expired", |state| {
            state.observed_at = Some(Instant::now() - std::time::Duration::from_secs(31))
        }),
        ("unobserved", |state| state.observed_at = None),
    ];
    for studio in [false, true] {
        for (name, invalidate) in cases {
            let mut state = controllable_workspace(Some(studio));
            assert!(state.scene_action("Target").is_some());
            invalidate(&mut state);
            assert!(!state.can_control(), "{name} state must disable controls");
            assert!(
                state.scene_action("Target").is_none(),
                "{name} state dispatched in studio={studio}"
            );
        }
    }
}
