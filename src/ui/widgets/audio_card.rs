//! Audio input card for the Live page mixer.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use adw::prelude::*;
use glib::{source::SourceId, timeout_add_local_once};
use gtk4::{Align, Box as GtkBox, Button, Label, Orientation, Scale, ToggleButton};
use i18n_embed_fl::fl;

use crate::controller::command::AppCommand;
use crate::controller::state::ObsStatus;
use crate::domain::audio::AudioInput;
use crate::domain::workflow::ScopeBinding;
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
    observed_volume: Cell<(f64, f64)>,
}

impl Drop for AudioCardHandle {
    fn drop(&mut self) {
        if let Some(source) = self.volume_debounce_source.borrow_mut().take() {
            source.remove();
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum VolumeDispatch {
    Debounced,
    Immediate,
}

#[derive(Clone)]
struct AudioTargetScope {
    epoch: u64,
    scope: ScopeBinding,
}

impl AudioTargetScope {
    fn capture(nav: &NavigationContext) -> Self {
        let state = nav.state.borrow();
        Self {
            epoch: state.workspace.epoch,
            scope: state.workflow_scope(),
        }
    }

    fn accepts(&self, epoch: u64, scope: &ScopeBinding, connected: bool) -> bool {
        connected
            && !self.scope.collection.is_empty()
            && self.epoch == epoch
            && &self.scope == scope
    }

    fn is_current(&self, nav: &NavigationContext) -> bool {
        let state = nav.state.borrow();
        self.accepts(
            state.workspace.epoch,
            &state.workflow_scope(),
            matches!(state.obs_status, ObsStatus::Connected { .. }),
        )
    }
}

struct VolumeChangeContext<'a> {
    scope: &'a AudioTargetScope,
    vol_scale: &'a Scale,
    db_label: &'a Label,
    vol_signal_id: Option<&'a glib::SignalHandlerId>,
    debouncer: &'a Rc<RefCell<VolumeChangeDebouncer>>,
    debounce_source: &'a Rc<RefCell<Option<SourceId>>>,
}

impl AudioCardHandle {
    /// Retain layout while membership is unknown, without letting a queued
    /// fader edit fire after its source leaves the displayed scene.
    pub(crate) fn set_available(&self, available: bool) {
        if !available && self.root.is_sensitive() {
            if let Some(source) = self.volume_debounce_source.borrow_mut().take() {
                source.remove();
            }
            let (mul, db) = self.observed_volume.get();
            *self.volume_debouncer.borrow_mut() = VolumeChangeDebouncer::new(mul);
            self.vol_scale.block_signal(self.vol_signal_id.as_ref());
            self.vol_scale
                .set_value(AudioService::slider_db_from_mul(mul));
            self.vol_scale.unblock_signal(self.vol_signal_id.as_ref());
            self.db_label.set_text(&AudioService::format_db(db));
        }
        self.root.set_sensitive(available);
    }

    /// Update mute state from an OBS event without triggering the dispatch signal.
    pub(crate) fn update_mute(&self, muted: bool) {
        self.mute_btn.block_signal(&self.mute_signal_id);
        self.mute_btn.set_active(muted);
        self.mute_btn.unblock_signal(&self.mute_signal_id);
        apply_mute_style(&self.mute_btn, muted);
    }

    /// Update volume from an OBS event without triggering the dispatch signal.
    pub(crate) fn update_volume(&self, mul: f64, db: f64) {
        let volume_mul = AudioService::sanitize_volume_mul(mul);
        let volume_db = AudioService::sanitize_volume_db(db);
        if self.observed_volume.replace((volume_mul, volume_db)) == (volume_mul, volume_db) {
            return;
        }
        if !self
            .volume_debouncer
            .borrow_mut()
            .reset_to_observed(volume_mul)
        {
            return;
        }
        if let Some(source_id) = self.volume_debounce_source.borrow_mut().take() {
            source_id.remove();
        }
        self.vol_scale.block_signal(self.vol_signal_id.as_ref());
        self.vol_scale
            .set_value(AudioService::slider_db_from_mul(volume_mul));
        self.vol_scale.unblock_signal(self.vol_signal_id.as_ref());
        self.db_label.set_text(&AudioService::format_db(volume_db));
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
    mute_btn.set_widget_name(&format!("audio-mute-{input_id}"));
    mute_btn.set_tooltip_text(Some(&fl!(LANGUAGE_LOADER, "audio-card-mute-tooltip")));
    mute_btn.update_property(&[gtk4::accessible::Property::Label(&fl!(
        LANGUAGE_LOADER,
        "audio-source-mute",
        source = input_id
    ))]);
    apply_mute_style(&mute_btn, muted);

    let signal_id = {
        let scope = AudioTargetScope::capture(nav);
        let nav = nav.clone();
        let input_id = input_id.to_string();
        mute_btn.connect_toggled(move |btn| {
            if !scope.is_current(&nav) {
                return;
            }
            nav.dispatch(AppCommand::SetInputMute {
                scope: scope.scope.clone(),
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
/// The state lives in `MixerSelection::locked_inputs` so structural rebuilds
/// and navigation preserve the operator's lock.
fn build_lock_button(
    nav: &NavigationContext,
    input_id: &str,
    locked: bool,
    vol_scale: &Scale,
) -> ToggleButton {
    let lock_btn = ToggleButton::builder()
        .icon_name("changes-prevent-symbolic")
        .active(locked)
        .build();
    lock_btn.set_tooltip_text(Some(&fl!(LANGUAGE_LOADER, "audio-card-lock-tooltip")));
    lock_btn.update_property(&[gtk4::accessible::Property::Label(&fl!(
        LANGUAGE_LOADER,
        "audio-source-lock",
        source = input_id
    ))]);
    lock_btn.add_css_class("flat");
    lock_btn.add_css_class("circular");
    lock_btn.connect_toggled({
        let nav = nav.clone();
        let input_id = input_id.to_string();
        let vol_scale = vol_scale.clone();
        move |btn| {
            let locked = btn.is_active();
            {
                let mut state = nav.state.borrow_mut();
                if locked {
                    state.mixer.locked_inputs.insert(input_id.clone());
                } else {
                    state.mixer.locked_inputs.remove(&input_id);
                }
            }
            vol_scale.set_sensitive(!locked);
            apply_lock_style(btn, locked);
        }
    });
    // The builder-set active state never fires connect_toggled, so the fader's
    // sensitivity for an initially locked card has to be set here.
    vol_scale.set_sensitive(!locked);
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
    scope: AudioTargetScope,
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
    let scope = AudioTargetScope::capture(nav);
    let vol_scale = Scale::with_range(
        Orientation::Vertical,
        AudioService::min_volume_db(),
        AudioService::max_volume_db(),
        0.5,
    );
    vol_scale.set_widget_name(&format!("audio-fader-{input_id}"));
    vol_scale.update_property(&[gtk4::accessible::Property::Label(&fl!(
        LANGUAGE_LOADER,
        "audio-source-volume",
        source = input_id
    ))]);
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
        // Reserve the longest readout ("-100.0 dB"). Otherwise changing gain
        // changes a FlowBox child's minimum width and can wrap the whole strip.
        .width_chars(9)
        .build();
    db_label.add_css_class("numeric");
    db_label.add_css_class("audio-card-db");

    let volume_debouncer = Rc::new(RefCell::new(VolumeChangeDebouncer::new(input.volume_mul)));
    let volume_debounce_source = Rc::new(RefCell::new(None));

    let vol_signal_id = {
        let scope = scope.clone();
        let nav = nav.clone();
        let input_id = input_id.to_string();
        let db_label = db_label.downgrade();
        let debouncer = volume_debouncer.clone();
        let debounce_source = volume_debounce_source.clone();
        vol_scale.connect_value_changed(move |scale| {
            let Some(db_label) = db_label.upgrade() else {
                return;
            };
            let volume_mul = AudioService::volume_db_to_mul(scale.value());
            let context = VolumeChangeContext {
                scope: &scope,
                vol_scale: scale,
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
        scope,
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
        scope: _,
        scale: vol_scale,
        db_label,
        debouncer: volume_debouncer,
        debounce_source: volume_debounce_source,
        signal_id: vol_signal_id,
    } = controls.clone();

    let lock_btn = build_lock_button(
        &nav,
        &input.id,
        nav.state.borrow().mixer.locked_inputs.contains(&input.id),
        &vol_scale,
    );

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
        observed_volume: Cell::new((
            AudioService::sanitize_volume_mul(input.volume_mul),
            AudioService::sanitize_volume_db(input.volume_db),
        )),
    }
}

/// Compact Live controls reuse the scoped mute/fader dispatch and debounce path.
pub(crate) fn build_compact(
    input: &AudioInput,
    nav: NavigationContext,
    expanded: bool,
) -> AudioCardHandle {
    let (mute_btn, mute_signal_id) = build_mute_button(&input.id, input.muted, &nav);
    let controls = build_volume_controls(input, &input.id, &nav);
    controls.scale.set_orientation(Orientation::Horizontal);
    controls.scale.set_inverted(false);
    controls.scale.set_height_request(-1);
    controls.scale.set_width_request(110);
    controls.scale.set_hexpand(true);
    let root = GtkBox::new(Orientation::Vertical, 4);
    root.add_css_class("live-audio-card");
    root.set_width_request(180);
    let header = GtkBox::new(Orientation::Horizontal, 6);
    header.append(&build_name_label(input));
    let meter = volume_meter::build_horizontal(&input.id, nav.clone());
    header.append(&meter.root);
    header.append(&mute_btn);
    root.append(&header);
    let gain = GtkBox::new(Orientation::Horizontal, 6);
    gain.append(&controls.scale);
    gain.append(&controls.db_label);
    root.append(&gain);
    let detail = GtkBox::new(Orientation::Horizontal, 6);
    detail.set_widget_name("live-audio-detail");
    detail.append(&build_fine_controls(input, &controls, &nav));
    detail.set_visible(expanded);
    root.append(&detail);
    AudioCardHandle {
        root,
        input_id: input.id.clone(),
        mute_btn,
        mute_signal_id,
        vol_scale: controls.scale,
        db_label: controls.db_label,
        volume_debouncer: controls.debouncer,
        volume_debounce_source: controls.debounce_source,
        vol_signal_id: controls.signal_id,
        observed_volume: Cell::new((
            AudioService::sanitize_volume_mul(input.volume_mul),
            AudioService::sanitize_volume_db(input.volume_db),
        )),
    }
}

pub(crate) fn set_compact_expanded(root: &gtk4::Widget, expanded: bool) {
    let mut child = root.first_child();
    while let Some(widget) = child {
        if widget.widget_name() == "live-audio-detail" {
            widget.set_visible(expanded);
        }
        child = widget.next_sibling();
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
    let owned_id = input_id.to_string();
    let owned_icon = icon.map(str::to_string);
    crate::ui::persist::persist_registry_field(
        nav,
        "audio source icon",
        |registry| registry.set_input_icon(input_id, icon),
        move || registry_storage::set_input_icon(&owned_id, owned_icon.as_deref()),
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
                scope: &controls.scope,
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
    if !context.scope.is_current(nav) {
        return;
    }
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

            let scope = context.scope.clone();
            let nav = nav.clone();
            let input_id = input_id.to_string();
            let debouncer = context.debouncer.clone();
            let debounce_source = context.debounce_source.clone();
            let debounce_source_for_timeout = debounce_source.clone();
            let source_id = timeout_add_local_once(VOLUME_SLIDER_DEBOUNCE, move || {
                debounce_source_for_timeout.borrow_mut().take();
                if !scope.is_current(&nav) {
                    return;
                }
                if let Some(volume_mul) = debouncer.borrow_mut().take_due() {
                    dispatch_volume(&nav, &scope, &input_id, volume_mul);
                }
            });
            *debounce_source.borrow_mut() = Some(source_id);
        }
        VolumeDispatch::Immediate => {
            if let Some(source_id) = context.debounce_source.borrow_mut().take() {
                source_id.remove();
            }
            context.debouncer.borrow_mut().mark_sent(volume_mul);
            dispatch_volume(nav, context.scope, input_id, volume_mul);
        }
    }
}

fn dispatch_volume(
    nav: &NavigationContext,
    scope: &AudioTargetScope,
    input_id: &str,
    volume_mul: f64,
) {
    nav.dispatch(AppCommand::SetInputVolume {
        scope: scope.scope.clone(),
        input: input_id.to_string(),
        volume_mul,
    });
    #[cfg(test)]
    tests::observe_gain_dispatch(&scope.scope, input_id, volume_mul);
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
    use super::{AudioTargetScope, OBS_FADER_INVERTED, OBS_FADER_MARKS_DB};
    use crate::domain::workflow::ScopeBinding;

    type GainDispatch = (ScopeBinding, String, f64);

    thread_local! {
        static GAIN_DISPATCHES: std::cell::RefCell<Option<Vec<GainDispatch>>> = const {
            std::cell::RefCell::new(None)
        };
    }

    // Observe calls at the actual NavigationContext dispatch boundary, not a
    // cloned debouncer. No controller/OBS connection is replaced by this hook.
    pub(super) fn observe_gain_dispatch(scope: &ScopeBinding, input: &str, gain: f64) {
        GAIN_DISPATCHES.with_borrow_mut(|dispatches| {
            if let Some(dispatches) = dispatches {
                dispatches.push((scope.clone(), input.into(), gain));
            }
        });
    }

    #[test]
    #[ignore = "requires a native GTK display"]
    fn native_multiple_outstanding_same_input_echoes_preserve_dispatched_gain() {
        crate::ui::test_support::native_test(
            "ui::widgets::audio_card::tests::native_multiple_outstanding_same_input_echoes_preserve_dispatched_gain",
            || {
                use super::*;
                use crate::ui::test_support::{navigation, pump};
                use std::time::Duration;

                fn minus_button(widget: &gtk4::Widget) -> Option<Button> {
                    if let Some(button) = widget.downcast_ref::<Button>() {
                        if button.label().as_deref() == Some("-") {
                            return Some(button.clone());
                        }
                    }
                    let mut child = widget.first_child();
                    while let Some(widget) = child {
                        if let Some(button) = minus_button(&widget) {
                            return Some(button);
                        }
                        child = widget.next_sibling();
                    }
                    None
                }

                let (nav, _stack, _runtime, _events) = navigation();
                {
                    let mut state = nav.state.borrow_mut();
                    state.obs_status = ObsStatus::Connected { obs_version: "fixture".into() };
                    state.scene_collections.current = Some("Fixture".into());
                }
                let input = AudioInput::new("Mic".into(), false, 1.0, 0.0);
                for compact in [false, true] {
                    for immediate_b in [false, true] {
                        for reverse_echoes in [false, true] {
                            GAIN_DISPATCHES.with_borrow_mut(|log| *log = Some(Vec::new()));
                            let scope = nav.state.borrow().workflow_scope();
                            let card = if compact {
                                build_compact(&input, nav.clone(), true)
                            } else {
                                build(&input, nav.clone())
                            };
                            let a = AudioService::volume_db_to_mul(-6.0);
                            card.vol_scale.set_value(-6.0);
                            pump(VOLUME_SLIDER_DEBOUNCE + Duration::from_millis(30));
                            let b_db = if immediate_b {
                                minus_button(card.root.upcast_ref()).unwrap().emit_clicked();
                                -7.0
                            } else {
                                card.vol_scale.set_value(-9.0);
                                pump(VOLUME_SLIDER_DEBOUNCE + Duration::from_millis(30));
                                -9.0
                            };
                            let b = AudioService::volume_db_to_mul(b_db);
                            let assert_dispatched = |gains: &[f64]| {
                                GAIN_DISPATCHES.with_borrow(|log| {
                                    let log = log.as_ref().unwrap();
                                    assert_eq!(log.len(), gains.len(), "unexpected volume dispatch count");
                                    for ((sent_scope, input, gain), expected) in log.iter().zip(gains) {
                                        assert_eq!(sent_scope, &scope);
                                        assert_eq!(input, "Mic");
                                        assert!((gain - expected).abs() < 1e-12, "wrong dispatched gain: {gain} != {expected}");
                                    }
                                });
                            };
                            assert_dispatched(&[a, b]);

                            card.vol_scale.set_value(-12.0);
                            for echo in if reverse_echoes { [b, a] } else { [a, b] } {
                                let echo = f64::from(echo as f32);
                                card.update_volume(echo, AudioService::volume_mul_to_db(echo));
                                assert_eq!(card.vol_scale.value(), -12.0, "own echo moved the newer same-input fader");
                                assert_eq!(card.db_label.text(), "-12.0 dB");
                                assert!(card.volume_debounce_source.borrow().is_some(), "own echo cancelled C's timer");
                            }
                            pump(VOLUME_SLIDER_DEBOUNCE + Duration::from_millis(30));
                            let c = AudioService::volume_db_to_mul(-12.0);
                            assert_dispatched(&[a, b, c]);

                            // Unmatched actual external observations still win.
                            card.vol_scale.set_value(-15.0);
                            card.update_volume(AudioService::volume_db_to_mul(-3.0), -3.0);
                            assert!((card.vol_scale.value() + 3.0).abs() < 0.0001);
                            assert!(card.volume_debounce_source.borrow().is_none());
                            pump(VOLUME_SLIDER_DEBOUNCE + Duration::from_millis(30));
                            assert_dispatched(&[a, b, c]);

                            // Scope invalidation must not dispatch even with retained sends.
                            card.vol_scale.set_value(-18.0);
                            nav.state.borrow_mut().workspace.epoch += 1;
                            pump(VOLUME_SLIDER_DEBOUNCE + Duration::from_millis(30));
                            assert!(card.volume_debounce_source.borrow().is_none());
                            assert_dispatched(&[a, b, c]);
                            GAIN_DISPATCHES.with_borrow_mut(|log| *log = None);
                        }
                    }
                }
            },
        );
    }

    #[test]
    #[ignore = "requires a native GTK display"]
    fn native_same_input_delayed_echo_preserves_pending_gain_and_timer() {
        crate::ui::test_support::native_test(
            "ui::widgets::audio_card::tests::native_same_input_delayed_echo_preserves_pending_gain_and_timer",
            || {
                use super::*;
                use crate::ui::test_support::{navigation, pump};
                use std::time::Duration;

                let (nav, _stack, _runtime, _events) = navigation();
                {
                    let mut state = nav.state.borrow_mut();
                    state.obs_status = ObsStatus::Connected { obs_version: "fixture".into() };
                    state.scene_collections.current = Some("Fixture".into());
                }
                let input = AudioInput::new("Mic".into(), false, 1.0, 0.0);
                for compact in [false, true] {
                    let card = if compact {
                        build_compact(&input, nav.clone(), true)
                    } else {
                        build(&input, nav.clone())
                    };
                    card.vol_scale.set_value(-6.0);
                    pump(VOLUME_SLIDER_DEBOUNCE + Duration::from_millis(30));
                    assert!(card.volume_debounce_source.borrow().is_none());
                    card.vol_scale.set_value(-9.0);
                    let sent = f64::from(AudioService::volume_db_to_mul(-6.0) as f32);
                    card.update_volume(sent, AudioService::volume_mul_to_db(sent));
                    assert_eq!(card.vol_scale.value(), -9.0, "older same-input echo moved the fader");
                    assert_eq!(card.db_label.text(), "-9.0 dB");
                    assert!(card.volume_debounce_source.borrow().is_some(), "older echo cancelled the newer timer");
                    pump(VOLUME_SLIDER_DEBOUNCE + Duration::from_millis(30));
                    assert!(card.volume_debounce_source.borrow().is_none());
                    let mut sent_state = card.volume_debouncer.borrow().clone();
                    sent_state.queue(AudioService::volume_db_to_mul(-9.0));
                    assert_eq!(sent_state.take_due(), None, "newer gain was not sent");

                    card.vol_scale.set_value(-12.0);
                    let external = AudioService::volume_db_to_mul(-3.0);
                    card.update_volume(external, -3.0);
                    assert!((card.vol_scale.value() + 3.0).abs() < 0.0001);
                    assert!(card.volume_debounce_source.borrow().is_none());
                    assert_eq!(card.volume_debouncer.borrow_mut().take_due(), None);

                    card.vol_scale.set_value(-15.0);
                    card.set_available(false);
                    pump(VOLUME_SLIDER_DEBOUNCE + Duration::from_millis(30));
                    assert!(card.volume_debounce_source.borrow().is_none());
                    assert_eq!(card.volume_debouncer.borrow_mut().take_due(), None, "unavailable input retained an unsent gain");
                    assert!((card.vol_scale.value() + 3.0).abs() < 0.0001);
                    card.set_available(true);
                    card.update_volume(external, -3.0);
                    assert!((card.vol_scale.value() + 3.0).abs() < 0.0001);
                    card.vol_scale.set_value(-15.0);
                    nav.state.borrow_mut().workspace.epoch += 1;
                    pump(VOLUME_SLIDER_DEBOUNCE + Duration::from_millis(30));
                    assert!(card.volume_debounce_source.borrow().is_none());
                    assert_eq!(card.volume_debouncer.borrow_mut().take_due(), Some(AudioService::volume_db_to_mul(-15.0)), "stale scope consumed a send");
                    let root = card.root.downgrade();
                    drop(card);
                    pump(Duration::from_millis(10));
                    assert!(root.upgrade().is_none());
                }
            },
        );
    }

    #[test]
    fn audio_callbacks_reject_another_session_collection_and_endpoint() {
        let scope = ScopeBinding {
            endpoint: "obs:4455".into(),
            collection: "Show".into(),
        };
        let binding = AudioTargetScope {
            epoch: 3,
            scope: scope.clone(),
        };
        assert!(binding.accepts(3, &scope, true));
        assert!(!binding.accepts(4, &scope, true));
        assert!(!binding.accepts(3, &scope, false));
        assert!(!binding.accepts(
            3,
            &ScopeBinding {
                endpoint: "other:4455".into(),
                ..scope.clone()
            },
            true
        ));
        assert!(!binding.accepts(
            3,
            &ScopeBinding {
                collection: "Other".into(),
                ..scope
            },
            true
        ));
    }

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
