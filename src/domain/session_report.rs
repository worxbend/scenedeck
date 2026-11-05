//! Bounded factual session reports. Acknowledgements never supply file evidence.

use super::{
    bookmark::Bookmark,
    output::OutputKind,
    output_result::OutputResult,
    stats::{ObsStats, StreamHealth},
    workflow::ActivityRecord,
};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub(crate) const EVENT_LIMIT: usize = 4_096;
pub(crate) const SAMPLE_LIMIT: usize = 2_048;
pub(crate) const HISTORY_LIMIT: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct HistoryPolicy {
    pub enabled: bool,
    pub retain_sessions: usize,
}

impl Default for HistoryPolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            retain_sessions: 8,
        }
    }
}

impl HistoryPolicy {
    pub fn limit(&self) -> usize {
        self.retain_sessions.clamp(1, HISTORY_LIMIT)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub(crate) enum ReportEvent {
    Activity(ActivityRecord),
    Output(OutputResult),
    OutputAcknowledged {
        kind: OutputKind,
    },
    Scene {
        collection: Option<String>,
        scene: Option<String>,
    },
    Health {
        kind: String,
        input: Option<String>,
        phase: Option<String>,
    },
    Ended {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ReportRecord {
    pub timestamp_unix_ms: u64,
    pub collection: Option<String>,
    pub event: ReportEvent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ReportSample {
    pub timestamp_unix_ms: u64,
    pub cpu_percent: Option<f64>,
    pub fps: Option<f64>,
    pub render_ms: Option<f64>,
    pub free_disk_mib: Option<f64>,
    pub bitrate_kbps: Option<f64>,
    pub render_skipped_frames: u32,
    pub output_skipped_frames: u32,
    pub stream_skipped_frames: Option<u32>,
}

fn measured(value: f64) -> Option<f64> {
    (value.is_finite() && value >= 0.0).then_some(value)
}

impl ReportSample {
    pub fn new(
        timestamp_unix_ms: u64,
        stats: ObsStats,
        bitrate: Option<f64>,
        stream: Option<StreamHealth>,
    ) -> Self {
        Self {
            timestamp_unix_ms,
            cpu_percent: measured(stats.cpu_usage_percent),
            fps: measured(stats.active_fps),
            render_ms: measured(stats.average_frame_render_time_ms),
            free_disk_mib: stats.disk_space_mb.and_then(measured),
            bitrate_kbps: bitrate.and_then(measured),
            render_skipped_frames: stats.render_skipped_frames,
            output_skipped_frames: stats.output_skipped_frames,
            stream_skipped_frames: stream
                .filter(|stream| stream.active)
                .map(|stream| stream.skipped_frames),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SessionReport {
    pub version: u32,
    pub id: String,
    pub session_id: u64,
    pub connection_id: String,
    #[serde(default)]
    pub connection_name: Option<String>,
    pub endpoint: String,
    pub started_unix_ms: u64,
    pub ended_unix_ms: Option<u64>,
    /// Loaded from a previous app run without a recorded session end.
    #[serde(default)]
    pub interrupted: bool,
    pub collection: Option<String>,
    pub events: VecDeque<ReportRecord>,
    pub omitted_events: u64,
    pub activity_gaps: u64,
    pub samples: VecDeque<ReportSample>,
    pub samples_seen: u64,
    pub omitted_samples: u64,
    pub peak_cpu_percent: Option<f64>,
    pub minimum_fps: Option<f64>,
    /// Resolve against the separately saved bookmark archive; deletion stays deletion.
    pub bookmark_ids: Vec<u64>,
}

impl SessionReport {
    pub fn new(
        session_id: u64,
        started_unix_ms: u64,
        connection_id: String,
        endpoint: String,
    ) -> Self {
        Self {
            version: 1,
            id: format!("{started_unix_ms}-{session_id}"),
            session_id,
            connection_id,
            connection_name: None,
            endpoint,
            started_unix_ms,
            ended_unix_ms: None,
            interrupted: false,
            collection: None,
            events: VecDeque::new(),
            omitted_events: 0,
            activity_gaps: 0,
            samples: VecDeque::new(),
            samples_seen: 0,
            omitted_samples: 0,
            peak_cpu_percent: None,
            minimum_fps: None,
            bookmark_ids: Vec::new(),
        }
    }

    pub fn push(&mut self, timestamp_unix_ms: u64, event: ReportEvent) {
        if let ReportEvent::Scene { collection, .. } = &event {
            self.collection = collection.clone();
        }
        while self.events.len() >= EVENT_LIMIT {
            self.events.pop_front();
            self.omitted_events += 1;
        }
        self.events.push_back(ReportRecord {
            timestamp_unix_ms,
            collection: self.collection.clone(),
            event,
        });
    }

    pub fn sample(&mut self, sample: ReportSample) {
        self.samples_seen += 1;
        if let Some(cpu) = sample.cpu_percent {
            self.peak_cpu_percent = Some(
                self.peak_cpu_percent
                    .map_or(cpu, |previous| previous.max(cpu)),
            );
        }
        if let Some(fps) = sample.fps {
            self.minimum_fps = Some(self.minimum_fps.map_or(fps, |previous| previous.min(fps)));
        }
        while self.samples.len() >= SAMPLE_LIMIT {
            self.samples.pop_front();
            self.omitted_samples += 1;
        }
        self.samples.push_back(sample);
    }

    pub fn bookmarks(&self, archive: &[Bookmark]) -> Vec<Bookmark> {
        archive
            .iter()
            .filter(|note| {
                note.session_id == self.session_id
                    && note.session_started_unix_ms == self.started_unix_ms
                    && self.bookmark_ids.contains(&note.id)
            })
            .cloned()
            .collect()
    }
}

/// A review/export snapshot. Bookmarks use their latest saved/local names.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ReportDocument {
    pub report: SessionReport,
    pub bookmarks: Vec<Bookmark>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_disclose_omissions_and_keep_aggregates_from_evicted_samples() {
        let mut report = SessionReport::new(1, 10, "show".into(), "obs:4455".into());
        for index in 0..EVENT_LIMIT + 3 {
            report.push(
                index as u64,
                ReportEvent::Scene {
                    collection: Some("Interview".into()),
                    scene: Some(index.to_string()),
                },
            );
        }
        assert_eq!(report.events.len(), EVENT_LIMIT);
        assert_eq!(report.omitted_events, 3);
        assert_eq!(report.events.front().unwrap().timestamp_unix_ms, 3);
        assert!(report
            .events
            .iter()
            .all(|event| event.collection.as_deref() == Some("Interview")));
        let mut stats = crate::domain::health::tests::stats();
        for index in 0..SAMPLE_LIMIT + 2 {
            stats.cpu_usage_percent = if index == 0 { 90.0 } else { 10.0 };
            stats.active_fps = if index == 0 { 12.0 } else { 60.0 };
            report.sample(ReportSample::new(index as u64, stats, None, None));
        }
        assert_eq!(report.samples.len(), SAMPLE_LIMIT);
        assert_eq!(report.samples_seen, SAMPLE_LIMIT as u64 + 2);
        assert_eq!(report.omitted_samples, 2);
        assert_eq!(report.peak_cpu_percent, Some(90.0));
        assert_eq!(report.minimum_fps, Some(12.0));
        assert_eq!(report.samples.front().unwrap().cpu_percent, Some(10.0));
    }

    #[test]
    fn invalid_measurements_remain_unavailable_after_json_roundtrip() {
        let mut stats = crate::domain::health::tests::stats();
        stats.cpu_usage_percent = f64::NAN;
        stats.active_fps = f64::INFINITY;
        stats.disk_space_mb = Some(-1.0);
        let sample = ReportSample::new(10, stats, Some(f64::NEG_INFINITY), None);
        let decoded: ReportSample =
            serde_json::from_str(&serde_json::to_string(&sample).unwrap()).unwrap();
        assert_eq!(decoded, sample);
        assert_eq!(sample.cpu_percent, None);
        assert_eq!(sample.fps, None);
        assert_eq!(sample.free_disk_mib, None);
        assert_eq!(sample.bitrate_kbps, None);
        assert_eq!(sample.stream_skipped_frames, None);
    }
}
