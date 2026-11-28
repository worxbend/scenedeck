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

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use crate::controller::state::AppState;
use crate::storage::config::write_config;
use crate::storage::registry::SceneRegistry;
use crate::storage::show_workspaces::RegistryTarget;
use crate::ui::background_io::SerialLane;
use crate::ui::navigation::NavigationContext;

mod config;
mod registry_scope;

pub(crate) use config::{persist_config, persist_config_snapshot, persist_config_with};
pub(crate) use registry_scope::select_registry_scope;

type RegistryWrite = std::sync::Arc<dyn Fn() -> std::io::Result<()> + Send + Sync>;

struct RegistryOperation {
    target: RegistryTarget,
    write: RegistryWrite,
    succeeded: bool,
}

#[derive(Default)]
struct RegistryWrites {
    revision: u64,
    // Keep successful later fields behind a failed predecessor: retrying an
    // old field must replay its successors, not overwrite newer accepted edits.
    operations: std::collections::BTreeMap<u64, RegistryOperation>,
    failures: Vec<crate::ui::background_io::FailureEvidence>,
    error: Option<String>,
    retrying: bool,
    scene_view: SceneViewSave,
}

/// A scene view edit waiting for the search to settle before it is saved.
#[derive(Default)]
struct SceneViewSave {
    /// Bumped by every edit, so only the timer of the latest edit saves.
    edit: u64,
    pending: Option<NavigationContext>,
}

thread_local! {
    static REGISTRY_WRITES: RefCell<RegistryWrites> = RefCell::new(RegistryWrites::default());
}

/// Confirmed/dirty state for the application-owned registry writer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RegistrySaveStatus {
    Saved,
    Saving,
    Failed(String),
}

pub(crate) fn registry_save_status() -> RegistrySaveStatus {
    REGISTRY_WRITES.with(|writes| {
        let writes = writes.borrow();
        if writes.retrying {
            RegistrySaveStatus::Saving
        } else if let Some(error) = &writes.error {
            RegistrySaveStatus::Failed(error.clone())
        } else if writes.operations.is_empty() {
            RegistrySaveStatus::Saved
        } else {
            RegistrySaveStatus::Saving
        }
    })
}

fn confirm_registry_through(writes: &mut RegistryWrites, revision: u64) {
    writes.operations.retain(|id, _| *id > revision);
}

fn confirm_registry_target_through(
    writes: &mut RegistryWrites,
    revision: u64,
    target: &RegistryTarget,
) {
    writes
        .operations
        .retain(|id, operation| *id > revision || &operation.target != target);
}

fn registry_write_finished(revision: u64, covers_snapshot: bool, result: std::io::Result<()>) {
    REGISTRY_WRITES.with(|writes| {
        let mut writes = writes.borrow_mut();
        match result {
            Ok(()) => {
                let Some(target) = writes
                    .operations
                    .get(&revision)
                    .map(|operation| operation.target.clone())
                else {
                    return;
                };
                if covers_snapshot {
                    confirm_registry_target_through(&mut writes, revision, &target);
                } else if let Some(operation) = writes.operations.get_mut(&revision) {
                    operation.succeeded = true;
                    let through = writes
                        .operations
                        .iter()
                        .filter(|(_, operation)| operation.target == target)
                        .take_while(|(_, operation)| operation.succeeded)
                        .map(|(id, _)| *id)
                        .last();
                    if let Some(through) = through {
                        confirm_registry_target_through(&mut writes, through, &target);
                    }
                }
            }
            Err(error) => {
                tracing::warn!(%error, revision, "failed to save the scene registry");
                writes
                    .failures
                    .push(crate::ui::background_io::record_failure());
                writes.error = Some(crate::ui::presenters::save_error::save_error_text(&error));
            }
        }
    });
}

fn submit_registry_write_to(
    what: &'static str,
    covers_snapshot: bool,
    target: RegistryTarget,
    write: RegistryWrite,
) {
    let revision = REGISTRY_WRITES.with(|writes| {
        let mut writes = writes.borrow_mut();
        writes.revision += 1;
        let revision = writes.revision;
        writes.operations.insert(
            revision,
            RegistryOperation {
                target,
                write: write.clone(),
                succeeded: false,
            },
        );
        revision
    });
    crate::ui::background_io::run_serialized(
        SerialLane::Registry,
        move || write(),
        move |result| {
            tracing::debug!(what, revision, "registry write completed");
            registry_write_finished(revision, covers_snapshot, result);
        },
    );
}

