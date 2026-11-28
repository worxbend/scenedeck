use crate::ui::test_support::{expect_named, find_named, find_where};

#[test]
#[ignore = "requires a disposable GTK display; synthetic source observations"]
fn native_visibility_and_filter_readback_retain_widgets_without_echoing_commands() {
    crate::ui::native_test!(|| {
        use super::*;
        use crate::domain::control::{ControlSnapshot, SourceFilter, SourceItem};
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        {
            let mut state = nav.state.borrow_mut();
            state.obs_status = ObsStatus::Connected {
                obs_version: "fixture".into(),
            };
            state.scene_collections.current = Some("Fixture".into());
            state.available_requests = vec![
                "SetSceneItemEnabled".into(),
                "SetSourceFilterEnabled".into(),
            ];
            state.workspace.observed_at = Some(std::time::Instant::now());
            state.workspace.snapshot = Some(ControlSnapshot {
                scene: Some("Main".into()),
                items: vec![SourceItem {
                    owner: "Main".into(),
                    item_id: 1,
                    input: "Camera".into(),
                    input_uuid: String::new(),
                    kind: None,
                    path: vec!["Main".into()],
                    enabled: true,
                    ancestor_enabled: true,
                    is_group: false,
                    is_scene: false,
                }],
                ..Default::default()
            });
            state.workspace.selected_input = Some("Camera".into());
            state.workspace.detail = Some(SourceDetail {
                input: "Camera".into(),
                filters: vec![SourceFilter {
                    name: "Crop".into(),
                    kind: "crop_filter".into(),
                    enabled: true,
                }],
                text: Some("Original".into()),
                audio: Some(crate::domain::control::AudioSettings {
                    balance: 0.5,
                    sync_offset_ms: 0,
                    monitor: AudioMonitor::Off,
                    tracks: [true; 6],
                }),
                ..Default::default()
            });
        }
        let (page, refresh) = build(nav.clone());
        let mut checks = Vec::new();
        visit(&page, &mut |widget| {
            if widget.widget_name().starts_with("control-focus-item-")
                || widget.widget_name().starts_with("control-focus-filter-")
            {
                if let Some(check) = widget.downcast_ref::<CheckButton>() {
                    checks.push(check.clone());
                }
            }
        });
        assert_eq!(checks.len(), 2);
        let first = page.first_child();
        let sequence = nav.state.borrow().workspace.sequence;
        for enabled in [false, true, false] {
            {
                let mut state = nav.state.borrow_mut();
                state.workspace.snapshot.as_mut().unwrap().items[0].enabled = enabled;
                state.workspace.snapshot.as_mut().unwrap().items[0].ancestor_enabled = enabled;
                let detail = state.workspace.detail.as_mut().unwrap();
                detail.filters[0].enabled = enabled;
                detail.text = Some(if enabled { "Enabled" } else { "Disabled" }.into());
                let audio = detail.audio.as_mut().unwrap();
                audio.monitor = if enabled {
                    AudioMonitor::MonitorOnly
                } else {
                    AudioMonitor::Off
                };
                audio.sync_offset_ms = if enabled { 150 } else { 250 };
                audio.balance = if enabled { 0.25 } else { 0.75 };
                audio.tracks = [enabled; 6];
            }
            refresh();
            assert_eq!(page.first_child(), first);
            assert!(checks
                .iter()
                .all(|check| check.parent().is_some() && check.is_active() == enabled));
            visit(&page, &mut |widget| {
                if let Some(view) = widget.downcast_ref::<gtk4::TextView>() {
                    let buffer = view.buffer();
                    assert_eq!(
                        buffer.text(&buffer.start_iter(), &buffer.end_iter(), true),
                        if enabled { "Enabled" } else { "Disabled" }
                    );
                }
                if widget.widget_name() == "control-focus-monitor-Camera" {
                    assert_eq!(
                        widget.downcast_ref::<DropDown>().unwrap().selected(),
                        u32::from(enabled)
                    );
                }
                if widget
                    .widget_name()
                    .starts_with("control-focus-track-Camera-")
                {
                    assert_eq!(
                        widget.downcast_ref::<CheckButton>().unwrap().is_active(),
                        enabled
                    );
                }
            });
            assert_eq!(
                nav.state.borrow().workspace.sequence,
                sequence,
                "readback emitted a command"
            );
            assert!(nav.state.borrow().workspace.activity.entries.is_empty());
        }
    });
}

#[test]
fn sources_content_ignores_publication_metadata_but_tracks_control_content() {
    use super::*;
    use crate::controller::test_support::state::app_state;
    let mut state = app_state();
    state.workspace.snapshot = Some(Default::default());
    state.workspace.detail = Some(SourceDetail {
        input: "Caption".into(),
        text: Some("Original".into()),
        ..Default::default()
    });
    let original = sources_content_from_state(&state);
    state.workspace.sequence += 1;
    state.workspace.touch();
    state.workspace.loading = true;
    state.workspace.pending = true;
    state.workspace.dirty = true;
    state.workspace.result = "Acknowledged".into();
    state.workspace.error = Some("Read failure".into());
    state.workspace.observed_at = Some(std::time::Instant::now());
    state.workspace.snapshot.as_mut().unwrap().studio.program = Some("Live output".into());
    state.workspace.detail.as_mut().unwrap().volume_db = Some(-6.0);
    assert!(original == sources_content_from_state(&state));
    state.workspace.detail.as_mut().unwrap().text = Some("External edit".into());
    assert!(original != sources_content_from_state(&state));
    state.workspace.detail.as_mut().unwrap().text = Some("Original".into());
    assert!(original == sources_content_from_state(&state));
    state.workspace.epoch += 1;
    assert!(original != sources_content_from_state(&state));
    state.workspace.epoch -= 1;
    state.scene_collections.current = Some("Different collection".into());
    assert!(original != sources_content_from_state(&state));
    state.scene_collections.current = None;
    state.available_requests.push("SetInputSettings".into());
    assert!(original != sources_content_from_state(&state));
    state.available_requests.clear();
    state.workspace.selected_input = Some("Other".into());
    assert!(original != sources_content_from_state(&state));
    state.workspace.selected_input = None;
    state.obs_status = ObsStatus::Connected {
        obs_version: "Synthetic".into(),
    };
    assert!(original != sources_content_from_state(&state));
    state.obs_status = ObsStatus::Disconnected;
    assert!(original == sources_content_from_state(&state));
    state
        .workspace
        .snapshot
        .as_mut()
        .unwrap()
        .warnings
        .push(crate::domain::control::WorkspaceWarning::InventoryTruncated);
    assert!(original != sources_content_from_state(&state));
}

