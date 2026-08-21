//! Audio input card for the Live page mixer.

use std::{cell::RefCell, rc::Rc};

use adw::prelude::*;
use glib::{source::SourceId, timeout_add_local_once};
use gtk4::{Align, Box as GtkBox, Button, Label, Orientation, Scale, ToggleButton};
use i18n_embed_fl::fl;

use crate::controller::command::AppCommand;
use crate::domain::audio::AudioInput;
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::services::audio_service::{AudioService, VolumeChangeDebouncer, VOLUME_SLIDER_DEBOUNCE};
use crate::storage::registry as registry_storage;
use crate::ui::navigation::NavigationContext;
use crate::ui::widgets::{icon_picker, volume_meter};

/// Icon on the card's overflow button, which opens the icon chooser.
const OVERFLOW_ICON: &str = "view-more-symbolic";

const OBS_FADER_MARKS_DB: &[(f64, &str)] = &[
    (0.0, "0"),
    (-6.0, "-6"),
    (-12.0, "-12"),
    (-18.0, "-18"),
    (-24.0, "-24"),
    (-30.0, "-30"),
    (-36.0, "-36"),
    (-42.0, "-42"),
    (-48.0, "-48"),
    (-54.0, "-54"),
    (-60.0, "-60"),
];
const OBS_FADER_INVERTED: bool = true;

// ── Public handle ─────────────────────────────────────────────────────────────

pub(crate) struct AudioCardHandle {
    pub(crate) root: GtkBox,
    pub(crate) input_id: String,
    mute_btn: ToggleButton,
    vol_scale: Scale,
    db_label: Label,
    volume_debouncer: Rc<RefCell<VolumeChangeDebouncer>>,
    volume_debounce_source: Rc<RefCell<Option<SourceId>>>,
    mute_signal_id: glib::SignalHandlerId,
    vol_signal_id: Rc<glib::SignalHandlerId>,
}

#[derive(Debug, Clone, Copy)]
enum VolumeDispatch {
    Debounced,
    Immediate,
}

struct VolumeChangeContext<'a> {
    vol_scale: &'a Scale,
    db_label: &'a Label,
    vol_signal_id: Option<&'a glib::SignalHandlerId>,
    debouncer: &'a Rc<RefCell<VolumeChangeDebouncer>>,
    debounce_source: &'a Rc<RefCell<Option<SourceId>>>,
}

impl AudioCardHandle {
    /// Update mute state from an OBS event without triggering the dispatch signal.
    pub(crate) fn update_mute(&self, muted: bool) {
        self.mute_btn.block_signal(&self.mute_signal_id);
        self.mute_btn.set_active(muted);
        self.mute_btn.unblock_signal(&self.mute_signal_id);
        apply_mute_style(&self.mute_btn, muted);
    }

    /// Update volume from an OBS event without triggering the dispatch signal.
    pub(crate) fn update_volume(&self, mul: f64, db: f64) {
        if let Some(source_id) = self.volume_debounce_source.borrow_mut().take() {
            source_id.remove();
        }
        let volume_mul = AudioService::sanitize_volume_mul(mul);
        self.volume_debouncer
            .borrow_mut()
            .reset_to_observed(volume_mul);
        self.vol_scale.block_signal(self.vol_signal_id.as_ref());
        self.vol_scale
            .set_value(AudioService::slider_db_from_mul(volume_mul));
        self.vol_scale.unblock_signal(self.vol_signal_id.as_ref());
        self.db_label
            .set_text(&AudioService::format_db(AudioService::sanitize_volume_db(
                db,
            )));
    }
}

// ── Card parts ────────────────────────────────────────────────────────────────

/// The mute toggle, plus the id of the handler that reports presses to OBS.
///
/// The caller keeps the handler id so it can silence the toggle while applying
/// a mute that came *from* OBS, instead of echoing it straight back.
fn build_mute_button(
    input_id: &str,
    muted: bool,
    nav: &NavigationContext,
) -> (ToggleButton, glib::SignalHandlerId) {
    let mute_btn = ToggleButton::builder().active(muted).build();
    mute_btn.set_tooltip_text(Some(&fl!(LANGUAGE_LOADER, "audio-card-mute-tooltip")));
    apply_mute_style(&mute_btn, muted);

    let signal_id = {
        let nav = nav.clone();
        let input_id = input_id.to_string();
        mute_btn.connect_toggled(move |btn| {
            nav.dispatch(AppCommand::SetInputMute {
                input: input_id.clone(),
                muted: btn.is_active(),
            });
        })
    };

    (mute_btn, signal_id)
}

