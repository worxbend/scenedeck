//! Audio input card for the Live page mixer.

use adw::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Label, Orientation, Scale, ToggleButton};

use crate::controller::command::AppCommand;
use crate::domain::audio::AudioInput;
use crate::services::audio_service::AudioService;
use crate::ui::navigation::NavigationContext;

// ── Public handle ─────────────────────────────────────────────────────────────

pub(crate) struct AudioCardHandle {
    pub(crate) root: GtkBox,
    pub(crate) input_id: String,
    mute_btn: ToggleButton,
    vol_scale: Scale,
    db_label: Label,
    mute_signal_id: glib::SignalHandlerId,
    vol_signal_id: glib::SignalHandlerId,
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
        self.vol_scale.block_signal(&self.vol_signal_id);
        self.vol_scale.set_value(mul);
        self.vol_scale.unblock_signal(&self.vol_signal_id);
        self.db_label.set_text(&AudioService::format_db(db));
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
        .lines(2)
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
    let vol_scale = Scale::with_range(Orientation::Vertical, 0.0, 1.0, 0.01);
    vol_scale.set_value(input.volume_mul);
    vol_scale.set_inverted(true);
    vol_scale.set_draw_value(false);
    vol_scale.set_vexpand(true);
    vol_scale.set_height_request(90);
    vol_scale.set_width_request(24);
    vol_scale.add_mark(1.0, gtk4::PositionType::Right, None);
    vol_scale.set_tooltip_text(Some("Volume fader"));

    let vol_signal_id = {
        let nav = nav.clone();
        let input_id = input_id.clone();
        vol_scale.connect_value_changed(move |scale| {
            nav.dispatch(AppCommand::SetInputVolume {
                input: input_id.clone(),
                volume_mul: scale.value(),
            });
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
        .spacing(5)
        .halign(Align::Center)
        .valign(Align::Center)
        .build();
    controls.add_css_class("audio-card-controls");
    controls.append(&mute_btn);
    controls.append(&lock_btn);

    // ── dB label ──────────────────────────────────────────────────────────────
    let db_label = Label::builder()
        .label(AudioService::format_db(input.volume_db))
        .width_chars(8)
        .xalign(0.5)
        .halign(Align::Center)
        .build();
    db_label.add_css_class("numeric");
    db_label.add_css_class("dim-label");

    let slider_col = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .halign(Align::Center)
        .valign(Align::Center)
        .build();
    slider_col.add_css_class("audio-card-slider");
    slider_col.append(&vol_scale);
    slider_col.append(&db_label);

    let fine_controls = build_fine_controls(input, &vol_scale, nav.clone());

    let body = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(7)
        .halign(Align::Center)
        .valign(Align::Center)
        .build();
    body.append(&controls);
    body.append(&slider_col);
    body.append(&fine_controls);

    // ── Card ─────────────────────────────────────────────────────────────────
    let root = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .halign(Align::Fill)
        .hexpand(true)
        .width_request(100)
        .build();
    root.add_css_class("card");
    root.add_css_class("audio-card");

    root.append(&scope_badge);
    root.append(&name_label);
    root.append(&body);

    AudioCardHandle {
        root,
        input_id,
        mute_btn,
        vol_scale,
        db_label,
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

fn build_fine_controls(input: &AudioInput, vol_scale: &Scale, nav: NavigationContext) -> GtkBox {
    let controls = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
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
        move |_| dispatch_db_adjust(&nav, &input_id, vol_scale.value(), 1.0)
    });

    reset.connect_clicked({
        let nav = nav.clone();
        let input_id = input.id.clone();
        move |_| {
            nav.dispatch(AppCommand::SetInputVolume {
                input: input_id.clone(),
                volume_mul: 1.0,
            });
        }
    });

    minus.connect_clicked({
        let nav = nav.clone();
        let input_id = input.id.clone();
        let vol_scale = vol_scale.clone();
        move |_| dispatch_db_adjust(&nav, &input_id, vol_scale.value(), -1.0)
    });

    controls.append(&plus);
    controls.append(&reset);
    controls.append(&minus);
    controls
}

fn dispatch_db_adjust(nav: &NavigationContext, input_id: &str, current_mul: f64, delta_db: f64) {
    let current_db = AudioService::volume_mul_to_db(current_mul);
    let next_db = AudioService::adjust_volume_db(current_db, delta_db);
    nav.dispatch(AppCommand::SetInputVolume {
        input: input_id.to_string(),
        volume_mul: AudioService::volume_db_to_mul(next_db),
    });
}
