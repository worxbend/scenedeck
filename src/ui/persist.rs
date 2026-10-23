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
use crate::storage::registry::{write_registry, SceneRegistry};
use crate::ui::background_io::SerialLane;
use crate::ui::navigation::NavigationContext;

type RegistryWrite = std::sync::Arc<dyn Fn() -> std::io::Result<()> + Send + Sync>;

struct RegistryOperation {
    write: RegistryWrite,
    succeeded: bool,
}

#[derive(Default)]
struct RegistryWrites {
    revision: u64,
    confirmed: u64,
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
    writes.confirmed = writes.confirmed.max(revision);
    let confirmed = writes.confirmed;
    writes.operations.retain(|id, _| *id > confirmed);
}

fn registry_write_finished(revision: u64, covers_snapshot: bool, result: std::io::Result<()>) {
    REGISTRY_WRITES.with(|writes| {
        let mut writes = writes.borrow_mut();
        match result {
            Ok(()) => {
                if covers_snapshot {
                    confirm_registry_through(&mut writes, revision);
                } else if let Some(operation) = writes.operations.get_mut(&revision) {
                    operation.succeeded = true;
                    while let Some((&id, operation)) = writes.operations.first_key_value() {
                        if !operation.succeeded {
                            break;
                        }
                        confirm_registry_through(&mut writes, id);
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

fn submit_registry_write(what: &'static str, covers_snapshot: bool, write: RegistryWrite) {
    let revision = REGISTRY_WRITES.with(|writes| {
        let mut writes = writes.borrow_mut();
        writes.revision += 1;
        let revision = writes.revision;
        writes.operations.insert(
            revision,
            RegistryOperation {
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
}

/// All registry mutation entry points share this gate through disk commit and
/// GTK publication. A dropped/panicked read also releases it via RAII.
struct RegistryImportGuard(Rc<Cell<bool>>);

impl Drop for RegistryImportGuard {
    fn drop(&mut self) {
        self.0.set(false);
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

/// Import is an exclusive transaction: edits/imports submitted while it is
/// pending are refused, not silently replayed against a pre-import snapshot.
/// Parse off-lane; commit after already-accepted edits; publish only on success.
pub(crate) fn import_registry(
    nav: &NavigationContext,
    source: std::path::PathBuf,
    complete: impl FnOnce(std::io::Result<()>) + 'static,
) -> bool {
    let nav = nav.clone();
    import_registry_to_path(
        source,
        crate::storage::xdg::registry_path(),
        move |registry| nav.state.borrow_mut().registry = registry,
        complete,
    )
}

fn import_registry_to_path(
    source: std::path::PathBuf,
    target: std::path::PathBuf,
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
                crate::ui::background_io::run_serialized(
                    SerialLane::Registry,
                    move || crate::storage::registry::write_registry_to_path(&target, &persisted),
                    move |result| {
                        if result.is_ok() {
                            publish(registry);
                            REGISTRY_WRITES.with(|writes| {
                                let mut writes = writes.borrow_mut();
                                let revision = writes.revision;
                                confirm_registry_through(&mut writes, revision);
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
    if !registry_edit_allowed() {
        return false;
    }
    let registry = {
        let mut state = nav.state.borrow_mut();
        if !update(&mut state.registry) {
            drop(state);
            retry_registry_writes();
            return false;
        }
        state.registry.clone()
    };

    submit_registry_write(
        what,
        true,
        std::sync::Arc::new(move || write_registry(&registry)),
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
    Disk: Fn() -> Result<bool, crate::storage::registry::RegistryMutationError>
        + Send
        + Sync
        + 'static,
{
    if !registry_edit_allowed() {
        return;
    }
    let changed = {
        let mut state = nav.state.borrow_mut();
        cached(&mut state.registry)
    };
    if !changed {
        retry_registry_writes();
        return;
    }
    submit_registry_write(
        what,
        false,
        std::sync::Arc::new(move || disk().map(|_| ()).map_err(std::io::Error::other)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, time::Duration};

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
        submit_registry_write(
            "old snapshot",
            true,
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
