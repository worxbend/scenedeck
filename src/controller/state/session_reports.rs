//! Report collection lives with application state, independent of open pages.

use super::AppState;
use crate::domain::{
    health::{AlertKind, AlertPhase},
    output::OutputKind,
    output_result::OutputResult,
    session_report::{ReportEvent, ReportSample, SessionReport},
};
use std::{
    collections::{BTreeMap, VecDeque},
    time::Instant,
};

const MEMORY_LIMIT: usize = 8;
#[cfg(test)]
mod tests;
type HealthKey = (String, Option<String>, Option<u64>);

/// Identifies only the observation interval stopped for a pending app exit.
#[derive(Debug, Clone)]
pub(crate) struct ReportExit {
    id: String,
    at: u64,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ReportsState {
    pub entries: VecDeque<SessionReport>,
    pub revision: u64,
    activity_after: u64,
    output_baseline: u64,
    outputs: BTreeMap<u64, OutputResult>,
    scene: Option<(Option<String>, Option<String>)>,
    sample_at: Option<Instant>,
    health: BTreeMap<HealthKey, AlertPhase>,
    generations: BTreeMap<u64, String>,
    next_run: u64,
}

impl ReportsState {
    fn current(&mut self) -> Option<&mut SessionReport> {
        self.entries
            .back_mut()
            .filter(|entry| entry.ended_unix_ms.is_none())
    }

    pub fn bind_generation(&mut self, generation: u64) {
        if let Some(report) = self.current() {
            let id = report.id.clone();
            self.generations.insert(generation, id);
        }
    }

    fn push(&mut self, timestamp: u64, event: ReportEvent) {
        if let Some(report) = self.current() {
            report.push(timestamp, event);
            self.revision += 1;
        }
    }
}

impl AppState {
    pub(crate) fn prepare_report_exit(&mut self, reason: &str) -> Option<ReportExit> {
        use crate::domain::output_result::OutputResultStatus;
        let now = crate::controller::workspace::unix_ms();
        self.collect_report(Instant::now(), now);
        let baseline = self.reports.output_baseline;
        let report = self.reports.current()?;
        // Uncertainty belongs to this report's observation boundary. Leave live
        // request state intact: a cancelled exit can still receive its real ack.
        for result in self.workspace.output_results.entries().filter(|result| {
            result.id > baseline
                && matches!(
                    result.status,
                    OutputResultStatus::Requested | OutputResultStatus::Acknowledged
                )
        }) {
            let mut result = result.clone();
            result.status = OutputResultStatus::Unconfirmed;
            result.updated_unix_ms = now;
            report.push(now, ReportEvent::Output(result));
        }
        report.push(
            now,
            ReportEvent::Ended {
                reason: reason.into(),
            },
        );
        report.ended_unix_ms = Some(now);
        let marker = ReportExit {
            id: report.id.clone(),
            at: now,
        };
        self.reports.revision += 1;
        self.stage_report_history();
        self.report_history.urgent = true;
        Some(marker)
    }

    pub(crate) fn resume_report_exit(&mut self, marker: &ReportExit, reason: &str) {
        if !matches!(self.obs_status, super::ObsStatus::Connected { .. }) {
            return;
        }
        let Some(report) = self
            .reports
            .entries
            .back_mut()
            .filter(|report| report.id == marker.id && report.ended_unix_ms == Some(marker.at))
        else {
            return;
        };
        if self
            .bookmarks
            .session_context()
            .is_none_or(|(_, start, _)| start != report.started_unix_ms)
        {
            return;
        }
        let now = crate::controller::workspace::unix_ms();
        report.ended_unix_ms = None;
        report.push(
            now,
            ReportEvent::Activity(crate::domain::workflow::ActivityRecord {
                timestamp_unix_ms: now,
                origin: crate::domain::workflow::ActivityOrigin::Connection,
                description: reason.into(),
            }),
        );
        self.reports.revision += 1;
        self.collect_report(Instant::now(), now);
        self.stage_report_history();
        self.report_history.urgent = true;
    }

