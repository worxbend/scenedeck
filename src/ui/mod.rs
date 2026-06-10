//! GTK / libadwaita UI modules.
//!
//! `ui::window` builds the top-level window.  Pages live under `ui::pages`.
//! Reusable leaf widgets live under `ui::widgets`.  `ui::navigation` owns the
//! `NavigationContext` type shared by all pages.

pub(crate) mod actions;
pub(crate) mod navigation;
pub(crate) mod pages;
pub(crate) mod widgets;
pub(crate) mod window;

pub use window::build_main_window;

use gtk4::{gdk, CssProvider};

const ICON_RESOURCE_PATH: &str = "/io/scenedeck/app/icons";

pub fn register_resources() {
    gio::resources_register_include!("scenedeck.gresource")
        .expect("SceneDeck resources should be compiled into the binary");

    if let Some(display) = gdk::Display::default() {
        gtk4::IconTheme::for_display(&display).add_resource_path(ICON_RESOURCE_PATH);
    }
}

pub fn load_css() {
    let css = include_str!("../../assets/scenedeck.css");
    let provider = CssProvider::new();
    provider.load_from_data(css);

    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
