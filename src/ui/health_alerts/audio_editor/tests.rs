use super::*;
use crate::storage::show_workspaces::RegistryTarget;
use crate::ui::{
    preflight::tests::{connected, find, window},
    test_support::{native_test, navigation, pump, pump_until},
};

fn setup(nav: &NavigationContext) -> RegistryTarget {
    connected(nav);
    let target = nav.state.borrow().desired_registry_target().unwrap();
    target.load_or_create(None).unwrap();
    nav.state.borrow_mut().show_registry.target = target.clone();
    target
}
fn toggle(root: &impl IsA<gtk4::Widget>, name: &str, active: bool) {
    find(root, name)
        .downcast::<CheckButton>()
        .unwrap()
        .set_active(active);
}
fn save(root: &impl IsA<gtk4::Widget>) {
    find(root, "health-audio-save")
        .downcast::<Button>()
        .unwrap()
        .emit_clicked();
}
fn saved() -> bool {
    crate::ui::persist::registry_save_status() == crate::ui::persist::RegistrySaveStatus::Saved
}

#[test]
#[ignore = "requires isolated GTK and scratch workspace storage"]
fn native_audio_rules_save_selected_inputs_and_preserve_missing_identity() {
    native_test("ui::health_alerts::audio_editor::tests::native_audio_rules_save_selected_inputs_and_preserve_missing_identity", || {
        let (nav, _stack, _runtime, _events) = navigation(); let target = setup(&nav);
        gtk4::Settings::default().unwrap().set_gtk_font_name(Some("Sans 22"));
        let parent = adw::Window::new(); parent.present(); show(parent.upcast_ref(), &nav);
        let editor = window("health-audio-editor"); editor.set_default_size(360, 600); pump(Duration::from_millis(300));
        assert!(editor.measure(Orientation::Horizontal, -1).0 <= 360);
        assert!(editor.measure(Orientation::Vertical, 360).0 <= 600);
        assert!(nav.state.borrow().registry.audio_alerts.is_empty());
        toggle(&editor, "health-audio-select-0", true); toggle(&editor, "health-audio-enabled-0", true);
        toggle(&editor, "health-audio-clipping-0", false);
        pump(Duration::from_millis(300)); assert!(find(&editor, "health-audio-save-notice").downcast::<Label>().unwrap().text().contains(&fl!(LANGUAGE_LOADER, "health-audio-unsaved")));
        save(&editor); assert!(pump_until(Duration::from_secs(3), saved));
        let rules = target.read().unwrap().audio_alerts; assert_eq!(rules.len(), 1); assert_eq!(rules[0].target.uuid.as_deref(), Some("input-1")); assert!(rules[0].enabled && rules[0].silence && !rules[0].clipping);
        assert_ne!(nav.state.borrow().connected_connection_id.as_deref(), Some(nav.state.borrow().config.connection_id.as_str()));
        editor.close();
        {
            let mut state = nav.state.borrow_mut(); state.workspace.input_catalog.as_mut().unwrap().as_mut().unwrap()[0].target.uuid = Some("replacement".into());
            state.workspace.audio[0].uuid = Some("replacement".into());
        }
        show(parent.upcast_ref(), &nav); let missing = window("health-audio-editor");
        assert!(find(&missing, "health-audio-select-0").downcast::<CheckButton>().unwrap().is_active());
        assert!(!find(&missing, "health-audio-select-1").downcast::<CheckButton>().unwrap().is_active());
        toggle(&missing, "health-audio-enabled-0", false); save(&missing); assert!(pump_until(Duration::from_secs(3), saved));
        let retained = target.read().unwrap().audio_alerts; assert!(!retained[0].enabled); assert_eq!(retained[0].target.uuid.as_deref(), Some("input-1"));
        toggle(&missing, "health-audio-select-0", false); save(&missing); assert!(pump_until(Duration::from_secs(3), saved)); assert!(target.read().unwrap().audio_alerts.is_empty());
        // A closed or retired editor cannot enqueue a later write.
        missing.close(); toggle(&missing, "health-audio-select-1", true); save(&missing); assert!(nav.state.borrow().registry.audio_alerts.is_empty());
        show(parent.upcast_ref(), &nav); let retired = window("health-audio-editor"); toggle(&retired, "health-audio-select-0", true);
        nav.state.borrow_mut().workspace.epoch += 1; save(&retired); assert!(nav.state.borrow().registry.audio_alerts.is_empty());
        pump(Duration::from_millis(300)); assert!(!find(&retired, "health-audio-save").is_sensitive());
        retired.close(); parent.close();
    });
}

#[test]
#[ignore = "requires isolated GTK and failed scratch workspace writes"]
fn native_audio_rule_save_failure_remains_visible_and_retryable() {
    native_test("ui::health_alerts::audio_editor::tests::native_audio_rule_save_failure_remains_visible_and_retryable", || {
        let (nav, _stack, _runtime, _events) = navigation(); let target = setup(&nav);
        let RegistryTarget::Workspace { path, .. } = &target else { unreachable!() };
        let original = std::fs::read(path).unwrap(); std::fs::write(path, b"invalid store").unwrap();
        let parent = adw::Window::new(); parent.present(); show(parent.upcast_ref(), &nav); let editor = window("health-audio-editor");
        toggle(&editor, "health-audio-select-0", true); toggle(&editor, "health-audio-enabled-0", true); save(&editor);
        assert!(pump_until(Duration::from_secs(3), || matches!(crate::ui::persist::registry_save_status(), crate::ui::persist::RegistrySaveStatus::Failed(_))));
        pump(Duration::from_millis(300)); assert!(editor.is_visible()); assert_ne!(find(&editor, "health-audio-save-notice").downcast::<Label>().unwrap().text(), fl!(LANGUAGE_LOADER, "health-audio-saved"));
        assert_eq!(std::fs::read(path).unwrap(), b"invalid store");
        std::fs::write(path, original).unwrap(); save(&editor); assert!(pump_until(Duration::from_secs(3), saved)); assert!(target.read().unwrap().audio_alerts[0].enabled);
        // Changed rules in another editor retire this draft instead of overwriting them.
        nav.state.borrow_mut().registry.audio_alerts[0].enabled = false; save(&editor); assert!(!nav.state.borrow().registry.audio_alerts[0].enabled);
        editor.close(); parent.close();
    });
}