/// Retry retained registry intent, admitted synchronously before Close/Quit.
/// Only an explicit successful retry resolves its captured failure tickets;
/// unrelated panics and failures from newer edits remain visible to the barrier.
pub(crate) fn retry_registry_writes() -> bool {
    if !registry_edit_allowed() {
        return false;
    }
    let retry = REGISTRY_WRITES.with(|writes| {
        let mut writes = writes.borrow_mut();
        if writes.retrying || writes.error.is_none() {
            return None;
        }
        writes.retrying = true;
        Some((
            writes.revision,
            writes
                .operations
                .values()
                .map(|operation| operation.write.clone())
                .collect::<Vec<_>>(),
            writes.failures.len(),
        ))
    });
    let Some((revision, operations, failure_count)) = retry else {
        return false;
    };
    crate::ui::background_io::run_serialized(
        SerialLane::Registry,
        move || operations.iter().try_for_each(|write| write()),
        move |result| {
            let succeeded = result.is_ok();
            registry_write_finished(revision, true, result);
            REGISTRY_WRITES.with(|writes| {
                let mut writes = writes.borrow_mut();
                writes.retrying = false;
                if succeeded {
                    confirm_registry_through(&mut writes, revision);
                    for evidence in writes.failures.drain(..failure_count) {
                        crate::ui::background_io::resolve_failure(evidence);
                    }
                    if writes.failures.is_empty() {
                        writes.error = None;
                    }
                }
            });
        },
    );
    true
}

thread_local! {
    static REGISTRY_IMPORTING: Rc<Cell<bool>> = Rc::new(Cell::new(false));
    static AFTER_REGISTRY_IMPORT: RefCell<Vec<Box<dyn FnOnce()>>> = RefCell::new(Vec::new());
}

/// Observed identity changes must follow an import's disk commit and cache
/// publication. Unlike user edits, they cannot be retried by clicking again.
/// Callers retain their session binding and recheck it when the job runs.
pub(crate) fn after_registry_import(job: impl FnOnce() + 'static) -> bool {
    if REGISTRY_IMPORTING.with(|busy| busy.get()) {
        AFTER_REGISTRY_IMPORT.with(|pending| {
            let mut pending = pending.borrow_mut();
            if pending.len() >= 64 {
                return false;
            }
            pending.push(Box::new(job));
            true
        })
    } else {
        job();
        true
    }
}

/// All registry mutation entry points share this gate through disk commit and
/// GTK publication. A dropped/panicked read also releases it via RAII.
struct RegistryImportGuard(Rc<Cell<bool>>);

impl Drop for RegistryImportGuard {
    fn drop(&mut self) {
        self.0.set(false);
        while !self.0.get() {
            let next = AFTER_REGISTRY_IMPORT.with(|jobs| {
                let mut jobs = jobs.borrow_mut();
                (!jobs.is_empty()).then(|| jobs.remove(0))
            });
            let Some(job) = next else {
                break;
            };
            job();
        }
    }
}

fn begin_registry_import() -> Option<RegistryImportGuard> {
    REGISTRY_IMPORTING.with(|busy| {
        if busy.replace(true) {
            None
        } else {
            Some(RegistryImportGuard(Rc::clone(busy)))
        }
    })
}

fn registry_edit_allowed() -> bool {
    let allowed = REGISTRY_IMPORTING.with(|busy| !busy.get());
    if !allowed {
        tracing::warn!("registry edit refused while an import is pending");
    }
    allowed
}

pub(crate) fn registry_import_busy() -> bool {
    REGISTRY_IMPORTING.with(|busy| busy.get())
}

pub(crate) fn registry_availability_message(nav: &NavigationContext) -> Option<String> {
    use crate::controller::state::RegistryStatus;
    use crate::infra::i18n::LANGUAGE_LOADER;
    use i18n_embed_fl::fl;
    match &nav.state.borrow().show_registry.status {
        RegistryStatus::Ready => None,
        RegistryStatus::Loading => Some(fl!(LANGUAGE_LOADER, "workspace-curation-loading")),
        RegistryStatus::Unavailable => Some(fl!(LANGUAGE_LOADER, "workspace-curation-unavailable")),
        RegistryStatus::Failed(error) => Some(fl!(
            LANGUAGE_LOADER,
            "workspace-curation-failed",
            error = error.as_str()
        )),
    }
}