#[test]
#[ignore = "requires a disposable GTK display; synthetic source observations"]
fn native_long_usage_paths_fit_without_hiding_the_editor() {
    crate::ui::native_test!(|| {
        use super::*;
        use crate::{
            controller::{app_controller::AppController, test_support::state::app_state},
            domain::control::{ControlSnapshot, SourceItem},
            ui::test_support::pump,
        };
        use std::time::Duration;
        const SETTLE: Duration = Duration::from_millis(250);
        crate::ui::theme::ThemeManager::apply(&Default::default());
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let (tx, _events) = std::sync::mpsc::sync_channel(16);
        let state = Rc::new(RefCell::new(app_state()));
        let owner = "Long owner with localized words — Довга назва сцени для перевірки";
        let item = SourceItem {
            owner: owner.into(), item_id: 1, input: "Shared caption".into(), input_uuid: String::new(),
            kind: None, path: vec![owner.into(), "AReallyLongUnbrokenNestedSceneIdentifierThatMustStillWrapWithoutForcingHorizontalClipping".into()],
            enabled: true, ancestor_enabled: true, is_group: false, is_scene: false,
        };
        {
            let mut state = state.borrow_mut();
            state.obs_status = ObsStatus::Connected {
                obs_version: "synthetic fixture".into(),
            };
            state.scene_collections.current = Some("Synthetic collection".into());
            state.workspace.observed_at = Some(std::time::Instant::now());
            state.workspace.selected_input = Some("Shared caption".into());
            state.workspace.snapshot = Some(ControlSnapshot {
                scene: Some(owner.into()),
                items: vec![item.clone()],
                ..Default::default()
            });
            state.workspace.detail = Some(SourceDetail {
                input: "Shared caption".into(),
                text: Some("Editable source text".into()),
                audio: Some(crate::domain::control::AudioSettings {
                    monitor: AudioMonitor::Off,
                    sync_offset_ms: 0,
                    balance: 0.5,
                    tracks: [true; 6],
                }),
                ..Default::default()
            });
            state.workspace.usage = (0..10)
                .map(|index| {
                    let mut path = item.clone();
                    path.item_id += index;
                    path.path.push(format!("Structural path {index}"));
                    path
                })
                .collect();
        }
        let nav = NavigationContext::new(
            state,
            gtk4::Stack::new(),
            Rc::new(RefCell::new(AppController::new(
                runtime.handle().clone(),
                tx,
            ))),
        );
        let root = column(0);
        let local = Rc::new(RefCell::new(LocalState::default()));
        local.borrow_mut().set_epoch(0);
        local.borrow_mut().show_detail = true;
        populate(&root, &nav, &local);
        let window = gtk4::Window::builder()
            .default_width(1100)
            .default_height(740)
            .child(&root)
            .build();
        window.present();
        for (width, height) in [(1100, 740), (640, 600)] {
            window.set_default_size(width, height);
            pump(SETTLE);
            let usage = expect_named::<gtk4::Expander>(root.upcast_ref(), "control-focus-usage");
            assert!(
                !usage.is_expanded(),
                "secondary paths should start collapsed"
            );
            let scroll = expect_named::<gtk4::Widget>(root.upcast_ref(), "sources-detail-scroll");
            let editor = expect_named::<gtk4::Widget>(
                root.upcast_ref(),
                "control-focus-text-Shared caption",
            );
            let bounds = editor.compute_bounds(&scroll).unwrap();
            assert!(bounds.y() >= 0.0 && bounds.y()+bounds.height() <= scroll.height() as f32,
                "editor should be visible before expanding ten structural paths at {width}px: {bounds:?}");
            usage.set_expanded(true);
            pump(SETTLE);
            let button = usage
                .child()
                .unwrap()
                .first_child()
                .unwrap()
                .downcast::<Button>()
                .unwrap();
            let label = button.child().and_downcast::<gtk4::Label>().unwrap();
            assert!(
                label.layout().line_count() > 1,
                "long paths must actually wrap"
            );
            let bounds = button.compute_bounds(&scroll).unwrap();
            assert!(
                bounds.x() >= 0.0 && bounds.x() + bounds.width() <= scroll.width() as f32,
                "expanded path must fit detail viewport at {width}px: {bounds:?}, viewport={}",
                scroll.width()
            );
            assert!(button.tooltip_text().unwrap().contains(owner));
            // Media/readback refresh replaces detail widgets; preserve the user's
            // explicit expansion rather than closing the path list every second.
            gtk4::prelude::RootExt::set_focus(&window, gtk4::Widget::NONE);
            crate::ui::clear_children(&root);
            populate(&root, &nav, &local);
            pump(SETTLE);
            let restored = expect_named::<gtk4::Expander>(root.upcast_ref(), "control-focus-usage");
            assert!(
                restored.is_expanded(),
                "readback rebuild must preserve expanded paths"
            );
            restored.set_expanded(false);
            pump(SETTLE);
        }
        window.close();

        // Exercise the actual page refresh closure, including its named focus
        // capture and idle restoration, rather than only rebuilding test widgets.
        let (page, refresh) = build(nav.clone());
        let window = gtk4::Window::builder()
            .default_width(1100)
            .default_height(740)
            .child(&page)
            .build();
        window.present();
        pump(SETTLE);
        let usage = expect_named::<gtk4::Expander>(&page, "control-focus-usage");
        usage.set_expanded(true);
        pump(SETTLE);
        let path = usage.child().unwrap().first_child().unwrap();
        let identity = path.widget_name();
        assert!(identity.starts_with("control-focus-usage-"));
        assert!(path.grab_focus());
        refresh();
        pump(SETTLE);
        let restored = gtk4::prelude::RootExt::focus(&window).expect("path focus restored");
        assert_eq!(
            restored, path,
            "unchanged refresh must retain the actual focused path widget"
        );
        assert!(
            restored.is::<Button>(),
            "focus must remain on the path button, not the expander"
        );
        assert_eq!(
            restored.widget_name(),
            identity,
            "focus must retain the same structural path"
        );
        // Real page status publications must not replace search, editor or browser
        // GObjects. Keep an unapplied draft and search focus through the entire
        // request -> acknowledgement -> identical readback cycle.
        {
            let mut state = nav.state.borrow_mut();
            state.workspace.loading = false;
            state.workspace.pending = false;
            state.workspace.dirty = false;
            state.workspace.observed_at = Some(std::time::Instant::now());
            state.available_requests.push("SetInputSettings".into());
            state.available_requests.push("SetSceneItemEnabled".into());
        }
        refresh();
        pump(SETTLE);
        let search = expect_named::<gtk4::SearchEntry>(&page, "control-focus-search");
        let editor = expect_named::<gtk4::TextView>(&page, "control-focus-text-Shared caption");
        let save = expect_named::<gtk4::Widget>(&page, "control-focus-save-Shared caption");
        let browser = expect_named::<gtk4::Widget>(&page, "sources-list-scroll");
        let sync_name = format!(
            "control-focus-number-{}",
            fl!(LANGUAGE_LOADER, "control-audio-sync")
        );
        let sync = expect_named::<SpinButton>(&page, &sync_name);
        sync.set_value(125.0);
        // Sync offsets are integer spin buttons: a decimal draft is rejected
        // by GTK before refresh. A lone sign is a valid incomplete integer edit.
        sync.set_text("-");
        assert_eq!(
            sync.text(),
            "-",
            "numeric draft fixture must be accepted before any refresh"
        );
        let track = expect_named::<CheckButton>(&page, "control-focus-track-Shared caption-1");
        track.set_active(false);
        editor.buffer().set_text("Unapplied caption draft");
        search.set_text("Shared");
        pump(SETTLE);
        assert!(search.grab_focus());
        let search_focus = gtk4::prelude::RootExt::focus(&window).unwrap();
        let browser_bounds = browser.compute_bounds(&window).unwrap();
        for stage in 0..4 {
            {
                let mut state = nav.state.borrow_mut();
                state.workspace.sequence += 1;
                state.workspace.touch();
                state.workspace.loading = stage == 1 || stage == 2;
                state.workspace.pending = stage == 1;
                state.workspace.result = format!("Synthetic stage {stage}");
                if stage == 3 {
                    state.workspace.observed_at = Some(std::time::Instant::now());
                }
            }
            refresh();
            pump(SETTLE);
            assert_eq!(
                expect_named::<gtk4::Widget>(&page, "control-focus-search"),
                search.clone().upcast::<gtk4::Widget>()
            );
            assert_eq!(
                expect_named::<gtk4::Widget>(&page, "control-focus-text-Shared caption"),
                editor.clone().upcast::<gtk4::Widget>()
            );
            assert_eq!(
                expect_named::<gtk4::Widget>(&page, "sources-list-scroll"),
                browser
            );
            assert_eq!(
                browser.compute_bounds(&window).unwrap(),
                browser_bounds,
                "status readback shifted the Sources browser"
            );
            assert_eq!(
                gtk4::prelude::RootExt::focus(&window).unwrap(),
                search_focus
            );
            assert_eq!(
                editor.buffer().text(
                    &editor.buffer().start_iter(),
                    &editor.buffer().end_iter(),
                    true
                ),
                "Unapplied caption draft"
            );
            assert_eq!(search.text(), "Shared");
            assert_eq!(
                expect_named::<gtk4::Widget>(&page, &sync_name),
                sync.clone().upcast::<gtk4::Widget>()
            );
            assert_eq!(sync.text(), "-");
            assert_eq!(sync.value(), 125.0);
            assert!(!track.is_active());
            assert_eq!(save.is_sensitive(), stage != 1);
            assert_eq!(
                expect_named::<gtk4::Widget>(&page, "control-focus-refresh").is_sensitive(),
                stage != 1 && stage != 2
            );
            assert_eq!(
                expect_named::<gtk4::Widget>(&page, "sources-status-loading").is_visible(),
                stage == 1 || stage == 2
            );
            assert_eq!(
                expect_named::<Label>(&page, "sources-status-result").label(),
                format!("Synthetic stage {stage}")
            );
        }
        assert!(editor.grab_focus());
        let editor_focus = gtk4::prelude::RootExt::focus(&window).unwrap();
        for loading in [true, false] {
            nav.state.borrow_mut().workspace.loading = loading;
            refresh();
            pump(SETTLE);
            assert_eq!(
                gtk4::prelude::RootExt::focus(&window).unwrap(),
                editor_focus
            );
            assert!(
                save.is_sensitive(),
                "background reads must preserve valid editors"
            );
        }
        {
            let mut state = nav.state.borrow_mut();
            state.workspace.observed_at =
                Some(std::time::Instant::now() - std::time::Duration::from_secs(40));
        }
        refresh();
        assert!(!save.is_sensitive());
        assert!(expect_named::<gtk4::Widget>(&page, "sources-status-stale").is_visible());
        assert!(expect_named::<Label>(&page, "sources-status-age")
            .label()
            .contains("40"));
        {
            let mut state = nav.state.borrow_mut();
            state.workspace.error = Some("Synthetic read failure".into());
            state.workspace.observed_at = None;
        }
        refresh();
        assert_eq!(
            expect_named::<gtk4::Widget>(&page, "control-focus-text-Shared caption"),
            editor.clone().upcast::<gtk4::Widget>()
        );
        assert_eq!(
            expect_named::<Label>(&page, "sources-status-error").label(),
            "Synthetic read failure"
        );
        assert!(!expect_named::<gtk4::Widget>(&page, "sources-status-age").is_visible());
        {
            let mut state = nav.state.borrow_mut();
            state.workspace.error = None;
            state.workspace.observed_at = Some(std::time::Instant::now());
            state.workspace.detail.as_mut().unwrap().text = Some("External caption".into());
        }
        refresh();
        let replaced = expect_named::<gtk4::Widget>(&page, "control-focus-text-Shared caption");
        assert_eq!(
            replaced,
            editor.clone().upcast::<gtk4::Widget>(),
            "text readback must preserve the editor and use current origins for Cancel"
        );
        expect_named::<Button>(&page, "control-focus-cancel-Shared caption").emit_clicked();
        let replaced = replaced.downcast::<gtk4::TextView>().unwrap();
        assert_eq!(
            replaced.buffer().text(
                &replaced.buffer().start_iter(),
                &replaced.buffer().end_iter(),
                true
            ),
            "External caption"
        );
        let old_usage = expect_named::<gtk4::Expander>(&page, "control-focus-usage")
            .child()
            .unwrap()
            .first_child()
            .unwrap()
            .downcast::<Button>()
            .unwrap();
        let old_scope = nav.scoped();
        nav.state.borrow_mut().workspace.epoch += 1;
        refresh();
        assert_ne!(
            expect_named::<gtk4::Widget>(&page, "control-focus-text-Shared caption"),
            replaced.upcast::<gtk4::Widget>()
        );
        assert!(!old_scope.is_current());
        let sequence = nav.state.borrow().workspace.sequence;
        old_usage.emit_clicked();
        assert_eq!(
            nav.state.borrow().workspace.sequence,
            sequence,
            "detached old callback must reject the new epoch"
        );
        // Keep native dropdown interaction alive across media-only replies,
        // including a reply that was already in flight when the popup opened.
        {
            let mut state = nav.state.borrow_mut();
            state.current_page = crate::controller::state::Page::Sources;
            state.workspace.loading = false;
            state.workspace.dirty = false;
            state.workspace.observed_at = Some(std::time::Instant::now());
            state.workspace.selected_input = Some("Media".into());
            state.workspace.usage.clear();
            state
                .available_requests
                .push("SetInputAudioMonitorType".into());
            state.workspace.detail = Some(SourceDetail {
                input: "Media".into(),
                media: Some(crate::domain::control::MediaStatus {
                    state: crate::domain::control::MediaState::Playing,
                    duration_ms: Some(4000),
                    cursor_ms: Some(0),
                }),
                audio: Some(crate::domain::control::AudioSettings {
                    monitor: AudioMonitor::Off,
                    sync_offset_ms: 0,
                    balance: 0.5,
                    tracks: [true; 6],
                }),
                ..Default::default()
            });
        }
        window.set_default_size(1100, 1100);
        refresh();
        pump(SETTLE);
        let selector = expect_named::<gtk4::Widget>(&page, "control-focus-monitor-Media");
        let popup = find_where(&selector, |widget| widget.is::<gtk4::Popover>())
            .and_downcast::<gtk4::Popover>()
            .expect("native dropdown popover");
        selector
            .first_child()
            .and_downcast::<gtk4::ToggleButton>()
            .unwrap()
            .set_active(true);
        pump(SETTLE);
        assert!(mapped_popover(&page), "dropdown must actually be mapped");
        for cursor in 1..=6 {
            nav.state
                .borrow_mut()
                .workspace
                .detail
                .as_mut()
                .unwrap()
                .media
                .as_mut()
                .unwrap()
                .cursor_ms = Some(cursor * 250);
            refresh();
            pump(SETTLE);
            assert!(
                popup.is_mapped(),
                "media tick must not close an open dropdown"
            );
            assert_eq!(
                expect_named::<gtk4::Widget>(&page, "control-focus-monitor-Media"),
                selector
            );
        }
        popup.popdown();
        pump(SETTLE);
        refresh();
        pump(SETTLE);
        let resumed = expect_named::<gtk4::Widget>(&page, "control-focus-monitor-Media");
        assert_eq!(
            resumed, selector,
            "media readback must retain the native selector"
        );
        assert_eq!(
            expect_named::<SpinButton>(&page, "control-focus-seek-position-Media").value(),
            1.5
        );
        let popup = find_where(&resumed, |widget| widget.is::<gtk4::Popover>())
            .and_downcast::<gtk4::Popover>()
            .expect("resumed dropdown popover");
        resumed
            .first_child()
            .and_downcast::<gtk4::ToggleButton>()
            .unwrap()
            .set_active(true);
        pump(SETTLE);
        assert!(popup.is_mapped());
        {
            let mut state = nav.state.borrow_mut();
            state.workspace.error = Some("Synthetic disconnected read".into());
            state.workspace.observed_at = None;
        }
        refresh();
        pump(SETTLE);
        assert!(
            !popup.is_mapped(),
            "read failure must invalidate an open dropdown immediately"
        );
        assert_eq!(
            expect_named::<gtk4::Widget>(&page, "control-focus-monitor-Media"),
            resumed,
            "status-only failure must disable, not replace the native selector"
        );
        assert!(!resumed.is_sensitive());
        assert!(expect_named::<gtk4::Widget>(&page, "sources-status-error").is_visible());
        window.close();
    });
}

