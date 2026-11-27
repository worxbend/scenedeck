//! Config saves from GTK callbacks: edit the cached snapshot in `AppState`,
//! then write it on the ordered config lane.

use crate::storage::config::{write_config, AppConfig};
use crate::ui::background_io::SerialLane;
use crate::ui::navigation::NavigationContext;

/// Edit the cached config and save it, retaining failures for the close barrier.
///
/// Replaceable snapshots share one pending write/publication per FIFO segment.
/// Acceptance is synchronous; superseded snapshots have no callback to lose.
/// A returned error or panic requires an explicit informed close decision;
/// later successful work does not silently acknowledge potentially lost edits.
pub(crate) fn persist_config(nav: &NavigationContext, update: impl FnOnce(&mut AppConfig)) {
    let config = {
        let mut state = nav.state.borrow_mut();
        update(&mut state.config);
        state.config.clone()
    };
    persist_config_snapshot(config);
}

/// Save a config the caller already holds, with the same failure retention as
/// `persist_config`.
pub(crate) fn persist_config_snapshot(config: AppConfig) {
    crate::ui::background_io::run_config_snapshot(move || write_config(&config));
}

/// Edit the cached config, save it, and report the outcome.
///
/// Unlike `persist_config`, this is a non-coalescible FIFO transaction barrier.
/// `complete` runs back on the GTK thread and is handed the config as it was
/// written, so a caller can re-apply it — the theme rows need exactly that, to
/// restyle from the values that actually reached disk.
pub(crate) fn persist_config_with<Update, Complete>(
    nav: &NavigationContext,
    update: Update,
    complete: Complete,
) where
    Update: FnOnce(&mut AppConfig),
    Complete: FnOnce(std::io::Result<()>, AppConfig) + 'static,
{
    let config = {
        let mut state = nav.state.borrow_mut();
        update(&mut state.config);
        state.config.clone()
    };
    let persisted = config.clone();
    crate::ui::background_io::run_serialized(
        SerialLane::Config,
        move || write_config(&persisted),
        move |result| complete(result, config),
    );
}
