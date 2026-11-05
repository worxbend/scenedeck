use super::*;
use crate::{
    controller::{
        event::{AppEvent, ConnectionInfo},
        state::ObsStatus,
    },
    domain::{
        output_result::{OutputResultKind, OutputResultStatus},
        workflow::{ActivityOrigin, ActivityRecord},
    },
};
use std::time::Duration;

fn connect(state: &mut AppState, name: &str) {
    state.apply_connected(ConnectionInfo {
        connection_id: name.into(),
        endpoint: "fixture:4455".into(),
        obs_version: "fixture".into(),
        websocket_version: "5".into(),
        available_requests: Vec::new(),
        supported_image_formats: Vec::new(),
    });
    state.scene_collections.current = Some("Interview".into());
    state.scene_inventory.current_id = Some("Camera".into());
}

fn state() -> AppState {
    AppState::new(
        Default::default(),
        Default::default(),
        Some("PASSWORD_MUST_NOT_EXPORT".into()),
        None,
    )
}
fn collect(state: &mut AppState) {
    state.collect_report(Instant::now(), crate::controller::workspace::unix_ms());
}

#[test]
fn cancelled_exit_retains_live_requests_and_resumes_the_same_report() {
    let mut state = state();
    connect(&mut state, "podcast");
    let request = state.workspace.output_results.request(
        OutputResultKind::ReplaySave,
        state.workflow_scope(),
        10,
    );
    let marker = state.prepare_report_exit("exit requested").unwrap();
    let report_id = state.reports.entries.back().unwrap().id.clone();
    assert!(state.prepare_report_exit("duplicate close").is_none());
    assert!(state.reports.entries.back().unwrap().events.iter().any(|record|
        matches!(&record.event, ReportEvent::Output(result) if result.id == request && result.status == OutputResultStatus::Unconfirmed)));
    assert_eq!(
        state
            .workspace
            .output_results
            .entries()
            .find(|result| result.id == request)
            .unwrap()
            .status,
        OutputResultStatus::Requested
    );
    state.workspace.output_results.acknowledge(request, 20);
    state.resume_report_exit(&marker, "exit cancelled");
    let report = state.reports.entries.back().unwrap();
    assert_eq!(report.id, report_id);
    assert_eq!(report.ended_unix_ms, None);
    assert!(report.events.iter().any(|record|
        matches!(&record.event, ReportEvent::Output(result) if result.id == request && result.status == OutputResultStatus::Acknowledged)));
    let revision = state.reports.revision;
    state.resume_report_exit(&marker, "duplicate response");
    assert_eq!(state.reports.revision, revision);
}

#[test]
fn cancelled_exit_cannot_reopen_a_retired_connection_report() {
    let mut state = state();
    connect(&mut state, "first");
    let marker = state.prepare_report_exit("exit requested").unwrap();
    state.reset_obs_session(ObsStatus::Disconnected);
    state.resume_report_exit(&marker, "cancel after disconnect");
    assert!(state.reports.entries[0].ended_unix_ms.is_some());
    connect(&mut state, "replacement");
    collect(&mut state);
    let reports = state.reports.entries.clone();
    state.resume_report_exit(&marker, "cancel after reconnect");
    assert_eq!(state.reports.entries, reports);
}

