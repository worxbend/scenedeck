//! Primary-scene card for the Live page.

use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Label, Orientation};
use i18n_embed_fl::fl;

use crate::controller::command::AppCommand;
use crate::domain::role::SceneRole;
use crate::domain::scene::SceneId;
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::ui::navigation::NavigationContext;

/// Build a scene-switch card.
///
/// The returned widget is still a `Button` for keyboard navigation and click
/// handling, but it is visually composed as a card.
pub(crate) fn build(
    scene_name: &str,
    scene_id: SceneId,
    scene_role: SceneRole,
    is_active: bool,
    is_previous: bool,
    nav: NavigationContext,
) -> Button {
    let presentation = SceneCardPresentation::for_state(is_active, is_previous);

    let card = Button::builder()
        .halign(Align::Start)
        .hexpand(false)
        .width_request(132)
        .build();
    card.add_css_class("card");
    card.add_css_class("scene-card");
    card.set_tooltip_text(Some(&fl!(
        LANGUAGE_LOADER,
        "scene-card-tooltip",
        status = presentation.tooltip,
        role = scene_role_subtitle(scene_role)
    )));

    let content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
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
        .label(presentation.status_label)
        .halign(Align::Start)
        .build();
    status.add_css_class(presentation.status_css_class);

    let spacer = GtkBox::builder().hexpand(true).build();

    let marker = Label::builder()
        .label(presentation.marker_label)
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
        .lines(1)
        .ellipsize(gtk4::pango::EllipsizeMode::End)
        .build();
    title.add_css_class("heading");
    title.add_css_class("scene-card-title");

    content.append(&header);
    content.append(&title);
    card.set_child(Some(&content));

    if let Some(class) = presentation.card_css_class {
        card.add_css_class(class);
    }

    card.connect_clicked(move |_| {
        nav.dispatch(AppCommand::SwitchPrimaryScene(scene_id.clone()));
    });

    card
}

fn scene_role_subtitle(role: SceneRole) -> String {
    fl!(
        LANGUAGE_LOADER,
        "scene-card-role-suffix",
        role = role.label()
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SceneCardPresentation {
    tooltip: &'static str,
    status_label: &'static str,
    status_css_class: &'static str,
    marker_label: &'static str,
    card_css_class: Option<&'static str>,
}

impl SceneCardPresentation {
    const fn for_state(active: bool, previous: bool) -> Self {
        if active {
            Self {
                tooltip: "Current program scene",
                status_label: "Live",
                status_css_class: "scene-card-status-live",
                marker_label: "On",
                card_css_class: Some("scene-card-active"),
            }
        } else if previous {
            Self {
                tooltip: "Previously live scene",
                status_label: "Prev",
                status_css_class: "scene-card-status-previous",
                marker_label: "Last",
                card_css_class: Some("scene-card-previous"),
            }
        } else {
            Self {
                tooltip: "Switch to this scene",
                status_label: "Ready",
                status_css_class: "scene-card-status-ready",
                marker_label: "",
                card_css_class: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_card_presentation_marks_active_scene_as_live() {
        assert_eq!(
            SceneCardPresentation::for_state(true, false),
            SceneCardPresentation {
                tooltip: "Current program scene",
                status_label: "Live",
                status_css_class: "scene-card-status-live",
                marker_label: "On",
                card_css_class: Some("scene-card-active")
            }
        );
    }

    #[test]
    fn scene_card_presentation_marks_previous_scene_as_previous() {
        assert_eq!(
            SceneCardPresentation::for_state(false, true),
            SceneCardPresentation {
                tooltip: "Previously live scene",
                status_label: "Prev",
                status_css_class: "scene-card-status-previous",
                marker_label: "Last",
                card_css_class: Some("scene-card-previous")
            }
        );
    }

    #[test]
    fn scene_card_presentation_marks_inactive_scene_as_ready() {
        assert_eq!(
            SceneCardPresentation::for_state(false, false),
            SceneCardPresentation {
                tooltip: "Switch to this scene",
                status_label: "Ready",
                status_css_class: "scene-card-status-ready",
                marker_label: "",
                card_css_class: None
            }
        );
    }

    #[test]
    fn scene_role_subtitle_uses_assigned_role_label() {
        assert_eq!(scene_role_subtitle(SceneRole::Primary), "Primary scene");
        assert_eq!(scene_role_subtitle(SceneRole::Secondary), "Secondary scene");
    }
}