/// The source name, ellipsized to one line with the full path in a tooltip.
fn build_name_label(input: &AudioInput) -> Label {
    let name_label = Label::builder()
        .label(&input.display_name)
        .xalign(0.0)
        .halign(Align::Fill)
        .hexpand(true)
        .wrap(true)
        .lines(1)
        .ellipsize(gtk4::pango::EllipsizeMode::End)
        .build();
    name_label.add_css_class("audio-card-title");

    // Nested and group sources are worth spelling out: the display name alone
    // does not say which scene a source is reached through.
    let tooltip = match input.source_path_label() {
        Some(path) => fl!(
            LANGUAGE_LOADER,
            "audio-card-source-path-tooltip",
            scope = input.source_scope.label(),
            path = path
        ),
        None => input.source_scope.label(),
    };
    name_label.set_tooltip_text(Some(&tooltip));

    name_label
}

/// The coloured header naming where a source comes from.
struct ScopeBar {
    root: GtkBox,
    /// Holds the chosen icon; the icon picker swaps its contents.
    icon_slot: GtkBox,
    /// The icon the registry currently has for this input, if any.
    selected_icon: Option<String>,
}

/// Build the scope bar.
///
/// OBS heads each mixer strip with a coloured scope tag; the user's chosen
/// icon rides in the same bar so the card is identifiable at a glance.
fn build_scope_bar(input: &AudioInput, input_id: &str, nav: &NavigationContext) -> ScopeBar {
    let root = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(5)
        .halign(Align::Fill)
        .hexpand(true)
        .build();
    root.add_css_class("audio-card-scope-bar");
    root.add_css_class(input.source_scope.css_class());

    let icon_slot = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .halign(Align::Center)
        .build();
    let selected_icon = nav
        .state
        .borrow()
        .registry
        .input_icon(input_id)
        .map(str::to_string);
    set_scope_icon(&icon_slot, selected_icon.as_deref());

    let scope_label = Label::builder()
        .label(input.source_scope.label())
        .halign(Align::Center)
        .hexpand(true)
        .build();
    scope_label.add_css_class("audio-card-scope-label");

    root.append(&icon_slot);
    root.append(&scope_label);

    ScopeBar {
        root,
        icon_slot,
        selected_icon,
    }
}

/// The local lock, which freezes this card's fader.
///
/// It disables SceneDeck's own slider only; the source stays unlocked in OBS.
fn build_lock_button(locked: bool, vol_scale: &Scale) -> ToggleButton {
    let lock_btn = ToggleButton::builder()
        .icon_name("changes-prevent-symbolic")
        .active(locked)
        .build();
    lock_btn.set_tooltip_text(Some(&fl!(LANGUAGE_LOADER, "audio-card-lock-tooltip")));
    lock_btn.add_css_class("flat");
    lock_btn.add_css_class("circular");
    lock_btn.connect_toggled({
        let vol_scale = vol_scale.clone();
        move |btn| {
            let locked = btn.is_active();
            vol_scale.set_sensitive(!locked);
            apply_lock_style(btn, locked);
        }
    });
    apply_lock_style(&lock_btn, locked);

    lock_btn
}

/// A centred row of small controls under the fader.
fn control_row(css_class: &str, children: &[gtk4::Widget]) -> GtkBox {
    let row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(4)
        .halign(Align::Center)
        .build();
    row.add_css_class(css_class);
    for child in children {
        row.append(child);
    }
    row
}

/// The fader, its decibel readout, and the debouncing that ties them to OBS.
///
/// Cloning shares the same widgets and debouncer state; the card and its fine
/// controls both need to reach them.
#[derive(Clone)]
struct VolumeControls {
    scale: Scale,
    db_label: Label,
    debouncer: Rc<RefCell<VolumeChangeDebouncer>>,
    debounce_source: Rc<RefCell<Option<glib::SourceId>>>,
    /// Shared because the fader, the fine controls, and incoming OBS updates
    /// all need to block it while writing a value they produced themselves.
    signal_id: Rc<glib::SignalHandlerId>,
}