#[test]
fn late_terminal_evidence_targets_only_retained_archive_without_resurrecting_removed_history() {
    let mut state = state();
    state.configure_report_history(true, 12);
    for generation in 1..=10 {
        if generation > 1 {
            state.reset_obs_session(ObsStatus::Disconnected);
        }
        connect(&mut state, &generation.to_string());
        state.reports.bind_generation(generation);
    }
    collect(&mut state);
    state.stage_report_history();
    assert_eq!(state.reports.entries.len(), MEMORY_LIMIT);
    assert!(!state
        .reports
        .entries
        .iter()
        .any(|report| report.connection_id == "1"));
    let previous = state.report_history.archive.clone();
    let memory = state.reports.entries.clone();
    let terminal = || crate::controller::lifecycle::SessionRetirement {
        generation: 1,
        reason: "late original socket termination".into(),
        operations: Vec::new(),
        omitted_sessions: 0,
    };
    state.apply_terminal_evidence(terminal(), false);
    collect(&mut state);
    state.stage_report_history();
    let archive = &state.report_history.archive;
    assert_eq!(archive.entries.len(), previous.entries.len());
    assert_eq!(state.reports.entries, memory);
    assert!(archive.entries[0].events.len() > previous.entries[0].events.len());
    for (current, previous) in archive
        .entries
        .iter()
        .skip(1)
        .zip(previous.entries.iter().skip(1))
    {
        assert_eq!(current, previous);
    }
    state.configure_report_history(true, 1);
    let retained = state.report_history.archive.clone();
    state.apply_terminal_evidence(terminal(), false);
    collect(&mut state);
    state.stage_report_history();
    assert_eq!(state.report_history.archive, retained);
    state.clear_report_history();
    state.apply_terminal_evidence(terminal(), false);
    collect(&mut state);
    state.stage_report_history();
    assert!(state.report_history.archive.entries.is_empty());
}

