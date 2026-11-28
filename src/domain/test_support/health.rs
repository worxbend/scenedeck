//! Health monitor samples and the incidents they raise.

use std::time::{Duration, Instant};

use crate::domain::health::{AlertKind, HealthMonitor};
use crate::domain::stats::{ObsStats, StreamHealth};

/// Healthy statistics: nothing skipped, plenty of disk.
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

/// A healthy active stream.
pub(crate) fn stream() -> StreamHealth {
    StreamHealth {
        active: true,
        reconnecting: false,
        congestion: 0.0,
        skipped_frames: 0,
        total_frames: 1000,
        bytes: 0,
    }
}

/// One stats sample with both outputs active, observed and received at `at`.
pub(crate) fn feed(
    monitor: &mut HealthMonitor,
    stats: ObsStats,
    stream: Option<StreamHealth>,
    at: Instant,
) {
    monitor.observe_stats(stats, stream, Some(true), Some(true), at, at);
}

/// Feed seven seconds of samples that raise the sampled alert `kind`, and
/// return the last sample.
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
