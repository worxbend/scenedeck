//! Primary-scene card for the Live page.

use gtk4::prelude::*;
#[cfg(test)]
use std::cell::RefCell;

use gtk4::{
    Align, Box as GtkBox, Button, CssProvider, Image, Label, Orientation, Overlay, Picture,
};
use i18n_embed_fl::fl;

use crate::controller::command::AppCommand;
use crate::domain::role::SceneRole;
use crate::domain::scene::SceneId;
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::storage::registry::parse_scene_accent;
use crate::ui::navigation::NavigationContext;
use crate::ui::widgets::icon_picker;

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
    /// Registry accent, as an `#RRGGBB` hex string.
    pub(crate) accent_color: Option<&'a str>,
    /// Keyboard shortcut for this card's position, if it has one.
    pub(crate) shortcut: Option<SceneShortcut>,
    /// Icon key chosen in Inventory, if any.
    pub(crate) icon: Option<&'a str>,
}

/// Build a scene-switch card.
///
/// The returned widget is still a `Button` for keyboard navigation and click
/// handling, but it is visually composed as a card.
pub(crate) struct SceneCardHandle {
    pub(crate) root: Button,
    pub(crate) scene_id: String,
    pub(crate) picture: Picture,
    program: Label,
    preview: Label,
    title: Label,
    role: SceneRole,
    shortcut: Option<SceneShortcut>,
    pending: gtk4::Spinner,
}

impl SceneCardHandle {
    pub(crate) fn fit_image_height(&self, viewport_height: i32) {
        let short = viewport_height < 180;
        if short {
            self.root.add_css_class("scene-card-short");
        } else {
            self.root.remove_css_class("scene-card-short");
        }
        self.title.set_lines(if short { 1 } else { 2 });
        if self.root.has_css_class("scene-card-preview-enabled") && self.root.width() > 0 {
            let height = preview_card_height(self.root.width(), viewport_height);
            if self.root.height_request() != height {
                self.root.set_height_request(height);
            }
        }
    }

    pub(crate) fn update_state(&self, nav: &NavigationContext, active: bool) {
        let state = nav.state.borrow();
        let (studio, preview) =
            studio_card_state(state.workspace.snapshot.as_ref(), &self.scene_id);
        let pending = state.workspace.pending_scene.as_deref() == Some(&self.scene_id);
        self.pending.set_spinning(pending);
        self.pending.set_opacity(if pending { 1.0 } else { 0.0 });
        self.root
            .update_state(&[gtk4::accessible::State::Busy(pending)]);
        self.root
            .set_sensitive(state.workspace.scene_action(&self.scene_id).is_some());
        let hint = scene_action_hint(studio);
        self.root.set_tooltip_text(Some(&format!(
            "{}\n{}",
            self.title.text(),
            scene_card_tooltip(&hint, self.role, self.shortcut.as_ref())
        )));
        self.root
            .update_property(&[gtk4::accessible::Property::Description(&format!(
                "{} · {hint}",
                tally_description(active, preview)
            ))]);
        self.program.set_visible(active);
        self.preview.set_visible(preview);
        for (class, enabled) in [
            ("scene-card-active", active),
            ("scene-card-preview", preview),
        ] {
            if enabled {
                self.root.add_css_class(class);
            } else {
                self.root.remove_css_class(class);
            }
        }
    }
}

