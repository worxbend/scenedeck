//! Named local bookmarks and ordered, retryable persistence.

use super::{
    background_io::{self, SerialLane},
    navigation::NavigationContext,
};
use crate::{
    controller::state::{BookmarkError, ObsStatus},
    domain::bookmark::{Bookmark, RecordingTime, NAME_LIMIT},
    infra::i18n::LANGUAGE_LOADER,
};
use adw::prelude::*;
use gtk4::{Box as GtkBox, Button, Entry, Label, Orientation};
use i18n_embed_fl::fl;
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

thread_local! {
    static FAILURES: RefCell<Vec<(u64, background_io::FailureEvidence)>> = const { RefCell::new(Vec::new()) };
}

fn save(nav: &NavigationContext) {
    let (archive, revision) = {
        let mut state = nav.state.borrow_mut();
        let notes = &mut state.bookmarks;
        if notes.load_error.is_some() {
            return;
        }
        notes.revision += 1;
        notes.pending += 1;
        (notes.archive.clone(), notes.revision)
    };
    let path = crate::storage::xdg::bookmarks_path();
    let nav = nav.clone();
    background_io::run_serialized(
        SerialLane::Bookmarks,
        move || crate::storage::bookmarks::write(&path, &archive),
        move |result| {
            let mut state = nav.state.borrow_mut();
            let notes = &mut state.bookmarks;
            notes.pending = notes.pending.saturating_sub(1);
            match result {
                Ok(()) => {
                    notes.saved_revision = notes.saved_revision.max(revision);
                    if revision >= notes.failed_revision {
                        notes.save_error = None;
                    }
                    // Every write is a full snapshot, including earlier edits.
                    FAILURES.with(|failures| {
                        failures.borrow_mut().retain(|(at, evidence)| {
                            if *at <= revision {
                                background_io::resolve_failure(*evidence);
                                false
                            } else {
                                true
                            }
                        })
                    });
                }
                Err(error) => {
                    if revision > notes.saved_revision {
                        if revision >= notes.failed_revision {
                            notes.failed_revision = revision;
                            notes.save_error = Some(error.to_string());
                        }
                        FAILURES.with(|failures| {
                            failures
                                .borrow_mut()
                                .push((revision, background_io::record_failure()))
                        });
                    }
                }
            }
        },
    );
}

fn text(value: &str) -> Label {
    Label::builder()
        .label(value)
        .wrap(true)
        .wrap_mode(gtk4::pango::WrapMode::WordChar)
        .width_chars(1)
        .xalign(0.0)
        .build()
}

fn labeled_button(label: &str) -> Button {
    let button = Button::new();
    button.set_child(Some(&text(label)));
    button
}

fn clock(ms: u64) -> String {
    let seconds = ms / 1000;
    format!(
        "{}:{:02}:{:02}.{:03}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60,
        ms % 1000
    )
}

pub(super) fn describe(note: &Bookmark) -> String {
    let recording = match note.recording {
        RecordingTime::Unavailable => fl!(LANGUAGE_LOADER, "bookmark-record-unknown"),
        RecordingTime::Inactive => fl!(LANGUAGE_LOADER, "bookmark-record-inactive"),
        RecordingTime::Observed {
            duration_ms,
            paused,
            age_ms,
        } => fl!(
            LANGUAGE_LOADER,
            "bookmark-record-time",
            time = clock(duration_ms),
            age = format!("{:.1}", age_ms as f64 / 1000.0),
            state = if paused {
                fl!(LANGUAGE_LOADER, "output-state-paused")
            } else {
                fl!(LANGUAGE_LOADER, "output-state-active")
            }
        ),
    };
    let unknown = || fl!(LANGUAGE_LOADER, "media-time-unavailable");
    fl!(
        LANGUAGE_LOADER,
        "bookmark-description",
        created = glib::DateTime::from_unix_utc(
            (note.created_unix_ms / 1000).min(i64::MAX as u64) as i64
        )
        .and_then(|date| date.format("%Y-%m-%d %H:%M:%S UTC"))
        .map(|value| value.to_string())
        .unwrap_or_else(|_| unknown()),
        session = note.session_id.to_string(),
        elapsed = clock(note.session_elapsed_ms),
        connection = note.connection_id.as_str(),
        endpoint = note.endpoint.as_str(),
        collection = note.collection.clone().unwrap_or_else(unknown),
        scene = note.scene.clone().unwrap_or_else(unknown),
        recording = recording
    )
}