#[test]
fn editing_keeps_original_value_when_a_refresh_observes_an_external_edit() {
    let mut local = super::LocalState::default();
    local.edit_text("Caption", "Original", "My draft".into());
    assert_eq!(local.text_draft("Caption", "External edit"), "My draft");
    local.edit_text("Caption", "External edit", "My updated draft".into());
    assert_eq!(
        local.text_origins.get("Caption").map(String::as_str),
        Some("Original")
    );
    local.text_draft("Caption", "My updated draft");
    assert!(!local.text_origins.contains_key("Caption"));
}
use super::LocalState;

#[test]
fn switching_inputs_retains_drafts_and_detects_only_the_changed_origin() {
    let mut local = LocalState::default();
    local.edit_text("Caption", "Original", "Unsent caption".into());
    local.edit_text("Title", "Original title", "Unsent title".into());
    assert!(!local.text_conflict("Caption", "Original"));
    assert!(local.text_conflict("Caption", "Edited externally"));
    assert!(!local.text_conflict("Title", "Original title"));
    assert_eq!(local.text_draft("Title", "Original title"), "Unsent title");
    assert_eq!(
        local.text_draft("Caption", "Edited externally"),
        "Unsent caption"
    );
}

#[test]
fn media_refresh_preserves_numeric_and_track_drafts_until_readback_or_scope_loss() {
    let mut local = LocalState::default();
    local.set_epoch(1);
    local
        .numeric_drafts
        .insert(("Media".into(), "sync".into()), 125.0);
    local
        .track_drafts
        .insert("Media".into(), [true, false, false, false, false, true]);
    assert_eq!(local.numeric_draft("Media", "sync", 0.0), 125.0);
    assert_eq!(local.numeric_draft("Other", "sync", 0.0), 0.0);
    assert!(local.tracks_draft("Media", [true; 6])[5]);
    assert!(!local.tracks_draft("Media", [true; 6])[1]);
    assert_eq!(local.numeric_draft("Media", "sync", 125.0), 125.0);
    assert!(local.numeric_drafts.is_empty());
    local.tracks_draft("Media", [true, false, false, false, false, true]);
    assert!(local.track_drafts.is_empty());
    local
        .numeric_drafts
        .insert(("Media".into(), "sync".into()), 50.0);
    local.set_epoch(2);
    assert_eq!(local.numeric_draft("Media", "sync", 0.0), 0.0);
}

