//! Application-wide `gio::Action` registrations.
//!
//! Actions can be triggered by keyboard shortcuts, app-menu items, or D-Bus.
//! Keep command-dispatch logic here rather than inline in signal handlers.

use adw::prelude::*;

use crate::app_info::{APP_ID, APP_NAME, APP_VERSION};
use crate::controller::command::AppCommand;
use crate::controller::state::Page;
use crate::ui::navigation::NavigationContext;

pub(super) fn install(
    app: &adw::Application,
    window: &adw::ApplicationWindow,
    nav: NavigationContext,
) {
    // Quit — Ctrl+Q
    let quit = gio::SimpleAction::new("quit", None);
    quit.connect_activate({
        let app = app.clone();
        move |_, _| app.quit()
    });
    app.add_action(&quit);
    app.set_accels_for_action("app.quit", &["<Primary>q"]);

    // About dialog
    let about = gio::SimpleAction::new("about", None);
    about.connect_activate({
        let window = window.clone();
        move |_, _| show_about(&window)
    });
    app.add_action(&about);

    // Reconnect to OBS — Ctrl+R
    let reconnect = gio::SimpleAction::new("reconnect", None);
    reconnect.connect_activate({
        let nav = nav.clone();
        move |_, _| nav.dispatch(AppCommand::Connect)
    });
    app.add_action(&reconnect);
    app.set_accels_for_action("app.reconnect", &["<Primary>r"]);

    // Jump to Settings — Ctrl+comma (the GNOME convention)
    let settings = gio::SimpleAction::new("settings", None);
    settings.connect_activate({
        let nav = nav.clone();
        move |_, _| nav.switch_to_page(Page::Settings)
    });
    app.add_action(&settings);
    app.set_accels_for_action("app.settings", &["<Primary>comma"]);
}

fn show_about(parent: &adw::ApplicationWindow) {
    let about = adw::AboutWindow::builder()
        .application_name(APP_NAME)
        .application_icon(APP_ID)
        .version(APP_VERSION)
        .developer_name("worxbend")
        .license_type(gtk4::License::MitX11)
        .transient_for(parent)
        .build();
    about.add_css_class("scenedeck-about-window");
    about.present();
}
