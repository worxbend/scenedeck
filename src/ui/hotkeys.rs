//! Window-level keyboard handling for Live-page scene shortcuts.

use std::cell::{Cell, RefCell};
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

/// Distinguishes identical notices armed at different times.
#[derive(Default)]
pub(crate) struct NoticeGeneration(Cell<u64>);

impl NoticeGeneration {
    pub(super) fn advance(&self) -> u64 {
        let next = self.0.get().wrapping_add(1);
        self.0.set(next);
        next
    }

    fn current(&self) -> u64 {
        self.0.get()
    }

    fn accepts(&self, token: u64) -> bool {
        self.current() == token
    }
}

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
        let window = window.downgrade();
        let live = Rc::downgrade(&live);
        move |_, keyval, keycode, modifier_state| {
            let (Some(window), Some(live)) = (window.upgrade(), live.upgrade()) else {
                return glib::Propagation::Proceed;
            };
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
            if text_input_has_focus(&window) {
                resolver.borrow_mut().disarm();
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
        HotkeyOutcome::LeaderArmed => {
            let notice = fl!(LANGUAGE_LOADER, "hotkey-hint-leader-armed");
            set_hotkey_hint(live, Some(&notice));
            expire_notice(live, hotkeys.leader_timeout());
        }
        HotkeyOutcome::LeaderCancelled => {
            set_hotkey_hint(live, None);
        }
        HotkeyOutcome::Activate(slot) => {
            let scene = {
                let state = nav.state.borrow();
                scene_for_slot(&state.scene_inventory, &state.registry, slot)
            };
            match scene {
                Some(scene_id) => {
                    tracing::debug!(slot, scene = %scene_id, "scene hotkey switch");
                    set_hotkey_hint(live, None);
                    nav.dispatch(AppCommand::SwitchPrimaryScene(scene_id));
                }
                None => {
                    let notice = fl!(
                        LANGUAGE_LOADER,
                        "hotkey-hint-empty-slot",
                        slot = (slot + 1).to_string()
                    );
                    set_hotkey_hint(live, Some(&notice));
                    expire_notice(live, HOTKEY_MISS_NOTICE);
                }
            }
        }
    }
}

fn expire_notice(live: &Rc<LivePageHandle>, timeout: Duration) {
    let token = live.hotkey_notice_generation.current();
    let live = Rc::downgrade(live);
    glib::timeout_add_local_once(timeout, move || {
        if let Some(live) = live.upgrade() {
            if live.hotkey_notice_generation.accepts(token) {
                crate::ui::pages::live::set_hotkey_hint(&live, None);
            }
        }
    });
}

pub(crate) fn scene_hint(nav: &NavigationContext, hotkeys: &SceneHotkeyConfig) -> Option<String> {
    let binding = hotkeys.hint_label()?;
    let studio = nav
        .state
        .borrow()
        .workspace
        .snapshot
        .as_ref()
        .is_some_and(|snapshot| snapshot.studio.enabled == Some(true));
    Some(if studio {
        fl!(LANGUAGE_LOADER, "hotkey-hint-stage-take", binding = binding)
    } else {
        binding
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::hotkey::{MAX_LEADER_TIMEOUT_MS, MIN_LEADER_TIMEOUT_MS};

    #[test]
    fn identical_rearmed_notices_reject_old_expiry_tokens() {
        let notice = NoticeGeneration::default();
        let first = notice.advance();
        let second = notice.advance();
        assert!(!notice.accepts(first));
        assert!(notice.accepts(second));
        notice.advance();
        assert!(!notice.accepts(second));
    }

    #[test]
    fn adapter_timeout_uses_the_resolvers_effective_window() {
        for (raw, expected) in [
            (0, MIN_LEADER_TIMEOUT_MS),
            (1_500, 1_500),
            (u64::MAX, MAX_LEADER_TIMEOUT_MS),
        ] {
            let hotkeys = SceneHotkeyConfig {
                leader_timeout_ms: raw,
                ..Default::default()
            };
            assert_eq!(hotkeys.leader_timeout(), Duration::from_millis(expected));
        }
    }

    #[test]
    #[ignore = "requires a native GTK display"]
    fn native_rearm_keeps_the_new_notice_until_its_own_expiry() {
        crate::ui::test_support::native_test(
            "ui::hotkeys::tests::native_rearm_keeps_the_new_notice_until_its_own_expiry",
            || {
                let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
                let live = Rc::new(crate::ui::pages::live::build(nav));
                let notice = "identical notice";
                crate::ui::pages::live::set_hotkey_hint(&live, Some(notice));
                expire_notice(&live, Duration::from_millis(30));
                crate::ui::test_support::pump(Duration::from_millis(15));
                crate::ui::pages::live::set_hotkey_hint(&live, Some(notice));
                expire_notice(&live, Duration::from_millis(100));
                crate::ui::test_support::pump(Duration::from_millis(40));
                assert_eq!(live.hotkey_hint.text(), notice);
                crate::ui::test_support::pump(Duration::from_millis(90));
                assert!(!live.hotkey_hint.is_visible());
                crate::ui::pages::live::set_hotkey_hint(&live, Some(notice));
                expire_notice(&live, Duration::from_secs(5));
                let weak = Rc::downgrade(&live);
                drop(live);
                assert!(
                    weak.upgrade().is_none(),
                    "notice timeout retained the page handle"
                );
            },
        );
    }
}
