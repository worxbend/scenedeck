//! Observed media timing; polling updates values without rebuilding seek editors.

use adw::prelude::*;
use gtk4::{Box as GtkBox, Label, ProgressBar};
use i18n_embed_fl::fl;

use crate::domain::control::{MediaState, MediaStatus};
use crate::infra::i18n::LANGUAGE_LOADER;

pub(super) fn build() -> GtkBox {
    let root = super::column(3);
    let progress = ProgressBar::new();
    progress.set_widget_name("sources-media-progress");
    progress.update_property(&[gtk4::accessible::Property::Label(&fl!(
        LANGUAGE_LOADER,
        "media-progress-label"
    ))]);
    root.append(&progress);
    for name in ["position", "duration", "remaining"] {
        let label = super::text("");
        label.set_widget_name(&format!("sources-media-{name}"));
        root.append(&label);
    }
    let hint = super::text(&fl!(LANGUAGE_LOADER, "media-progress-hint"));
    hint.add_css_class("caption");
    root.append(&hint);
    root
}

// A missing/invalid cursor is not zero; zero/unknown duration is not completion.
fn timing(media: Option<&MediaStatus>, fresh: bool) -> (Option<i64>, Option<i64>) {
    let Some(media) = media.filter(|_| fresh) else {
        return (None, None);
    };
    let duration = media.duration_ms.filter(|value| *value > 0);
    let position = media.cursor_ms.filter(|value| {
        *value >= 0
            && duration.is_none_or(|duration| *value <= duration)
            && !matches!(
                media.state,
                MediaState::None | MediaState::Opening | MediaState::Error | MediaState::Unknown
            )
    });
    (position, duration)
}

fn clock(ms: i64) -> String {
    let seconds = ms / 1000;
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{:02}:{:02}", seconds / 60, seconds % 60)
    }
}

