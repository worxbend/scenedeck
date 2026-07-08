//! Primary-scene card for the Live page.

use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Label, Orientation};

use crate::controller::command::AppCommand;
use crate::domain::role::SceneRole;
use crate::domain::scene::SceneId;
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
    nav: NavigationContext,
) -> Button {
    let presentation = SceneCardPresentation::for_active(is_active);

    let card = Button::builder()
        .halign(Align::Fill)
        .hexpand(true)
        .width_request(188)
        .build();
    card.add_css_class("card");
    card.add_css_class("scene-card");
    card.set_tooltip_text(Some(presentation.tooltip));

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
        .lines(2)
        .build();
    title.add_css_class("heading");

    let subtitle = Label::builder()
        .label(scene_role_subtitle(scene_role))
        .xalign(0.0)
        .halign(Align::Fill)
        .build();
    subtitle.add_css_class("caption");
    subtitle.add_css_class("dim-label");

    content.append(&header);
    content.append(&title);
    content.append(&subtitle);
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
    format!("{} scene", role.label())
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
    const fn for_active(active: bool) -> Self {
        if active {
            Self {
                tooltip: "Current program scene",
                status_label: "Live",
                status_css_class: "scene-card-status-live",
                marker_label: "On air",
                card_css_class: Some("scene-card-active"),
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
            SceneCardPresentation::for_active(true),
            SceneCardPresentation {
                tooltip: "Current program scene",
                status_label: "Live",
                status_css_class: "scene-card-status-live",
                marker_label: "On air",
                card_css_class: Some("scene-card-active")
            }
        );
    }

    #[test]
    fn scene_card_presentation_marks_inactive_scene_as_ready() {
        assert_eq!(
            SceneCardPresentation::for_active(false),
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
