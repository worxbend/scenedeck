//! Maps obws response types to our domain types.
//!
//! All OBS-specific types stop here.  Nothing above this module imports from `obws`.

use obws::events::{Event, OutputState};
use obws::responses::general::Version;
use obws::responses::recording::RecordStatus;
use obws::responses::scenes::Scenes;
use obws::responses::streaming::StreamStatus;

use crate::controller::event::ConnectionInfo;
use crate::domain::output::{OutputRunState, OutputStatus};
use crate::domain::scene::{Scene, SceneInventory};
use crate::obs::event::ObsEvent;

/// Convert OBS version metadata into controller connection info.
pub(super) fn map_version(v: &Version) -> ConnectionInfo {
    ConnectionInfo {
        obs_version: v.obs_studio_version.to_string(),
        websocket_version: v.obs_web_socket_version.to_string(),
    }
}

/// Convert the `GetSceneList` response into a domain `SceneInventory`.
///
/// OBS returns scenes in reverse order (highest index = first in the UI switcher),
/// so we reverse the list here to match the user's scene switcher order.
pub(super) fn map_scenes(resp: Scenes) -> SceneInventory {
    let current_id = resp.current_program_scene.map(|s| s.name.clone());

    let scenes = resp
        .scenes
        .iter()
        .rev()
        .map(|s| Scene {
            id: s.id.name.clone(),
            name: s.id.name.clone(),
        })
        .collect();

    SceneInventory { scenes, current_id }
}

/// Convert an OBS output lifecycle state into the app's normalized state.
fn map_output_state(state: OutputState) -> OutputRunState {
    match state {
        OutputState::Starting => OutputRunState::Starting,
        OutputState::Started => OutputRunState::Active,
        OutputState::Stopping => OutputRunState::Stopping,
        OutputState::Stopped => OutputRunState::Inactive,
        OutputState::Reconnecting => OutputRunState::Reconnecting,
        OutputState::Reconnected => OutputRunState::Active,
        OutputState::Paused => OutputRunState::Paused,
        OutputState::Resumed => OutputRunState::Active,
        OutputState::Unknown => OutputRunState::Unknown,
        _ => OutputRunState::Unknown,
    }
}

/// Convert an OBS output event payload into a domain output status.
fn map_output_status(active: bool, state: OutputState, detail: Option<String>) -> OutputStatus {
    OutputStatus::new(active, map_output_state(state)).with_optional_detail(detail)
}

/// Convert `GetStreamStatus` into the app's normalized output status.
pub(super) fn map_stream_status(status: StreamStatus) -> OutputStatus {
    OutputStatus::new(
        status.active,
        if status.reconnecting {
            OutputRunState::Reconnecting
        } else if status.active {
            OutputRunState::Active
        } else {
            OutputRunState::Inactive
        },
    )
}

/// Convert `GetRecordStatus` into the app's normalized output status.
pub(super) fn map_record_status(status: RecordStatus) -> OutputStatus {
    OutputStatus::new(
        status.active,
        if status.paused {
            OutputRunState::Paused
        } else if status.active {
            OutputRunState::Active
        } else {
            OutputRunState::Inactive
        },
    )
}