/// Build the fader and its readout.
///
/// Dragging a fader produces a continuous stream of values, so changes are
/// debounced before they reach OBS rather than sending a request per pixel.
/// The caller keeps the handler id to silence the fader while applying a
/// volume that came from OBS, and the debouncer state so a rebuild does not
/// lose an in-flight drag.
fn build_volume_controls(
    input: &AudioInput,
    input_id: &str,
    nav: &NavigationContext,
) -> VolumeControls {
    let vol_scale = Scale::with_range(
        Orientation::Vertical,
        AudioService::min_volume_db(),
        AudioService::max_volume_db(),
        0.5,
    );
    vol_scale.set_value(AudioService::slider_db_from_mul(input.volume_mul));
    vol_scale.set_inverted(OBS_FADER_INVERTED);
    vol_scale.set_draw_value(false);
    vol_scale.set_vexpand(false);
    vol_scale.set_height_request(volume_meter::METER_HEIGHT);
    vol_scale.set_width_request(24);
    vol_scale.add_css_class("audio-volume-fader");
    vol_scale.set_tooltip_text(Some(&fl!(LANGUAGE_LOADER, "audio-card-fader-tooltip")));

    let db_label = Label::builder()
        .label(AudioService::format_db(AudioService::sanitize_volume_db(
            input.volume_db,
        )))
        .xalign(0.0)
        .halign(Align::Start)
        .build();
    db_label.add_css_class("numeric");
    db_label.add_css_class("audio-card-db");

    let volume_debouncer = Rc::new(RefCell::new(VolumeChangeDebouncer::new(input.volume_mul)));
    let volume_debounce_source = Rc::new(RefCell::new(None));

    let vol_signal_id = {
        let nav = nav.clone();
        let input_id = input_id.to_string();
        let vol_scale_for_update = vol_scale.clone();
        let db_label = db_label.clone();
        let debouncer = volume_debouncer.clone();
        let debounce_source = volume_debounce_source.clone();
        vol_scale.connect_value_changed(move |scale| {
            let volume_mul = AudioService::volume_db_to_mul(scale.value());
            let context = VolumeChangeContext {
                vol_scale: &vol_scale_for_update,
                db_label: &db_label,
                vol_signal_id: None,
                debouncer: &debouncer,
                debounce_source: &debounce_source,
            };
            apply_volume_change(
                &nav,
                &input_id,
                volume_mul,
                VolumeDispatch::Debounced,
                context,
            );
        })
    };

    VolumeControls {
        scale: vol_scale,
        db_label,
        debouncer: volume_debouncer,
        debounce_source: volume_debounce_source,
        signal_id: Rc::new(vol_signal_id),
    }
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build a single mixer card for `input` and return a handle.
pub(crate) fn build(input: &AudioInput, nav: NavigationContext) -> AudioCardHandle {
    let input_id = input.id.clone();

    let (mute_btn, mute_signal_id) = build_mute_button(&input_id, input.muted, &nav);

    let name_label = build_name_label(input);

    let ScopeBar {
        root: scope_bar,
        icon_slot: scope_icon_slot,
        selected_icon,
    } = build_scope_bar(input, &input_id, &nav);

    let controls = build_volume_controls(input, &input_id, &nav);
    let VolumeControls {
        scale: vol_scale,
        db_label,
        debouncer: volume_debouncer,
        debounce_source: volume_debounce_source,
        signal_id: vol_signal_id,
    } = controls.clone();

    let lock_btn = build_lock_button(input.locked_locally, &vol_scale);

    let meter = volume_meter::build(&input_id, nav.clone());

    let fader_row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(4)
        .halign(Align::Center)
        .valign(Align::Center)
        .build();
    fader_row.add_css_class("audio-fader-row");
    fader_row.append(&vol_scale);
    fader_row.append(&meter.root);
    fader_row.append(&build_meter_ruler());

    // Mute and lock sit side by side under the fader, where OBS puts mute and
    // monitoring.
    let buttons = control_row(
        "audio-card-controls",
        &[mute_btn.clone().upcast(), lock_btn.upcast()],
    );

    let icon_button = icon_picker::build(
        selected_icon.as_deref(),
        &fl!(LANGUAGE_LOADER, "mixer-input-icon-tooltip"),
        icon_picker::PickerDisplay::Fixed(OVERFLOW_ICON),
        {
            let nav = nav.clone();
            let input_id = input_id.clone();
            let scope_icon_slot = scope_icon_slot.clone();
            move |icon| {
                set_scope_icon(&scope_icon_slot, icon.as_deref());
                set_input_icon(&nav, &input_id, icon.as_deref());
            }
        },
    );

    let fine_controls = build_fine_controls(input, &controls, &nav);

    let overflow = control_row(
        "audio-card-overflow",
        &[icon_button.upcast(), fine_controls.upcast()],
    );

    // ── Card ─────────────────────────────────────────────────────────────────
    let root = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .halign(Align::Start)
        .valign(Align::Start)
        .hexpand(false)
        .vexpand(false)
        .width_request(136)
        .build();
    root.add_css_class("card");
    root.add_css_class("audio-card");

    root.append(&scope_bar);
    root.append(&name_label);
    root.append(&db_label);
    root.append(&fader_row);
    root.append(&buttons);
    root.append(&overflow);

    AudioCardHandle {
        root,
        input_id,
        mute_btn,
        vol_scale,
        db_label,
        volume_debouncer,
        volume_debounce_source,
        mute_signal_id,
        vol_signal_id,
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Replace the icon shown in the card's scope bar.
///
/// The slot stays in the tree whether or not an icon is set, so choosing one
/// does not reflow the header.
fn set_scope_icon(slot: &GtkBox, icon: Option<&str>) {
    while let Some(child) = slot.first_child() {
        slot.remove(&child);
    }
    if let Some(image) = icon_picker::header_icon(icon) {
        image.add_css_class("audio-card-scope-icon");
        slot.append(&image);
    }
}

/// Persist one audio input's icon and mirror it into the cached registry.
fn set_input_icon(nav: &NavigationContext, input_id: &str, icon: Option<&str>) {
    if !nav
        .state
        .borrow_mut()
        .registry
        .set_input_icon(input_id, icon)
    {
        return;
    }
    let input_id = input_id.to_string();
    let icon = icon.map(str::to_string);
    crate::ui::background_io::run(
        move || registry_storage::set_input_icon(&input_id, icon.as_deref()),
        |result| {
            if let Err(error) = result {
                tracing::warn!(%error, "failed to save the audio source icon");
            }
        },
    );
}

fn apply_mute_style(btn: &ToggleButton, muted: bool) {
    if muted {
        btn.set_icon_name("audio-volume-muted-symbolic");
        btn.add_css_class("destructive-action");
        btn.remove_css_class("flat");
    } else {
        btn.set_icon_name("audio-volume-high-symbolic");
        btn.remove_css_class("destructive-action");
        btn.add_css_class("flat");
    }
    if !btn.has_css_class("circular") {
        btn.add_css_class("circular");
    }
}

fn apply_lock_style(btn: &ToggleButton, locked: bool) {
    if locked {
        btn.add_css_class("suggested-action");
    } else {
        btn.remove_css_class("suggested-action");
    }
}

/// Decibel scale printed beside the fader and the meter.
///
/// Both of those run from −60 dB to unity over the same height, so one ruler
/// reads for the level you are setting and the level you are getting.
fn build_meter_ruler() -> GtkBox {
    let labels = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .homogeneous(true)
        .halign(Align::Start)
        .valign(Align::Center)
        .height_request(volume_meter::METER_HEIGHT)
        .build();
    labels.add_css_class("audio-meter-labels");

    for (_, label) in OBS_FADER_MARKS_DB {
        let label = Label::builder()
            .label(*label)
            .xalign(0.0)
            .halign(Align::Start)
            .build();
        label.add_css_class("audio-meter-label");
        labels.append(&label);
    }

    labels
}

fn build_fine_controls(
    input: &AudioInput,
    controls: &VolumeControls,
    nav: &NavigationContext,
) -> GtkBox {
    let controls_box = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(1)
        .halign(Align::Center)
        .valign(Align::Center)
        .build();
    controls_box.add_css_class("audio-fine-controls");

    let plus = Button::builder()
        .label("+")
        .tooltip_text(fl!(LANGUAGE_LOADER, "audio-card-fine-plus-tooltip"))
        .build();
    let reset = Button::builder()
        .icon_name("edit-undo-symbolic")
        .tooltip_text(fl!(LANGUAGE_LOADER, "audio-card-fine-reset-tooltip"))
        .build();
    let minus = Button::builder()
        .label("-")
        .tooltip_text(fl!(LANGUAGE_LOADER, "audio-card-fine-minus-tooltip"))
        .build();

    for button in [&plus, &reset, &minus] {
        button.add_css_class("flat");
        button.add_css_class("circular");
    }

    // The three buttons differ only in what they ask for; everything they need
    // in order to ask is the same.
    connect_volume_action(&plus, &input.id, controls, nav, |nav, id, value, ctx| {
        dispatch_db_adjust(nav, id, value, 1.0, ctx);
    });
    connect_volume_action(&reset, &input.id, controls, nav, |nav, id, _, ctx| {
        apply_volume_change(nav, id, 1.0, VolumeDispatch::Immediate, ctx);
    });
    connect_volume_action(&minus, &input.id, controls, nav, |nav, id, value, ctx| {
        dispatch_db_adjust(nav, id, value, -1.0, ctx);
    });

    controls_box.append(&plus);
    controls_box.append(&reset);
    controls_box.append(&minus);
    controls_box
}

/// Wire one fine-control button to an action on this card's volume.
///
/// The action is handed the current fader value and a `VolumeChangeContext`
/// already set up to block the fader's own signal, so it cannot mistake the
/// value it is writing for one the user just dragged.
fn connect_volume_action(
    button: &Button,
    input_id: &str,
    controls: &VolumeControls,
    nav: &NavigationContext,
    action: impl Fn(&NavigationContext, &str, f64, VolumeChangeContext<'_>) + 'static,
) {
    button.connect_clicked({
        let nav = nav.clone();
        let input_id = input_id.to_string();
        let controls = controls.clone();
        move |_| {
            let context = VolumeChangeContext {
                vol_scale: &controls.scale,
                db_label: &controls.db_label,
                vol_signal_id: Some(controls.signal_id.as_ref()),
                debouncer: &controls.debouncer,
                debounce_source: &controls.debounce_source,
            };
            action(&nav, &input_id, controls.scale.value(), context);
        }
    });
}

fn dispatch_db_adjust(
    nav: &NavigationContext,
    input_id: &str,
    current_db: f64,
    delta_db: f64,
    context: VolumeChangeContext<'_>,
) {
    let next_db = AudioService::adjust_volume_db(current_db, delta_db);
    apply_volume_change(
        nav,
        input_id,
        AudioService::volume_db_to_mul(next_db),
        VolumeDispatch::Immediate,
        context,
    );
}

fn apply_volume_change(
    nav: &NavigationContext,
    input_id: &str,
    volume_mul: f64,
    dispatch: VolumeDispatch,
    context: VolumeChangeContext<'_>,
) {
    let volume_mul = AudioService::sanitize_volume_mul(volume_mul);
    update_visible_volume(
        context.vol_scale,
        context.db_label,
        context.vol_signal_id,
        volume_mul,
    );

    match dispatch {
        VolumeDispatch::Debounced => {
            context.debouncer.borrow_mut().queue(volume_mul);
            if let Some(source_id) = context.debounce_source.borrow_mut().take() {
                source_id.remove();
            }

            let nav = nav.clone();
            let input_id = input_id.to_string();
            let debouncer = context.debouncer.clone();
            let debounce_source = context.debounce_source.clone();
            let debounce_source_for_timeout = debounce_source.clone();
            let source_id = timeout_add_local_once(VOLUME_SLIDER_DEBOUNCE, move || {
                debounce_source_for_timeout.borrow_mut().take();
                if let Some(volume_mul) = debouncer.borrow_mut().take_due() {
                    nav.dispatch(AppCommand::SetInputVolume {
                        input: input_id,
                        volume_mul,
                    });
                }
            });
            *debounce_source.borrow_mut() = Some(source_id);
        }
        VolumeDispatch::Immediate => {
            if let Some(source_id) = context.debounce_source.borrow_mut().take() {
                source_id.remove();
            }
            context.debouncer.borrow_mut().mark_sent(volume_mul);
            nav.dispatch(AppCommand::SetInputVolume {
                input: input_id.to_string(),
                volume_mul,
            });
        }
    }
}

fn update_visible_volume(
    vol_scale: &Scale,
    db_label: &Label,
    vol_signal_id: Option<&glib::SignalHandlerId>,
    volume_mul: f64,
) {
    if let Some(signal_id) = vol_signal_id {
        vol_scale.block_signal(signal_id);
        vol_scale.set_value(AudioService::slider_db_from_mul(volume_mul));
        vol_scale.unblock_signal(signal_id);
    } else {
        vol_scale.set_value(AudioService::slider_db_from_mul(volume_mul));
    }
    db_label.set_text(&AudioService::format_db(AudioService::volume_mul_to_db(
        volume_mul,
    )));
}

#[cfg(test)]
mod tests {
    use super::{OBS_FADER_INVERTED, OBS_FADER_MARKS_DB};

    #[test]
    fn obs_fader_marks_start_at_unity_and_descend() {
        assert_eq!(OBS_FADER_MARKS_DB.first(), Some(&(0.0, "0")));
        assert_eq!(OBS_FADER_MARKS_DB.last(), Some(&(-60.0, "-60")));
        assert!(OBS_FADER_MARKS_DB.iter().all(|(db, _)| *db <= 0.0));
        assert!(OBS_FADER_MARKS_DB
            .windows(2)
            .all(|pair| pair[0].0 > pair[1].0));
    }

    #[test]
    fn obs_fader_is_inverted_so_unity_is_at_the_top() {
        const { assert!(OBS_FADER_INVERTED) };
    }
}
