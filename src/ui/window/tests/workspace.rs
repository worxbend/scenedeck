//! Show workspace switches: edits and imports stay with their workspace.

use crate::ui::window::*;

#[test]
#[ignore = "requires a disposable GTK display; scratch workspace files and synthetic OBS events"]
fn native_workspace_switch_isolates_edits_and_imports() {
    crate::ui::native_test!(|| {
        use crate::controller::state::RegistryStatus;
        use crate::storage::registry::{write_registry_yaml_to_path, SceneRegistry};
        use crate::storage::show_workspaces::RegistryTarget;
        use crate::ui::persist::{import_registry, persist_registry};
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        let mut legacy = SceneRegistry::default();
        legacy.set_scene_icon("Camera", Some("legacy"));
        RegistryTarget::legacy().write(&legacy).unwrap();
        {
            let mut state = nav.state.borrow_mut();
            state.obs_status = ObsStatus::Connected {
                obs_version: "fixture".into(),
            };
            state.connected_endpoint = Some("127.0.0.1:4455".into());
            state.connected_connection_id = Some("default".into());
        }
        let flush = || {
            let done = Rc::new(RefCell::new(None));
            crate::ui::background_io::flush_pending(Duration::from_secs(10), {
                let done = done.clone();
                move |result| *done.borrow_mut() = Some(result)
            });
            assert!(crate::ui::test_support::pump_until(
                Duration::from_secs(11),
                || done.borrow().is_some()
            ));
            assert_eq!(*done.borrow(), Some(Ok(())));
        };
        let select = |name: &str| {
            apply_event(
                &nav,
                AppEvent::SceneCollectionsUpdated(crate::domain::obs::ObsNamedList {
                    current: Some(name.into()),
                    items: vec![name.into()],
                }),
                None,
            );
        };
        select("Podcast");
        assert_eq!(
            nav.state.borrow().show_registry.status,
            RegistryStatus::Loading
        );
        assert!(!persist_registry(&nav, "loading edit", |registry| registry
            .set_scene_icon("Camera", Some("wrong"))));
        flush();
        assert_eq!(
            nav.state.borrow().registry.scene_icon("Camera"),
            Some("legacy")
        );
        let podcast = nav.state.borrow().show_registry.target.clone();
        let old_editor = nav.registry_scoped();
        assert!(persist_registry(&old_editor, "podcast", |registry| {
            registry.set_scene_icon("Camera", Some("podcast"))
        }));
        let source = crate::storage::xdg::registry_path().with_extension("yaml");
        let mut imported = SceneRegistry::default();
        imported.set_scene_icon("Camera", Some("imported podcast"));
        write_registry_yaml_to_path(&source, &imported).unwrap();
        assert!(import_registry(
            &old_editor,
            source.clone(),
            |result| assert!(result.is_ok())
        ));
        select("Gaming");
        apply_event(
            &nav,
            AppEvent::SceneRenamed {
                uuid: None,
                old: "Camera".into(),
                new: "Game".into(),
            },
            None,
        );
        flush();
        assert!(nav.state.borrow().registry.scenes.is_empty());
        assert_eq!(
            podcast.read().unwrap().scene_icon("Camera"),
            Some("imported podcast")
        );
        assert!(!persist_registry(&old_editor, "stale editor", |registry| {
            registry.set_scene_icon("Camera", Some("wrong"))
        }));
        assert!(!import_registry(&old_editor, source, |_| panic!(
            "stale dialog must not import"
        )));
        assert!(persist_registry(&nav, "gaming", |registry| registry
            .set_scene_icon("Camera", Some("gaming"))));
        let gaming = nav.state.borrow().show_registry.target.clone();
        select("Podcast");
        assert_eq!(
            nav.state.borrow().registry.scene_icon("Camera"),
            Some("imported podcast")
        );
        flush();
        assert_eq!(gaming.read().unwrap().scene_icon("Camera"), Some("gaming"));
        assert_eq!(
            RegistryTarget::legacy()
                .read()
                .unwrap()
                .scene_icon("Camera"),
            Some("legacy")
        );

        // Returning to a cached workspace before its import completes
        // must not leave the old cache displayed or writable afterward.
        let source = crate::storage::xdg::registry_path().with_extension("yaml");
        imported.set_scene_icon("Camera", Some("returned podcast"));
        write_registry_yaml_to_path(&source, &imported).unwrap();
        assert!(import_registry(&nav, source, |result| assert!(
            result.is_ok()
        )));
        select("Gaming");
        select("Podcast");
        flush();
        assert_eq!(
            nav.state.borrow().registry.scene_icon("Camera"),
            Some("returned podcast")
        );
        assert_eq!(
            podcast.read().unwrap().scene_icon("Camera"),
            Some("returned podcast")
        );

        for (connection, endpoint) in [("other", "127.0.0.1:4455"), ("default", "other:4455")] {
            {
                let mut state = nav.state.borrow_mut();
                state.invalidate_collection_runtime();
                state.connected_connection_id = Some(connection.into());
                state.connected_endpoint = Some(endpoint.into());
            }
            select("Podcast");
            flush();
            assert!(nav.state.borrow().registry.scenes.is_empty());
        }
        let path = crate::storage::xdg::show_workspaces_path();
        let original = std::fs::read(&path).unwrap();
        std::fs::write(&path, b"invalid workspace store").unwrap();
        select("Broken");
        flush();
        assert!(matches!(
            nav.state.borrow().show_registry.status,
            RegistryStatus::Failed(_)
        ));
        assert!(!persist_registry(&nav, "failed load edit", |registry| {
            registry.set_scene_icon("Camera", Some("wrong"))
        }));
        assert_eq!(std::fs::read(&path).unwrap(), b"invalid workspace store");
        std::fs::write(&path, original).unwrap();
        select("Broken");
        flush();
        assert_eq!(
            nav.state.borrow().show_registry.status,
            RegistryStatus::Ready
        );
    });
}
