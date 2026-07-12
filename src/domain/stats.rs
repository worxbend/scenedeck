//! OBS process and rendering statistics (`GetStats`).

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropped_frames_reports_output_skipped_frames() {
        let stats = ObsStats {
            cpu_usage_percent: 12.5,
            memory_usage_mb: 512.0,
            active_fps: 59.9,
            average_frame_render_time_ms: 4.2,
            render_skipped_frames: 1,
            render_total_frames: 1000,
            output_skipped_frames: 7,
            output_total_frames: 999,
        };

        assert_eq!(stats.dropped_frames(), 7);
    }
}
