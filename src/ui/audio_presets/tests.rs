use super::*;
use crate::{
    domain::{audio::AudioInput, operator_input::InputIdentity},
    storage::show_workspaces::RegistryTarget,
    ui::{
        preflight::tests::{connected, find, window},
        test_support::{native_test, navigation, pump, pump_until},
    },
};

fn setup(nav: &NavigationContext) -> RegistryTarget {
    connected(nav);
    let target = nav.state.borrow().desired_registry_target().unwrap();
    target.load_or_create(None).unwrap();
    nav.state.borrow_mut().show_registry.target = target.clone();
    target
}
fn click(root: &impl IsA<gtk4::Widget>, name: &str) {
    find(root, name)
        .downcast::<Button>()
        .unwrap()
        .emit_clicked();
}
fn name(root: &impl IsA<gtk4::Widget>, value: &str) {
    find(root, "preset-name")
        .downcast::<gtk4::Entry>()
        .unwrap()
        .set_text(value);
}
fn select(root: &impl IsA<gtk4::Widget>, index: usize, selected: bool) {
    find(root, &format!("preset-input-{index}"))
        .downcast::<CheckButton>()
        .unwrap()
        .set_active(selected);
}
fn saved() -> bool {
    super::super::persist::registry_save_status()
        == super::super::persist::RegistrySaveStatus::Saved
}
fn save(root: &impl IsA<gtk4::Widget>) {
    click(root, "preset-save");
    assert!(pump_until(Duration::from_secs(3), saved));
}

#[test]
#[ignore = "requires isolated GTK and scratch workspace storage"]
fn native_presets_capture_preview_rename_and_reject_missing_or_replaced_inputs() {
    native_test("ui::audio_presets::tests::native_presets_capture_preview_rename_and_reject_missing_or_replaced_inputs", || {
        crate::infra::i18n::init(crate::domain::appearance::Language::Uk);
        gtk4::Settings::default().unwrap().set_gtk_font_name(Some("Sans 22"));
        let (nav, _stack, _runtime, events) = navigation(); let target = setup(&nav);
        {
            let mut state = nav.state.borrow_mut();
            state.audio_inputs.clear(); // Capture is independent of filtered Live/Mixer cards.
            state.workspace.input_catalog.as_mut().unwrap().as_mut().unwrap().push(InputIdentity {
                target: ActionTarget { name: "Music <b>literal</b>".into(), uuid: Some("music".into()) }, kind: "audio".into(),
            });
            state.workspace.audio.push(AudioInput::new("Music <b>literal</b>".into(), true, 2.0, 6.0206).with_identity(Some("music".into())));
        }
        let parent = adw::Window::new(); parent.present(); show(parent.upcast_ref(), &nav);
        let editor = window("audio-presets"); editor.set_default_size(360, 600);
        select(&editor, 0, true); select(&editor, 1, true); name(&editor, "Podcast"); save(&editor);
        let preset = target.read().unwrap().audio_presets["Podcast"].clone();
        assert_eq!(preset.inputs.len(), 2); assert_eq!(f64::from(preset.inputs[1].gain), 2.0);
        assert!(preset.inputs[1].muted); assert_eq!(preset.inputs[0].target.uuid.as_deref(), Some("input-1"));
        {
            let mut state = nav.state.borrow_mut();
            state.workspace.audio[0].muted = true; state.workspace.audio[0].volume_mul = 0.0;
            state.workspace.audio[0].id = "Renamed microphone".into();
            state.workspace.input_catalog.as_mut().unwrap().as_mut().unwrap()[0].target.name = "Renamed microphone".into();
            state.workspace.audio.pop(); state.workspace.input_catalog.as_mut().unwrap().as_mut().unwrap().pop();
        }
        pump(Duration::from_millis(300));
        let preview = find(&editor, "preset-preview").downcast::<Label>().unwrap().text();
        assert!(preview.contains("Renamed microphone") && preview.contains("−∞"));
        assert!(preview.contains(&fl!(LANGUAGE_LOADER, "preset-missing")));
        save(&editor); assert_eq!(target.read().unwrap().audio_presets["Podcast"], preset, "missing selection cannot silently disappear on replace");
        select(&editor, 1, false); save(&editor);
        let updated = target.read().unwrap().audio_presets["Podcast"].clone();
        assert_eq!(updated.inputs.len(), 1); assert_eq!(updated.inputs[0].target.name, "Renamed microphone");
        assert_eq!(f64::from(updated.inputs[0].gain), 0.0);
        name(&editor, "Interview"); click(&editor, "preset-rename"); assert!(pump_until(Duration::from_secs(3), saved));
        assert!(!target.read().unwrap().audio_presets.contains_key("Podcast"));
        {
            let mut state = nav.state.borrow_mut();
            state.workspace.audio[0].uuid = Some("replacement".into());
            state.workspace.input_catalog.as_mut().unwrap().as_mut().unwrap()[0].target.uuid = Some("replacement".into());
        }
        pump(Duration::from_millis(300)); save(&editor);
        assert_eq!(target.read().unwrap().audio_presets["Interview"].inputs[0].target.uuid.as_deref(), Some("input-1"));
        assert!(editor.measure(Orientation::Horizontal, -1).0 <= 360);
        assert!(editor.measure(Orientation::Vertical, 360).0 <= 600);
        click(&editor, "preset-delete"); assert!(pump_until(Duration::from_secs(3), saved));
        assert!(target.read().unwrap().audio_presets.is_empty());
        assert!(events.try_iter().next().is_none(), "capture/review never dispatches an OBS mutation");
        editor.close(); name(&editor, "Retired"); select(&editor, 2, true); click(&editor, "preset-save");
        assert!(nav.state.borrow().registry.audio_presets.is_empty()); parent.close();
    });
}