#[test]
fn ordinary_refresh_and_failed_save_preserve_text_draft() {
    let mut local = LocalState::default();
    local.set_epoch(1);
    local
        .text_drafts
        .insert("Caption".into(), "Unsent edit".into());
    local.set_epoch(1);
    assert_eq!(local.text_draft("Caption", "Original"), "Unsent edit");
    assert_eq!(local.text_draft("Caption", "Original"), "Unsent edit");
    assert_eq!(local.text_draft("Caption", "Unsent edit"), "Unsent edit");
    assert!(!local.text_drafts.contains_key("Caption"));
}

#[test]
fn another_session_cannot_inherit_a_same_named_source_draft() {
    let mut local = LocalState::default();
    local.set_epoch(1);
    local
        .text_drafts
        .insert("Caption".into(), "Old session".into());
    local.query = "Old query".into();
    local.usage_expanded = true;
    local.set_epoch(2);
    assert!(!local.usage_expanded);
    assert_eq!(
        local.text_draft("Caption", "New collection"),
        "New collection"
    );
    assert!(local.query.is_empty());
}

// ── Section characterization (P0-T13) ──────────────────────────────────
//
// Each native test below pins the widget-tree facts one Sources section
// exposes today: the widget names other code finds, their labels and their
// sensitivity. Splitting this file must keep every one of them.