pub(crate) fn build(model: SceneCardModel<'_>, nav: NavigationContext) -> SceneCardHandle {
    let SceneCardModel {
        scene_name,
        scene_id,
        scene_role,
        is_active,
        accent_color,
        shortcut,
        icon,
    } = model;
    let (studio, preview, writable) = {
        let state = nav.state.borrow();
        let (studio, preview) = studio_card_state(state.workspace.snapshot.as_ref(), &scene_id);
        let writable = state.workspace.scene_action(&scene_id).is_some();
        (studio, preview, writable)
    };
    let images = nav.state.borrow().config.live.scene_previews;
    let action_hint = scene_action_hint(studio);
    let card = Button::builder()
        .halign(Align::Fill)
        .hexpand(true)
        .width_request(if images { 256 } else { 160 })
        .build();
    card.add_css_class("scene-card");
    card.set_sensitive(writable);
    card.set_overflow(gtk4::Overflow::Hidden);
    if images {
        card.add_css_class("scene-card-preview-enabled");
    }
    card.update_property(&[
        gtk4::accessible::Property::Label(scene_name),
        gtk4::accessible::Property::Description(&format!(
            "{} · {action_hint}",
            tally_description(is_active, preview)
        )),
    ]);
    card.set_tooltip_text(Some(&format!(
        "{scene_name}\n{}",
        scene_card_tooltip(&action_hint, scene_role, shortcut.as_ref())
    )));
    if let Some(class) = accent_color.and_then(|value| install_accent_class(&card, value)) {
        card.add_css_class(&class);
    }
    let overlay = Overlay::new();
    let picture = Picture::builder()
        .can_shrink(true)
        .content_fit(gtk4::ContentFit::Contain)
        .build();
    picture.add_css_class("scene-card-preview-image");
    let matte = GtkBox::new(Orientation::Vertical, 0);
    overlay.set_child(Some(&matte));
    overlay.add_overlay(&picture);
    let fallback = Image::from_icon_name("image-x-generic-symbolic");
    fallback.set_pixel_size(24);
    fallback.set_halign(Align::Center);
    fallback.set_valign(Align::Center);
    fallback.add_css_class("scene-card-image-placeholder");
    fallback.set_tooltip_text(Some(&fl!(LANGUAGE_LOADER, "scene-card-image-pending")));
    fallback.set_visible(images);
    overlay.add_overlay(&fallback);
    picture.connect_paintable_notify(move |picture| {
        fallback.set_visible(images && picture.paintable().is_none())
    });
    let scrim = GtkBox::new(Orientation::Vertical, 0);
    scrim.add_css_class("scene-card-scrim");
    scrim.set_visible(images);
    overlay.add_overlay(&scrim);
    let content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .build();
    content.add_css_class("scene-card-content");
    let header = GtkBox::new(Orientation::Horizontal, 6);
    let pending = gtk4::Spinner::new();
    pending.set_size_request(16, 16);
    pending.set_opacity(0.0);
    header.append(&pending);
    if let Some(shortcut) = shortcut.as_ref() {
        let badge = Label::new(Some(&shortcut.badge));
        badge.add_css_class("scene-card-hotkey");
        header.append(&badge);
    }
    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    header.append(&spacer);
    let program = Label::new(Some(&fl!(LANGUAGE_LOADER, "scene-card-program-label")));
    program.add_css_class("scene-card-status-active");
    program.set_visible(is_active);
    header.append(&program);
    let preview_label = Label::new(Some(&fl!(LANGUAGE_LOADER, "scene-card-preview-label")));
    preview_label.add_css_class("scene-card-status-preview");
    preview_label.set_visible(preview);
    header.append(&preview_label);
    content.append(&header);
    let spacer = GtkBox::new(Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    content.append(&spacer);
    let title_row = GtkBox::new(Orientation::Horizontal, 6);
    if accent_color.is_some() {
        let chip = GtkBox::new(Orientation::Horizontal, 0);
        chip.add_css_class("scene-card-accent");
        title_row.append(&chip);
    }
    if let Some(icon) = icon_picker::header_icon(icon) {
        title_row.append(&icon);
    }
    let title = Label::builder()
        .label(scene_name)
        .xalign(0.0)
        .hexpand(true)
        .wrap(true)
        .wrap_mode(gtk4::pango::WrapMode::WordChar)
        .max_width_chars(if images { 24 } else { 16 })
        .lines(2)
        .ellipsize(gtk4::pango::EllipsizeMode::End)
        .build();
    title.add_css_class("scene-card-title");
    title_row.append(&title);
    content.append(&title_row);
    overlay.add_overlay(&content);
    overlay.set_measure_overlay(&content, true);
    card.set_child(Some(&overlay));
    if is_active {
        card.add_css_class("scene-card-active");
    }
    if preview {
        card.add_css_class("scene-card-preview");
    }
    let id = scene_id.clone();
    let nav = nav.scoped();
    card.connect_clicked(move |_| nav.dispatch(AppCommand::SwitchPrimaryScene(id.clone())));
    SceneCardHandle {
        root: card,
        scene_id,
        picture,
        program,
        preview: preview_label,
        title,
        role: scene_role,
        shortcut,
        pending,
    }
}

fn scene_action_hint(studio: bool) -> String {
    if studio {
        fl!(LANGUAGE_LOADER, "scene-card-stage-hint")
    } else {
        fl!(LANGUAGE_LOADER, "scene-card-tooltip-ready")
    }
}

fn preview_card_height(width: i32, viewport_height: i32) -> i32 {
    // Keep the title in view in short windows; Picture contains the image on its matte.
    let natural = (((width - 4).max(1) as f64 * 9.0 / 16.0).round() as i32 + 4).max(148);
    let minimum = if viewport_height < 180 { 104 } else { 148 };
    natural.min((viewport_height - 12).max(minimum))
}

fn tally_description(program: bool, preview: bool) -> String {
    let mut states = Vec::new();
    if program {
        states.push(fl!(LANGUAGE_LOADER, "scene-card-program-label"));
    }
    if preview {
        states.push(fl!(LANGUAGE_LOADER, "scene-card-preview-label"));
    }
    states.join(" · ")
}

fn studio_card_state(
    snapshot: Option<&crate::domain::control::ControlSnapshot>,
    scene: &str,
) -> (bool, bool) {
    let studio = snapshot.is_some_and(|snapshot| snapshot.studio.enabled == Some(true));
    let preview =
        studio && snapshot.and_then(|snapshot| snapshot.studio.preview.as_deref()) == Some(scene);
    (studio, preview)
}

#[cfg(test)]
thread_local! {
    static TEST_ACCENT_PROVIDERS: RefCell<Vec<glib::WeakRef<CssProvider>>> = const { RefCell::new(Vec::new()) };
}

fn install_accent_class(card: &Button, value: &str) -> Option<String> {
    let (red, green, blue) = parse_scene_accent(value)?;
    let class = format!("scene-accent-{red:02x}{green:02x}{blue:02x}");
    let display = card.display();
    let provider = CssProvider::new();
    #[cfg(test)]
    TEST_ACCENT_PROVIDERS.with(|providers| providers.borrow_mut().push(provider.downgrade()));
    provider.load_from_data(&accent_css(&class, red, green, blue));
    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
    );
    // Follow the actual widget, not its handle: GTK may still own a visible card.
    card.add_weak_ref_notify_local(move || {
        gtk4::style_context_remove_provider_for_display(&display, &provider);
    });
    Some(class)
}