/// Reconcile only the selected ready registry. This also runs inside the import
/// gate before publication finishes, so no editor can target pre-rename data.
pub(crate) fn reconcile_registry(nav: &NavigationContext) {
    let snapshot = {
        let mut state = nav.state.borrow_mut();
        if state.show_registry.status != crate::controller::state::RegistryStatus::Ready {
            return;
        }
        let inventory = state.scene_inventory.clone();
        if !state.registry.reconcile_scenes(&inventory) {
            return;
        }
        state.show_registry.generation += 1;
        (state.show_registry.target.clone(), state.registry.clone())
    };
    let (target, registry) = snapshot;
    submit_registry_write_to(
        "scene identities",
        true,
        target.clone(),
        std::sync::Arc::new(move || target.write(&registry)),
    );
}

/// Import is an exclusive transaction: edits/imports submitted while it is
/// pending are refused, not silently replayed against a pre-import snapshot.
/// Parse off-lane; commit after already-accepted edits; publish only on success.
pub(crate) fn import_registry(
    nav: &NavigationContext,
    source: std::path::PathBuf,
    complete: impl FnOnce(std::io::Result<()>) + 'static,
) -> bool {
    if !nav.registry_is_current() {
        return false;
    }
    let nav = nav.registry_scoped();
    let target = nav.state.borrow().show_registry.target.clone();
    import_registry_to_target(
        source,
        target.clone(),
        move |registry| {
            nav.state
                .borrow_mut()
                .publish_registry_import(target, registry);
            reconcile_registry(&nav);
        },
        complete,
    )
}

fn import_registry_to_target(
    source: std::path::PathBuf,
    target: RegistryTarget,
    publish: impl FnOnce(SceneRegistry) + 'static,
    complete: impl FnOnce(std::io::Result<()>) + 'static,
) -> bool {
    let Some(guard) = begin_registry_import() else {
        return false;
    };
    crate::ui::background_io::run(
        move || crate::storage::registry::read_registry_yaml_from_path(&source),
        move |parsed| match parsed {
            Ok(registry) => {
                let persisted = registry.clone();
                let destination = target.clone();
                crate::ui::background_io::run_serialized(
                    SerialLane::Registry,
                    move || destination.write(&persisted),
                    move |result| {
                        if result.is_ok() {
                            let revision = REGISTRY_WRITES.with(|writes| writes.borrow().revision);
                            publish(registry);
                            REGISTRY_WRITES.with(|writes| {
                                let mut writes = writes.borrow_mut();
                                confirm_registry_target_through(&mut writes, revision, &target);
                            });
                        }
                        drop(guard);
                        complete(result);
                    },
                );
            }
            Err(error) => {
                drop(guard);
                complete(Err(error));
            }
        },
    );
    true
}

/// Re-admit every write whose failure blocks Close, from the close dialog's
/// Retry. Config and bookmark snapshots are rebuilt from `owner`, the state
/// that holds the current values; registry, history and export retries keep
/// their own retained intent. Worker panics name no store, so no retry can
/// prove their data reached disk: they still need an informed abandon.
pub(crate) fn retry_all_durable_writes(owner: &Weak<RefCell<AppState>>) {
    retry_registry_writes();
    super::report_history::retry_active();
    super::session_reports::retry_failed_exports();
    if let Some(state) = owner.upgrade() {
        let config = state.borrow().config.clone();
        crate::ui::background_io::retry_config_snapshot(move || write_config(&config));
        super::bookmarks::retry_failed_save(&state);
    }
}

/// Edit the cached scene registry and save it if anything changed.
///
/// `update` reports whether it changed anything; unchanged dirty intent retries
/// its retained write, while unchanged confirmed intent does not write. That matters because several callers fire on signals that repeat —
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
    if !registry_edit_allowed() || !nav.registry_is_current() {
        return false;
    }
    if !update_cached_registry(nav, update) {
        retry_registry_writes();
        return false;
    }
    submit_registry_snapshot(nav, what);
    true
}

/// How long the scene search must stay unchanged before its view is saved.
const SCENE_VIEW_SETTLE: std::time::Duration = std::time::Duration::from_secs(1);