/// Edit `AppState` through `NavigationContext::update`, as pages should.
fn edit_state(
    nav: &crate::ui::navigation::NavigationContext,
    edit: impl FnOnce(&mut crate::controller::state::AppState),
) {
    nav.update(edit)
        .expect("no AppState borrow outlives a test step");
}

/// Change the observed source detail, as a readback would.
fn edit_detail(
    nav: &crate::ui::navigation::NavigationContext,
    edit: impl FnOnce(&mut super::SourceDetail),
) {
    edit_state(nav, |state| {
        edit(
            state
                .workspace
                .detail
                .as_mut()
                .expect("a source detail is observed"),
        )
    });
}

/// Connected, freshly observed Sources state that admits every listed request.
fn observe_sources(
    nav: &crate::ui::navigation::NavigationContext,
    items: Vec<crate::domain::control::SourceItem>,
    detail: Option<super::SourceDetail>,
    capabilities: &[&str],
) {
    edit_state(nav, |state| {
        state.obs_status = super::ObsStatus::Connected {
            obs_version: "fixture".into(),
        };
        state.scene_collections.current = Some("Fixture".into());
        state.available_requests = capabilities
            .iter()
            .map(|request| (*request).to_owned())
            .collect();
        state.workspace.observed_at = Some(std::time::Instant::now());
        state.workspace.selected_input = detail.as_ref().map(|detail| detail.input.clone());
        state.workspace.snapshot = Some(crate::domain::control::ControlSnapshot {
            scene: Some("Main".into()),
            items,
            ..Default::default()
        });
        state.workspace.detail = detail;
    });
}

