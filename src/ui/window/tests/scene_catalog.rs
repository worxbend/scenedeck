//! Scene identity and curation: registry load, import, rename and retired
//! editors across scene inventory events.

use crate::ui::window::*;

#[test]
#[ignore = "requires a disposable GTK display; scratch files and synthetic identities"]
fn native_scene_identity_reconciles_load_import_and_retired_editors() {
    crate::ui::native_test!(|| {
        use crate::domain::scene::{Scene, SceneInventory};
        use crate::storage::registry::{write_registry_yaml_to_path, SceneRegistry};
        use crate::storage::show_workspaces::RegistryTarget;
        use crate::ui::persist::{import_registry, persist_registry};
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        let inventory = |name: &str, uuid: &str| SceneInventory {
            scenes: vec![Scene {
                id: name.into(),
                uuid: Some(uuid.into()),
            }],
            ..Default::default()
        };
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
        let mut stored = SceneRegistry::default();
        stored.set_scene_icon("Camera", Some("original"));
        stored.reconcile_scenes(&inventory("Camera", "original uuid"));
        RegistryTarget::legacy().write(&stored).unwrap();
        {
            let mut state = nav.state.borrow_mut();
            state.obs_status = ObsStatus::Connected {
                obs_version: "fixture".into(),
            };
            state.connected_endpoint = Some("127.0.0.1:4455".into());
            state.connected_connection_id = Some("default".into());
        }
        apply_event(
            &nav,
            AppEvent::SceneCollectionsUpdated(crate::domain::obs::ObsNamedList {
                current: Some("Show".into()),
                items: vec!["Show".into()],
            }),
            None,
        );
        // Inventory arrived while the workspace file was still loading.
        apply_event(
            &nav,
            AppEvent::SceneInventoryUpdated(inventory("Guest", "original uuid")),
            None,
        );
        flush();
        let target = nav.state.borrow().show_registry.target.clone();
        assert_eq!(target.read().unwrap().scene_icon("Guest"), Some("original"));
        let old_editor = nav.registry_scoped();
        apply_event(
            &nav,
            AppEvent::SceneInventoryUpdated(inventory("Guest", "replacement")),
            None,
        );
        assert!(!persist_registry(&old_editor, "retired editor", |r| r
            .set_scene_icon("Guest", Some("wrong"))));
        flush();
        assert_eq!(target.read().unwrap().scene_icon("Guest"), None);
        assert_eq!(nav.state.borrow().registry.retained_scenes.len(), 1);

        let source = crate::storage::xdg::registry_path().with_extension("yaml");
        let mut imported = SceneRegistry::default();
        imported.set_scene_icon("Host", Some("portable"));
        imported.reconcile_scenes(&inventory("Host", "foreign uuid"));
        write_registry_yaml_to_path(&source, &imported).unwrap();
        assert!(import_registry(&nav, source.clone(), |result| assert!(
            result.is_ok()
        )));
        apply_event(
            &nav,
            AppEvent::SceneInventoryUpdated(inventory("Host", "replacement")),
            None,
        );
        flush();
        let disk = target.read().unwrap();
        assert_eq!(disk.scene_icon("Host"), Some("portable"));
        assert_eq!(disk.scene_uuids["Host"], "replacement");
        assert_eq!(disk, nav.state.borrow().registry);

        // A failed import still releases deferred identity reconciliation.
        std::fs::write(&source, "scenes: [invalid").unwrap();
        assert!(import_registry(&nav, source, |result| assert!(
            result.is_err()
        )));
        apply_event(
            &nav,
            AppEvent::SceneInventoryUpdated(inventory("Presenter", "replacement")),
            None,
        );
        flush();
        assert_eq!(
            target.read().unwrap().scene_icon("Presenter"),
            Some("portable")
        );
        // A retired OBS session cannot rewrite the selected workspace.
        apply_event(
            &nav,
            AppEvent::SessionEvent {
                generation: u64::MAX,
                collection: Some(0),
                event: Box::new(AppEvent::SceneInventoryUpdated(inventory(
                    "Wrong",
                    "replacement",
                ))),
            },
            None,
        );
        flush();
        assert_eq!(
            target.read().unwrap().scene_icon("Presenter"),
            Some("portable")
        );
    });
}

