//! Saving local state from GTK callbacks.
//!
//! Local persistence deliberately does not travel through the
//! `AppCommand`/`AppEvent` stream (see CLAUDE.md): the config snapshot lives in
//! `AppState`, page callbacks edit it, and the blocking write goes out through
//! `ui::background_io`. That protocol has three steps that are easy to get
//! subtly wrong — take the borrow, edit the cached snapshot, clone it before it
//! moves into the worker thread, and let the borrow go before the write starts
//! — so it lives here once instead of being retyped at every call site.
//!
//! This is separate from `background_io`, which is deliberately storage
//! agnostic and also carries keyring access, CSS loads, and YAML exports.

use crate::storage::config::{write_config, AppConfig};
use crate::storage::registry::{write_registry, SceneRegistry};
use crate::ui::navigation::NavigationContext;

/// Edit the cached config and save it, logging a failure.
///
/// The common case: nothing to report to the user beyond the fact that the
/// setting did not stick.
pub(crate) fn persist_config(nav: &NavigationContext, update: impl FnOnce(&mut AppConfig)) {
    persist_config_with(nav, update, |result, _| {
        if let Err(error) = result {
            tracing::warn!(%error, "failed to save configuration");
        }
    });
}

/// Edit the cached config, save it, and report the outcome.
///
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
    crate::ui::background_io::run(
        move || write_config(&persisted),
        move |result| complete(result, config),
    );
}

/// Edit the cached scene registry and save it if anything changed.
///
/// `update` reports whether it changed anything; when it did not, nothing is
/// written. That matters because several callers fire on signals that repeat —
/// a colour picker closing on the colour it opened with, a drag that ends where
/// it started — and rewriting the file for those would be pure churn.
///
/// Returns what `update` reported, which callers use to decide whether to
/// update their widgets.
///
/// `what` names the change for the log, because "failed to write registry"
/// alone does not say whether the user lost a role, an accent, or the order.
pub(crate) fn persist_registry(
    nav: &NavigationContext,
    what: &'static str,
    update: impl FnOnce(&mut SceneRegistry) -> bool,
) -> bool {
    let registry = {
        let mut state = nav.state.borrow_mut();
        if !update(&mut state.registry) {
            return false;
        }
        state.registry.clone()
    };

    crate::ui::background_io::run(
        move || write_registry(&registry),
        move |result| {
            if let Err(error) = result {
                tracing::warn!(%error, what, "failed to save the scene registry");
            }
        },
    );
    true
}

/// Edit one field of the cached scene registry, then apply the same change to
/// the file on disk without rewriting the parts of it we did not touch.
///
/// This is the read-modify-write sibling of [`persist_registry`], and the
/// difference matters. `persist_registry` sends the whole cached snapshot to
/// the file, which is correct for changes that own the snapshot (reordering
/// scenes) but destructive if the file on disk failed to parse at startup:
/// the cache would be `SceneRegistry::default()` and the write would replace
/// the user's registry with an empty one. The `disk` closure here goes through
/// `storage::registry::mutate_registry`, which re-reads the file first and
/// refuses to overwrite one it cannot parse.
///
/// `cached` updates the in-memory snapshot so the UI reads back what the user
/// just picked; returning `false` means nothing changed and skips the write.
/// `what` names the change for the log, because "failed to write registry"
/// alone does not say whether the user lost an icon, a role, or an accent.
pub(crate) fn persist_registry_field<Cached, Disk>(
    nav: &NavigationContext,
    what: &'static str,
    cached: Cached,
    disk: Disk,
) where
    Cached: FnOnce(&mut SceneRegistry) -> bool,
    Disk:
        FnOnce() -> Result<bool, crate::storage::registry::RegistryMutationError> + Send + 'static,
{
    let changed = {
        let mut state = nav.state.borrow_mut();
        cached(&mut state.registry)
    };
    if !changed {
        return;
    }
    crate::ui::background_io::run(disk, move |result| {
        if let Err(error) = result {
            tracing::warn!(%error, what, "failed to save the scene registry");
        }
    });
}