/// Settle the admitted action as acknowledged and freshly read back, so the
/// page admits the next one.
fn acknowledge(nav: &crate::ui::navigation::NavigationContext) {
    edit_state(nav, |state| {
        state.workspace.pending = false;
        state.workspace.loading = false;
        state.workspace.active_action = None;
        state.workspace.observed_at = Some(std::time::Instant::now());
    });
}

fn active_action(
    nav: &crate::ui::navigation::NavigationContext,
) -> Option<crate::domain::control::ControlAction> {
    nav.read(|state| state.workspace.active_action.clone())
}

fn source_item(input: &str, item_id: i64) -> crate::domain::control::SourceItem {
    crate::domain::control::SourceItem {
        owner: "Main".into(),
        item_id,
        input: input.into(),
        input_uuid: String::new(),
        kind: None,
        path: vec!["Main".into()],
        enabled: true,
        ancestor_enabled: true,
        is_group: false,
        is_scene: false,
    }
}

/// Unnamed status labels are only reachable by their text.
fn label_with_text(root: &gtk4::Widget, text: &str) -> Option<super::Label> {
    use adw::prelude::*;
    find_where(root, |widget| {
        widget
            .downcast_ref::<super::Label>()
            .is_some_and(|label| label.label() == text)
    })
    .and_downcast()
}

fn buffer_text(view: &gtk4::TextView) -> String {
    use adw::prelude::*;
    let buffer = view.buffer();
    buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), true)
        .to_string()
}

#[test]
#[ignore = "requires a disposable GTK display; synthetic source observations"]
fn native_sources_browser_rows_name_item_checks_inspect_buttons_and_search() {
    crate::ui::native_test!(|| {
        use super::*;
        use crate::domain::{
            control::{ControlAction, SourceItem},
            scene::Scene,
        };
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        let group = SourceItem {
            is_group: true,
            ..source_item("Group", 2)
        };
        let nested = SourceItem {
            enabled: false,
            ancestor_enabled: false,
            path: vec!["Main".into(), "Group".into()],
            ..source_item("Hidden mic", 3)
        };
        observe_sources(
            &nav,
            vec![source_item("Camera", 1), group, nested],
            None,
            &["SetSceneItemEnabled"],
        );
        edit_state(&nav, |state| {
            state.workspace.selected_input = Some("Camera".into());
            state.scene_inventory.scenes = vec![Scene {
                id: "Main".into(),
                uuid: None,
            }];
        });
        let (page, _refresh) = build(nav.clone());

        assert_eq!(
            expect_named::<DropDown>(&page, "control-focus-scene").selected(),
            0
        );
        let camera: CheckButton = expect_named(&page, "control-focus-item-Main-1-Main");
        assert!(camera.is_active() && camera.is_sensitive());
        assert!(camera.has_css_class("sources-mutation"));
        assert_eq!(camera.tooltip_text().as_deref(), Some("Camera"));
        let hidden: CheckButton = expect_named(&page, "control-focus-item-Main-3-Main/Group");
        assert!(!hidden.is_active());
        assert!(!expect_named::<Label>(&page, "sources-hidden-parent-Main-1-Main").is_visible());
        assert!(
            expect_named::<Label>(&page, "sources-hidden-parent-Main-3-Main/Group").is_visible()
        );
        let inspect: Button = expect_named(&page, "control-focus-inspect-Main-1-Main");
        assert!(inspect.is_sensitive());
        assert!(inspect.has_css_class("sources-inspect"));
        assert!(inspect.has_css_class("suggested-action"));
        let group_inspect: Button = expect_named(&page, "control-focus-inspect-Main-2-Main");
        assert!(!group_inspect.is_sensitive());
        assert!(!group_inspect.has_css_class("sources-inspect"));
        assert!(find_named(&page, "sources-list-scroll").is_some());
        assert!(find_named(&page, "sources-detail-scroll").is_some());
        assert!(find_named(&page, "control-focus-back").is_some());
        assert!(
            label_with_text(&page, &fl!(LANGUAGE_LOADER, "control-request-hint")).is_some(),
            "a selected source without a detail read asks for one"
        );

        let search: gtk4::SearchEntry = expect_named(&page, "control-focus-search");
        let row_of = |check: &CheckButton| check.parent().and_then(|line| line.parent());
        let (camera_row, hidden_row) = (row_of(&camera).unwrap(), row_of(&hidden).unwrap());
        let no_matches =
            label_with_text(&page, &fl!(LANGUAGE_LOADER, "control-no-matches")).unwrap();
        search.set_text("MIC");
        search.emit_by_name::<()>("search-changed", &[]);
        assert!(!camera_row.is_visible() && hidden_row.is_visible());
        assert!(!no_matches.is_visible());
        search.set_text("no such source");
        search.emit_by_name::<()>("search-changed", &[]);
        assert!(!hidden_row.is_visible());
        assert!(no_matches.is_visible());

        camera.set_active(false);
        assert_eq!(
            active_action(&nav),
            Some(ControlAction::SetItemEnabled {
                owner: "Main".into(),
                item_id: 1,
                enabled: false,
            })
        );
        acknowledge(&nav);
        search.set_text("hidden");
        search.emit_by_name::<()>("activate", &[]);
        assert_eq!(
            nav.read(|state| state.workspace.selected_input.clone()),
            Some("Hidden mic".into())
        );
    });
}