/// Edit the scene view in the cached registry now; save it once edits settle.
///
/// Search fires on every typing pause, and each save rewrites the whole
/// workspace store, so the cached view changes at once while one pending save
/// waits until `SCENE_VIEW_SETTLE` passes without another edit. Close submits
/// it early through `flush_scene_view`; any whole-registry save carries it.
/// An unchanged view does nothing: it echoes a widget sync, not new intent.
pub(crate) fn persist_scene_view(
    nav: &NavigationContext,
    update: impl FnOnce(&mut SceneRegistry) -> bool,
) {
    if !registry_edit_allowed() || !nav.registry_is_current() {
        return;
    }
    if !update_cached_registry(nav, update) {
        return;
    }
    let edit = REGISTRY_WRITES.with(|writes| {
        let save = &mut writes.borrow_mut().scene_view;
        save.edit = save.edit.wrapping_add(1);
        save.pending = Some(nav.registry_scoped());
        save.edit
    });
    glib::timeout_add_local_once(SCENE_VIEW_SETTLE, move || save_settled_scene_view(edit));
}

/// Save the pending scene view only if `edit` is still the latest edit.
fn save_settled_scene_view(edit: u64) {
    let settled = REGISTRY_WRITES.with(|writes| {
        let save = &mut writes.borrow_mut().scene_view;
        (save.edit == edit).then(|| save.pending.take()).flatten()
    });
    if let Some(nav) = settled {
        save_scene_view(&nav);
    }
}

/// Submit a scene view that has not settled yet, before the window closes.
pub(crate) fn flush_scene_view() {
    if let Some(nav) = take_pending_scene_view() {
        save_scene_view(&nav);
    }
}

fn take_pending_scene_view() -> Option<NavigationContext> {
    REGISTRY_WRITES.with(|writes| writes.borrow_mut().scene_view.pending.take())
}

fn save_scene_view(nav: &NavigationContext) {
    // A workspace switch, disconnect or import since the edit leaves it unsaved
    // here: the workspace cache keeps the view for this session, and an import
    // replaces it.
    if nav.registry_is_current() {
        submit_registry_snapshot(nav, "scene view");
    }
}

/// Apply `update` to the cached registry and report whether it changed.
fn update_cached_registry(
    nav: &NavigationContext,
    update: impl FnOnce(&mut SceneRegistry) -> bool,
) -> bool {
    let mut state = nav.state.borrow_mut();
    if !update(&mut state.registry) {
        return false;
    }
    state.show_registry.revision = state.show_registry.revision.wrapping_add(1);
    true
}

/// Save the cached registry of `nav`'s workspace as one whole snapshot. It
/// carries any pending scene view, which therefore no longer needs its own save.
fn submit_registry_snapshot(nav: &NavigationContext, what: &'static str) {
    take_pending_scene_view();
    let (target, registry) =
        nav.read(|state| (state.show_registry.target.clone(), state.registry.clone()));
    submit_registry_write_to(
        what,
        true,
        target.clone(),
        std::sync::Arc::new(move || target.write(&registry)),
    );
}

/// Edit one field of the cached scene registry, then apply the same change to
/// the file on disk without rewriting the parts of it we did not touch.
///
/// This read-modify-write sibling preserves disk fields outside the change.
/// Both it and whole-snapshot writes refuse unreadable existing registries;
/// neither may replace a corrupt original with the startup fallback.
///
/// `cached` updates the in-memory snapshot so the UI reads back what the user
/// just picked; returning `false` retries failed intent, or skips confirmed work.
/// `disk` must be an idempotent local metadata mutation: retries replay accepted
/// operations in order, retaining successful successors behind failed writes.
/// `what` names the change for the log, because "failed to write registry"
/// alone does not say whether the user lost an icon, a role, or an accent.
pub(crate) fn persist_registry_field<Cached, Disk>(
    nav: &NavigationContext,
    what: &'static str,
    cached: Cached,
    disk: Disk,
) where
    Cached: FnOnce(&mut SceneRegistry) -> bool,
    Disk: Fn(&mut SceneRegistry) -> bool + Send + Sync + 'static,
{
    if !registry_edit_allowed() || !nav.registry_is_current() {
        return;
    }
    let target = nav.state.borrow().show_registry.target.clone();
    let changed = {
        let mut state = nav.state.borrow_mut();
        let changed = cached(&mut state.registry);
        if changed {
            state.show_registry.revision = state.show_registry.revision.wrapping_add(1);
        }
        changed
    };
    if !changed {
        retry_registry_writes();
        return;
    }
    submit_registry_write_to(
        what,
        false,
        target.clone(),
        std::sync::Arc::new(move || {
            let mut registry = target.read()?;
            if disk(&mut registry) {
                target.write(&registry)?;
            }
            Ok(())
        }),
    );
}

#[cfg(test)]
mod tests;
