//! Selected-input expectations: fresh observations are required for both failure and recovery.
use super::*;
use crate::domain::{
    audio::AudioInput,
    meter::InputLevelSnapshot,
    operator_input::{resolve_input, InputIdentity},
};

const METER_GAP: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AudioState {
    Disabled,
    Unavailable,
    Missing,
    Monitoring,
    Muted,
}

pub(crate) struct AudioSnapshot<'a> {
    pub inputs: Option<&'a [InputIdentity]>,
    pub audio: &'a [AudioInput],
    pub levels: &'a InputLevelSnapshot,
}

#[derive(Debug, Clone)]
struct Watch {
    name: String,
    state: AudioState,
    last_meter: Option<Instant>,
    silence: Sustained,
    clipping: Sustained,
}
impl Watch {
    fn new(rule: &AudioRule) -> Self {
        Self {
            name: rule.target.name.clone(),
            state: AudioState::Unavailable,
            last_meter: None,
            silence: Sustained::new(METER_GAP, Duration::from_secs(15)),
            clipping: Sustained::new(METER_GAP, Duration::from_secs(1)),
        }
    }
    fn unavailable(&mut self, state: AudioState) {
        self.state = state;
        self.silence.unavailable();
        self.clipping.unavailable();
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct AudioMonitor {
    scope: Option<(u64, u64)>,
    rules: Vec<AudioRule>,
    started: Option<Instant>,
    watches: Vec<Watch>,
}
impl AudioMonitor {
    pub fn observe(
        &mut self,
        scope: Option<(u64, u64)>,
        rules: &[AudioRule],
        data: AudioSnapshot<'_>,
        now: Instant,
    ) {
        let rules = if scope.is_some() { rules } else { &[] };
        if self.scope != scope || self.rules != rules {
            self.scope = scope;
            self.rules = rules.to_vec();
            self.started = Some(now);
            self.watches = rules.iter().map(Watch::new).collect();
        }
        for (rule, watch) in rules.iter().zip(&mut self.watches) {
            if !rule.enabled || (!rule.silence && !rule.clipping) {
                watch.unavailable(AudioState::Disabled);
                continue;
            }
            if rules
                .iter()
                .filter(|other| {
                    rule.target
                        .matches(&other.target.name, other.target.uuid.as_deref())
                })
                .count()
                != 1
            {
                watch.unavailable(AudioState::Unavailable);
                continue;
            }
            let Some(inputs) = data.inputs else {
                watch.unavailable(AudioState::Unavailable);
                continue;
            };
            let Some(input) = resolve_input(inputs, &rule.target) else {
                watch.unavailable(AudioState::Missing);
                continue;
            };
            watch.name = input.target.name.clone();
            let mut matches = data
                .audio
                .iter()
                .filter(|audio| input.target.matches(&audio.id, audio.uuid.as_deref()));
            let audio = matches.next().filter(|_| matches.next().is_none());
            let reading = data
                .levels
                .get(&input.target.name)
                .zip(data.levels.observed_at)
                .filter(|(levels, at)| {
                    levels.valid
                        && !levels.channels.is_empty()
                        && levels.channels.iter().all(|ch| {
                            [ch.peak_db, ch.input_peak_db]
                                .into_iter()
                                .all(|db| db.is_finite() || db == f64::NEG_INFINITY)
                        })
                        && self.started.is_some_and(|start| *at >= start)
                        && fresh_within(*at, now, METER_GAP)
                });
            let (Some(audio), Some((levels, at))) = (audio, reading) else {
                watch.unavailable(AudioState::Unavailable);
                continue;
            };
            if watch.last_meter.is_some_and(|last| at <= last) {
                continue;
            }
            watch.last_meter = Some(at);
            watch.state = if audio.muted {
                AudioState::Muted
            } else {
                AudioState::Monitoring
            };
            watch.silence.observe(
                Some(
                    rule.silence
                        && (audio.muted || levels.channels.iter().all(|ch| ch.peak_db <= -50.0)),
                ),
                at,
            );
            watch.clipping.observe(
                Some(
                    rule.clipping
                        && !audio.muted
                        && levels.channels.iter().any(|ch| ch.input_peak_db >= 0.0),
                ),
                at,
            );
        }
    }
    pub fn name(&self, index: usize) -> String {
        self.watches
            .get(index)
            .map(|watch| watch.name.clone())
            .unwrap_or_default()
    }
    pub fn states(&self, now: Instant) -> Vec<(String, AudioState)> {
        self.watches
            .iter()
            .map(|watch| {
                let state = if matches!(watch.state, AudioState::Monitoring | AudioState::Muted)
                    && watch
                        .last_meter
                        .is_none_or(|at| !fresh_within(at, now, METER_GAP))
                {
                    AudioState::Unavailable
                } else {
                    watch.state
                };
                (watch.name.clone(), state)
            })
            .collect()
    }
    pub(super) fn alerts(&self, now: Instant) -> Vec<Alert> {
        self.watches
            .iter()
            .enumerate()
            .flat_map(|(index, watch)| {
                [
                    (AlertKind::AudioSilence(index), watch.silence.phase(now)),
                    (AlertKind::AudioClipping(index), watch.clipping.phase(now)),
                ]
            })
            .filter_map(|(kind, phase)| {
                phase.map(|phase| Alert {
                    kind,
                    phase,
                    event_id: None,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{meter::InputLevels, operator_action::ActionTarget};

    struct Fixture {
        start: Instant,
        rules: Vec<AudioRule>,
        inputs: Vec<InputIdentity>,
        audio: Vec<AudioInput>,
        levels: InputLevelSnapshot,
        monitor: AudioMonitor,
    }
    impl Fixture {
        fn new() -> Self {
            let target = ActionTarget {
                name: "Mic".into(),
                uuid: Some("mic-id".into()),
            };
            Self {
                start: Instant::now(),
                rules: vec![AudioRule {
                    target: target.clone(),
                    enabled: true,
                    silence: true,
                    clipping: true,
                }],
                inputs: vec![InputIdentity {
                    target,
                    kind: "audio".into(),
                }],
                audio: vec![AudioInput::new("Mic".into(), false, 1.0, 0.0)
                    .with_identity(Some("mic-id".into()))],
                levels: InputLevelSnapshot::default(),
                monitor: AudioMonitor::default(),
            }
        }
        fn tick(&mut self, second: u64) {
            self.monitor.observe(
                Some((1, 1)),
                &self.rules,
                AudioSnapshot {
                    inputs: Some(&self.inputs),
                    audio: &self.audio,
                    levels: &self.levels,
                },
                self.start + Duration::from_secs(second),
            );
        }
        fn feed(&mut self, second: u64, peak: f32, pre_fader: f32) {
            self.levels.update_at(
                vec![InputLevels::from_mul(
                    self.inputs[0].target.name.clone(),
                    &[[peak, peak, pre_fader]],
                )],
                self.start + Duration::from_secs(second),
            );
            self.tick(second);
        }
        fn alerts(&self, second: u64) -> Vec<Alert> {
            self.monitor
                .alerts(self.start + Duration::from_secs(second))
        }
    }

    #[test]
    fn duplicate_rules_are_unavailable_until_resolved() {
        let mut f = Fixture::new();
        f.rules.push(f.rules[0].clone());
        for second in 0..=20 {
            f.feed(second, 0.0, 0.0);
            assert!(f.alerts(second).is_empty());
        }
        assert!(f
            .monitor
            .states(f.start + Duration::from_secs(20))
            .iter()
            .all(|(_, state)| *state == AudioState::Unavailable));
        f.rules.pop();
        f.feed(21, 0.0, 0.0);
        assert!(f.alerts(21).is_empty());
    }

    #[test]
    fn audio_failure_and_recovery_require_distinct_sustained_observations() {
        let mut f = Fixture::new();
        for second in 0..=15 {
            f.feed(second, 0.0, 0.1);
        }
        assert_eq!(
            f.alerts(15),
            [Alert {
                kind: AlertKind::AudioSilence(0),
                phase: AlertPhase::Active,
                event_id: None
            }]
        );
        for second in 16..=25 {
            f.feed(second, 0.1, 0.1);
            assert_eq!(f.alerts(second)[0].phase, AlertPhase::Recovering);
        }
        f.feed(26, 0.1, 0.1);
        assert!(f.alerts(26).is_empty());
        f.feed(27, 0.1, 1.0);
        assert!(f.alerts(27).is_empty());
        f.feed(28, 0.1, 1.0);
        assert_eq!(f.alerts(28)[0].kind, AlertKind::AudioClipping(0));
        f.feed(29, 0.1, 0.1);
        assert_eq!(f.alerts(29)[0].phase, AlertPhase::Recovering);
        f.feed(30, 0.1, 1.0);
        assert_eq!(f.alerts(30)[0].phase, AlertPhase::Active);
    }

    #[test]
    fn frozen_invalid_missing_and_delayed_meters_never_mean_silence_or_recovery() {
        let mut f = Fixture::new();
        f.feed(0, 0.0, 0.0);
        for second in 1..=20 {
            f.tick(second);
            assert!(f.alerts(second).is_empty());
        }
        assert_eq!(
            f.monitor.states(f.start + Duration::from_secs(20))[0].1,
            AudioState::Unavailable
        );
        for second in 21..=40 {
            f.feed(second, f32::NAN, f32::NAN);
            assert!(f.alerts(second).is_empty());
        }
        for second in 41..=56 {
            f.feed(second, 0.0, 0.0);
        }
        assert_eq!(f.alerts(56)[0].phase, AlertPhase::Active);
        // The UI's later clock cannot make an old healthy observation recover.
        for second in 57..=70 {
            f.tick(second);
            assert_eq!(
                f.alerts(second)[0].phase,
                if second <= 58 {
                    AlertPhase::Active
                } else {
                    AlertPhase::Unobserved
                }
            );
        }
        f.levels
            .update_at(vec![], f.start + Duration::from_secs(71));
        f.tick(71);
        assert_eq!(f.alerts(71)[0].phase, AlertPhase::Unobserved);
        for second in 72..=90 {
            f.feed(second, -1.0, 0.1);
            assert_eq!(f.alerts(second)[0].phase, AlertPhase::Unobserved);
        }
    }

    #[test]
    fn scope_configuration_and_enable_changes_retire_old_evidence() {
        let mut f = Fixture::new();
        for second in 0..=15 {
            f.feed(second, 0.0, 0.0);
        }
        f.rules[0].enabled = false;
        f.tick(16);
        assert!(f.alerts(16).is_empty());
        assert_eq!(
            f.monitor.states(f.start + Duration::from_secs(16))[0].1,
            AudioState::Disabled
        );
        f.rules[0].enabled = true;
        f.tick(17);
        assert_eq!(
            f.monitor.states(f.start + Duration::from_secs(17))[0].1,
            AudioState::Unavailable
        );
        for second in 18..=32 {
            f.feed(second, 0.0, 0.0);
            assert!(f.alerts(second).is_empty());
        }
        f.feed(33, 0.0, 0.0);
        assert_eq!(f.alerts(33).len(), 1);
        f.monitor.observe(
            None,
            &f.rules,
            AudioSnapshot {
                inputs: Some(&f.inputs),
                audio: &f.audio,
                levels: &f.levels,
            },
            f.start + Duration::from_secs(34),
        );
        assert!(f.monitor.states(f.start).is_empty());
        assert!(f.alerts(34).is_empty());
        f.tick(35);
        assert!(f.alerts(35).is_empty());
    }

    #[test]
    fn uuid_renames_follow_current_names_but_name_reuse_and_ambiguity_are_missing() {
        let mut f = Fixture::new();
        f.feed(0, 0.1, 0.1);
        f.inputs[0].target.name = "Renamed".into();
        f.audio[0].id = "Renamed".into();
        f.feed(1, 0.1, 0.1);
        assert_eq!(f.monitor.name(0), "Renamed");
        assert_eq!(
            f.monitor.states(f.start + Duration::from_secs(1))[0].1,
            AudioState::Monitoring
        );
        f.inputs[0].target.name = "Mic".into();
        f.inputs[0].target.uuid = Some("replacement".into());
        f.feed(2, 0.0, 0.0);
        assert_eq!(
            f.monitor.states(f.start + Duration::from_secs(2))[0].1,
            AudioState::Missing
        );
        f.inputs[0].target = f.rules[0].target.clone();
        f.inputs.push(f.inputs[0].clone());
        f.feed(3, 0.0, 0.0);
        assert_eq!(
            f.monitor.states(f.start + Duration::from_secs(3))[0].1,
            AudioState::Missing
        );
        assert!(f.alerts(3).is_empty());
    }

    #[test]
    fn mute_is_a_silence_condition_only_with_fresh_identity_and_meter_evidence() {
        let mut f = Fixture::new();
        f.audio[0].muted = true;
        for second in 0..=15 {
            f.feed(second, 1.0, 1.0);
        }
        assert_eq!(
            f.alerts(15)
                .iter()
                .map(|alert| alert.kind)
                .collect::<Vec<_>>(),
            [AlertKind::AudioSilence(0)]
        );
        assert_eq!(
            f.monitor.states(f.start + Duration::from_secs(15))[0].1,
            AudioState::Muted
        );
        f.audio.clear();
        f.feed(16, 0.0, 0.0);
        assert_eq!(f.alerts(16)[0].phase, AlertPhase::Unobserved);
        f.monitor.observe(
            Some((1, 1)),
            &f.rules,
            AudioSnapshot {
                inputs: None,
                audio: &[],
                levels: &f.levels,
            },
            f.start + Duration::from_secs(17),
        );
        assert_eq!(f.alerts(17)[0].phase, AlertPhase::Unobserved);
    }

    #[test]
    fn long_gaps_and_replayed_timestamps_cannot_complete_a_pending_alert() {
        let mut f = Fixture::new();
        for second in 0..=10 {
            f.feed(second, 0.0, 0.0);
        }
        f.feed(15, 0.0, 0.0);
        assert!(f.alerts(15).is_empty());
        for second in 16..=29 {
            f.feed(second, 0.0, 0.0);
            assert!(f.alerts(second).is_empty());
        }
        f.levels.update_at(
            vec![InputLevels::from_mul("Mic".into(), &[[0.0; 3]])],
            f.start + Duration::from_secs(28),
        );
        f.tick(29);
        assert!(f.alerts(29).is_empty());
        f.feed(30, 0.0, 0.0);
        assert_eq!(f.alerts(30)[0].phase, AlertPhase::Active);
    }
}