#[test]
#[ignore = "requires a disposable GTK display; synthetic source observations"]
fn native_sources_text_detail_names_editor_actions_and_conflict_warning() {
    crate::ui::native_test!(|| {
        use super::*;
        use crate::domain::control::ControlAction;
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        observe_sources(
            &nav,
            vec![source_item("Caption", 1)],
            Some(SourceDetail {
                input: "Caption".into(),
                text: Some("Hello".into()),
                ..Default::default()
            }),
            &["SetInputSettings"],
        );
        let (page, refresh) = build(nav.clone());

        let editor: gtk4::TextView = expect_named(&page, "control-focus-text-Caption");
        assert_eq!(buffer_text(&editor), "Hello");
        let save: Button = expect_named(&page, "control-focus-save-Caption");
        assert!(save.is_sensitive());
        assert!(save.has_css_class("sources-mutation"));
        assert!(save.has_css_class("sources-text-save"));
        let cancel: Button = expect_named(&page, "control-focus-cancel-Caption");
        let conflict: Label = expect_named(&page, "sources-text-conflict");
        assert!(!conflict.is_visible());
        assert!(find_named(&page, "source-media-controls").is_none());
        assert!(find_named(&page, "control-focus-monitor-Caption").is_none());

        editor.buffer().set_text("Draft");
        edit_detail(&nav, |detail| detail.text = Some("External".into()));
        refresh();
        assert_eq!(
            expect_named::<gtk4::TextView>(&page, "control-focus-text-Caption"),
            editor
        );
        assert_eq!(buffer_text(&editor), "Draft");
        assert!(conflict.is_visible());
        assert!(
            !save.is_sensitive(),
            "a conflicting draft must not be saved"
        );

        cancel.emit_clicked();
        assert_eq!(buffer_text(&editor), "External");
        assert!(!conflict.is_visible());
        assert!(save.is_sensitive());

        editor.buffer().set_text("Edited");
        save.emit_clicked();
        assert_eq!(
            active_action(&nav),
            Some(ControlAction::SetText {
                input: "Caption".into(),
                text: "Edited".into(),
            })
        );

        acknowledge(&nav);
        edit_detail(&nav, |detail| detail.text_from_file = true);
        refresh();
        assert!(find_named(&page, "control-focus-text-Caption").is_none());
        assert!(find_named(&page, "control-focus-save-Caption").is_none());
        assert!(label_with_text(&page, &fl!(LANGUAGE_LOADER, "control-text-file")).is_some());
    });
}

#[test]
#[ignore = "requires a disposable GTK display; synthetic source observations"]
fn native_sources_media_detail_names_transport_seek_and_section() {
    crate::ui::native_test!(|| {
        use super::*;
        use crate::domain::control::{ControlAction, MediaState, MediaStatus};
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        observe_sources(
            &nav,
            vec![source_item("Clip", 1)],
            Some(SourceDetail {
                input: "Clip".into(),
                media: Some(MediaStatus {
                    state: MediaState::Playing,
                    duration_ms: Some(4000),
                    cursor_ms: Some(1000),
                }),
                ..Default::default()
            }),
            &["TriggerMediaInputAction", "SetMediaInputCursor"],
        );
        let (page, refresh) = build(nav.clone());

        assert!(find_named(&page, "source-media-controls").is_some());
        let status: Label = expect_named(&page, "sources-media-state");
        assert_eq!(status.text(), MediaState::Playing.label());
        for action in [
            MediaAction::Play,
            MediaAction::Pause,
            MediaAction::Stop,
            MediaAction::Restart,
            MediaAction::Previous,
            MediaAction::Next,
        ] {
            let control = ControlAction::Media {
                input: "Clip".into(),
                action,
            };
            let button: Button = expect_named(&page, &format!("control-focus-action-{control:?}"));
            assert!(button.is_sensitive() && button.has_css_class("sources-mutation"));
        }
        let position: SpinButton = expect_named(&page, "control-focus-seek-position-Clip");
        assert_eq!(position.value(), 1.0);
        assert_eq!(position.adjustment().upper(), 4.0);
        let seek: Button = expect_named(&page, "control-focus-seek-Clip");
        assert!(seek.is_sensitive() && seek.has_css_class("sources-mutation"));

        position.set_value(3.0);
        seek.emit_clicked();
        assert_eq!(
            active_action(&nav),
            Some(ControlAction::SeekMedia {
                input: "Clip".into(),
                position_ms: 3000,
            })
        );

        acknowledge(&nav);
        edit_detail(&nav, |detail| {
            let media = detail.media.as_mut().unwrap();
            media.state = MediaState::Paused;
            media.cursor_ms = Some(2000);
        });
        refresh();
        assert_eq!(expect_named::<Label>(&page, "sources-media-state"), status);
        assert_eq!(status.text(), MediaState::Paused.label());
        assert_eq!(
            expect_named::<SpinButton>(&page, "control-focus-seek-position-Clip"),
            position
        );
        assert_eq!(position.value(), 2.0);

        edit_detail(&nav, |detail| {
            detail.media.as_mut().unwrap().duration_ms = None
        });
        refresh();
        assert!(find_named(&page, "control-focus-seek-position-Clip").is_none());
        assert!(find_named(&page, "control-focus-seek-Clip").is_none());
        assert!(find_named(&page, "source-media-controls").is_some());
    });
}

