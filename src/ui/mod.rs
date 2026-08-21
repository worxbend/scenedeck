//! GTK / libadwaita UI modules.
//!
//! `ui::window` builds the top-level window.  Pages live under `ui::pages`.
//! Reusable leaf widgets live under `ui::widgets`.  `ui::navigation` owns the
//! `NavigationContext` type shared by all pages.

pub(crate) mod actions;
pub(crate) mod background_io;
pub(crate) mod navigation;
pub(crate) mod pages;
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
