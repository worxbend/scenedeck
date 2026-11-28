use super::*;
use std::{cell::RefCell, time::Duration};

fn submit_registry_write(what: &'static str, covers_snapshot: bool, write: RegistryWrite) {
    submit_registry_write_to(what, covers_snapshot, RegistryTarget::legacy(), write);
}

fn import_registry_to_path(
    source: std::path::PathBuf,
    target: std::path::PathBuf,
    publish: impl FnOnce(SceneRegistry) + 'static,
    complete: impl FnOnce(std::io::Result<()>) + 'static,
) -> bool {
    import_registry_to_target(source, RegistryTarget::File(target), publish, complete)
}

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
    let dir = std::env::temp_dir().join(format!("scenedeck-registry-retry-{}", std::process::id()));
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

/// A field save of the Camera icon to the legacy registry file, resolved
/// when it runs, the way `persist_registry_field` writes one.
fn legacy_camera_icon_write(icon: &'static str) -> RegistryWrite {
    std::sync::Arc::new(move || {
        let target = RegistryTarget::legacy();
        let mut registry = target.read()?;
        if registry.set_scene_icon("Camera", Some(icon)) {
            target.write(&registry)?;
        }
        Ok(())
    })
}

#[test]
fn registry_field_retry_replays_successful_successors_and_preserves_external_fields() {
    if !crate::ui::background_io::isolated_test_child(
        "ui::persist::tests::registry_field_retry_replays_successful_successors_and_preserves_external_fields",
    ) { return; }
    let dir = std::env::temp_dir().join(format!("scenedeck-field-retry-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::env::set_var("XDG_CONFIG_HOME", &dir);
    let target = crate::storage::xdg::registry_path();
    let parent = target.parent().unwrap();
    std::fs::write(parent, b"blocked").unwrap();
    submit_registry_write("older icon", false, legacy_camera_icon_write("older"));
    drain_expect(Err(crate::ui::background_io::FlushError::WorkerFailed));
    std::fs::remove_file(parent).unwrap();
    let mut external = SceneRegistry::default();
    external.set_input_icon("External", Some("preserved"));
    crate::storage::registry::write_registry_to_path(&target, &external).unwrap();
    submit_registry_write("newer icon", false, legacy_camera_icon_write("newer"));
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
    let dir = std::env::temp_dir().join(format!("scenedeck-import-retry-{}", std::process::id()));
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

    // Parse failure, a source over an import cap and a corrupt destination
    // leave confirmed cache and destination untouched and release the
    // exclusive edit gate.
    enum Broken {
        Source(String),
        Destination,
    }
    let decks: String = (0..1_000).map(|n| format!("  deck-{n}: {{}}\n")).collect();
    let over_deck_cap = format!("decks:\n{decks}");
    for broken in [
        Broken::Source("scenes: [invalid".into()),
        Broken::Source(over_deck_cap),
        Broken::Destination,
    ] {
        match broken {
            Broken::Source(raw) => std::fs::write(&source, raw).unwrap(),
            Broken::Destination => {
                crate::storage::registry::write_registry_yaml_to_path(&source, &imported).unwrap();
                std::fs::write(&target, b"broken JSON").unwrap();
            }
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