pub(crate) fn button(nav: NavigationContext) -> Button {
    let button = Button::new();
    button.set_child(Some(&text(&fl!(LANGUAGE_LOADER, "bookmarks-title"))));
    button.set_widget_name("bookmarks-open");
    button.connect_clicked(move |button| {
        if let Some(parent) = button.root().and_downcast::<gtk4::Window>() {
            show(&parent, &nav);
        }
    });
    button
}

pub(super) fn show(parent: &gtk4::Window, nav: &NavigationContext) {
    for window in gtk4::Window::list_toplevels() {
        if let Ok(window) = window.downcast::<adw::Window>() {
            if window.is_visible()
                && window.widget_name() == "bookmarks-window"
                && window.transient_for().as_ref() == Some(parent)
            {
                window.present();
                return;
            }
        }
    }
    let window = adw::Window::builder()
        .transient_for(parent)
        .destroy_with_parent(true)
        .modal(true)
        .default_width(640)
        .default_height(620)
        .title(fl!(LANGUAGE_LOADER, "bookmarks-title"))
        .build();
    window.set_widget_name("bookmarks-window");
    let root = GtkBox::new(Orientation::Vertical, 8);
    root.append(&adw::HeaderBar::new());
    let body = GtkBox::new(Orientation::Vertical, 8);
    body.set_margin_start(12);
    body.set_margin_end(12);
    body.set_margin_bottom(12);
    body.set_vexpand(true);
    body.append(&text(&fl!(LANGUAGE_LOADER, "bookmark-policy")));
    let name = Entry::builder()
        .placeholder_text(fl!(LANGUAGE_LOADER, "bookmark-name"))
        .max_length(NAME_LIMIT as i32)
        .hexpand(true)
        .width_chars(1)
        .build();
    name.set_widget_name("bookmark-name");
    name.update_property(&[gtk4::accessible::Property::Label(&fl!(
        LANGUAGE_LOADER,
        "bookmark-name"
    ))]);
    body.append(&name);
    let actions = GtkBox::new(Orientation::Horizontal, 6);
    let add = labeled_button(&fl!(LANGUAGE_LOADER, "bookmark-add"));
    add.set_widget_name("bookmark-add");
    let retry = labeled_button(&fl!(LANGUAGE_LOADER, "bookmark-retry"));
    retry.set_widget_name("bookmark-retry");
    actions.append(&add);
    actions.append(&retry);
    body.append(&actions);
    let status = text("");
    status.set_widget_name("bookmark-status");
    body.append(&status);
    let notice = text("");
    notice.set_widget_name("bookmark-notice");
    body.append(&notice);
    let rows = GtkBox::new(Orientation::Vertical, 12);
    rows.set_widget_name("bookmark-rows");
    body.append(&rows);
    let scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .child(&body)
        .build();
    root.append(&scroll);
    window.set_content(Some(&root));
    let scoped = nav.scoped();
    let displayed = Rc::new(RefCell::new(Vec::<Bookmark>::new()));
    let render: Rc<dyn Fn()> = Rc::new({
        let nav = nav.clone();
        let scoped = scoped.clone();
        let window = window.downgrade();
        let rows = rows.downgrade();
        let add = add.downgrade();
        let retry = retry.downgrade();
        let status = status.downgrade();
        move || {
            let (Some(window), Some(rows), Some(add), Some(retry), Some(status)) = (
                window.upgrade(),
                rows.upgrade(),
                add.upgrade(),
                retry.upgrade(),
                status.upgrade(),
            ) else {
                return;
            };
            let state = nav.state.borrow();
            let notes = &state.bookmarks;
            add.set_sensitive(
                scoped.is_current()
                    && matches!(state.obs_status, ObsStatus::Connected { .. })
                    && notes.load_error.is_none(),
            );
            retry.set_sensitive(!notes.loading && notes.pending == 0);
            retry.set_visible(notes.load_error.is_some() || notes.save_error.is_some());
            status.set_text(&if notes.loading || notes.pending > 0 {
                fl!(LANGUAGE_LOADER, "bookmark-saving")
            } else if let Some(error) = notes.load_error.as_ref().or(notes.save_error.as_ref()) {
                fl!(
                    LANGUAGE_LOADER,
                    "bookmark-save-error",
                    error = error.as_str()
                )
            } else if !scoped.is_current() {
                fl!(LANGUAGE_LOADER, "bookmark-scope-changed")
            } else {
                fl!(
                    LANGUAGE_LOADER,
                    "bookmark-saved",
                    count = notes.archive.entries.len().to_string()
                )
            });
            let entries = notes.archive.entries.clone();
            drop(state);
            if *displayed.borrow() == entries {
                return;
            }
            *displayed.borrow_mut() = entries.clone();
            super::rebuild_page(&rows, |rows| {
                for note in entries.iter().rev() {
                    rows.append(&row(&nav, note));
                }
            });
            let _ = window;
        }
    });
    add.connect_clicked({
        let nav = nav.clone();
        let name = name.clone();
        let notice = notice.clone();
        let window = window.downgrade();
        let render = render.clone();
        move |_| {
            if window.upgrade().is_none_or(|window| !window.is_visible()) || !scoped.is_current() {
                return;
            }
            let title = if name.text().trim().is_empty() {
                fl!(
                    LANGUAGE_LOADER,
                    "bookmark-default",
                    number = nav.state.borrow().bookmarks.archive.next_id.to_string()
                )
            } else {
                name.text().to_string()
            };
            let result = nav.state.borrow_mut().capture_bookmark(
                &title,
                Instant::now(),
                crate::controller::workspace::unix_ms(),
            );
            match result {
                Ok(_) => {
                    save(&nav);
                    name.set_text("");
                    notice.set_text("");
                }
                Err(error) => notice.set_text(&error_label(error)),
            }
            render();
        }
    });
    name.connect_activate({
        let add = add.clone();
        move |_| {
            if add.is_sensitive() {
                add.emit_clicked();
            }
        }
    });
    retry.connect_clicked({
        let nav = nav.clone();
        let window = window.downgrade();
        let render = render.clone();
        move |_| {
            if window.upgrade().is_none_or(|window| !window.is_visible()) {
                return;
            }
            let load = nav.state.borrow().bookmarks.load_error.is_some();
            if load {
                if nav.state.borrow().bookmarks.loading {
                    return;
                }
                nav.state.borrow_mut().bookmarks.loading = true;
                let nav = nav.clone();
                let path = crate::storage::xdg::bookmarks_path();
                background_io::run_serialized(
                    SerialLane::Bookmarks,
                    move || {
                        crate::storage::bookmarks::read(&path).map_err(|error| error.to_string())
                    },
                    move |result| nav.state.borrow_mut().load_bookmarks(result),
                );
            } else {
                save(&nav);
            }
            render();
        }
    });
    render();
    window.present();
    glib::timeout_add_local(Duration::from_millis(200), {
        let window = window.downgrade();
        move || {
            if window.upgrade().is_none_or(|window| !window.is_visible()) {
                return glib::ControlFlow::Break;
            }
            render();
            glib::ControlFlow::Continue
        }
    });
}

