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

/// Keyboard shortcut assigned to a scene card by its position on Live.
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct SceneShortcut {
    /// Digit shown on the card badge.
    pub(crate) badge: String,
    /// Full binding named in the tooltip, e.g. `Ctrl+1`.
    pub(crate) label: String,
}

/// Everything one scene card renders from.
pub(crate) struct SceneCardModel<'a> {
    /// User-visible scene name.
    pub(crate) scene_name: &'a str,
    /// OBS scene id dispatched when the card is activated.
    pub(crate) scene_id: SceneId,
    /// Role assigned in the registry.
    pub(crate) scene_role: SceneRole,
    /// Whether this is the current program scene.
    pub(crate) is_active: bool,
    /// Whether this was the program scene before the current one.
    pub(crate) is_previous: bool,
    /// Registry accent, as an `#RRGGBB` hex string.
    pub(crate) accent_color: Option<&'a str>,
    /// Keyboard shortcut for this card's position, if it has one.
    pub(crate) shortcut: Option<SceneShortcut>,
}

/// Build a scene-switch card.
///
/// The returned widget is still a `Button` for keyboard navigation and click
/// handling, but it is visually composed as a card.
pub(crate) fn build(model: SceneCardModel<'_>, nav: NavigationContext) -> Button {
    let SceneCardModel {
        scene_name,
        scene_id,
        scene_role,
        is_active,
        is_previous,
        accent_color,
        shortcut,
    } = model;
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
    card.set_tooltip_text(Some(&scene_card_tooltip(
        presentation.tooltip,
        scene_role,
        shortcut.as_ref(),
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

    // The badge carries only the digit; the modifier is named once above the
    // scene grid, so a 132 px card stays readable.
    if let Some(shortcut) = shortcut.as_ref() {
        let badge = Label::builder()
            .label(&shortcut.badge)
            .halign(Align::Start)
            .build();
        badge.add_css_class("scene-card-hotkey");
        header.append(&badge);
    }

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

fn scene_card_tooltip(status: &str, role: SceneRole, shortcut: Option<&SceneShortcut>) -> String {
    match shortcut {
        Some(shortcut) => fl!(
            LANGUAGE_LOADER,
            "scene-card-tooltip-with-hotkey",
            status = status,
            role = scene_role_subtitle(role),
            hotkey = shortcut.label.as_str()
        ),
        None => fl!(
            LANGUAGE_LOADER,
            "scene-card-tooltip",
            status = status,
            role = scene_role_subtitle(role)
        ),
    }
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
    fn tooltip_names_the_shortcut_when_one_is_assigned() {
        let shortcut = SceneShortcut {
            badge: "1".to_string(),
            label: "Ctrl+1".to_string(),
        };

        assert_eq!(
            scene_card_tooltip("Switch to this scene", SceneRole::Primary, Some(&shortcut)),
            "Switch to this scene (Primary scene) · Ctrl+1"
        );
        assert_eq!(
            scene_card_tooltip("Switch to this scene", SceneRole::Primary, None),
            "Switch to this scene (Primary scene)"
        );
    }

    #[test]
    fn accent_css_always_uses_half_transparency() {
        let css = accent_css("scene-accent-123456", 18, 52, 86);
        assert!(css.contains("rgba(18, 52, 86, 0.5)"));
    }
}
