//! Icon chooser shared by Inventory scenes and Mixer audio sources.
//!
//! A menu button showing the current icon, opening a popover grid of the
//! curated catalogue in `domain::icon` plus a "no icon" entry. The widget owns
//! no persistence: it reports the chosen key and lets the page decide where it
//! goes.

use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Align, Button, FlowBox, FlowBoxChild, Image, MenuButton, Popover};

use crate::domain::icon::{icon_name, no_icon_label, ICON_CATALOGUE};

/// Icon shown when a scene or source has none of its own.
pub(crate) const PLACEHOLDER_ICON: &str = "image-missing-symbolic";
/// Columns in the picker grid.
const GRID_COLUMNS: u32 = 6;

/// What the picker's own button shows.
#[derive(Debug, Clone, Copy)]
pub(crate) enum PickerDisplay {
    /// Show the chosen icon, so the button doubles as the current value.
    CurrentIcon,
    /// Keep a fixed icon, for a button that opens a menu somewhere the icon is
    /// already displayed.
    Fixed(&'static str),
}

/// Build an icon chooser.
///
/// `selected` is the currently persisted key, if any. `on_choose` is called
/// with the new key — `None` when the user picks "no icon" — and is responsible
/// for persisting it. The button's own image updates before the callback runs,
/// so the picker reflects the choice even if saving fails.
pub(crate) fn build<F>(
    selected: Option<&str>,
    tooltip: &str,
    display: PickerDisplay,
    on_choose: F,
) -> MenuButton
where
    F: Fn(Option<String>) + 'static,
{
    let button = MenuButton::builder()
        .icon_name(match display {
            PickerDisplay::CurrentIcon => button_icon_name(selected),
            PickerDisplay::Fixed(name) => name,
        })
        .valign(Align::Center)
        .tooltip_text(tooltip)
        .build();
    button.add_css_class("flat");
    button.add_css_class("icon-picker-button");

    let popover = Popover::builder().build();
    popover.add_css_class("icon-picker-popover");

    let on_choose: Rc<dyn Fn(Option<String>)> = Rc::new(on_choose);
    let grid = FlowBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .min_children_per_line(GRID_COLUMNS)
        .max_children_per_line(GRID_COLUMNS)
        .row_spacing(2)
        .column_spacing(2)
        .homogeneous(true)
        .build();

    let clear = icon_button(PLACEHOLDER_ICON, &no_icon_label(), selected.is_none());
    clear.connect_clicked({
        let button = button.clone();
        let popover = popover.clone();
        let on_choose = Rc::clone(&on_choose);
        move |_| {
            if let PickerDisplay::CurrentIcon = display {
                button.set_icon_name(PLACEHOLDER_ICON);
            }
            popover.popdown();
            on_choose(None);
        }
    });
    insert_grid_child(&grid, &clear);

    for icon in ICON_CATALOGUE {
        let is_selected = selected == Some(icon.key);
        let choice = icon_button(icon.icon_name, &icon.label(), is_selected);
        choice.connect_clicked({
            let button = button.clone();
            let popover = popover.clone();
            let on_choose = Rc::clone(&on_choose);
            move |_| {
                if let PickerDisplay::CurrentIcon = display {
                    button.set_icon_name(icon.icon_name);
                }
                popover.popdown();
                on_choose(Some(icon.key.to_string()));
            }
        });
        insert_grid_child(&grid, &choice);
    }

    popover.set_child(Some(&grid));
    button.set_popover(Some(&popover));
    button
}

/// Icon name to show for a persisted key.
///
/// An unknown key — a hand-edited registry, or one written by a newer build —
/// shows the placeholder rather than nothing, so the picker is still findable.
fn button_icon_name(selected: Option<&str>) -> &'static str {
    selected.and_then(icon_name).unwrap_or(PLACEHOLDER_ICON)
}

fn icon_button(icon_name: &str, label: &str, selected: bool) -> Button {
    let button = Button::builder()
        .child(&Image::from_icon_name(icon_name))
        .tooltip_text(label)
        .build();
    button.add_css_class("flat");
    button.add_css_class("icon-picker-choice");
    if selected {
        button.add_css_class("icon-picker-choice-selected");
    }
    button
}

fn insert_grid_child(grid: &FlowBox, button: &Button) {
    let child = FlowBoxChild::new();
    child.set_child(Some(button));
    child.set_focusable(false);
    grid.insert(&child, -1);
}

/// Label pairing an icon with text, used by card headers.
///
/// Returns the icon image so callers can hide it when there is no icon rather
/// than showing a placeholder in a title bar.
pub(crate) fn header_icon(selected: Option<&str>) -> Option<Image> {
    let name = selected.and_then(icon_name)?;
    let image = Image::from_icon_name(name);
    image.add_css_class("scenedeck-icon");
    Some(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_known_key_maps_to_its_catalogue_icon() {
        assert_eq!(
            button_icon_name(Some("camera")),
            "nf-fa-video-camera-symbolic"
        );
    }

    #[test]
    fn no_icon_and_unknown_keys_both_show_the_placeholder() {
        assert_eq!(button_icon_name(None), PLACEHOLDER_ICON);
        assert_eq!(button_icon_name(Some("not-an-icon")), PLACEHOLDER_ICON);
    }

    #[test]
    fn the_grid_holds_the_whole_catalogue_plus_the_clear_entry() {
        // Six columns keeps the popover square-ish for a catalogue this size.
        let entries = ICON_CATALOGUE.len() + 1;
        assert_eq!(entries % GRID_COLUMNS as usize, 1);
        assert!(entries / GRID_COLUMNS as usize <= 6);
    }
}
