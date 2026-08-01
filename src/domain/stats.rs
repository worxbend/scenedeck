//! OBS process and rendering statistics (`GetStats`, `GetStreamStatus`).
//!
//! obs-websocket v5 has no server-pushed statistics event, so these snapshots
//! arrive from a poll loop that runs for as long as the OBS session is alive.
//! [`StatsHistory`] keeps the recent samples so the Stats page can draw them as
//! time series without re-querying OBS.

use std::collections::VecDeque;

/// Number of samples kept for the Stats page charts. At the one-second poll
/// cadence this is two minutes of history.
pub const STATS_HISTORY_CAPACITY: usize = 120;

/// Snapshot of OBS-reported performance counters, refreshed on a timer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObsStats {
    /// OBS process CPU usage in percent, as reported by OBS itself.
    pub cpu_usage_percent: f64,
    /// Memory currently used by OBS, in megabytes.
    pub memory_usage_mb: f64,
    /// Current FPS being rendered by OBS.
    pub active_fps: f64,
    /// Average time in milliseconds OBS takes to render a frame.
    pub average_frame_render_time_ms: f64,
    /// Frames skipped by the render thread since OBS started.
    pub render_skipped_frames: u32,
    /// Total frames produced by the render thread since OBS started.
    pub render_total_frames: u32,
    /// Frames skipped by the output (encoder) thread since OBS started.
    pub output_skipped_frames: u32,
    /// Total frames produced by the output thread since OBS started.
    pub output_total_frames: u32,
}

impl ObsStats {
    /// Frames dropped by the output encoder, the number streamers care about most.
    pub const fn dropped_frames(&self) -> u32 {
        self.output_skipped_frames
    }

    /// Share of render-thread frames that were missed, as a `0.0..=1.0`
    /// fraction. Zero while OBS has not produced any frames yet.
    pub fn render_skipped_ratio(&self) -> f64 {
        skipped_ratio(self.render_skipped_frames, self.render_total_frames)
    }

    /// Share of output-thread frames that were skipped, as a `0.0..=1.0`
    /// fraction. Zero while OBS has not produced any frames yet.
    pub fn output_skipped_ratio(&self) -> f64 {
        skipped_ratio(self.output_skipped_frames, self.output_total_frames)
    }
}

fn skipped_ratio(skipped: u32, total: u32) -> f64 {
    if total == 0 {
        return 0.0;
    }
    f64::from(skipped) / f64::from(total)
}

/// Stream-output health from `GetStreamStatus`. Only meaningful while the
/// stream output is active; OBS reports zeroed counters otherwise.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StreamHealth {
    /// Whether the stream output is currently active.
    pub active: bool,
    /// Whether the stream output is trying to reconnect to the ingest server.
    pub reconnecting: bool,
    /// Network congestion reported by OBS as a `0.0..=1.0` fraction.
    pub congestion: f64,
    /// Frames dropped by the stream output since it started.
    pub skipped_frames: u32,
    /// Total frames handled by the stream output since it started.
    pub total_frames: u32,
    /// Cumulative bytes sent by the stream output.
    pub bytes: u64,
}

impl StreamHealth {
    /// Share of stream-output frames that were dropped, as a `0.0..=1.0`
    /// fraction. Zero before the output has handled any frames.
    pub fn dropped_ratio(&self) -> f64 {
        skipped_ratio(self.skipped_frames, self.total_frames)
    }
}

/// One point in the recent statistics history.
///
/// OBS reports frame counters cumulatively since the process (or output)
/// started, so the deltas are derived on insert: they are what a "frames
/// dropped right now" chart needs, and the cumulative totals cannot show.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatsSample {
    /// Process counters at the time of this sample.
    pub stats: ObsStats,
    /// Rolling stream bitrate derived from consecutive byte counter reads.
    pub bitrate_kbps: Option<f64>,
    /// Stream output health, when the stream status read succeeded.
    pub stream: Option<StreamHealth>,
    /// Render-thread frames missed since the previous sample.
    pub render_skipped_delta: u32,
    /// Output-thread frames skipped since the previous sample.
    pub output_skipped_delta: u32,
}

