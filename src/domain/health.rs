//! Sustained incidents from recent OBS observations, never cumulative totals alone.
pub(crate) mod audio;
mod events;

/// Explicit opt-in audio expectations, stored in the connection/collection registry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct AudioRule {
    pub target: super::operator_action::ActionTarget,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub silence: bool,
    #[serde(default)]
    pub clipping: bool,
}

use super::stats::{ObsStats, StreamHealth};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

const MAX_GAP: Duration = Duration::from_secs(5);
const RAISE_AFTER: Duration = Duration::from_secs(5);
/// Recovery cooldown: brief healthy intervals do not repeatedly clear/recreate an incident.
const CLEAR_AFTER: Duration = Duration::from_secs(10);
const SKIPPED_RATIO: f64 = 0.01;
const LOW_DISK_MIB: f64 = 1024.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum AlertKind {
    ConnectionLost,
    StreamStopped,
    RecordStopped,
    NetworkDrops,
    RenderOverload,
    EncoderOverload,
    LowDisk,
    AudioSilence(usize),
    AudioClipping(usize),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AlertPhase {
    Active,
    Recovering,
    Unobserved,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Alert {
    pub event_id: Option<u64>,
    pub kind: AlertKind,
    pub phase: AlertPhase,
}

#[derive(Debug, Clone)]
struct Sustained {
    max_gap: Duration,
    raise_after: Duration,
    last: Option<Instant>,
    condition: Option<bool>,
    pending_since: Option<Instant>,
    clear_since: Option<Instant>,
    raised: bool,
}
impl Default for Sustained {
    fn default() -> Self {
        Self::new(MAX_GAP, RAISE_AFTER)
    }
}
impl Sustained {
    fn new(max_gap: Duration, raise_after: Duration) -> Self {
        Self {
            max_gap,
            raise_after,
            last: None,
            condition: None,
            pending_since: None,
            clear_since: None,
            raised: false,
        }
    }
    fn unavailable(&mut self) {
        self.condition = None;
        self.pending_since = None;
        self.clear_since = None;
    }
    fn observe(&mut self, condition: Option<bool>, at: Instant) {
        if self.last.is_some_and(|last| at <= last) {
            return;
        }
        if self
            .last
            .is_some_and(|last| at.duration_since(last) > self.max_gap)
        {
            self.pending_since = None;
            self.clear_since = None;
        }
        self.last = Some(at);
        self.condition = condition;
        match condition {
            Some(true) => {
                self.clear_since = None;
                let since = *self.pending_since.get_or_insert(at);
                if at.duration_since(since) >= self.raise_after {
                    self.raised = true;
                }
            }
            Some(false) => {
                self.pending_since = None;
                if self.raised {
                    let since = *self.clear_since.get_or_insert(at);
                    if at.duration_since(since) >= CLEAR_AFTER {
                        self.raised = false;
                        self.clear_since = None;
                    }
                }
            }
            None => {
                self.pending_since = None;
                self.clear_since = None;
            }
        }
    }
    fn phase(&self, now: Instant) -> Option<AlertPhase> {
        if !self.raised {
            return None;
        }
        Some(
            if self.condition.is_none()
                || self
                    .last
                    .is_none_or(|at| !fresh_within(at, now, self.max_gap))
            {
                AlertPhase::Unobserved
            } else if self.condition == Some(false) {
                AlertPhase::Recovering
            } else {
                AlertPhase::Active
            },
        )
    }
}
fn fresh(at: Instant, now: Instant) -> bool {
    fresh_within(at, now, MAX_GAP)
}
fn fresh_within(at: Instant, now: Instant, limit: Duration) -> bool {
    now.checked_duration_since(at)
        .is_some_and(|age| age <= limit)
}
fn skipped(current: (u32, u32), previous: (u32, u32)) -> Option<bool> {
    let (missed, total) = (
        current.0.checked_sub(previous.0)?,
        current.1.checked_sub(previous.1)?,
    );
    if current.0 > current.1 || previous.0 > previous.1 || total == 0 || missed > total {
        return None;
    }
    Some(f64::from(missed) / f64::from(total) >= SKIPPED_RATIO)
}

#[derive(Debug, Clone, Copy)]
struct Sample {
    at: Instant,
    stats: ObsStats,
    stream: Option<StreamHealth>,
    encoding: Option<bool>,
}
#[derive(Debug, Clone, Default)]
pub(crate) struct HealthMonitor {
    pub audio: audio::AudioMonitor,
    was_connected: bool,
    expected_disconnect: bool,
    outputs: [events::OutputWatch; 2],
    next_event: u64,
    events: BTreeMap<AlertKind, u64>,
    previous: Option<Sample>,
    conditions: BTreeMap<AlertKind, Sustained>,
}
impl HealthMonitor {
    pub fn observe_stats(
        &mut self,
        stats: ObsStats,
        stream: Option<StreamHealth>,
        recording: Option<bool>,
        encoding: Option<bool>,
        at: Instant,
        now: Instant,
    ) {
        if self.previous.is_some_and(|previous| at <= previous.at) {
            return;
        }
        let previous = self.previous.filter(|previous| fresh(previous.at, at));
        let valid = fresh(at, now);
        let render = previous.filter(|_| valid).and_then(|previous| {
            skipped(
                (stats.render_skipped_frames, stats.render_total_frames),
                (
                    previous.stats.render_skipped_frames,
                    previous.stats.render_total_frames,
                ),
            )
        });
        let encoder = if !valid {
            None
        } else if encoding == Some(false) {
            Some(false)
        } else {
            previous
                .filter(|previous| encoding == Some(true) && previous.encoding == Some(true))
                .and_then(|previous| {
                    skipped(
                        (stats.output_skipped_frames, stats.output_total_frames),
                        (
                            previous.stats.output_skipped_frames,
                            previous.stats.output_total_frames,
                        ),
                    )
                })
        };
        let network = stream.filter(|_| valid).and_then(|stream| {
            if !stream.active {
                return Some(false);
            }
            previous
                .and_then(|previous| previous.stream)
                .filter(|previous| previous.active)
                .and_then(|previous| {
                    skipped(
                        (stream.skipped_frames, stream.total_frames),
                        (previous.skipped_frames, previous.total_frames),
                    )
                })
        });
        let disk = if !valid {
            None
        } else if recording == Some(false) {
            Some(false)
        } else {
            stats
                .disk_space_mb
                .filter(|_| recording == Some(true))
                .filter(|space| space.is_finite() && *space >= 0.0)
                .map(|space| space < LOW_DISK_MIB)
        };
        for (kind, condition) in [
            (AlertKind::NetworkDrops, network),
            (AlertKind::RenderOverload, render),
            (AlertKind::EncoderOverload, encoder),
            (AlertKind::LowDisk, disk),
        ] {
            self.conditions
                .entry(kind)
                .or_default()
                .observe(condition, if valid { at } else { now });
        }
        self.previous = valid.then_some(Sample {
            at,
            stats,
            stream,
            encoding,
        });
    }
    pub fn alerts(&self, now: Instant) -> Vec<Alert> {
        let mut alerts: Vec<_> = self
            .conditions
            .iter()
            .filter_map(|(kind, condition)| {
                condition.phase(now).map(|phase| Alert {
                    kind: *kind,
                    phase,
                    event_id: None,
                })
            })
            .collect();
        alerts.extend(self.events.iter().map(|(kind, id)| Alert {
            kind: *kind,
            phase: AlertPhase::Active,
            event_id: Some(*id),
        }));
        alerts.extend(self.audio.alerts(now));
        alerts.sort_by_key(|alert| alert.kind);
        alerts
    }
    pub fn clear_samples(&mut self) {
        self.previous = None;
        self.conditions.clear();
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) fn stats() -> ObsStats {
        ObsStats {
            disk_space_mb: Some(4096.0),
            cpu_usage_percent: 10.0,
            memory_usage_mb: 128.0,
            configured_fps: Some(60.0),
            active_fps: 60.0,
            average_frame_render_time_ms: 1.0,
            render_skipped_frames: 0,
            render_total_frames: 1000,
            output_skipped_frames: 0,
            output_total_frames: 1000,
        }
    }
    fn stream() -> StreamHealth {
        StreamHealth {
            active: true,
            reconnecting: false,
            congestion: 0.0,
            skipped_frames: 0,
            total_frames: 1000,
            bytes: 0,
        }
    }
    fn feed(
        monitor: &mut HealthMonitor,
        stats: ObsStats,
        stream: Option<StreamHealth>,
        at: Instant,
    ) {
        monitor.observe_stats(stats, stream, Some(true), Some(true), at, at);
    }
    pub(crate) fn raise(
        monitor: &mut HealthMonitor,
        kind: AlertKind,
        start: Instant,
    ) -> (ObsStats, StreamHealth) {
        let mut stats = stats();
        let mut stream = stream();
        for seconds in 0..=6 {
            stats.render_total_frames += 100;
            stats.output_total_frames += 100;
            stream.total_frames += 100;
            match kind {
                AlertKind::NetworkDrops => stream.skipped_frames += 10,
                AlertKind::RenderOverload => stats.render_skipped_frames += 10,
                AlertKind::EncoderOverload => stats.output_skipped_frames += 10,
                AlertKind::LowDisk => stats.disk_space_mb = Some(512.0),
                _ => panic!("performance fixture requires a sampled alert"),
            }
            feed(
                monitor,
                stats,
                Some(stream),
                start + Duration::from_secs(seconds),
            );
        }
        (stats, stream)
    }
    #[test]
    fn cumulative_history_and_short_bursts_do_not_create_current_incidents() {
        let start = Instant::now();
        let mut monitor = HealthMonitor::default();
        let mut stats = stats();
        let mut stream = stream();
        stats.render_skipped_frames = 500;
        stats.output_skipped_frames = 500;
        stream.skipped_frames = 500;
        for seconds in 0..=20 {
            stats.render_total_frames += 100;
            stats.output_total_frames += 100;
            stream.total_frames += 100;
            if seconds == 3 {
                stats.render_skipped_frames += 20;
                stats.output_skipped_frames += 20;
                stream.skipped_frames += 20;
            }
            feed(
                &mut monitor,
                stats,
                Some(stream),
                start + Duration::from_secs(seconds),
            );
            assert!(monitor
                .alerts(start + Duration::from_secs(seconds))
                .is_empty());
        }
    }
    #[test]
    fn sustained_incidents_remain_distinct_and_cool_down_only_on_healthy_samples() {
        for kind in [
            AlertKind::NetworkDrops,
            AlertKind::RenderOverload,
            AlertKind::EncoderOverload,
            AlertKind::LowDisk,
        ] {
            let start = Instant::now();
            let mut monitor = HealthMonitor::default();
            let (mut stats, mut stream) = raise(&mut monitor, kind, start);
            assert_eq!(
                monitor.alerts(start + Duration::from_secs(6)),
                [Alert {
                    event_id: None,
                    kind,
                    phase: AlertPhase::Active
                }]
            );
            stats.disk_space_mb = Some(4096.0);
            for seconds in 7..=17 {
                stats.render_total_frames += 100;
                stats.output_total_frames += 100;
                stream.total_frames += 100;
                feed(
                    &mut monitor,
                    stats,
                    Some(stream),
                    start + Duration::from_secs(seconds),
                );
                assert_eq!(
                    monitor.alerts(start + Duration::from_secs(seconds)),
                    if seconds == 17 {
                        vec![]
                    } else {
                        vec![Alert {
                            event_id: None,
                            kind,
                            phase: AlertPhase::Recovering,
                        }]
                    }
                );
            }
        }
    }
    #[test]
    fn gaps_missing_or_stale_reads_are_unknown_and_never_advance_recovery() {
        let start = Instant::now();
        let mut monitor = HealthMonitor::default();
        let (mut stats, stream) = raise(&mut monitor, AlertKind::NetworkDrops, start);
        assert_eq!(
            monitor.alerts(start + Duration::from_secs(12))[0].phase,
            AlertPhase::Unobserved
        );
        feed(&mut monitor, stats, None, start + Duration::from_secs(13));
        assert_eq!(
            monitor.alerts(start + Duration::from_secs(13))[0].phase,
            AlertPhase::Unobserved
        );
        for seconds in 14..=30 {
            stats.render_total_frames += 100;
            stats.output_total_frames += 100;
            let at = start + Duration::from_secs(seconds);
            monitor.observe_stats(
                stats,
                Some(stream),
                Some(true),
                Some(true),
                at,
                at + Duration::from_secs(6),
            );
        }
        assert_eq!(
            monitor.alerts(start + Duration::from_secs(36))[0].phase,
            AlertPhase::Unobserved
        );
    }
    #[test]
    fn reset_or_invalid_frame_counters_do_not_look_healthy() {
        assert_eq!(skipped((2, 100), (0, 0)), Some(true));
        assert_eq!(skipped((0, 100), (0, 0)), Some(false));
        for (current, previous) in [
            ((1, 10), (50, 100)),
            ((100, 200), (1, 199)),
            ((101, 100), (0, 0)),
            ((1, 100), (0, 100)),
        ] {
            assert_eq!(skipped(current, previous), None);
        }
        let start = Instant::now();
        let mut monitor = HealthMonitor::default();
        let (mut stats, stream) = raise(&mut monitor, AlertKind::RenderOverload, start);
        stats.render_skipped_frames = 0;
        stats.render_total_frames = 1;
        feed(
            &mut monitor,
            stats,
            Some(stream),
            start + Duration::from_secs(7),
        );
        assert_eq!(
            monitor.alerts(start + Duration::from_secs(7))[0].phase,
            AlertPhase::Unobserved
        );
        monitor.clear_samples();
        assert!(monitor.alerts(start + Duration::from_secs(7)).is_empty());
        feed(
            &mut monitor,
            stats,
            Some(stream),
            start + Duration::from_secs(8),
        );
        assert!(monitor.alerts(start + Duration::from_secs(8)).is_empty());
    }
    #[test]
    fn duplicate_out_of_order_and_render_ticks_do_not_count_as_sustained_evidence() {
        let start = Instant::now();
        let mut monitor = HealthMonitor::default();
        let mut stats = stats();
        feed(&mut monitor, stats, None, start);
        stats.render_total_frames += 100;
        stats.render_skipped_frames += 10;
        feed(&mut monitor, stats, None, start + Duration::from_secs(1));
        for seconds in 2..=10 {
            feed(&mut monitor, stats, None, start + Duration::from_secs(1));
            feed(&mut monitor, stats, None, start);
            assert!(monitor
                .alerts(start + Duration::from_secs(seconds))
                .is_empty());
        }
        let at = start + Duration::from_secs(11);
        stats.render_total_frames += 100;
        stats.render_skipped_frames += 10;
        feed(&mut monitor, stats, None, at);
        assert!(monitor.alerts(at).is_empty(), "a gap restarts the baseline");
    }
    #[test]
    fn low_space_needs_recording_and_invalid_or_unknown_evidence_cannot_clear_it() {
        let start = Instant::now();
        let mut monitor = HealthMonitor::default();
        let mut stats = stats();
        stats.disk_space_mb = Some(0.0);
        for seconds in 0..=6 {
            let at = start + Duration::from_secs(seconds);
            monitor.observe_stats(stats, None, Some(false), Some(false), at, at);
        }
        assert!(monitor.alerts(start + Duration::from_secs(6)).is_empty());
        monitor.clear_samples();
        let (mut stats, _) = raise(&mut monitor, AlertKind::LowDisk, start);
        for (index, value) in [None, Some(f64::NAN), Some(f64::INFINITY), Some(-1.0)]
            .into_iter()
            .enumerate()
        {
            stats.disk_space_mb = value;
            let at = start + Duration::from_secs(index as u64 + 7);
            feed(&mut monitor, stats, None, at);
            assert_eq!(monitor.alerts(at)[0].phase, AlertPhase::Unobserved);
        }
        stats.disk_space_mb = Some(4096.0);
        for seconds in 11..=22 {
            let at = start + Duration::from_secs(seconds);
            monitor.observe_stats(stats, None, None, None, at, at);
            assert_eq!(monitor.alerts(at)[0].phase, AlertPhase::Unobserved);
        }
    }
    #[test]
    fn recurrence_interrupts_recovery_without_flickering_the_incident() {
        let start = Instant::now();
        let mut monitor = HealthMonitor::default();
        let (mut stats, stream) = raise(&mut monitor, AlertKind::LowDisk, start);
        for seconds in 7..=10 {
            stats.disk_space_mb = Some(4096.0);
            feed(
                &mut monitor,
                stats,
                Some(stream),
                start + Duration::from_secs(seconds),
            );
        }
        stats.disk_space_mb = Some(0.0);
        feed(
            &mut monitor,
            stats,
            Some(stream),
            start + Duration::from_secs(11),
        );
        assert_eq!(
            monitor.alerts(start + Duration::from_secs(11)),
            [Alert {
                event_id: None,
                kind: AlertKind::LowDisk,
                phase: AlertPhase::Active
            }]
        );
    }
}