#[test]
#[ignore = "requires isolated GTK and failed scratch workspace writes"]
fn native_presets_recover_failed_saves_and_retire_on_workspace_or_external_edits() {
    native_test("ui::audio_presets::tests::native_presets_recover_failed_saves_and_retire_on_workspace_or_external_edits", || {
        let (nav, _stack, _runtime, _events) = navigation(); let target = setup(&nav);
        let RegistryTarget::Workspace { path, .. } = &target else { unreachable!() };
        let original = std::fs::read(path).unwrap(); std::fs::write(path, b"invalid store").unwrap();
        let parent = adw::Window::new(); parent.present(); show(parent.upcast_ref(), &nav); let editor = window("audio-presets");
        name(&editor, "Show"); select(&editor, 0, true); click(&editor, "preset-save");
        assert!(pump_until(Duration::from_secs(3), || matches!(crate::ui::persist::registry_save_status(), crate::ui::persist::RegistrySaveStatus::Failed(_))));
        pump(Duration::from_millis(300)); assert!(!find(&editor, "preset-notice").downcast::<Label>().unwrap().text().is_empty());
        assert_eq!(std::fs::read(path).unwrap(), b"invalid store");
        std::fs::write(path, original).unwrap(); save(&editor); assert!(target.read().unwrap().audio_presets.contains_key("Show"));
        nav.state.borrow_mut().workspace.epoch += 1; click(&editor, "preset-delete"); assert!(nav.state.borrow().registry.audio_presets.contains_key("Show"));
        pump(Duration::from_millis(300)); assert!(!find(&editor, "preset-save").is_sensitive()); editor.close();
        show(parent.upcast_ref(), &nav); let editor = window("audio-presets"); click(&editor, "preset-select");
        nav.state.borrow_mut().registry.audio_presets.clear(); name(&editor, "Old draft"); click(&editor, "preset-save");
        assert!(nav.state.borrow().registry.audio_presets.is_empty()); editor.close(); parent.close();
    });
}

#[test]
#[ignore = "requires isolated GTK; fake controller only"]
fn native_preset_review_fences_apply_and_results_survive_editor_close() {
    native_test("ui::audio_presets::tests::native_preset_review_fences_apply_and_results_survive_editor_close", || {
        let (nav, _stack, _runtime, _events) = navigation(); setup(&nav);
        {
            let mut state = nav.state.borrow_mut();
            state.available_requests.extend(["GetInputList", "GetInputMute", "GetInputVolume", "SetInputVolume", "GetSceneCollectionList"].map(str::to_owned));
            let mut inputs = snapshot(&state).unwrap(); inputs[0].muted = true; inputs[0].gain = PresetGain::try_from(0.5).unwrap();
            state.registry.audio_presets.insert("Interview".into(), AudioPreset { inputs });
        }
        let parent = adw::Window::new(); parent.present(); show(parent.upcast_ref(), &nav); let editor = window("audio-presets");
        click(&editor, "preset-select"); assert!(!find(&editor, "preset-apply").is_sensitive());
        click(&editor, "preset-review"); assert!(find(&editor, "preset-apply").is_sensitive());
        nav.state.borrow_mut().workspace.audio[0].volume_mul = 0.25;
        click(&editor, "preset-apply"); assert!(nav.state.borrow().preset_run.is_none(), "changed preview cannot mutate even through a retained signal");
        click(&editor, "preset-review"); click(&editor, "preset-apply");
        let plan = nav.state.borrow().preset_run.as_ref().unwrap().plan.clone();
        click(&editor, "preset-apply"); assert_eq!(nav.state.borrow().preset_run.as_ref().unwrap().plan.id, plan.id);
        editor.close();
        nav.state.borrow_mut().preset_progress(plan.epoch, plan.id, 0, crate::domain::audio_preset::PresetOutcome { phase: crate::domain::audio_preset::PresetPhase::Unconfirmed, gain_ack: true, detail: Some("fixture readback unavailable".into()), ..Default::default() });
        show(parent.upcast_ref(), &nav); let editor = window("audio-presets");
        let results = find(&editor, "preset-results").downcast::<Label>().unwrap().text();
        assert!(results.contains("Interview") && results.contains("Microphone") && results.contains("fixture readback unavailable"));
        assert!(!find(&editor, "preset-apply").is_sensitive());
        editor.close(); parent.close();
    });
}
