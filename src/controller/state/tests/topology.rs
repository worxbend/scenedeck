//! Scene inventory updates and scene-graph topology observations.

use super::fixtures::{app_state, input};
use crate::controller::event::AppEvent;
use crate::controller::state::{
    previous_scene_for_inventory_update, ActiveAudioStatus, ObsStatus, TopologyStatus,
};
use crate::domain::graph::SceneGraph;
use crate::domain::scene::SceneInventory;
use crate::infra::error::AppError;

#[test]
fn inventory_refresh_tracks_previous_scene_only_when_current_changes() {
    assert_eq!(
        previous_scene_for_inventory_update(Some("A"), None, Some("B")),
        Some("A".to_string())
    );
    assert_eq!(
        previous_scene_for_inventory_update(Some("A"), Some("Prev"), Some("A")),
        Some("Prev".to_string())
    );
    assert_eq!(
        previous_scene_for_inventory_update(None, Some("Prev"), Some("A")),
        Some("Prev".to_string())
    );
}

#[test]
fn program_change_moves_previous_and_invalidates_scene_audio() {
    let mut state = app_state();
    let inventory = |current: &str| SceneInventory {
        current_id: Some(current.into()),
        ..SceneInventory::default()
    };
    state.apply_scene_inventory(inventory("A"));
    state.audio_inputs = vec![input("Mic")];
    state.apply_scene_inventory(inventory("A"));
    assert_eq!(state.audio_inputs.len(), 1, "same scene keeps audio");
    state.apply_scene_inventory(inventory("B"));
    assert_eq!(state.scene_inventory.previous_id.as_deref(), Some("A"));
    assert!(state.audio_inputs.is_empty());
    assert!(matches!(
        state.active_audio_status,
        ActiveAudioStatus::Loading
    ));
}

#[test]
fn topology_missing_is_distinct_from_a_successful_empty_observation() {
    let mut state = app_state();
    assert_eq!(state.topology_status, TopologyStatus::Missing);
    assert!(state.scene_graph.edges.is_empty());
    assert!(state.topology_status.notice().is_some());

    state.set_topology_ready(SceneGraph::default());
    assert_eq!(state.topology_status, TopologyStatus::Ready);
    assert!(state.scene_graph.edges.is_empty());
    assert!(state.topology_status.notice().is_none());
}

#[test]
fn topology_failure_after_ready_retires_old_graph_evidence() {
    let mut state = app_state();
    state.set_topology_ready(SceneGraph {
        edges: std::collections::BTreeMap::from([("Main".into(), vec!["Child".into()])]),
    });
    state.set_topology_failed("Group could not be read".into());

    assert!(state.scene_graph.edges.is_empty());
    assert_eq!(
        state.topology_status,
        TopologyStatus::Failed("Group could not be read".into())
    );
    let (title, description) = state.topology_status.notice().unwrap();
    assert_eq!(title, "Scene topology could not be read");
    assert!(description.contains("Group could not be read"));
}

#[test]
fn topology_rerun_is_loading_until_a_complete_success() {
    let mut state = app_state();
    state.set_topology_ready(SceneGraph::default());
    for _ in 0..2 {
        state.set_topology_loading();
        assert_eq!(state.topology_status, TopologyStatus::Loading);
        assert!(state.topology_status.notice().is_some());
        state.set_topology_failed("Incomplete scan".into());
        assert!(state.topology_status.notice().is_some());
    }
    state.set_topology_loading();
    state.set_topology_ready(SceneGraph::default());
    assert_eq!(state.topology_status, TopologyStatus::Ready);
    assert!(state.topology_status.notice().is_none());
}

#[test]
fn topology_session_and_collection_resets_clear_every_observation_status() {
    for retire_session in [false, true] {
        for status in [
            TopologyStatus::Missing,
            TopologyStatus::Loading,
            TopologyStatus::Ready,
            TopologyStatus::Failed("old failure".into()),
        ] {
            let mut state = app_state();
            state.topology_status = status;
            state
                .scene_graph
                .edges
                .insert("Main".into(), vec!["Child".into()]);
            if retire_session {
                state.reset_obs_session(ObsStatus::Disconnected);
            } else {
                state.invalidate_collection_runtime();
            }
            assert_eq!(state.topology_status, TopologyStatus::Missing);
            assert!(state.scene_graph.edges.is_empty());
        }
    }
}

#[test]
fn graph_reducer_rejects_superseded_success_failure_and_invalidation() {
    let mut state = app_state();
    state
        .reconcile_observation(10, AppEvent::GraphInvalidated)
        .unwrap();
    state.set_topology_loading();
    state
        .reconcile_observation(12, AppEvent::GraphInvalidated)
        .unwrap();
    assert!(state
        .reconcile_observation(10, AppEvent::GraphUpdated(Default::default()))
        .is_none());
    assert!(state
        .reconcile_observation(
            10,
            AppEvent::GraphFailed(AppError::Request("old failure".into()))
        )
        .is_none());
    assert!(state
        .reconcile_observation(11, AppEvent::GraphInvalidated)
        .is_none());
    assert_eq!(state.topology_status, TopologyStatus::Loading);
    let graph = match state
        .reconcile_observation(12, AppEvent::GraphUpdated(Default::default()))
        .unwrap()
    {
        AppEvent::GraphUpdated(graph) => graph,
        _ => unreachable!(),
    };
    state.set_topology_ready(graph);
    assert_eq!(
        state.topology_status,
        TopologyStatus::Ready,
        "valid empty topology remains Ready"
    );
}