/// A metric that can be plotted as a time series from [`StatsHistory`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatsMetric {
    /// Frames per second currently rendered by OBS.
    Fps,
    /// Average frame render time in milliseconds.
    FrameRenderTimeMs,
    /// Render-thread frames missed per poll interval.
    RenderSkippedPerInterval,
    /// Output-thread frames skipped per poll interval.
    OutputSkippedPerInterval,
    /// Stream bitrate in kbps; zero while no bitrate has been derived yet.
    BitrateKbps,
    /// OBS process CPU usage in percent.
    CpuPercent,
    /// Stream network congestion as a `0.0..=1.0` fraction.
    Congestion,
}

impl StatsMetric {
    /// Extract this metric's plottable value from a sample.
    pub fn value(self, sample: &StatsSample) -> f64 {
        match self {
            Self::Fps => sample.stats.active_fps,
            Self::FrameRenderTimeMs => sample.stats.average_frame_render_time_ms,
            Self::RenderSkippedPerInterval => f64::from(sample.render_skipped_delta),
            Self::OutputSkippedPerInterval => f64::from(sample.output_skipped_delta),
            Self::BitrateKbps => sample.bitrate_kbps.unwrap_or(0.0),
            Self::CpuPercent => sample.stats.cpu_usage_percent,
            Self::Congestion => sample.stream.map_or(0.0, |stream| stream.congestion),
        }
    }
}

/// Bounded ring buffer of recent statistics samples.
#[derive(Debug, Clone)]
pub struct StatsHistory {
    samples: VecDeque<StatsSample>,
    capacity: usize,
}

impl Default for StatsHistory {
    fn default() -> Self {
        Self::with_capacity(STATS_HISTORY_CAPACITY)
    }
}