#[test]
fn collector_preserves_transitions_and_noops_do_not_duplicate_evidence() {
    let mut state = state();
    connect(&mut state, "podcast");
    let id = state.workspace.output_results.request(
        OutputResultKind::ReplaySave,
        state.workflow_scope(),
        10,
    );
    state
        .workspace
        .log(ActivityOrigin::Requested, "Save replay".into());
    collect(&mut state);
    state.workspace.output_results.acknowledge(id, 20);
    collect(&mut state);
    state.workspace.output_results.observe(
        OutputResultKind::ReplaySave,
        state.workflow_scope(),
        Some("/obs/replay.mkv".into()),
        30,
    );
    collect(&mut state);
    let revision = state.reports.revision;
    for _ in 0..5 {
        collect(&mut state);
    }
    assert_eq!(state.reports.revision, revision);
    let report = state.reports.entries.back().unwrap();
    let outputs: Vec<_> = report
        .events
        .iter()
        .filter_map(|record| {
            if let ReportEvent::Output(result) = &record.event {
                Some(result)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(outputs.len(), 3);
    assert_eq!(outputs[0].status, OutputResultStatus::Requested);
    assert_eq!(outputs[1].status, OutputResultStatus::Acknowledged);
    assert_eq!(outputs[1].path, None);
    assert_eq!(outputs[2].status, OutputResultStatus::Observed);
    assert_ne!(outputs[1].id, outputs[2].id);
    assert_eq!(outputs[2].path.as_deref(), Some("/obs/replay.mkv"));
    assert!(!serde_json::to_string(report)
        .unwrap()
        .contains("PASSWORD_MUST_NOT_EXPORT"));
}

#[test]
fn disconnect_captures_unconfirmed_results_and_old_terminal_audit_stays_with_its_session() {
    let mut state = state();
    connect(&mut state, "first");
    state.reports.bind_generation(41);
    let request = state.workspace.output_results.request(
        OutputResultKind::RecordingStop,
        state.workflow_scope(),
        10,
    );
    state.apply_output_event(&AppEvent::OutputCommandSucceeded(OutputKind::Record), 20);
    state.reset_obs_session(ObsStatus::Disconnected);
    assert!(state.reports.entries[0].ended_unix_ms.is_some());
    assert!(state.reports.entries[0].events.iter().any(|record| matches!(&record.event, ReportEvent::Output(result) if result.id == request && result.status == OutputResultStatus::Unconfirmed)));
    connect(&mut state, "second");
    state.reports.bind_generation(42);
    state.apply_terminal_evidence(
        crate::controller::lifecycle::SessionRetirement {
            generation: 41,
            reason: "old socket lost".into(),
            operations: Vec::new(),
            omitted_sessions: 0,
        },
        false,
    );
    collect(&mut state);
    assert!(state.reports.entries[0].events.iter().any(|record| matches!(&record.event, ReportEvent::Activity(activity) if activity.description.contains("old socket lost"))));
    assert!(!state.reports.entries[1].events.iter().any(|record| matches!(&record.event, ReportEvent::Activity(activity) if activity.description.contains("old socket lost"))));
    assert!(!state.reports.entries[1]
        .events
        .iter()
        .any(|record| matches!(record.event, ReportEvent::Output(_))));
    assert_ne!(state.reports.entries[0].id, state.reports.entries[1].id);
    for _ in 0..10 {
        state.reset_obs_session(ObsStatus::Disconnected);
        connect(&mut state, "next");
    }
    assert_eq!(state.reports.entries.len(), MEMORY_LIMIT);
}

#[test]
fn samples_use_source_time_and_gaps_are_explicit() {
    let mut state = state();
    connect(&mut state, "podcast");
    let now = Instant::now();
    state.set_obs_stats_at(
        crate::domain::health::tests::stats(),
        None,
        None,
        now - Duration::from_secs(2),
    );
    state.collect_report(now, 5_000);
    assert_eq!(state.reports.entries[0].samples[0].timestamp_unix_ms, 3_000);
    state.collect_report(now, 6_000);
    assert_eq!(state.reports.entries[0].samples_seen, 1);
    state.obs_stats_observed_at = Some(now - Duration::from_secs(3));
    state.collect_report(now, 7_000);
    assert_eq!(state.reports.entries[0].samples_seen, 1);
    state.obs_stats_observed_at = Some(now + Duration::from_secs(1));
    state.collect_report(now, 7_000);
    assert_eq!(state.reports.entries[0].samples_seen, 1);
    for index in 0..210 {
        state.workspace.activity.push(ActivityRecord {
            timestamp_unix_ms: index,
            origin: ActivityOrigin::Observed,
            description: index.to_string(),
        });
    }
    collect(&mut state);
    assert_eq!(state.reports.entries[0].activity_gaps, 10);
    collect(&mut state);
    assert_eq!(state.reports.entries[0].activity_gaps, 10);
}

#[test]
fn bookmarks_follow_edits_and_deletion_without_crossing_sessions() {
    let mut state = state();
    connect(&mut state, "podcast");
    let id = state
        .capture_bookmark("First note", Instant::now(), 50)
        .unwrap();
    collect(&mut state);
    state.reset_obs_session(ObsStatus::Disconnected);
    assert!(state.bookmarks.rename(id, "Renamed"));
    let report = state.reports.entries[0].clone();
    assert_eq!(
        report.bookmarks(&state.bookmarks.archive.entries)[0].name,
        "Renamed"
    );
    assert!(state.bookmarks.remove(id));
    assert!(report
        .bookmarks(&state.bookmarks.archive.entries)
        .is_empty());
    connect(&mut state, "podcast");
    state
        .capture_bookmark("Second note", Instant::now(), 60)
        .unwrap();
    collect(&mut state);
    assert!(report
        .bookmarks(&state.bookmarks.archive.entries)
        .is_empty());
    assert_eq!(
        state.reports.entries[1]
            .bookmarks(&state.bookmarks.archive.entries)
            .len(),
        1
    );
}

#[test]
fn health_disappearance_is_not_a_claim_of_recovery() {
    let mut state = state();
    connect(&mut state, "podcast");
    let now = Instant::now();
    crate::domain::health::tests::raise(
        &mut state.health,
        AlertKind::NetworkDrops,
        now - Duration::from_secs(6),
    );
    state.collect_report(now, 10);
    state.health = Default::default();
    state.collect_report(now, 20);
    let alerts: Vec<_> = state.reports.entries[0]
        .events
        .iter()
        .filter_map(|record| match &record.event {
            ReportEvent::Health { kind, phase, .. } => Some((kind.as_str(), phase.as_deref())),
            _ => None,
        })
        .collect();
    assert_eq!(
        alerts,
        [("network_drops", Some("active")), ("network_drops", None)]
    );
}