#[test]
#[ignore = "requires a disposable GTK display; scratch registry and synthetic OBS events"]
fn native_scene_rename_preserves_imported_curation_and_rejects_retired_scope() {
    crate::ui::native_test!(|| {
        use crate::storage::registry::{read_registry, write_registry_yaml_to_path, SceneRegistry};
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        {
            let mut state = nav.state.borrow_mut();
            state.obs_status = ObsStatus::Connected {
                obs_version: "fixture".into(),
            };
            state.scene_collections.current = Some("Show".into());
            state.registry.set_scene_icon("Camera", Some("camera"));
            state.registry.scene_order = vec!["Camera".into()];
        }
        let flush = || {
            let done = Rc::new(RefCell::new(None));
            crate::ui::background_io::flush_pending(Duration::from_secs(5), {
                let done = done.clone();
                move |result| *done.borrow_mut() = Some(result)
            });
            assert!(crate::ui::test_support::pump_until(
                Duration::from_secs(6),
                || done.borrow().is_some()
            ));
            assert_eq!(*done.borrow(), Some(Ok(())));
        };
        apply_event(
            &nav,
            AppEvent::SceneRenamed {
                uuid: None,
                old: "Camera".into(),
                new: "Guest".into(),
            },
            None,
        );
        flush();
        assert_eq!(read_registry().unwrap().scene_icon("Guest"), Some("camera"));
        assert_eq!(read_registry().unwrap().scene_order, ["Guest"]);

        let source = crate::storage::xdg::registry_path().with_extension("yaml");
        let mut imported = SceneRegistry::default();
        imported.set_scene_icon("Guest", Some("imported"));
        write_registry_yaml_to_path(&source, &imported).unwrap();
        assert!(crate::ui::persist::import_registry(
            &nav,
            source.clone(),
            |result| assert!(result.is_ok())
        ));
        apply_event(
            &nav,
            AppEvent::SceneRenamed {
                uuid: None,
                old: "Guest".into(),
                new: "Host".into(),
            },
            None,
        );
        assert_eq!(
            nav.state.borrow().registry.scene_icon("Guest"),
            Some("camera")
        );
        flush();
        assert_eq!(
            nav.state.borrow().registry.scene_icon("Host"),
            Some("imported")
        );
        assert_eq!(
            read_registry().unwrap().scene_icon("Host"),
            Some("imported")
        );
        assert!(read_registry().unwrap().scene_icon("Guest").is_none());

        apply_event(
            &nav,
            AppEvent::SessionEvent {
                generation: u64::MAX,
                collection: Some(0),
                event: Box::new(AppEvent::SceneRenamed {
                    uuid: None,
                    old: "Host".into(),
                    new: "Wrong".into(),
                }),
            },
            None,
        );
        assert!(nav.state.borrow().registry.scene_icon("Wrong").is_none());

        write_registry_yaml_to_path(&source, &read_registry().unwrap()).unwrap();
        assert!(crate::ui::persist::import_registry(
            &nav,
            source,
            |result| assert!(result.is_ok())
        ));
        apply_event(
            &nav,
            AppEvent::SceneRenamed {
                uuid: None,
                old: "Host".into(),
                new: "Retired".into(),
            },
            None,
        );
        nav.state.borrow_mut().invalidate_collection_runtime();
        flush();
        assert_eq!(
            read_registry().unwrap().scene_icon("Host"),
            Some("imported")
        );
        assert!(read_registry().unwrap().scene_icon("Retired").is_none());
    });
}