fn error_label(error: BookmarkError) -> String {
    match error {
        BookmarkError::Unavailable => fl!(LANGUAGE_LOADER, "bookmark-scope-changed"),
        BookmarkError::InvalidName => fl!(LANGUAGE_LOADER, "bookmark-invalid-name"),
        BookmarkError::Full => fl!(LANGUAGE_LOADER, "bookmark-full"),
    }
}

fn row(nav: &NavigationContext, note: &Bookmark) -> GtkBox {
    let root = GtkBox::new(Orientation::Vertical, 4);
    root.set_widget_name(&format!("bookmark-{}", note.id));
    let name = Entry::builder()
        .text(&note.name)
        .max_length(NAME_LIMIT as i32)
        .width_chars(1)
        .hexpand(true)
        .build();
    name.update_property(&[gtk4::accessible::Property::Label(&fl!(
        LANGUAGE_LOADER,
        "bookmark-name"
    ))]);
    root.append(&name);
    root.append(&text(&describe(note)));
    let actions = GtkBox::new(Orientation::Horizontal, 6);
    let rename = labeled_button(&fl!(LANGUAGE_LOADER, "bookmark-rename"));
    let remove = labeled_button(&fl!(LANGUAGE_LOADER, "bookmark-remove"));
    let error = text("");
    let id = note.id;
    name.set_widget_name(&format!("bookmark-name-{id}"));
    rename.set_widget_name(&format!("bookmark-rename-{id}"));
    remove.set_widget_name(&format!("bookmark-remove-{id}"));
    rename.connect_clicked({
        let nav = nav.clone();
        let error = error.clone();
        let root = root.downgrade();
        move |_| {
            if root.upgrade().is_none_or(|root| {
                root.root()
                    .and_downcast::<gtk4::Window>()
                    .is_none_or(|window| !window.is_visible())
            }) {
                return;
            }
            if nav.state.borrow_mut().bookmarks.rename(id, &name.text()) {
                save(&nav);
                error.set_text("");
            } else {
                error.set_text(&fl!(LANGUAGE_LOADER, "bookmark-invalid-name"));
            }
        }
    });
    remove.connect_clicked({
        let nav = nav.clone();
        let root = root.downgrade();
        move |_| {
            if root.upgrade().is_none_or(|root| {
                root.root()
                    .and_downcast::<gtk4::Window>()
                    .is_none_or(|window| !window.is_visible())
            }) {
                return;
            }
            if nav.state.borrow_mut().bookmarks.remove(id) {
                save(&nav);
            }
        }
    });
    actions.append(&rename);
    actions.append(&remove);
    root.append(&actions);
    root.append(&error);
    root
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{
        preflight::tests::{find, window},
        test_support::{native_test, navigation, pump, pump_until},
    };
    use crate::{
        controller::event::{AppEvent, ConnectionInfo},
        domain::output::{OutputKind, OutputRunState, OutputStatus},
    };

    fn connect(nav: &NavigationContext) {
        let mut state = nav.state.borrow_mut();
        state.apply_connected(ConnectionInfo {
            connection_id: "podcast".into(),
            endpoint: "fixture:4455".into(),
            obs_version: "fixture".into(),
            websocket_version: "5".into(),
            available_requests: Vec::new(),
            supported_image_formats: Vec::new(),
        });
        state.scene_collections.current = Some("Interview".into());
        state.scene_inventory.current_id = Some("Camera".into());
        state.apply_output_event(
            &AppEvent::OutputStatusUpdated(
                OutputKind::Record,
                OutputStatus {
                    active: true,
                    state: OutputRunState::Paused,
                    detail: None,
                    duration_ms: Some(42_000),
                },
                Instant::now(),
            ),
            crate::controller::workspace::unix_ms(),
        );
    }

    fn finish_save(nav: &NavigationContext) {
        assert!(pump_until(Duration::from_secs(4), || nav
            .state
            .borrow()
            .bookmarks
            .pending
            == 0));
        pump(Duration::from_millis(250));
    }

    #[test]
    #[ignore = "requires isolated GTK and temporary XDG files"]
    fn native_bookmarks_capture_rename_retry_reload_and_retire_closed_callbacks() {
        native_test("ui::bookmarks::tests::native_bookmarks_capture_rename_retry_reload_and_retire_closed_callbacks", || {
            let (nav, stack, _runtime, events) = navigation(); connect(&nav);
            let parent = adw::Window::builder().default_width(640).default_height(620).build();
            parent.set_content(Some(&stack)); parent.present();
            assert!(super::super::action_catalog::execute(&nav, &super::super::action_catalog::CatalogAction::Bookmarks));
            let dialog = window("bookmarks-window"); pump(Duration::from_millis(100));
            let name = find(&dialog, "bookmark-name").downcast::<Entry>().unwrap();
            let add = find(&dialog, "bookmark-add").downcast::<Button>().unwrap();
            name.set_text("Topic one"); add.emit_clicked(); finish_save(&nav);
            let path = crate::storage::xdg::bookmarks_path();
            let stored = crate::storage::bookmarks::read(&path).unwrap();
            assert_eq!(stored.entries.len(), 1);
            assert_eq!(stored.entries[0].name, "Topic one");
            assert!(matches!(stored.entries[0].recording, RecordingTime::Observed { duration_ms: 42_000, paused: true, .. }));
            assert_eq!(stored.entries[0].connection_id, "podcast");
            assert_eq!(stored.entries[0].collection.as_deref(), Some("Interview"));
            assert!(events.try_iter().next().is_none(), "local bookmarks must not mutate OBS");
            let edit = find(&dialog, "bookmark-name-1").downcast::<Entry>().unwrap();
            edit.set_text("Rename draft"); edit.grab_focus();
            let focus = gtk4::prelude::RootExt::focus(&dialog);
            nav.state.borrow_mut().record_status.duration_ms = Some(43_000);
            pump(Duration::from_millis(250));
            assert_eq!(find(&dialog, "bookmark-name-1"), edit.clone().upcast::<gtk4::Widget>());
            assert_eq!(edit.text(), "Rename draft");
            assert_eq!(gtk4::prelude::RootExt::focus(&dialog), focus);
            find(&dialog, "bookmark-rename-1").downcast::<Button>().unwrap().emit_clicked(); finish_save(&nav);
            assert_eq!(crate::storage::bookmarks::read(&path).unwrap().entries[0].name, "Rename draft");

            let backup = path.with_extension("backup"); std::fs::rename(&path, &backup).unwrap(); std::fs::create_dir(&path).unwrap();
            name.set_text("Keep after failed save"); add.emit_clicked(); finish_save(&nav);
            assert!(nav.state.borrow().bookmarks.save_error.is_some());
            assert!(!FAILURES.with(|failures| failures.borrow().is_empty()));
            assert!(find(&dialog, "bookmark-retry").is_visible());
            std::fs::remove_dir(&path).unwrap(); std::fs::rename(&backup, &path).unwrap();
            find(&dialog, "bookmark-retry").downcast::<Button>().unwrap().emit_clicked(); finish_save(&nav);
            assert!(nav.state.borrow().bookmarks.save_error.is_none());
            assert!(FAILURES.with(|failures| failures.borrow().is_empty()));
            assert_eq!(crate::storage::bookmarks::read(&path).unwrap().entries.len(), 2);

            nav.state.borrow_mut().invalidate_collection_runtime();
            nav.state.borrow_mut().scene_collections.current = Some("Other".into());
            pump(Duration::from_millis(250)); assert!(!add.is_sensitive());
            let count = nav.state.borrow().bookmarks.archive.entries.len(); add.emit_clicked();
            assert_eq!(nav.state.borrow().bookmarks.archive.entries.len(), count);
            let remove = find(&dialog, "bookmark-remove-1").downcast::<Button>().unwrap();
            dialog.close(); add.emit_clicked(); remove.emit_clicked(); finish_save(&nav);
            assert_eq!(nav.state.borrow().bookmarks.archive.entries.len(), count);

            nav.state.borrow_mut().reset_obs_session(ObsStatus::Disconnected);
            let stored = crate::storage::bookmarks::read(&path).unwrap();
            let fresh = crate::controller::state::AppState::new(Default::default(), Default::default(), None, None);
            *nav.state.borrow_mut() = fresh; nav.state.borrow_mut().load_bookmarks(Ok(stored));
            show(parent.upcast_ref(), &nav); let reopened = window("bookmarks-window"); pump(Duration::from_millis(100));
            assert!(!find(&reopened, "bookmark-add").is_sensitive());
            assert_eq!(find(&reopened, "bookmark-name-2").downcast::<Entry>().unwrap().text(), "Keep after failed save");
            find(&reopened, "bookmark-remove-1").downcast::<Button>().unwrap().emit_clicked(); finish_save(&nav);
            assert_eq!(crate::storage::bookmarks::read(&path).unwrap().entries[0].id, 2);
            reopened.close(); parent.close();
        });
    }

    #[test]
    #[ignore = "requires isolated GTK and temporary XDG files"]
    fn native_bookmarks_load_failure_retry_and_large_text_layout() {
        native_test(
            "ui::bookmarks::tests::native_bookmarks_load_failure_retry_and_large_text_layout",
            || {
                crate::infra::i18n::init(crate::domain::appearance::Language::Uk);
                gtk4::Settings::default()
                    .unwrap()
                    .set_gtk_font_name(Some("Sans 22"));
                let (nav, stack, _runtime, _events) = navigation();
                let path = crate::storage::xdg::bookmarks_path();
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(&path, "{broken").unwrap();
                nav.state.borrow_mut().load_bookmarks(
                    crate::storage::bookmarks::read(&path).map_err(|error| error.to_string()),
                );
                connect(&nav);
                let parent = adw::Window::builder()
                    .default_width(360)
                    .default_height(600)
                    .build();
                parent.set_content(Some(&stack));
                parent.present();
                show(parent.upcast_ref(), &nav);
                let dialog = window("bookmarks-window");
                dialog.set_default_size(360, 600);
                pump(Duration::from_millis(150));
                assert!(!find(&dialog, "bookmark-add").is_sensitive());
                find(&dialog, "bookmark-add")
                    .downcast::<Button>()
                    .unwrap()
                    .emit_clicked();
                assert_eq!(std::fs::read_to_string(&path).unwrap(), "{broken");
                std::fs::remove_file(&path).unwrap();
                find(&dialog, "bookmark-retry")
                    .downcast::<Button>()
                    .unwrap()
                    .emit_clicked();
                assert!(pump_until(Duration::from_secs(4), || !nav
                    .state
                    .borrow()
                    .bookmarks
                    .loading));
                pump(Duration::from_millis(250));
                assert!(find(&dialog, "bookmark-add").is_sensitive());
                find(&dialog, "bookmark-name")
                    .downcast::<Entry>()
                    .unwrap()
                    .set_text("Довга назва розділу подкасту для перевірки локальних закладок");
                find(&dialog, "bookmark-add")
                    .downcast::<Button>()
                    .unwrap()
                    .emit_clicked();
                finish_save(&nav);
                assert!(nav.state.borrow().bookmarks.load_error.is_none());
                assert_eq!(
                    crate::storage::bookmarks::read(&path)
                        .unwrap()
                        .entries
                        .len(),
                    1
                );
                assert!(
                    dialog.measure(Orientation::Horizontal, -1).0 <= 360,
                    "bookmarks must fit a narrow window"
                );
                assert!(
                    dialog.measure(Orientation::Vertical, 360).0 <= 600,
                    "bookmark controls must remain reachable at large text"
                );
                dialog.close();
                parent.close();
            },
        );
    }
}