impl StatsHistory {
    /// Create a history that keeps at most `capacity` samples. A zero capacity
    /// is treated as one so the latest sample is always retrievable.
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            samples: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Append a poll result, deriving the per-interval frame deltas against the
    /// previous sample and evicting the oldest sample once full.
    ///
    /// Counters that move backwards (OBS restarted, or the stream output was
    /// restarted) record a zero delta rather than a nonsensical spike.
    pub fn push(
        &mut self,
        stats: ObsStats,
        bitrate_kbps: Option<f64>,
        stream: Option<StreamHealth>,
    ) {
        let previous = self.samples.back().map(|sample| sample.stats);
        let sample = StatsSample {
            stats,
            bitrate_kbps,
            stream,
            render_skipped_delta: previous.map_or(0, |previous| {
                stats
                    .render_skipped_frames
                    .saturating_sub(previous.render_skipped_frames)
            }),
            output_skipped_delta: previous.map_or(0, |previous| {
                stats
                    .output_skipped_frames
                    .saturating_sub(previous.output_skipped_frames)
            }),
        };

        if self.samples.len() == self.capacity {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }

    /// Most recent sample, if any poll has completed.
    pub fn latest(&self) -> Option<&StatsSample> {
        self.samples.back()
    }

    /// Oldest-to-newest values for `metric`, ready to plot.
    pub fn series(&self, metric: StatsMetric) -> Vec<f64> {
        self.samples
            .iter()
            .map(|sample| metric.value(sample))
            .collect()
    }

    /// Number of samples currently retained.
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Whether no sample has been recorded yet.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Drop every retained sample, e.g. when the OBS session ends.
    pub fn clear(&mut self) {
        self.samples.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats() -> ObsStats {
        ObsStats {
            cpu_usage_percent: 12.5,
            memory_usage_mb: 512.0,
            active_fps: 59.9,
            average_frame_render_time_ms: 4.2,
            render_skipped_frames: 1,
            render_total_frames: 1000,
            output_skipped_frames: 7,
            output_total_frames: 999,
        }
    }

    fn health() -> StreamHealth {
        StreamHealth {
            active: true,
            reconnecting: false,
            congestion: 0.25,
            skipped_frames: 4,
            total_frames: 400,
            bytes: 1_000_000,
        }
    }

    #[test]
    fn dropped_frames_reports_output_skipped_frames() {
        assert_eq!(stats().dropped_frames(), 7);
    }

    #[test]
    fn skipped_ratios_are_zero_before_any_frame_is_produced() {
        let idle = ObsStats {
            render_total_frames: 0,
            output_total_frames: 0,
            ..stats()
        };

        assert_eq!(idle.render_skipped_ratio(), 0.0);
        assert_eq!(idle.output_skipped_ratio(), 0.0);
    }

    #[test]
    fn skipped_ratios_divide_by_the_matching_total() {
        let stats = ObsStats {
            render_skipped_frames: 5,
            render_total_frames: 100,
            output_skipped_frames: 2,
            output_total_frames: 50,
            ..stats()
        };

        assert!((stats.render_skipped_ratio() - 0.05).abs() < f64::EPSILON);
        assert!((stats.output_skipped_ratio() - 0.04).abs() < f64::EPSILON);
    }

    #[test]
    fn stream_dropped_ratio_divides_skipped_by_total() {
        assert!((health().dropped_ratio() - 0.01).abs() < f64::EPSILON);
    }

    #[test]
    fn first_sample_has_zero_deltas() {
        let mut history = StatsHistory::default();
        history.push(stats(), None, None);

        let sample = history.latest().expect("a sample was pushed");
        assert_eq!(sample.render_skipped_delta, 0);
        assert_eq!(sample.output_skipped_delta, 0);
    }

    #[test]
    fn deltas_are_measured_against_the_previous_sample() {
        let mut history = StatsHistory::default();
        history.push(stats(), None, None);
        history.push(
            ObsStats {
                render_skipped_frames: 4,
                output_skipped_frames: 10,
                ..stats()
            },
            None,
            None,
        );

        let sample = history.latest().expect("a second sample was pushed");
        assert_eq!(sample.render_skipped_delta, 3);
        assert_eq!(sample.output_skipped_delta, 3);
    }

    #[test]
    fn counter_resets_record_a_zero_delta_instead_of_underflowing() {
        let mut history = StatsHistory::default();
        history.push(stats(), None, None);
        history.push(
            ObsStats {
                render_skipped_frames: 0,
                output_skipped_frames: 0,
                ..stats()
            },
            None,
            None,
        );

        let sample = history.latest().expect("a second sample was pushed");
        assert_eq!(sample.render_skipped_delta, 0);
        assert_eq!(sample.output_skipped_delta, 0);
    }

    #[test]
    fn history_evicts_the_oldest_sample_once_full() {
        let mut history = StatsHistory::with_capacity(2);
        for fps in [10.0, 20.0, 30.0] {
            history.push(
                ObsStats {
                    active_fps: fps,
                    ..stats()
                },
                None,
                None,
            );
        }

        assert_eq!(history.len(), 2);
        assert_eq!(history.series(StatsMetric::Fps), vec![20.0, 30.0]);
    }

    #[test]
    fn series_reads_metrics_oldest_to_newest() {
        let mut history = StatsHistory::default();
        history.push(stats(), Some(6000.0), Some(health()));
        history.push(
            ObsStats {
                output_skipped_frames: 9,
                ..stats()
            },
            Some(6100.0),
            Some(health()),
        );

        assert_eq!(
            history.series(StatsMetric::BitrateKbps),
            vec![6000.0, 6100.0]
        );
        assert_eq!(
            history.series(StatsMetric::OutputSkippedPerInterval),
            vec![0.0, 2.0]
        );
        assert_eq!(history.series(StatsMetric::Congestion), vec![0.25, 0.25]);
    }

    #[test]
    fn missing_optional_metrics_plot_as_zero() {
        let mut history = StatsHistory::default();
        history.push(stats(), None, None);

        assert_eq!(history.series(StatsMetric::BitrateKbps), vec![0.0]);
        assert_eq!(history.series(StatsMetric::Congestion), vec![0.0]);
    }

    #[test]
    fn clearing_drops_every_sample() {
        let mut history = StatsHistory::default();
        history.push(stats(), None, None);
        history.clear();

        assert!(history.is_empty());
        assert!(history.latest().is_none());
    }
}