    pub(super) fn begin_report(&mut self) {
        let (Some((session_id, timestamp, _)), Some(connection_id), Some(endpoint)) = (
            self.bookmarks.session_context(),
            self.connected_connection_id.clone(),
            self.connected_endpoint.clone(),
        ) else {
            return;
        };
        let reports = &mut self.reports;
        while reports.entries.len() >= MEMORY_LIMIT {
            reports.entries.pop_front();
        }
        reports.entries.push_back(SessionReport::new(
            session_id,
            timestamp,
            connection_id,
            endpoint,
        ));
        reports.next_run += 1;
        reports
            .entries
            .back_mut()
            .unwrap()
            .id
            .push_str(&format!("-{}", reports.next_run));
        let report = reports.entries.back_mut().unwrap();
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        report
            .id
            .push_str(&format!("-{}-{nonce}", std::process::id()));
        while self
            .report_history
            .archive
            .entries
            .iter()
            .any(|old| old.id == report.id)
        {
            report.id.push('x');
        }
        report.connection_name = self
            .config
            .connection_profiles()
            .into_iter()
            .find(|profile| profile.id == report.connection_id)
            .map(|profile| profile.name);
        if self.config.session_history.enabled {
            self.report_history.eligible.insert(report.id.clone());
            self.report_history.owned.insert(report.id.clone());
        }
        reports.activity_after = self.workspace.activity.sequence();
        reports.output_baseline = self
            .workspace
            .output_results
            .entries()
            .map(|entry| entry.id)
            .max()
            .unwrap_or(0);
        reports.outputs.clear();
        reports.scene = None;
        reports.sample_at = None;
        reports.health.clear();
        reports.revision += 1;
        reports.generations.retain(|_, id| {
            reports.entries.iter().any(|report| &report.id == id)
                || self
                    .report_history
                    .archive
                    .entries
                    .iter()
                    .any(|report| &report.id == id)
        });
    }

    pub(crate) fn finish_report(&mut self, reason: &str) {
        let now = crate::controller::workspace::unix_ms();
        self.collect_report(Instant::now(), now);
        if let Some(report) = self.reports.current() {
            report.push(
                now,
                ReportEvent::Ended {
                    reason: reason.into(),
                },
            );
            report.ended_unix_ms = Some(now);
            self.reports.revision += 1;
            self.stage_report_history();
            self.report_history.urgent = true;
        }
    }

    pub(super) fn report_output_ack(&mut self, kind: OutputKind, timestamp: u64) {
        self.collect_report(Instant::now(), timestamp);
        self.reports
            .push(timestamp, ReportEvent::OutputAcknowledged { kind });
    }

    /// Terminal audit may arrive after reset or after a replacement connects.
    /// Its connection generation, never its endpoint text, identifies its report.
    pub(super) fn report_terminal_audit(&mut self, generation: u64) {
        let reports = &mut self.reports;
        reports.activity_after = self.workspace.activity.sequence();
        let Some(id) = reports.generations.get(&generation) else {
            return;
        };
        let report = reports.entries.iter_mut().find(|report| &report.id == id);
        let report = if let Some(report) = report {
            report
        } else {
            let Some(report) = self
                .report_history
                .archive
                .entries
                .iter_mut()
                .find(|report| &report.id == id)
            else {
                return;
            };
            self.report_history.revision += 1;
            self.report_history.urgent = true;
            report
        };
        if let Some(record) = self.workspace.activity.entries.back() {
            report.push(
                record.timestamp_unix_ms,
                ReportEvent::Activity(record.clone()),
            );
            reports.revision += 1;
        }
    }

