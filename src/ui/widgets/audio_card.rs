//! Audio input card for the Live page mixer.

use std::{cell::RefCell, rc::Rc};

use adw::prelude::*;
use glib::{source::SourceId, timeout_add_local_once};
use gtk4::{Align, Box as GtkBox, Button, Label, Orientation, Scale, ToggleButton};

use crate::controller::command::AppCommand;
use crate::domain::audio::AudioInput;
use crate::services::audio_service::{AudioService, VolumeChangeDebouncer, VOLUME_SLIDER_DEBOUNCE};
use crate::ui::navigation::NavigationContext;

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

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build a single mixer card for `input` and return a handle.
pub(crate) fn build(input: &AudioInput, nav: NavigationContext) -> AudioCardHandle {
    let input_id = input.id.clone();

    // ── Mute toggle ───────────────────────────────────────────────────────────
    let mute_btn = ToggleButton::builder().active(input.muted).build();
    mute_btn.set_tooltip_text(Some("Mute input"));
    apply_mute_style(&mute_btn, input.muted);

    let mute_signal_id = {
        let nav = nav.clone();
        let input_id = input_id.clone();
        mute_btn.connect_toggled(move |btn| {
            nav.dispatch(AppCommand::SetInputMute {
                input: input_id.clone(),
                muted: btn.is_active(),
            });
        })
    };

    // ── Name label ────────────────────────────────────────────────────────────
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
    if let Some(path) = input.source_path_label() {
        name_label.set_tooltip_text(Some(&format!("{}: {path}", input.source_scope.label())));
    } else {
        name_label.set_tooltip_text(Some(input.source_scope.label()));
    }

    let scope_badge = Label::builder()
        .label(input.source_scope.label())
        .halign(Align::Start)
        .build();
    scope_badge.add_css_class("audio-source-badge");
    scope_badge.add_css_class(input.source_scope.css_class());

    // ── Volume scale ──────────────────────────────────────────────────────────
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
    vol_scale.set_height_request(128);
    vol_scale.set_width_request(22);
    vol_scale.add_css_class("audio-volume-fader");
    add_obs_fader_marks(&vol_scale);
    vol_scale.set_tooltip_text(Some("Volume fader"));

    // ── dB label ──────────────────────────────────────────────────────────────
    let db_label = Label::builder()
        .label(AudioService::format_db(AudioService::sanitize_volume_db(
            input.volume_db,
        )))
        .width_chars(7)
        .xalign(0.5)
        .halign(Align::Center)
        .build();
    db_label.add_css_class("numeric");
    db_label.add_css_class("dim-label");

    let volume_debouncer = Rc::new(RefCell::new(VolumeChangeDebouncer::new(input.volume_mul)));
    let volume_debounce_source = Rc::new(RefCell::new(None));

    let vol_signal_id = {
        let nav = nav.clone();
        let input_id = input_id.clone();
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

    // ── Lock control ─────────────────────────────────────────────────────────
    let lock_btn = ToggleButton::builder()
        .icon_name("changes-prevent-symbolic")
        .active(input.locked_locally)
        .build();
    lock_btn.set_tooltip_text(Some("Lock volume slider"));
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
    apply_lock_style(&lock_btn, input.locked_locally);

    let controls = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(1)
        .halign(Align::Center)
        .valign(Align::Center)
        .build();
    controls.add_css_class("audio-card-controls");
    controls.append(&mute_btn);
    controls.append(&lock_btn);

    let slider_col = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(1)
        .halign(Align::Center)
        .valign(Align::Center)
        .build();
    slider_col.add_css_class("audio-card-slider");
    let fader_row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(4)
        .halign(Align::Center)
        .valign(Align::Center)
        .build();
    fader_row.add_css_class("audio-fader-row");
    let meter = build_obs_meter();
    fader_row.append(&vol_scale);
    fader_row.append(&meter);
    slider_col.append(&fader_row);
    slider_col.append(&db_label);

    let vol_signal_id = Rc::new(vol_signal_id);

    let fine_controls = build_fine_controls(
        input,
        &vol_scale,
        &db_label,
        vol_signal_id.clone(),
        nav.clone(),
        volume_debouncer.clone(),
        volume_debounce_source.clone(),
    );

    let body = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(2)
        .halign(Align::Center)
        .valign(Align::Center)
        .build();
    body.append(&controls);
    body.append(&slider_col);

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

    root.append(&scope_badge);
    root.append(&name_label);
    root.append(&body);
    root.append(&fine_controls);

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

fn add_obs_fader_marks(vol_scale: &Scale) {
    for (db, _) in OBS_FADER_MARKS_DB {
        vol_scale.add_mark(*db, gtk4::PositionType::Right, None);
    }
}

fn build_obs_meter() -> GtkBox {
    let meter = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(3)
        .halign(Align::Center)
        .valign(Align::Center)
        .height_request(128)
        .build();
    meter.add_css_class("audio-meter-ruler");

    let bar = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .width_request(10)
        .height_request(128)
        .build();
    bar.add_css_class("audio-meter-bar");
    meter.append(&bar);

    let labels = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .homogeneous(true)
        .height_request(128)
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

    meter.append(&labels);
    meter
}

fn build_fine_controls(
    input: &AudioInput,
    vol_scale: &Scale,
    db_label: &Label,
    vol_signal_id: Rc<glib::SignalHandlerId>,
    nav: NavigationContext,
    volume_debouncer: Rc<RefCell<VolumeChangeDebouncer>>,
    volume_debounce_source: Rc<RefCell<Option<SourceId>>>,
) -> GtkBox {
    let controls = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(1)
        .halign(Align::Center)
        .valign(Align::Center)
        .build();
    controls.add_css_class("audio-fine-controls");

    let plus = Button::builder().label("+").tooltip_text("+1 dB").build();
    let reset = Button::builder()
        .icon_name("edit-undo-symbolic")
        .tooltip_text("Reset to 0.0 dB")
        .build();
    let minus = Button::builder().label("-").tooltip_text("-1 dB").build();

    for button in [&plus, &reset, &minus] {
        button.add_css_class("flat");
        button.add_css_class("circular");
    }

    plus.connect_clicked({
        let nav = nav.clone();
        let input_id = input.id.clone();
        let vol_scale = vol_scale.clone();
        let db_label = db_label.clone();
        let vol_signal_id = vol_signal_id.clone();
        let debouncer = volume_debouncer.clone();
        let debounce_source = volume_debounce_source.clone();
        move |_| {
            let context = VolumeChangeContext {
                vol_scale: &vol_scale,
                db_label: &db_label,
                vol_signal_id: Some(vol_signal_id.as_ref()),
                debouncer: &debouncer,
                debounce_source: &debounce_source,
            };
            dispatch_db_adjust(&nav, &input_id, vol_scale.value(), 1.0, context)
        }
    });

    reset.connect_clicked({
        let nav = nav.clone();
        let input_id = input.id.clone();
        let vol_scale = vol_scale.clone();
        let db_label = db_label.clone();
        let vol_signal_id = vol_signal_id.clone();
        let debouncer = volume_debouncer.clone();
        let debounce_source = volume_debounce_source.clone();
        move |_| {
            let context = VolumeChangeContext {
                vol_scale: &vol_scale,
                db_label: &db_label,
                vol_signal_id: Some(vol_signal_id.as_ref()),
                debouncer: &debouncer,
                debounce_source: &debounce_source,
            };
            apply_volume_change(&nav, &input_id, 1.0, VolumeDispatch::Immediate, context);
        }
    });

    minus.connect_clicked({
        let nav = nav.clone();
        let input_id = input.id.clone();
        let vol_scale = vol_scale.clone();
        let db_label = db_label.clone();
        let vol_signal_id = vol_signal_id.clone();
        let debouncer = volume_debouncer.clone();
        let debounce_source = volume_debounce_source.clone();
        move |_| {
            let context = VolumeChangeContext {
                vol_scale: &vol_scale,
                db_label: &db_label,
                vol_signal_id: Some(vol_signal_id.as_ref()),
                debouncer: &debouncer,
                debounce_source: &debounce_source,
            };
            dispatch_db_adjust(&nav, &input_id, vol_scale.value(), -1.0, context)
        }
    });

    controls.append(&plus);
    controls.append(&reset);
    controls.append(&minus);
    controls
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
        assert!(OBS_FADER_INVERTED);
    }
}
