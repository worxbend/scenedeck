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
    let app = app.clone();
    let window = window.clone();
    let nav = nav.clone();

    register_simple_action(&app, "quit", Some("<Primary>q"), {
        let app = app.clone();
        move || app.quit()
    });

    register_simple_action(&app, "about", None, {
        let window = window.clone();
        move || show_about(&window)
    });

    register_simple_action(&app, "reconnect", Some("<Primary>r"), {
        let nav = nav.clone();
        move || nav.dispatch(AppCommand::Connect)
    });

    register_simple_action(&app, "settings", Some("<Primary>comma"), {
        let nav = nav.clone();
        move || nav.switch_to_page(Page::Settings)
    });
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

fn register_simple_action(
    app: &adw::Application,
    action_name: &'static str,
    accelerator: Option<&'static str>,
    handler: impl Fn() + 'static,
) {
    let action = gio::SimpleAction::new(action_name, None);
    action.connect_activate(move |_, _| handler());
    app.add_action(&action);

    if let Some(accel) = accelerator {
        let action_name = format!("app.{action_name}");
        app.set_accels_for_action(&action_name, &[accel]);
    }
}
