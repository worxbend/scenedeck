//! GTK / libadwaita UI modules.
//!
//! `ui::window` builds the top-level window.  Pages live under `ui::pages`.
//! Reusable leaf widgets live under `ui::widgets`.  `ui::navigation` owns the
//! `NavigationContext` type shared by all pages.

pub(crate) mod actions;
pub(crate) mod background_io;
mod event_pump;
mod event_reconciliation;
mod hotkeys;
pub(crate) mod navigation;
pub(crate) mod pages;
pub(crate) mod persist;
mod sidebar;
pub(crate) mod theme;
pub(crate) mod widgets;
pub(crate) mod window;
pub(crate) mod workspace;

pub use window::build_main_window;

/// Build a `gtk4::StringList` model from owned label strings.
///
/// Dropdown labels come out of the translation loader as `String`s, but
/// `StringList` wants `&[&str]`, so every combo row was borrowing the whole
/// vector into a second one just to hand it over. This does that step once.
pub(crate) fn string_list(labels: &[String]) -> gtk4::StringList {
    let borrowed: Vec<&str> = labels.iter().map(String::as_str).collect();
    gtk4::StringList::new(&borrowed)
}

/// Build a page whose descendants change only when its presentation changes.
/// The key must contain displayed content and callback scope, not event counters
/// or unrelated app state. Revisiting an unchanged page retains focus, scroll,
/// expanded sections and, for Graph, user-arranged node positions.
///
/// Returns the widget to add to the navigation stack and the refresh callback.
///
/// `hexpand` is deliberately a conditional builder step rather than
/// `.hexpand(hexpand)`. Setting the property to `false` is not the same as
/// leaving it unset: it also sets GTK's `hexpand-set` flag, which stops the
/// box inheriting horizontal expansion from its children. The pages that pass
/// `false` here depend on that inheritance.
pub(crate) fn stateful_page<K: PartialEq + 'static>(
    css_class: &str,
    hexpand: bool,
    presentation: impl Fn() -> K + 'static,
    populate: impl Fn(&gtk4::Box) + 'static,
) -> (gtk4::Widget, std::rc::Rc<dyn Fn()>) {
    use gtk4::prelude::*;

    let builder = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .vexpand(true);
    let builder = if hexpand {
        builder.hexpand(true)
    } else {
        builder
    };
    let container = builder.build();
    container.add_css_class("app-page");
    container.add_css_class(css_class);

    let rendered = std::cell::RefCell::new(presentation());
    populate(&container);

    let refresh: std::rc::Rc<dyn Fn()> = std::rc::Rc::new({
        let container = container.downgrade();
        move || {
            let Some(container) = container.upgrade() else {
                return;
            };
            let next = presentation();
            if *rendered.borrow() == next {
                return;
            }
            rebuild_page(&container, |container| populate(container));
            *rendered.borrow_mut() = next;
        }
    });

    container.connect_map({
        let refresh = refresh.clone();
        let weak = container.downgrade();
        move |_| {
            let refresh = refresh.clone();
            let weak = weak.clone();
            // Finish GTK's map/focus traversal before replacing descendants.
            glib::idle_add_local_once(move || {
                if weak.upgrade().is_some_and(|page| page.is_mapped()) {
                    refresh();
                }
            });
        }
    });

    (container.upcast(), refresh)
}

/// Replace a page without leaving GTK's focus chain on detached descendants.
pub(crate) fn rebuild_page(container: &gtk4::Box, populate: impl FnOnce(&gtk4::Box)) {
    use gtk4::prelude::*;
    let root = container.root();
    let old_focus = root.as_ref().and_then(|root| root.focus());
    let owns_focus = old_focus
        .as_ref()
        .is_some_and(|focus| focus.is_ancestor(container));
    let mut named_focus = None;
    if owns_focus {
        let mut focus = old_focus.clone();
        while let Some(widget) = focus {
            if widget == *container {
                break;
            }
            if widget.widget_name().as_str() != widget.type_().name() {
                named_focus = Some(widget.widget_name().to_string());
                break;
            }
            focus = widget.parent();
        }
        if let Some(root) = &root {
            root.set_focus(gtk4::Widget::NONE);
        }
    }
    clear_children(container);
    populate(container);
    if let Some(name) = named_focus {
        let container = container.downgrade();
        glib::idle_add_local_once(move || {
            // Keep the former focused child alive until unparenting completes.
            let _old_focus = old_focus;
            let Some(container) = container.upgrade() else {
                return;
            };
            if !container.is_mapped() {
                return;
            }
            if container
                .root()
                .and_then(|root| root.focus())
                .is_some_and(|focus| !focus.is_ancestor(&container))
            {
                return;
            }
            fn restore(widget: &gtk4::Widget, name: &str) -> bool {
                if widget.widget_name() == name && widget.is_sensitive() && widget.is_visible() {
                    return widget.grab_focus();
                }
                let mut child = widget.first_child();
                while let Some(current) = child {
                    if restore(&current, name) {
                        return true;
                    }
                    child = current.next_sibling();
                }
                false
            }
            restore(container.upcast_ref(), &name);
        });
    }
}

