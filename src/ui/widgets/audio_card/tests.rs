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
    crate::ui::native_test!(|| {
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
            state.obs_status = ObsStatus::Connected {
                obs_version: "fixture".into(),
            };
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
                                assert!(
                                    (gain - expected).abs() < 1e-12,
                                    "wrong dispatched gain: {gain} != {expected}"
                                );
                            }
                        });
                    };
                    assert_dispatched(&[a, b]);

                    card.vol_scale.set_value(-12.0);
                    for echo in if reverse_echoes { [b, a] } else { [a, b] } {
                        let echo = f64::from(echo as f32);
                        card.update_volume(echo, AudioService::volume_mul_to_db(echo));
                        assert_eq!(
                            card.vol_scale.value(),
                            -12.0,
                            "own echo moved the newer same-input fader"
                        );
                        assert_eq!(card.db_label.text(), "-12.0 dB");
                        assert!(
                            card.volume_debounce_source.borrow().is_some(),
                            "own echo cancelled C's timer"
                        );
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
    });
}

#[test]
#[ignore = "requires a native GTK display"]
fn native_same_input_delayed_echo_preserves_pending_gain_and_timer() {
    crate::ui::native_test!(|| {
        use super::*;
        use crate::ui::test_support::{navigation, pump};
        use std::time::Duration;

        let (nav, _stack, _runtime, _events) = navigation();
        {
            let mut state = nav.state.borrow_mut();
            state.obs_status = ObsStatus::Connected {
                obs_version: "fixture".into(),
            };
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
            assert_eq!(
                card.vol_scale.value(),
                -9.0,
                "older same-input echo moved the fader"
            );
            assert_eq!(card.db_label.text(), "-9.0 dB");
            assert!(
                card.volume_debounce_source.borrow().is_some(),
                "older echo cancelled the newer timer"
            );
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
            assert_eq!(
                card.volume_debouncer.borrow_mut().take_due(),
                None,
                "unavailable input retained an unsent gain"
            );
            assert!((card.vol_scale.value() + 3.0).abs() < 0.0001);
            card.set_available(true);
            card.update_volume(external, -3.0);
            assert!((card.vol_scale.value() + 3.0).abs() < 0.0001);
            card.vol_scale.set_value(-15.0);
            nav.state.borrow_mut().workspace.epoch += 1;
            pump(VOLUME_SLIDER_DEBOUNCE + Duration::from_millis(30));
            assert!(card.volume_debounce_source.borrow().is_none());
            assert_eq!(
                card.volume_debouncer.borrow_mut().take_due(),
                Some(AudioService::volume_db_to_mul(-15.0)),
                "stale scope consumed a send"
            );
            let root = card.root.downgrade();
            drop(card);
            pump(Duration::from_millis(10));
            assert!(root.upgrade().is_none());
        }
    });
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