    /// Call after each accepted event and on the app-owned health tick. Only new
    /// evidence is copied; a timer tick does not synthesize an OBS measurement.
    pub(crate) fn collect_report(&mut self, now: Instant, unix_ms: u64) {
        let reports = &mut self.reports;
        if reports.current().is_none() {
            return;
        }
        reports.current().unwrap().collection = self.scene_collections.current.clone();
        if let Some((first, _)) = self.workspace.activity.numbered().next() {
            let missed = first.saturating_sub(reports.activity_after.saturating_add(1));
            if missed > 0 {
                reports.current().unwrap().activity_gaps += missed;
                reports.revision += 1;
            }
        }
        for (sequence, record) in self.workspace.activity.numbered() {
            if sequence > reports.activity_after {
                reports.push(
                    record.timestamp_unix_ms,
                    ReportEvent::Activity(record.clone()),
                );
            }
        }
        reports.activity_after = self.workspace.activity.sequence();
        for entry in self.workspace.output_results.entries() {
            if entry.id > reports.output_baseline && reports.outputs.get(&entry.id) != Some(entry) {
                reports.push(entry.updated_unix_ms, ReportEvent::Output(entry.clone()));
                reports.outputs.insert(entry.id, entry.clone());
            }
        }
        reports.outputs.retain(|id, _| {
            self.workspace
                .output_results
                .entries()
                .any(|entry| entry.id == *id)
        });

        let scene = (
            self.scene_collections.current.clone(),
            self.scene_inventory.current_id.clone(),
        );
        if reports.scene.as_ref() != Some(&scene) {
            reports.push(
                unix_ms,
                ReportEvent::Scene {
                    collection: scene.0.clone(),
                    scene: scene.1.clone(),
                },
            );
            reports.scene = Some(scene);
        }
        if let (Some(at), Some(stats)) = (self.obs_stats_observed_at, self.obs_stats) {
            if at <= now && reports.sample_at.is_none_or(|previous| at > previous) {
                let timestamp =
                    unix_ms.saturating_sub(
                        now.duration_since(at).as_millis().min(u64::MAX as u128) as u64,
                    );
                reports.current().unwrap().sample(ReportSample::new(
                    timestamp,
                    stats,
                    self.stream_bitrate_kbps,
                    self.stream_health,
                ));
                reports.sample_at = Some(at);
                reports.revision += 1;
            }
        }
        let mut alerts = BTreeMap::new();
        for alert in self.health.alerts(now) {
            let (kind, input) = match alert.kind {
                AlertKind::ConnectionLost => ("connection_lost", None),
                AlertKind::StreamStopped => ("stream_stopped", None),
                AlertKind::RecordStopped => ("record_stopped", None),
                AlertKind::NetworkDrops => ("network_drops", None),
                AlertKind::RenderOverload => ("render_overload", None),
                AlertKind::EncoderOverload => ("encoder_overload", None),
                AlertKind::LowDisk => ("low_disk", None),
                AlertKind::AudioSilence(index) => {
                    ("audio_silence", Some(self.health.audio.name(index)))
                }
                AlertKind::AudioClipping(index) => {
                    ("audio_clipping", Some(self.health.audio.name(index)))
                }
            };
            alerts.insert((kind.to_owned(), input, alert.event_id), alert.phase);
        }
        for ((kind, input, id), phase) in &alerts {
            if reports.health.get(&(kind.clone(), input.clone(), *id)) != Some(phase) {
                let phase = match phase {
                    AlertPhase::Active => "active",
                    AlertPhase::Recovering => "recovering",
                    AlertPhase::Unobserved => "unobserved",
                };
                reports.push(
                    unix_ms,
                    ReportEvent::Health {
                        kind: kind.clone(),
                        input: input.clone(),
                        phase: Some(phase.into()),
                    },
                );
            }
        }
        let cleared: Vec<_> = reports
            .health
            .keys()
            .filter(|key| !alerts.contains_key(*key))
            .cloned()
            .collect();
        for (kind, input, _) in cleared {
            reports.push(
                unix_ms,
                ReportEvent::Health {
                    kind,
                    input,
                    phase: None,
                },
            );
        }
        reports.health = alerts;
        // A failed bookmark archive load may be retried during this connection.
        // Its session number can then become available without changing the clock.
        if let Some((session_id, started, _)) = self.bookmarks.session_context() {
            let report = reports.current().unwrap();
            if started == report.started_unix_ms {
                let ids: Vec<_> = self
                    .bookmarks
                    .archive
                    .entries
                    .iter()
                    .filter(|note| {
                        note.session_id == session_id && note.session_started_unix_ms == started
                    })
                    .map(|note| note.id)
                    .collect();
                if report.session_id != session_id || report.bookmark_ids != ids {
                    report.session_id = session_id;
                    report.bookmark_ids = ids;
                    reports.revision += 1;
                }
            }
        }
    }
}
