//! Tracing subscriber setup.

use tracing_subscriber::EnvFilter;

/// Initialize tracing from `RUST_LOG`, with a SceneDeck-focused default.
pub fn init() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("scenedeck=debug,warn")),
        )
        .init();
}
