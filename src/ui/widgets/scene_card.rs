//! Primary-scene card for the Live page.

use gtk4::prelude::*;
use std::cell::RefCell;
use std::collections::HashSet;

use gtk4::{gdk, Align, Box as GtkBox, Button, CssProvider, Label, Orientation};
use i18n_embed_fl::fl;

use crate::controller::command::AppCommand;
use crate::domain::role::SceneRole;
use crate::domain::scene::SceneId;
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::storage::registry::parse_scene_accent;
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
    accent_color: Option<&str>,
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
    if let Some(class) = accent_color.and_then(install_accent_class) {
        card.add_css_class(&class);
    }
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

thread_local! {
    static INSTALLED_ACCENTS: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

fn install_accent_class(value: &str) -> Option<String> {
    let (red, green, blue) = parse_scene_accent(value)?;
    let class = format!("scene-accent-{red:02x}{green:02x}{blue:02x}");
    let is_new = INSTALLED_ACCENTS.with(|installed| installed.borrow_mut().insert(class.clone()));
    if is_new {
        let Some(display) = gdk::Display::default() else {
            return Some(class);
        };
        let provider = CssProvider::new();
        provider.load_from_data(&accent_css(&class, red, green, blue));
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
        );
    }
    Some(class)
}

fn accent_css(class: &str, red: u8, green: u8, blue: u8) -> String {
    format!(
        "button.scene-card.{class} {{ background-image: none; background-color: rgba({red}, {green}, {blue}, 0.5); }}"
    )
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
                status_label: "Active",
                status_css_class: "scene-card-status-active",
                marker_label: "On",
                card_css_class: Some("scene-card-active"),
            }
        } else if previous {
            Self {
                tooltip: "Previously active scene",
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
    fn scene_card_presentation_marks_active_scene_as_active() {
        assert_eq!(
            SceneCardPresentation::for_state(true, false),
            SceneCardPresentation {
                tooltip: "Current program scene",
                status_label: "Active",
                status_css_class: "scene-card-status-active",
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
                tooltip: "Previously active scene",
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

    #[test]
    fn accent_css_always_uses_half_transparency() {
        let css = accent_css("scene-accent-123456", 18, 52, 86);
        assert!(css.contains("rgba(18, 52, 86, 0.5)"));
    }
}