pub(super) fn update(root: &GtkBox, media: Option<&MediaStatus>, fresh: bool) {
    let (position, duration) = timing(media, fresh);
    let remaining = position
        .zip(duration)
        .map(|(position, duration)| duration - position);
    let unavailable = || fl!(LANGUAGE_LOADER, "media-time-unavailable");
    let position_text = fl!(
        LANGUAGE_LOADER,
        "media-position-observed",
        time = position.map(clock).unwrap_or_else(unavailable)
    );
    let duration_text = fl!(
        LANGUAGE_LOADER,
        "media-duration-observed",
        time = duration
            .map(clock)
            .unwrap_or_else(|| fl!(LANGUAGE_LOADER, "media-duration-unavailable"))
    );
    let remaining_text = fl!(
        LANGUAGE_LOADER,
        "media-time-remaining",
        time = remaining.map(clock).unwrap_or_else(unavailable)
    );
    super::visit(root.upcast_ref(), &mut |widget| {
        if let Some(label) = widget.downcast_ref::<Label>() {
            let value = match widget.widget_name().as_str() {
                "sources-media-position" => Some(&position_text),
                "sources-media-duration" => Some(&duration_text),
                "sources-media-remaining" => Some(&remaining_text),
                _ => None,
            };
            if let Some(value) = value {
                label.set_text(value);
            }
        }
        if widget.widget_name() == "sources-media-progress" {
            if let Some(progress) = widget.downcast_ref::<ProgressBar>() {
                let fraction = position
                    .zip(duration)
                    .map(|(position, duration)| position as f64 / duration as f64);
                progress.set_visible(fraction.is_some());
                progress.set_fraction(fraction.unwrap_or(0.0));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_timing_preserves_unknowns_and_rejects_invalid_positions() {
        let mut media = MediaStatus {
            state: MediaState::Playing,
            duration_ms: Some(10_000),
            cursor_ms: Some(2_000),
        };
        assert_eq!(timing(Some(&media), true), (Some(2_000), Some(10_000)));
        assert_eq!(timing(Some(&media), false), (None, None));
        for cursor in [None, Some(-1), Some(10_001)] {
            media.cursor_ms = cursor;
            assert_eq!(timing(Some(&media), true), (None, Some(10_000)));
        }
        media.cursor_ms = Some(2_000);
        for duration in [None, Some(0), Some(-1)] {
            media.duration_ms = duration;
            assert_eq!(timing(Some(&media), true), (Some(2_000), None));
        }
        for state in [
            MediaState::None,
            MediaState::Opening,
            MediaState::Error,
            MediaState::Unknown,
        ] {
            media.state = state;
            assert_eq!(timing(Some(&media), true), (None, None));
        }
        assert_eq!(clock(3_661_999), "1:01:01");
        assert_eq!(clock(0), "00:00");
    }

    #[test]
    #[ignore = "requires isolated GTK; synthetic media observations only"]
    fn native_media_progress_preserves_seek_drafts_and_marks_unavailable_timing() {
        crate::ui::test_support::native_test(
            "ui::widgets::operating_panel::media_progress::tests::native_media_progress_preserves_seek_drafts_and_marks_unavailable_timing",
            || {
                use std::time::{Duration, Instant};
                use crate::controller::state::{ObsStatus, Page};
                use crate::domain::control::{ControlSnapshot, SourceDetail};
                use crate::ui::test_support::{navigation, pump};
                fn named(root: &gtk4::Widget, name: &str) -> gtk4::Widget {
                    let mut found = None;
                    super::super::visit(root, &mut |widget| {
                        if widget.widget_name() == name { found = Some(widget.clone()); }
                    });
                    found.expect(name)
                }
                let (nav, _stack, _runtime, _events) = navigation();
                {
                    let mut state = nav.state.borrow_mut();
                    state.obs_status = ObsStatus::Connected { obs_version: "fixture".into() };
                    state.current_page = Page::Sources;
                    state.scene_collections.current = Some("Fixture".into());
                    state.available_requests = vec!["SetMediaInputCursor".into()];
                    state.workspace.snapshot = Some(ControlSnapshot::default());
                    state.workspace.selected_input = Some("Media".into());
                    state.workspace.detail = Some(SourceDetail {
                        input: "Media".into(),
                        media: Some(MediaStatus { state: MediaState::Playing, duration_ms: Some(10_000), cursor_ms: Some(2_000) }),
                        ..Default::default()
                    });
                }
                let (page, refresh) = super::super::build(nav.clone());
                let window = gtk4::Window::builder().default_width(1100).default_height(900).child(&page).build();
                window.present();
                pump(Duration::from_millis(100));
                {
                    let mut state = nav.state.borrow_mut();
                    state.workspace.dirty = false;
                    state.workspace.loading = false;
                    state.workspace.observed_at = Some(Instant::now());
                }
                refresh();
                pump(Duration::from_millis(100));
                let progress = named(&page, "sources-media-progress").downcast::<ProgressBar>().unwrap();
                let position = named(&page, "sources-media-position").downcast::<Label>().unwrap();
                let remaining = named(&page, "sources-media-remaining").downcast::<Label>().unwrap();
                let seek = named(&page, "control-focus-seek-position-Media").downcast::<gtk4::SpinButton>().unwrap();
                assert!(progress.is_visible());
                assert!((progress.fraction() - 0.2).abs() < 0.001);
                assert!(position.label().contains("00:02"));
                assert!(remaining.label().contains("00:08"));
                assert!(seek.grab_focus());
                seek.set_value(7.0);
                seek.set_text("7");
                assert_eq!(seek.text(), "7", "fixture uses the whole-second seek field");
                let focus = gtk4::prelude::RootExt::focus(&window).unwrap();
                let sequence = nav.state.borrow().workspace.sequence;
                for state in [MediaState::Playing, MediaState::Paused, MediaState::Buffering] {
                    {
                        let mut app = nav.state.borrow_mut();
                        let media = app.workspace.detail.as_mut().unwrap().media.as_mut().unwrap();
                        media.state = state;
                        media.cursor_ms = Some(3_000);
                    }
                    refresh();
                    assert_eq!(named(&page, "sources-media-progress"), progress.clone().upcast::<gtk4::Widget>());
                    assert_eq!(gtk4::prelude::RootExt::focus(&window).unwrap(), focus);
                    assert_eq!(seek.text(), "7");
                    assert!((progress.fraction() - 0.3).abs() < 0.001);
                    assert!(position.label().contains("00:03"));
                    assert!(remaining.label().contains("00:07"));
                }
                for cursor in [None, Some(-1), Some(10_001)] {
                    nav.state.borrow_mut().workspace.detail.as_mut().unwrap().media.as_mut().unwrap().cursor_ms = cursor;
                    refresh();
                    assert!(!progress.is_visible());
                    assert!(position.label().contains(&fl!(LANGUAGE_LOADER, "media-time-unavailable")));
                    assert!(remaining.label().contains(&fl!(LANGUAGE_LOADER, "media-time-unavailable")));
                    assert_eq!(seek.text(), "7");
                }
                {
                    let mut state = nav.state.borrow_mut();
                    state.workspace.detail.as_mut().unwrap().media.as_mut().unwrap().cursor_ms = Some(3_000);
                    state.workspace.observed_at = Some(Instant::now() - Duration::from_secs(40));
                }
                refresh();
                assert!(!progress.is_visible());
                assert!(!named(&page, "control-focus-seek-Media").is_sensitive());
                assert_eq!(seek.text(), "7");
                assert_eq!(nav.state.borrow().workspace.sequence, sequence, "readouts must never dispatch a seek");
                {
                    let mut state = nav.state.borrow_mut();
                    state.workspace.observed_at = Some(Instant::now());
                    state.workspace.detail.as_mut().unwrap().media.as_mut().unwrap().duration_ms = None;
                }
                refresh();
                let duration = named(&page, "sources-media-duration").downcast::<Label>().unwrap();
                assert!(duration.label().contains(&fl!(LANGUAGE_LOADER, "media-duration-unavailable")));
                assert!(named(&page, "sources-media-position").downcast::<Label>().unwrap().label().contains("00:03"));
                assert!(!named(&page, "sources-media-progress").is_visible());
                window.close();

                // The same readout fits narrow layouts with large text and long times.
                gtk4::Settings::default().unwrap().set_gtk_font_name(Some("Sans 22"));
                let readout = build();
                update(&readout, Some(&MediaStatus { state: MediaState::Paused, duration_ms: Some(360_000_000), cursor_ms: Some(36_000_000) }), true);
                let window = gtk4::Window::builder().default_width(320).default_height(600).child(&readout).build();
                window.present();
                pump(Duration::from_millis(100));
                assert!(window.width() <= 320, "readout must not force a wider window");
                super::super::visit(readout.upcast_ref(), &mut |widget| {
                    if widget.is_visible() {
                        if let Some(bounds) = widget.compute_bounds(&readout) {
                            assert!(bounds.x() >= -1.0 && bounds.x() + bounds.width() <= readout.width() as f32 + 1.0);
                        }
                    }
                });
                window.close();
            },
        );
    }
}
