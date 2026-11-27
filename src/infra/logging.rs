//! Tracing subscriber setup.

use tracing_subscriber::EnvFilter;

/// Initialize tracing from `RUST_LOG`, with a SceneDeck-focused default.
pub fn init() {
    let subscriber = tracing_subscriber::fmt().with_env_filter(
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("scenedeck=debug,warn")),
    );
    // A close event carries time.busy and time.idle for each perf-trace span.
    #[cfg(feature = "perf-trace")]
    let subscriber = subscriber.with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE);
    subscriber.init();
}