fn accent_css(class: &str, red: u8, green: u8, blue: u8) -> String {
    format!(
        "button.scene-card.{class} .scene-card-accent {{ background-color: rgb({red}, {green}, {blue}); }}"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a native GTK display"]
    fn native_replaced_scene_accents_release_display_providers() {
        crate::ui::test_support::native_test(
            "ui::widgets::scene_card::tests::native_replaced_scene_accents_release_display_providers",
            || {
                let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
                TEST_ACCENT_PROVIDERS.with(|providers| providers.borrow_mut().clear());
                for index in 0..128 {
                    let color = format!("#{index:06x}");
                    let card = build(
                        SceneCardModel {
                            scene_name: "Main",
                            scene_id: "Main".into(),
                            scene_role: SceneRole::Primary,
                            is_active: false,
                            accent_color: Some(&color),
                            shortcut: None,
                            icon: None,
                        },
                        nav.clone(),
                    );
                    let provider = TEST_ACCENT_PROVIDERS
                        .with(|providers| providers.borrow().last().unwrap().clone());
                    let retained_root = card.root.clone();
                    drop(card);
                    assert!(provider.upgrade().is_some(), "a visible card lost its accent");
                    drop(retained_root);
                    assert!(provider.upgrade().is_none(), "obsolete display provider retained");
                }
                TEST_ACCENT_PROVIDERS.with(|providers| {
                    let providers = providers.borrow();
                    assert_eq!(providers.len(), 128);
                    assert!(providers.iter().all(|provider| provider.upgrade().is_none()));
                });
            },
        );
    }

    #[test]
    fn image_cards_keep_the_canvas_ratio_as_the_grid_expands() {
        assert_eq!(preview_card_height(296, 600), 168);
        assert_eq!(preview_card_height(340, 600), 193);
        assert_eq!(preview_card_height(256, 600), 148);
        assert_eq!(preview_card_height(576, 190), 178);
        assert_eq!(preview_card_height(576, 150), 138);
        assert_eq!(preview_card_height(576, 110), 104);
    }
    #[test]
    fn accessible_tallies_name_both_observed_states() {
        assert_eq!(tally_description(true, true), "Program · Preview");
        assert_eq!(tally_description(false, false), "");
    }
    #[test]
    fn preview_tally_requires_observed_studio_mode_and_matching_scene() {
        let mut snapshot = crate::domain::control::ControlSnapshot::default();
        snapshot.studio.preview = Some("Next".into());
        assert_eq!(studio_card_state(Some(&snapshot), "Next"), (false, false));
        snapshot.studio.enabled = Some(true);
        snapshot.studio.program = Some("On air".into());
        assert_eq!(studio_card_state(Some(&snapshot), "Next"), (true, true));
        assert_eq!(studio_card_state(Some(&snapshot), "On air"), (true, false));
        assert_eq!(studio_card_state(None, "Next"), (false, false));
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
    fn accent_css_only_styles_the_accent_chip() {
        let css = accent_css("scene-accent-123456", 18, 52, 86);
        assert!(css.contains(".scene-card-accent"));
        assert!(!css.contains("background-image"));
    }
}
