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
use std::rc::Rc;

use crate::storage::config::{write_config, AppConfig};
use crate::storage::registry::SceneRegistry;
use crate::storage::show_workspaces::RegistryTarget;
use crate::ui::background_io::SerialLane;
use crate::ui::navigation::NavigationContext;

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
                writes.error = Some(error.to_string());
            }
        }
    });
}

#[cfg(test)]
fn submit_registry_write(what: &'static str, covers_snapshot: bool, write: RegistryWrite) {
    submit_registry_write_to(what, covers_snapshot, RegistryTarget::legacy(), write);
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

/// Resolve curation only after OBS reports its actual collection. The registry
/// lane and import gate keep migration, imports, and accepted edits in order.
pub(crate) fn select_registry_scope(nav: &NavigationContext, refresh: impl Fn() + 'static) {
    let Some(target) = nav.state.borrow().desired_registry_target() else {
        return;
    };
    let ticket = nav.state.borrow_mut().select_registry(target.clone());
    if ticket.is_none() && registry_edit_allowed() {
        reconcile_registry(nav);
    }
    refresh();
    let Some(generation) = ticket else {
        return;
    };
    let nav = nav.scoped();
    let failed_nav = nav.clone();
    let failed_target = target.clone();
    let refresh: Rc<dyn Fn()> = Rc::new(refresh);
    let failed_refresh = refresh.clone();
    if !after_registry_import(move || {
        if !nav.is_current()
            || !nav
                .state
                .borrow()
                .accepts_registry_load(generation, &target)
        {
            return;
        }
        let guard = begin_registry_import().expect("deferred registry jobs run outside the gate");
        let legacy = {
            let state = nav.state.borrow();
            let original_endpoint = state
                .config
                .legacy_registry_endpoint
                .clone()
                .unwrap_or_else(|| format!("{}:{}", state.config.obs.host, state.config.obs.port));
            matches!(&target, RegistryTarget::Workspace { key, .. }
                if key.connection == "default" && key.endpoint == original_endpoint)
            .then(crate::storage::xdg::registry_path)
        };
        let destination = target.clone();
        crate::ui::background_io::run_serialized(
            SerialLane::Registry,
            move || destination.load_or_create(legacy.as_deref()),
            move |result| {
                if nav.is_current() {
                    nav.state
                        .borrow_mut()
                        .finish_registry_load(generation, &target, result);
                }
                if nav.is_current() {
                    reconcile_registry(&nav);
                }
                drop(guard);
                refresh();
            },
        );
    }) {
        failed_nav.state.borrow_mut().finish_registry_load(
            generation,
            &failed_target,
            Err(std::io::Error::other("too many pending workspace changes")),
        );
        failed_refresh();
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

#[cfg(test)]
fn import_registry_to_path(
    source: std::path::PathBuf,
    target: std::path::PathBuf,
    publish: impl FnOnce(SceneRegistry) + 'static,
    complete: impl FnOnce(std::io::Result<()>) + 'static,
) -> bool {
    import_registry_to_target(source, RegistryTarget::File(target), publish, complete)
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
    let (target, registry) = {
        let mut state = nav.state.borrow_mut();
        if !update(&mut state.registry) {
            drop(state);
            retry_registry_writes();
            return false;
        }
        state.show_registry.revision = state.show_registry.revision.wrapping_add(1);
        (state.show_registry.target.clone(), state.registry.clone())
    };

    submit_registry_write_to(
        what,
        true,
        target.clone(),
        std::sync::Arc::new(move || target.write(&registry)),
    );
    true
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
mod tests {
    use super::*;
    use std::{cell::RefCell, time::Duration};

    #[test]
    fn successful_workspace_snapshot_cannot_retire_another_workspaces_failed_intent() {
        if !crate::ui::background_io::isolated_test_child(
            "ui::persist::tests::successful_workspace_snapshot_cannot_retire_another_workspaces_failed_intent",
        ) { return; }
        let dir =
            std::env::temp_dir().join(format!("scenedeck-workspace-retry-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let a = RegistryTarget::File(dir.join("a.json"));
        let b = RegistryTarget::File(dir.join("b.json"));
        let fail = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let mut registry = SceneRegistry::default();
        registry.set_scene_icon("Camera", Some("A"));
        submit_registry_write_to(
            "A",
            true,
            a.clone(),
            std::sync::Arc::new({
                let a = a.clone();
                let fail = fail.clone();
                let registry = registry.clone();
                move || {
                    if fail.load(std::sync::atomic::Ordering::SeqCst) {
                        Err(std::io::Error::other("A failed"))
                    } else {
                        a.write(&registry)
                    }
                }
            }),
        );
        registry.set_scene_icon("Camera", Some("B"));
        submit_registry_write_to(
            "B",
            true,
            b.clone(),
            std::sync::Arc::new({
                let b = b.clone();
                move || b.write(&registry)
            }),
        );
        drain_expect(Err(crate::ui::background_io::FlushError::WorkerFailed));
        assert_eq!(b.read().unwrap().scene_icon("Camera"), Some("B"));
        fail.store(false, std::sync::atomic::Ordering::SeqCst);
        assert!(retry_registry_writes());
        drain();
        assert_eq!(a.read().unwrap().scene_icon("Camera"), Some("A"));
        assert_eq!(b.read().unwrap().scene_icon("Camera"), Some("B"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn observed_registry_changes_wait_for_import_publication_and_are_bounded() {
        let guard = begin_registry_import().unwrap();
        let observed = Rc::new(RefCell::new(Vec::new()));
        for index in 0..64 {
            let observed = observed.clone();
            assert!(after_registry_import(move || observed
                .borrow_mut()
                .push(index)));
        }
        assert!(!after_registry_import(|| panic!("overflow must not run")));
        assert!(observed.borrow().is_empty());
        drop(guard);
        assert_eq!(*observed.borrow(), (0..64).collect::<Vec<_>>());
        let ran = Rc::new(Cell::new(false));
        assert!(after_registry_import({
            let ran = ran.clone();
            move || ran.set(true)
        }));
        assert!(ran.get());
    }

    fn drain() {
        drain_expect(Ok(()));
    }

    fn await_registry_worker() {
        // This fixture measures revision/publication ordering, not fsync speed.
        // Wait for the real writer with a FIFO marker before the unchanged
        // one-second GTK publication assertion. No main-context iteration or
        // completion acknowledgement occurs during this test-only wait.
        let (finished, observed) = std::sync::mpsc::channel();
        crate::ui::background_io::run_serialized(
            SerialLane::Registry,
            move || finished.send(()).unwrap(),
            |()| {},
        );
        observed
            .recv_timeout(Duration::from_secs(10))
            .expect("registry writer completion");
    }

    fn drain_expect(expected: Result<(), crate::ui::background_io::FlushError>) {
        let main_loop = glib::MainLoop::new(None, false);
        crate::ui::background_io::flush_pending(Duration::from_secs(1), {
            let main_loop = main_loop.clone();
            move |result| {
                assert_eq!(result, expected);
                main_loop.quit();
            }
        });
        main_loop.run();
    }

    #[test]
    fn failed_registry_snapshot_is_retryable_without_rolling_back_newer_edits() {
        if !crate::ui::background_io::isolated_test_child(
            "ui::persist::tests::failed_registry_snapshot_is_retryable_without_rolling_back_newer_edits",
        ) {
            return;
        }
        let dir =
            std::env::temp_dir().join(format!("scenedeck-registry-retry-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let parent = dir.join("blocked");
        std::fs::write(&parent, b"not a directory").unwrap();
        let target = parent.join("registry.json");
        let mut cached = SceneRegistry::default();
        cached.set_scene_icon("Camera", Some("older"));
        let first = cached.clone();
        submit_registry_write(
            "icon",
            true,
            std::sync::Arc::new({
                let target = target.clone();
                move || crate::storage::registry::write_registry_to_path(&target, &first)
            }),
        );
        let main_loop = glib::MainLoop::new(None, false);
        crate::ui::background_io::flush_pending(Duration::from_secs(2), {
            let main_loop = main_loop.clone();
            move |result| {
                assert_eq!(
                    result,
                    Err(crate::ui::background_io::FlushError::WorkerFailed)
                );
                main_loop.quit();
            }
        });
        main_loop.run();
        assert!(matches!(
            registry_save_status(),
            RegistrySaveStatus::Failed(_)
        ));
        assert!(
            !cached.set_scene_icon("Camera", Some("older")),
            "same-value retry must not need another edit"
        );
        std::fs::remove_file(&parent).unwrap();
        std::fs::create_dir(&parent).unwrap();
        let (release, held) = std::sync::mpsc::channel();
        crate::ui::background_io::run_serialized(
            SerialLane::Registry,
            move || held.recv().unwrap(),
            |()| {},
        );
        assert!(retry_registry_writes());
        cached.set_scene_icon("Camera", Some("newer"));
        let newer = cached.clone();
        submit_registry_write(
            "newer icon",
            true,
            std::sync::Arc::new({
                let target = target.clone();
                move || crate::storage::registry::write_registry_to_path(&target, &newer)
            }),
        );
        release.send(()).unwrap();
        await_registry_worker();
        drain();
        assert!(matches!(registry_save_status(), RegistrySaveStatus::Saved));
        let disk = crate::storage::registry::read_registry_from_path(&target).unwrap();
        assert_eq!(disk.scene_icon("Camera"), Some("newer"));
        assert_eq!(cached.scene_icon("Camera"), Some("newer"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn registry_field_retry_replays_successful_successors_and_preserves_external_fields() {
        if !crate::ui::background_io::isolated_test_child(
            "ui::persist::tests::registry_field_retry_replays_successful_successors_and_preserves_external_fields",
        ) { return; }
        let dir =
            std::env::temp_dir().join(format!("scenedeck-field-retry-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("XDG_CONFIG_HOME", &dir);
        let target = crate::storage::xdg::registry_path();
        let parent = target.parent().unwrap();
        std::fs::write(parent, b"blocked").unwrap();
        submit_registry_write(
            "older icon",
            false,
            std::sync::Arc::new(|| {
                crate::storage::registry::set_scene_icon("Camera", Some("older"))
                    .map(|_| ())
                    .map_err(std::io::Error::other)
            }),
        );
        drain_expect(Err(crate::ui::background_io::FlushError::WorkerFailed));
        std::fs::remove_file(parent).unwrap();
        let mut external = SceneRegistry::default();
        external.set_input_icon("External", Some("preserved"));
        crate::storage::registry::write_registry_to_path(&target, &external).unwrap();
        submit_registry_write(
            "newer icon",
            false,
            std::sync::Arc::new(|| {
                crate::storage::registry::set_scene_icon("Camera", Some("newer"))
                    .map(|_| ())
                    .map_err(std::io::Error::other)
            }),
        );
        drain_expect(Err(crate::ui::background_io::FlushError::WorkerFailed));
        assert_eq!(
            REGISTRY_WRITES.with(|writes| writes.borrow().operations.len()),
            2
        );
        assert!(retry_registry_writes());
        await_registry_worker();
        drain();
        let disk = crate::storage::registry::read_registry_from_path(&target).unwrap();
        assert_eq!(disk.scene_icon("Camera"), Some("newer"));
        assert_eq!(disk.input_icon("External"), Some("preserved"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn registry_retry_does_not_resolve_unrelated_or_newer_failures() {
        if !crate::ui::background_io::isolated_test_child(
            "ui::persist::tests::registry_retry_does_not_resolve_unrelated_or_newer_failures",
        ) {
            return;
        }
        let first_fails = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        submit_registry_write(
            "first",
            true,
            std::sync::Arc::new({
                let first_fails = first_fails.clone();
                move || {
                    if first_fails.load(std::sync::atomic::Ordering::SeqCst) {
                        Err(std::io::Error::other("first failed"))
                    } else {
                        Ok(())
                    }
                }
            }),
        );
        drain_expect(Err(crate::ui::background_io::FlushError::WorkerFailed));
        let unrelated = crate::ui::background_io::record_failure();
        first_fails.store(false, std::sync::atomic::Ordering::SeqCst);
        let (release, held) = std::sync::mpsc::channel();
        crate::ui::background_io::run_serialized(
            SerialLane::Registry,
            move || held.recv().unwrap(),
            |()| {},
        );
        assert!(retry_registry_writes());
        let second_fails = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        submit_registry_write(
            "second",
            true,
            std::sync::Arc::new({
                let second_fails = second_fails.clone();
                move || {
                    if second_fails.load(std::sync::atomic::Ordering::SeqCst) {
                        Err(std::io::Error::other("second failed"))
                    } else {
                        Ok(())
                    }
                }
            }),
        );
        release.send(()).unwrap();
        drain_expect(Err(crate::ui::background_io::FlushError::WorkerFailed));
        assert_eq!(
            registry_save_status(),
            RegistrySaveStatus::Failed("second failed".into())
        );
        second_fails.store(false, std::sync::atomic::Ordering::SeqCst);
        assert!(retry_registry_writes());
        drain_expect(Err(crate::ui::background_io::FlushError::WorkerFailed));
        assert_eq!(registry_save_status(), RegistrySaveStatus::Saved);
        crate::ui::background_io::resolve_failure(unrelated);
        drain();
    }

    #[test]
    fn import_supersedes_failed_intent_without_replaying_it_over_imported_data() {
        if !crate::ui::background_io::isolated_test_child(
            "ui::persist::tests::import_supersedes_failed_intent_without_replaying_it_over_imported_data",
        ) { return; }
        let dir =
            std::env::temp_dir().join(format!("scenedeck-import-retry-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("registry.json");
        let source = dir.join("import.yaml");
        let first_fails = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let mut older = SceneRegistry::default();
        older.set_scene_icon("Camera", Some("older"));
        submit_registry_write_to(
            "old snapshot",
            true,
            RegistryTarget::File(target.clone()),
            std::sync::Arc::new({
                let target = target.clone();
                let first_fails = first_fails.clone();
                move || {
                    if first_fails.load(std::sync::atomic::Ordering::SeqCst) {
                        Err(std::io::Error::other("save failed"))
                    } else {
                        crate::storage::registry::write_registry_to_path(&target, &older)
                    }
                }
            }),
        );
        drain_expect(Err(crate::ui::background_io::FlushError::WorkerFailed));
        let mut imported = SceneRegistry::default();
        imported.set_scene_icon("Camera", Some("imported"));
        crate::storage::registry::write_registry_yaml_to_path(&source, &imported).unwrap();
        assert!(import_registry_to_path(
            source,
            target.clone(),
            |_| {},
            |result| assert!(result.is_ok())
        ));
        drain_expect(Err(crate::ui::background_io::FlushError::WorkerFailed));
        first_fails.store(false, std::sync::atomic::Ordering::SeqCst);
        assert!(retry_registry_writes());
        drain();
        assert_eq!(
            crate::storage::registry::read_registry_from_path(&target)
                .unwrap()
                .scene_icon("Camera"),
            Some("imported")
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn import_gates_every_edit_until_commit_and_cache_publication() {
        if !crate::ui::background_io::isolated_test_child(
            "ui::persist::tests::import_gates_every_edit_until_commit_and_cache_publication",
        ) {
            return;
        }
        let dir = std::env::temp_dir().join(format!("scenedeck-import-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("import.yaml");
        let target = dir.join("registry.json");
        let mut imported = SceneRegistry::default();
        imported.set_scene_icon("Imported", Some("nf-md-camera"));
        crate::storage::registry::write_registry_yaml_to_path(&source, &imported).unwrap();
        let cached = Rc::new(RefCell::new(SceneRegistry::default()));
        crate::storage::registry::write_registry_to_path(&target, &cached.borrow()).unwrap();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        crate::ui::background_io::run_serialized(
            SerialLane::Registry,
            move || release_rx.recv().unwrap(),
            |()| {},
        );
        assert!(import_registry_to_path(
            source.clone(),
            target.clone(),
            {
                let cached = cached.clone();
                move |registry| *cached.borrow_mut() = registry
            },
            |result| assert!(result.is_ok())
        ));
        assert!(!registry_edit_allowed());
        assert!(!import_registry_to_path(
            source.clone(),
            target.clone(),
            |_| panic!("second import published"),
            |_| panic!("second import ran")
        ));
        let edit_ran = Cell::new(false);
        if registry_edit_allowed() {
            edit_ran.set(true);
            cached
                .borrow_mut()
                .set_scene_icon("DuringImport", Some("nf-md-camera"));
        }
        assert!(!edit_ran.get());
        let main_loop = glib::MainLoop::new(None, false);
        crate::ui::background_io::flush_pending(Duration::from_millis(60), {
            let main_loop = main_loop.clone();
            move |result| {
                assert_eq!(result, Err(crate::ui::background_io::FlushError::TimedOut));
                main_loop.quit();
            }
        });
        main_loop.run();
        assert!(
            cached.borrow().scenes.is_empty(),
            "must not publish an uncommitted import"
        );
        assert!(!registry_edit_allowed());
        release_tx.send(()).unwrap();
        drain();
        assert!(registry_edit_allowed());
        cached
            .borrow_mut()
            .set_scene_icon("LaterEdit", Some("nf-md-camera"));
        crate::storage::registry::write_registry_to_path(&target, &cached.borrow()).unwrap();
        let disk = crate::storage::registry::read_registry_from_path(&target).unwrap();
        assert!(disk.scenes.contains_key("Imported"));
        assert!(disk.scenes.contains_key("LaterEdit"));
        assert_eq!(
            serde_json::to_value(disk).unwrap(),
            serde_json::to_value(&*cached.borrow()).unwrap()
        );

        // Both parse failure and a corrupt destination leave confirmed cache
        // and destination untouched and release the exclusive edit gate.
        for corrupt_source in [true, false] {
            if corrupt_source {
                std::fs::write(&source, b"scenes: [invalid").unwrap();
            } else {
                crate::storage::registry::write_registry_yaml_to_path(&source, &imported).unwrap();
                std::fs::write(&target, b"broken JSON").unwrap();
            }
            let original = std::fs::read(&target).unwrap();
            assert!(import_registry_to_path(
                source.clone(),
                target.clone(),
                |_| panic!("failed import published"),
                |result| assert!(result.is_err())
            ));
            drain();
            assert!(registry_edit_allowed());
            assert_eq!(std::fs::read(&target).unwrap(), original);
            assert!(cached.borrow().scenes.contains_key("LaterEdit"));
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}
