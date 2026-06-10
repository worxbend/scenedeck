//! Primary-scene card for the Live page.

use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Label, Orientation};

use crate::controller::command::AppCommand;
use crate::domain::scene::SceneId;
use crate::ui::navigation::NavigationContext;

/// Build a scene-switch card.
///
/// The returned widget is still a `Button` for keyboard navigation and click
/// handling, but it is visually composed as a card.
pub(crate) fn build(
    scene_name: &str,
    scene_id: SceneId,
    is_active: bool,
    nav: NavigationContext,
) -> Button {
    let card = Button::builder()
        .halign(Align::Fill)
        .hexpand(true)
        .width_request(188)
        .build();
    card.add_css_class("card");
    card.add_css_class("scene-card");

    let content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .halign(Align::Fill)
        .hexpand(true)
        .build();

    let header = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .halign(Align::Fill)
        .hexpand(true)
        .build();

    let status = Label::builder()
        .label(if is_active { "Live" } else { "Ready" })
        .halign(Align::Start)
        .build();
    status.add_css_class(if is_active {
        "scene-card-status-live"
    } else {
        "scene-card-status-ready"
    });

    let spacer = GtkBox::builder().hexpand(true).build();

    let marker = Label::builder()
        .label(if is_active { "On air" } else { "" })
        .halign(Align::End)
        .build();
    marker.add_css_class("caption");
    marker.add_css_class("dim-label");

    header.append(&status);
    header.append(&spacer);
    header.append(&marker);

    let title = Label::builder()
        .label(scene_name)
        .xalign(0.0)
        .halign(Align::Fill)
        .hexpand(true)
        .wrap(true)
        .lines(2)
        .build();
    title.add_css_class("heading");

    let subtitle = Label::builder()
        .label("Primary scene")
        .xalign(0.0)
        .halign(Align::Fill)
        .build();
    subtitle.add_css_class("caption");
    subtitle.add_css_class("dim-label");

    content.append(&header);
    content.append(&title);
    content.append(&subtitle);
    card.set_child(Some(&content));

    if is_active {
        card.add_css_class("scene-card-active");
    }

    card.connect_clicked(move |_| {
        nav.dispatch(AppCommand::SwitchPrimaryScene(scene_id.clone()));
    });

    card
}
