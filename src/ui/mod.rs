//! GTK / libadwaita UI modules.
//!
//! `ui::window` builds the top-level window.  Pages live under `ui::pages`.
//! Reusable leaf widgets live under `ui::widgets`.  `ui::navigation` owns the
//! `NavigationContext` type shared by all pages.

pub(crate) mod actions;
pub(crate) mod background_io;
pub(crate) mod navigation;
pub(crate) mod pages;
pub(crate) mod persist;
pub(crate) mod theme;
pub(crate) mod widgets;
pub(crate) mod window;

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