/// Convert a raw OBS event into the controller-facing event model.
pub(super) fn map_event(event: Event) -> ObsEvent {
    match event {
        Event::StreamStateChanged { active, state } => {
            ObsEvent::StreamStatusUpdated(map_output_status(active, state, None))
        }
        Event::RecordStateChanged {
            active,
            state,
            path,
        } => ObsEvent::RecordStatusUpdated(map_output_status(active, state, path)),
        Event::RecordFileChanged { path } => {
            ObsEvent::RecordStatusUpdated(OutputStatus::active_with_detail(path))
        }
        Event::CurrentProgramSceneChanged { id } => ObsEvent::CurrentProgramSceneChanged(id.name),
        Event::InputMuteStateChanged { id, muted } => ObsEvent::InputMuteChanged {
            input: id.name,
            muted,
        },
        Event::InputVolumeChanged { id, mul, db } => ObsEvent::InputVolumeChanged {
            input: id.name,
            volume_mul: mul,
            volume_db: db,
        },
        Event::InputCreated { .. }
        | Event::InputRemoved { .. }
        | Event::InputNameChanged { .. } => ObsEvent::InputsChanged,
        Event::SceneItemCreated { .. }
        | Event::SceneItemRemoved { .. }
        | Event::SceneItemListReindexed { .. }
        | Event::SceneItemEnableStateChanged { .. } => ObsEvent::SceneItemsChanged,
        Event::CurrentProfileChanged { name } => ObsEvent::CurrentProfileChanged(name),
        Event::ProfileListChanged { profiles } => ObsEvent::ProfileListChanged(profiles),
        Event::CurrentSceneCollectionChanged { name } => {
            ObsEvent::CurrentSceneCollectionChanged(name)
        }
        Event::SceneCollectionListChanged { collections } => {
            ObsEvent::SceneCollectionListChanged(collections)
        }
        Event::SceneListChanged { .. } => ObsEvent::SceneListChanged,
        _ => ObsEvent::Ignored,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use obws::events::OutputState;

    #[test]
    fn output_state_mapping_preserves_lifecycle_meaning() {
        assert_eq!(
            map_output_state(OutputState::Starting),
            OutputRunState::Starting
        );
        assert_eq!(
            map_output_state(OutputState::Started),
            OutputRunState::Active
        );
        assert_eq!(
            map_output_state(OutputState::Stopping),
            OutputRunState::Stopping
        );
        assert_eq!(
            map_output_state(OutputState::Stopped),
            OutputRunState::Inactive
        );
        assert_eq!(
            map_output_state(OutputState::Reconnecting),
            OutputRunState::Reconnecting
        );
        assert_eq!(
            map_output_state(OutputState::Paused),
            OutputRunState::Paused
        );
        assert_eq!(
            map_output_state(OutputState::Resumed),
            OutputRunState::Active
        );
        assert_eq!(
            map_output_state(OutputState::Unknown),
            OutputRunState::Unknown
        );
    }

    #[test]
    fn output_status_mapping_keeps_active_flag_and_detail() {
        let status = map_output_status(
            true,
            OutputState::Reconnected,
            Some("/tmp/recording.mkv".to_string()),
        );

        assert!(status.active);
        assert_eq!(status.state, OutputRunState::Active);
        assert_eq!(status.detail.as_deref(), Some("/tmp/recording.mkv"));
    }

    #[test]
    fn event_mapping_normalizes_output_status_events() {
        assert_eq!(
            map_event(Event::StreamStateChanged {
                active: true,
                state: OutputState::Started,
            }),
            ObsEvent::StreamStatusUpdated(OutputStatus::active())
        );

        assert_eq!(
            map_event(Event::RecordStateChanged {
                active: false,
                state: OutputState::Stopped,
                path: Some("/tmp/done.mkv".to_string()),
            }),
            ObsEvent::RecordStatusUpdated(OutputStatus::inactive_with_detail("/tmp/done.mkv"))
        );

        assert_eq!(
            map_event(Event::RecordFileChanged {
                path: "/tmp/live.mkv".to_string(),
            }),
            ObsEvent::RecordStatusUpdated(OutputStatus::active_with_detail("/tmp/live.mkv"))
        );
    }

    #[test]
    fn stream_status_mapping_prioritizes_reconnecting() {
        let status = map_stream_status(StreamStatus {
            active: true,
            reconnecting: true,
            ..Default::default()
        });

        assert!(status.active);
        assert_eq!(status.state, OutputRunState::Reconnecting);
        assert_eq!(status.detail, None);
    }

    #[test]
    fn stream_status_mapping_reports_active_and_inactive() {
        let active = map_stream_status(StreamStatus {
            active: true,
            ..Default::default()
        });
        let inactive = map_stream_status(StreamStatus::default());

        assert_eq!(active.state, OutputRunState::Active);
        assert_eq!(inactive.state, OutputRunState::Inactive);
    }

    #[test]
    fn record_status_mapping_prioritizes_paused() {
        let status = map_record_status(RecordStatus {
            active: true,
            paused: true,
            ..Default::default()
        });

        assert!(status.active);
        assert_eq!(status.state, OutputRunState::Paused);
        assert_eq!(status.detail, None);
    }

    #[test]
    fn record_status_mapping_reports_active_and_inactive() {
        let active = map_record_status(RecordStatus {
            active: true,
            ..Default::default()
        });
        let inactive = map_record_status(RecordStatus::default());

        assert_eq!(active.state, OutputRunState::Active);
        assert_eq!(inactive.state, OutputRunState::Inactive);
    }
}