/// Remove every child of `container`.
///
/// `gtk4::Box` and `gtk4::FlowBox` need separate helpers: `Box::remove` comes
/// from a trait while `FlowBox::remove` is inherent, so no single generic
/// bound covers both, and unparenting a `FlowBox` child directly would skip
/// the bookkeeping `FlowBox` does for its own children.
pub(crate) fn clear_children(container: &gtk4::Box) {
    use gtk4::prelude::*;

    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

/// Insert `widget` into `flow` wrapped in a child sized to its contents.
///
/// A `FlowBox` stretches its children to fill the row it puts them on, which
/// turns a row of scene cards into a row of stretched rectangles. Pinning the
/// wrapper's alignment to the start and switching expansion off keeps each
/// card at its natural size. The Live and Mixer pages each carried their own
/// identical copy of this.
pub(crate) fn insert_compact_flow_child<W: gtk4::glib::object::IsA<gtk4::Widget>>(
    flow: &gtk4::FlowBox,
    widget: &W,
) {
    use gtk4::prelude::*;

    let child = gtk4::FlowBoxChild::new();
    child.set_halign(gtk4::Align::Start);
    child.set_valign(gtk4::Align::Start);
    child.set_hexpand(false);
    child.set_vexpand(false);
    child.set_child(Some(widget));
    flow.insert(&child, -1);
}

/// Position of `value` within `all`, for seeding a dropdown from a domain enum.
///
/// Falls back to 0 rather than failing: a value missing from the list means
/// the list is wrong, and showing the first entry is better than showing none.
pub(crate) fn index_of<T: PartialEq>(all: &[T], value: T) -> u32 {
    all.iter().position(|item| *item == value).unwrap_or(0) as u32
}

const ICON_RESOURCE_PATH: &str = "/io/scenedeck/app/icons";

pub fn register_resources() {
    gio::resources_register_include!("scenedeck.gresource")
        .expect("SceneDeck resources should be compiled into the binary");

    // Scene and audio-source icons come from the Nerd Fonts symbol set, which
    // ships as its own embedded GResource. A failure here costs the icons, not
    // the app, so it is logged rather than fatal.
    if let Err(error) = nerd_gtk_icons::register_icons() {
        tracing::warn!(%error, "failed to register the Nerd Fonts icon resources");
    }

    if let Some(display) = gtk4::gdk::Display::default() {
        let theme = gtk4::IconTheme::for_display(&display);
        theme.add_resource_path(ICON_RESOURCE_PATH);
        theme.add_resource_path(nerd_gtk_icons::ICONS_RESOURCE_PATH);
    }
}

pub(crate) mod previews;

#[cfg(test)]
pub(crate) mod test_support {
    use super::navigation::NavigationContext;
    use crate::controller::{app_controller::AppController, event::AppEvent, state::AppState};
    use std::{cell::RefCell, rc::Rc, sync::mpsc, time::Duration};

    /// GTK tests run in separate processes, not separate Rust test threads.
    pub(crate) fn native_test(name: &str, test: impl FnOnce()) {
        if std::env::var("SCENEDECK_NATIVE_TEST_CHILD").as_deref() != Ok(name) {
            let mut child = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", name, "--ignored", "--test-threads=1"])
                .env("SCENEDECK_NATIVE_TEST_CHILD", name)
                .env("GTK_A11Y", "test")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap();
            let deadline = std::time::Instant::now() + Duration::from_secs(60);
            while child.try_wait().unwrap().is_none() {
                if std::time::Instant::now() >= deadline {
                    child.kill().unwrap();
                    let output = child.wait_with_output().unwrap();
                    panic!(
                        "native child timed out: {}\n{}",
                        String::from_utf8_lossy(&output.stdout),
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            let output = child.wait_with_output().unwrap();
            assert!(
                output.status.success(),
                "native child failed: {}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        adw::init().expect("native GTK display");
        gtk4::prelude::ObjectExt::set_property(
            &gtk4::Settings::default().unwrap(),
            "gtk-enable-animations",
            false,
        );
        super::register_resources();
        test();
    }

    pub(crate) fn navigation() -> (
        NavigationContext,
        gtk4::Stack,
        tokio::runtime::Runtime,
        mpsc::Receiver<AppEvent>,
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let (tx, rx) = mpsc::sync_channel(128);
        let controller = Rc::new(RefCell::new(AppController::new(
            runtime.handle().clone(),
            tx,
        )));
        let state = Rc::new(RefCell::new(AppState::new(
            Default::default(),
            Default::default(),
            None,
            None,
        )));
        let stack = gtk4::Stack::new();
        (
            NavigationContext::new(state, stack.clone(), controller),
            stack,
            runtime,
            rx,
        )
    }

    pub(crate) fn pump(duration: Duration) {
        let until = std::time::Instant::now() + duration;
        let context = glib::MainContext::default();
        while std::time::Instant::now() < until {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

#[cfg(test)]
mod tests {
    use gtk4::prelude::*;

    #[test]
    #[ignore = "requires a native GTK display"]
    fn native_rebuildable_page_releases_root_even_with_refresher_alive() {
        super::test_support::native_test(
            "ui::tests::native_rebuildable_page_releases_root_even_with_refresher_alive",
            || {
                let (page, refresh) = super::stateful_page(
                    "fixture",
                    false,
                    || (),
                    |root| {
                        root.append(&gtk4::Button::with_label("fixture"));
                    },
                );
                let weak = page.downgrade();
                refresh();
                drop(page);
                assert!(weak.upgrade().is_none());
                refresh();
            },
        );
    }
}