#[test]
#[ignore = "requires a disposable GTK display; synthetic source observations"]
fn native_sources_audio_detail_names_monitor_numbers_and_tracks() {
    crate::ui::native_test!(|| {
        use super::*;
        use crate::domain::control::{AudioSettings, ControlAction};
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        observe_sources(
            &nav,
            vec![source_item("Mic", 1)],
            Some(SourceDetail {
                input: "Mic".into(),
                audio: Some(AudioSettings {
                    monitor: AudioMonitor::MonitorOnly,
                    sync_offset_ms: 150,
                    balance: 0.25,
                    tracks: [true, false, true, false, true, false],
                }),
                ..Default::default()
            }),
            &["SetInputAudioMonitorType", "SetInputAudioTracks"],
        );
        let (page, _refresh) = build(nav.clone());
        let sync_title = fl!(LANGUAGE_LOADER, "control-audio-sync");
        let balance_title = fl!(LANGUAGE_LOADER, "control-audio-balance");

        let monitor: DropDown = expect_named(&page, "control-focus-monitor-Mic");
        assert_eq!(monitor.selected(), 1);
        assert!(monitor.is_sensitive() && monitor.has_css_class("sources-mutation"));
        let sync: SpinButton = expect_named(&page, &format!("control-focus-number-{sync_title}"));
        assert_eq!(sync.value(), 150.0);
        let sync_apply: Button = expect_named(&page, &format!("control-focus-apply-{sync_title}"));
        assert!(
            !sync_apply.is_sensitive() && !sync_apply.has_css_class("sources-mutation"),
            "an unsupported request stays disabled"
        );
        let balance: SpinButton =
            expect_named(&page, &format!("control-focus-number-{balance_title}"));
        assert_eq!(balance.value(), 0.25);
        assert_eq!(balance.digits(), 2);
        let balance_apply: Button =
            expect_named(&page, &format!("control-focus-apply-{balance_title}"));
        assert!(!balance_apply.is_sensitive());
        let tracks: Vec<CheckButton> = (0..6)
            .map(|index| expect_named(&page, &format!("control-focus-track-Mic-{index}")))
            .collect();
        assert_eq!(
            tracks
                .iter()
                .map(CheckButton::is_active)
                .collect::<Vec<_>>(),
            [true, false, true, false, true, false]
        );
        let apply: Button = expect_named(&page, "control-focus-tracks-apply-Mic");
        assert!(apply.is_sensitive() && apply.has_css_class("sources-mutation"));

        monitor.set_selected(2);
        assert_eq!(
            active_action(&nav),
            Some(ControlAction::SetAudioMonitor {
                input: "Mic".into(),
                monitor: AudioMonitor::MonitorAndOutput,
            })
        );
        acknowledge(&nav);
        tracks[1].set_active(true);
        apply.emit_clicked();
        assert_eq!(
            active_action(&nav),
            Some(ControlAction::SetAudioTracks {
                input: "Mic".into(),
                tracks: [true, true, true, false, true, false],
            })
        );
    });
}

#[test]
#[ignore = "requires a disposable GTK display; synthetic source observations"]
fn native_sources_numeric_drafts_survive_readback_until_observed_or_scope_loss() {
    crate::ui::native_test!(|| {
        use super::*;
        use crate::domain::control::AudioSettings;
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        observe_sources(
            &nav,
            vec![source_item("Mic", 1)],
            Some(SourceDetail {
                input: "Mic".into(),
                audio: Some(AudioSettings {
                    monitor: AudioMonitor::Off,
                    sync_offset_ms: 100,
                    balance: 0.5,
                    tracks: [true; 6],
                }),
                ..Default::default()
            }),
            &[],
        );
        let (page, refresh) = build(nav.clone());
        // Spin buttons only hold typed text once they are realized.
        let window = gtk4::Window::builder().child(&page).build();
        window.present();
        crate::ui::test_support::pump(std::time::Duration::from_millis(100));
        let sync_name = format!(
            "control-focus-number-{}",
            fl!(LANGUAGE_LOADER, "control-audio-sync")
        );
        let balance_name = format!(
            "control-focus-number-{}",
            fl!(LANGUAGE_LOADER, "control-audio-balance")
        );
        let observe_audio = |sync_offset_ms: i64, balance: f32| {
            edit_detail(&nav, |detail| {
                let audio = detail.audio.as_mut().unwrap();
                audio.sync_offset_ms = sync_offset_ms;
                audio.balance = balance;
            });
        };
        let sync: SpinButton = expect_named(&page, &sync_name);
        let balance: SpinButton = expect_named(&page, &balance_name);

        sync.set_value(300.0);
        // GTK rejects a decimal separator foreign to the locale; a lone
        // sign is an incomplete edit in every locale.
        balance.set_text("-");
        observe_audio(200, 0.75);
        refresh();
        assert_eq!(expect_named::<SpinButton>(&page, &sync_name), sync);
        assert_eq!(
            sync.value(),
            300.0,
            "a value draft outlives other readbacks"
        );
        assert_eq!(balance.text(), "-", "a text draft outlives other readbacks");

        observe_audio(300, 0.75);
        refresh();
        assert_eq!(sync.value(), 300.0);
        observe_audio(400, 0.75);
        refresh();
        assert_eq!(
            sync.value(),
            400.0,
            "a readback equal to the draft retires it, so later readbacks show"
        );
        assert_eq!(balance.text(), "-");

        edit_state(&nav, |state| state.workspace.epoch += 1);
        refresh();
        let rebuilt: SpinButton = expect_named(&page, &balance_name);
        assert_ne!(rebuilt, balance, "a new epoch rebuilds the editors");
        assert_eq!(rebuilt.value(), 0.75);
        assert_eq!(expect_named::<SpinButton>(&page, &sync_name).value(), 400.0);
        window.destroy();
    });
}
