//! Window-level keyboard handling for Live-page scene shortcuts.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use adw::prelude::*;
use gtk4::gdk;
use gtk4::gdk::prelude::DisplayExtManual;
use i18n_embed_fl::fl;

use crate::controller::command::AppCommand;
use crate::controller::state::Page;
use crate::domain::hotkey::{KeyStroke, KeySymbol, Modifiers, SceneHotkeyConfig};
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::services::hotkey_service::{HotkeyOutcome, SceneHotkeyResolver};
use crate::ui::navigation::NavigationContext;
use crate::ui::pages::live::LivePageHandle;

/// How long the "no scene in that slot" notice replaces the shortcut caption.
const HOTKEY_MISS_NOTICE: Duration = Duration::from_secs(2);

/// Wire Live-page scene shortcuts to a window-level key controller.
///
/// The controller runs in the capture phase so bare digits and the leader key
/// are seen before any focused widget consumes them, and it re-reads the
/// current settings on every press so changes apply immediately.
pub(super) fn install(
    window: &adw::ApplicationWindow,
    nav: &NavigationContext,
    live: Rc<LivePageHandle>,
) {
    let resolver = Rc::new(RefCell::new(SceneHotkeyResolver::new()));
    let controller = gtk4::EventControllerKey::new();
    controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    controller.connect_key_pressed({
        let nav = nav.clone();
        let window = window.clone();
        move |_, keyval, keycode, modifier_state| {
            let hotkeys = {
                let state = nav.state.borrow();
                if state.current_page != Page::Live {
                    resolver.borrow_mut().disarm();
                    return glib::Propagation::Proceed;
                }
                state.config.hotkeys
            };
            if !hotkeys.enabled {
                resolver.borrow_mut().disarm();
                return glib::Propagation::Proceed;
            }
            if !binds_with_modifiers(&hotkeys) && text_input_has_focus(&window) {
                return glib::Propagation::Proceed;
            }

            let stroke = KeyStroke::new(
                key_symbol(&window, keyval, keycode),
                modifiers_from_state(modifier_state),
            );
            let outcome = resolver
                .borrow_mut()
                .resolve(&hotkeys, stroke, Instant::now());
            apply_outcome(&nav, &live, &hotkeys, outcome);

            if outcome.consumes_event() {
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        }
    });
    window.add_controller(controller);
}

fn apply_outcome(
    nav: &NavigationContext,
    live: &Rc<LivePageHandle>,
    hotkeys: &SceneHotkeyConfig,
    outcome: HotkeyOutcome,
) {
    use crate::ui::pages::live::{scene_for_slot, set_hotkey_hint};

    match outcome {
        HotkeyOutcome::Ignored => {}
        HotkeyOutcome::LeaderArmed => set_hotkey_hint(
            live,
            Some(&fl!(LANGUAGE_LOADER, "hotkey-hint-leader-armed")),
        ),
        HotkeyOutcome::LeaderCancelled => {
            set_hotkey_hint(live, hotkeys.hint_label().as_deref());
        }
        HotkeyOutcome::Activate(slot) => {
            let scene = {
                let state = nav.state.borrow();
                scene_for_slot(&state.scene_inventory, &state.registry, slot)
            };
            match scene {
                Some(scene_id) => {
                    tracing::debug!(slot, scene = %scene_id, "scene hotkey switch");
                    set_hotkey_hint(live, hotkeys.hint_label().as_deref());
                    nav.dispatch(AppCommand::SwitchPrimaryScene(scene_id));
                }
                None => {
                    let notice = fl!(
                        LANGUAGE_LOADER,
                        "hotkey-hint-empty-slot",
                        slot = (slot + 1).to_string()
                    );
                    set_hotkey_hint(live, Some(&notice));
                    glib::timeout_add_local_once(HOTKEY_MISS_NOTICE, {
                        let live = live.clone();
                        let nav = nav.clone();
                        move || {
                            if live.hotkey_hint.text() != notice {
                                return;
                            }
                            let hotkeys = nav.state.borrow().config.hotkeys;
                            set_hotkey_hint(&live, hotkeys.hint_label().as_deref());
                        }
                    });
                }
            }
        }
    }
}

fn binds_with_modifiers(hotkeys: &SceneHotkeyConfig) -> bool {
    hotkeys
        .style
        .modifiers()
        .is_some_and(|modifiers| !modifiers.is_empty())
}

fn text_input_has_focus(window: &adw::ApplicationWindow) -> bool {
    gtk4::prelude::RootExt::focus(window).is_some_and(|widget| {
        widget.is::<gtk4::Editable>()
            || widget.is::<gtk4::TextView>()
            || widget.is::<gtk4::SearchEntry>()
    })
}

fn modifiers_from_state(state: gdk::ModifierType) -> Modifiers {
    Modifiers::new(
        state.contains(gdk::ModifierType::CONTROL_MASK),
        state.contains(gdk::ModifierType::ALT_MASK),
        state.contains(gdk::ModifierType::SHIFT_MASK),
        state.contains(gdk::ModifierType::SUPER_MASK),
    )
}

fn key_symbol(window: &adw::ApplicationWindow, keyval: gdk::Key, keycode: u32) -> KeySymbol {
    match keyval {
        gdk::Key::space | gdk::Key::KP_Space => KeySymbol::Space,
        gdk::Key::comma | gdk::Key::KP_Separator => KeySymbol::Comma,
        gdk::Key::semicolon => KeySymbol::Semicolon,
        gdk::Key::backslash => KeySymbol::Backslash,
        gdk::Key::grave | gdk::Key::dead_grave => KeySymbol::Grave,
        gdk::Key::Escape => KeySymbol::Escape,
        gdk::Key::Shift_L
        | gdk::Key::Shift_R
        | gdk::Key::Control_L
        | gdk::Key::Control_R
        | gdk::Key::Alt_L
        | gdk::Key::Alt_R
        | gdk::Key::Meta_L
        | gdk::Key::Meta_R
        | gdk::Key::Super_L
        | gdk::Key::Super_R
        | gdk::Key::Hyper_L
        | gdk::Key::Hyper_R
        | gdk::Key::Caps_Lock
        | gdk::Key::Shift_Lock
        | gdk::Key::Num_Lock
        | gdk::Key::ISO_Level3_Shift => KeySymbol::Modifier,
        _ => match digit_for_keyval(keyval).or_else(|| digit_for_keycode(window, keycode)) {
            Some(digit) => KeySymbol::Digit(digit),
            None => KeySymbol::Other,
        },
    }
}

fn digit_for_keyval(keyval: gdk::Key) -> Option<u8> {
    match keyval {
        gdk::Key::KP_0 => return Some(0),
        gdk::Key::KP_1 => return Some(1),
        gdk::Key::KP_2 => return Some(2),
        gdk::Key::KP_3 => return Some(3),
        gdk::Key::KP_4 => return Some(4),
        gdk::Key::KP_5 => return Some(5),
        gdk::Key::KP_6 => return Some(6),
        gdk::Key::KP_7 => return Some(7),
        gdk::Key::KP_8 => return Some(8),
        gdk::Key::KP_9 => return Some(9),
        _ => {}
    }
    keyval
        .to_unicode()
        .and_then(|character| character.to_digit(10))
        .map(|digit| digit as u8)
}

fn digit_for_keycode(window: &adw::ApplicationWindow, keycode: u32) -> Option<u8> {
    let display = WidgetExt::display(window);
    display
        .map_keycode(keycode)?
        .into_iter()
        .find_map(|(_, keyval)| digit_for_keyval(keyval))
}
