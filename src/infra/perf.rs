//! Opt-in hot-path counters for the `perf-trace` feature.
//!
//! Timing comes from `tracing::instrument` spans on the hot functions, gated
//! by the same feature. This module owns the counts a span cannot express and
//! logs their per-second rates at debug level every ten seconds. Without the
//! feature, [`count`] is an empty function and nothing else is compiled.

/// One counted hot-path occurrence.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Counter {
    /// One `AppEvent` applied by the GTK event pump.
    EventDrained,
    /// One event-pump tick that applied at least one event.
    PumpBatch,
    /// One volume-meter frame the controller could not queue.
    MeterFrameDropped,
    /// One `AppState::collect_report` call.
    ReportCollection,
}

/// Record one occurrence of `counter`.
#[cfg(feature = "perf-trace")]
pub(crate) fn count(counter: Counter) {
    enabled::COUNTERS.add(counter);
}

/// Record one occurrence of `counter`. The `perf-trace` feature is off.
#[cfg(not(feature = "perf-trace"))]
pub(crate) fn count(_counter: Counter) {}

#[cfg(feature = "perf-trace")]
pub(crate) use enabled::start_reporter;

#[cfg(feature = "perf-trace")]
mod enabled {
    use super::Counter;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, Instant};

    const REPORT_INTERVAL: Duration = Duration::from_secs(10);

    pub(super) static COUNTERS: Counters = Counters::new();

    pub(super) struct Counters {
        events_drained: AtomicU64,
        pump_batches: AtomicU64,
        meter_frames_dropped: AtomicU64,
        report_collections: AtomicU64,
    }

    /// Counts taken since the previous [`Counters::take`].
    #[derive(Debug, PartialEq, Eq)]
    struct Counts {
        events_drained: u64,
        pump_batches: u64,
        meter_frames_dropped: u64,
        report_collections: u64,
    }

    impl Counters {
        const fn new() -> Self {
            Self {
                events_drained: AtomicU64::new(0),
                pump_batches: AtomicU64::new(0),
                meter_frames_dropped: AtomicU64::new(0),
                report_collections: AtomicU64::new(0),
            }
        }

        pub(super) fn add(&self, counter: Counter) {
            let slot = match counter {
                Counter::EventDrained => &self.events_drained,
                Counter::PumpBatch => &self.pump_batches,
                Counter::MeterFrameDropped => &self.meter_frames_dropped,
                Counter::ReportCollection => &self.report_collections,
            };
            // Each counter is independent; no ordering with other memory is needed.
            slot.fetch_add(1, Ordering::Relaxed);
        }

        /// Read every counter and reset it to zero.
        fn take(&self) -> Counts {
            Counts {
                events_drained: self.events_drained.swap(0, Ordering::Relaxed),
                pump_batches: self.pump_batches.swap(0, Ordering::Relaxed),
                meter_frames_dropped: self.meter_frames_dropped.swap(0, Ordering::Relaxed),
                report_collections: self.report_collections.swap(0, Ordering::Relaxed),
            }
        }
    }

    /// Spawn the thread that logs counter rates every ten seconds.
    ///
    /// It sleeps between reports and ends with the process. A spawn failure
    /// costs only the rates, so it is logged and the app carries on.
    pub(crate) fn start_reporter() {
        let spawned = std::thread::Builder::new()
            .name("perf-trace".into())
            .spawn(report_forever);
        if let Err(error) = spawned {
            tracing::warn!(%error, "perf-trace reporter thread did not start");
        }
    }

    fn report_forever() {
        let mut since = Instant::now();
        loop {
            std::thread::sleep(REPORT_INTERVAL);
            let counts = COUNTERS.take();
            let window = since.elapsed();
            since = Instant::now();
            tracing::debug!(
                events_drained_per_s = per_second(counts.events_drained, window),
                pump_batches_per_s = per_second(counts.pump_batches, window),
                meter_frames_dropped_per_s = per_second(counts.meter_frames_dropped, window),
                collect_report_calls_per_s = per_second(counts.report_collections, window),
                "perf-trace counter rates"
            );
        }
    }

    /// `window` is at least one sleep long, so it is never zero.
    fn per_second(count: u64, window: Duration) -> f64 {
        count as f64 / window.as_secs_f64()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn take_returns_each_counter_and_resets_all_of_them() {
            let counters = Counters::new();
            for counter in [
                Counter::EventDrained,
                Counter::EventDrained,
                Counter::EventDrained,
                Counter::PumpBatch,
                Counter::MeterFrameDropped,
                Counter::MeterFrameDropped,
                Counter::ReportCollection,
            ] {
                counters.add(counter);
            }

            assert_eq!(
                counters.take(),
                Counts {
                    events_drained: 3,
                    pump_batches: 1,
                    meter_frames_dropped: 2,
                    report_collections: 1,
                }
            );
            assert_eq!(
                counters.take(),
                Counts {
                    events_drained: 0,
                    pump_batches: 0,
                    meter_frames_dropped: 0,
                    report_collections: 0,
                }
            );
        }

        #[test]
        fn per_second_divides_by_the_measured_window() {
            assert_eq!(per_second(25, Duration::from_secs(10)), 2.5);
            assert_eq!(per_second(0, Duration::from_secs(10)), 0.0);
            assert_eq!(per_second(3, Duration::from_millis(1_500)), 2.0);
        }
    }
}
