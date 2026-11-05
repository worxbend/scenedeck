//! Operator-defined readiness requirements, separate from the active collection.

use super::operator_action::ActionTarget;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct PreflightPlan {
    pub expected_collection: String,
    pub required_scenes: Vec<ActionTarget>,
    pub check_microphone: bool,
    pub microphone: Option<ActionTarget>,
    /// None explicitly skips the recording-space check; zero is invalid.
    pub minimum_disk_mib: Option<u64>,
    pub manual_items: Vec<String>,
    pub review_before_output: bool,
}

impl Default for PreflightPlan {
    fn default() -> Self {
        Self {
            expected_collection: String::new(),
            required_scenes: Vec::new(),
            check_microphone: true,
            microphone: None,
            minimum_disk_mib: Some(1024),
            manual_items: Vec::new(),
            review_before_output: false,
        }
    }
}

/// An expectation follows the connected production, not the OBS collection picker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SavedPreflight {
    pub connection: String,
    pub endpoint: String,
    pub plan: PreflightPlan,
}

use super::{
    audio::AudioInput,
    meter::InputLevelSnapshot,
    operator_input::{resolve_input, InputIdentity},
    scene::Scene,
};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CheckStatus {
    Passed,
    Failed,
    Unknown,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreflightReport {
    pub collection: CheckStatus,
    pub scenes: Vec<CheckStatus>,
    /// Presence, unmuted, then recent post-fader signal.
    pub microphone: [CheckStatus; 3],
    pub microphone_name: Option<String>,
    pub disk: CheckStatus,
}

pub(crate) struct PreflightSnapshot<'a> {
    pub connected: bool,
    pub fresh: bool,
    pub collection: Option<&'a str>,
    pub scenes: &'a [Scene],
    pub inputs: Option<&'a [InputIdentity]>,
    pub audio: &'a [AudioInput],
    pub levels: &'a InputLevelSnapshot,
    pub disk: Option<(f64, Instant)>,
}

/// Volatile evidence for one review. Never serialize checkmarks or signal latches.
pub(crate) struct PreflightReview {
    pub checked: Vec<bool>,
    started_at: Instant,
    signal_at: Option<Instant>,
}

fn fresh(at: Instant, now: Instant, limit: Duration) -> bool {
    now.checked_duration_since(at)
        .is_some_and(|age| age <= limit)
}
fn outcome(passed: bool) -> CheckStatus {
    if passed {
        CheckStatus::Passed
    } else {
        CheckStatus::Failed
    }
}

impl PreflightReview {
    pub fn new(plan: &PreflightPlan, now: Instant) -> Self {
        Self {
            checked: vec![false; plan.manual_items.len()],
            started_at: now,
            signal_at: None,
        }
    }

    pub fn evaluate(
        &mut self,
        plan: &PreflightPlan,
        data: PreflightSnapshot<'_>,
        now: Instant,
    ) -> PreflightReport {
        use CheckStatus::*;
        let collection = if !data.connected
            || !data.fresh
            || plan.expected_collection.is_empty()
            || data.collection.is_none()
        {
            Unknown
        } else {
            outcome(data.collection == Some(plan.expected_collection.as_str()))
        };
        let mut report = PreflightReport {
            collection,
            scenes: plan
                .required_scenes
                .iter()
                .map(|target| {
                    if collection != Passed {
                        return Unknown;
                    }
                    match data
                        .scenes
                        .iter()
                        .filter(|scene| target.matches(&scene.id, scene.uuid.as_deref()))
                        .count()
                    {
                        0 => Failed,
                        1 => Passed,
                        _ => Unknown,
                    }
                })
                .collect(),
            microphone: [Unknown; 3],
            microphone_name: None,
            disk: match plan.minimum_disk_mib {
                None => Skipped,
                Some(minimum) => data
                    .disk
                    .filter(|(space, at)| {
                        data.connected
                            && minimum > 0
                            && space.is_finite()
                            && *space >= 0.0
                            && fresh(*at, now, Duration::from_secs(5))
                    })
                    .map_or(Unknown, |(space, _)| outcome(space >= minimum as f64)),
            },
        };
        if !plan.check_microphone {
            report.microphone = [Skipped; 3];
        } else if collection == Passed {
            if let (Some(target), Some(inputs)) = (&plan.microphone, data.inputs) {
                if let Some(input) = resolve_input(inputs, target) {
                    report.microphone_name = Some(input.target.name.clone());
                    report.microphone[0] = Passed;
                    let mut audio = data
                        .audio
                        .iter()
                        .filter(|audio| input.target.matches(&audio.id, audio.uuid.as_deref()));
                    if let Some(input_audio) = audio.next().filter(|_| audio.next().is_none()) {
                        report.microphone[1] = outcome(!input_audio.muted);
                        if !input_audio.muted {
                            if let Some((levels, at)) = data
                                .levels
                                .get(&input.target.name)
                                .zip(data.levels.observed_at)
                                .filter(|(levels, at)| {
                                    *at >= self.started_at
                                        && fresh(*at, now, Duration::from_secs(2))
                                        && levels.valid
                                        && !levels.channels.is_empty()
                                        && levels.channels.iter().all(|channel| {
                                            channel.peak_db.is_finite()
                                                || channel.peak_db == f64::NEG_INFINITY
                                        })
                                })
                            {
                                if levels
                                    .channels
                                    .iter()
                                    .any(|channel| channel.peak_db > -50.0)
                                {
                                    self.signal_at = Some(at);
                                }
                                report.microphone[2] = outcome(
                                    self.signal_at
                                        .is_some_and(|at| fresh(at, now, Duration::from_secs(10))),
                                );
                            }
                        }
                    }
                } else if !inputs
                    .iter()
                    .any(|input| target.matches(&input.target.name, input.target.uuid.as_deref()))
                {
                    report.microphone[0] = Failed;
                }
            }
        }
        if report.microphone[1] != Passed {
            self.signal_at = None;
        }
        report
    }

    pub fn ready(&self, report: &PreflightReport) -> bool {
        std::iter::once(&report.collection)
            .chain(&report.scenes)
            .chain(&report.microphone)
            .chain(std::iter::once(&report.disk))
            .all(|status| matches!(status, CheckStatus::Passed | CheckStatus::Skipped))
            && self.checked.iter().all(|checked| *checked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::meter::{ChannelLevel, InputLevels};
    struct Fixture {
        plan: PreflightPlan,
        scenes: Vec<Scene>,
        inputs: Vec<InputIdentity>,
        audio: Vec<AudioInput>,
        levels: InputLevelSnapshot,
        start: Instant,
    }
    impl Fixture {
        fn new() -> Self {
            let start = Instant::now();
            let mic = ActionTarget {
                name: "Mic".into(),
                uuid: Some("mic-id".into()),
            };
            let mut fixture = Self {
                plan: PreflightPlan {
                    expected_collection: "Show".into(),
                    required_scenes: vec![ActionTarget {
                        name: "Camera".into(),
                        uuid: Some("scene-id".into()),
                    }],
                    microphone: Some(mic.clone()),
                    manual_items: vec!["Guests ready".into()],
                    ..Default::default()
                },
                scenes: vec![Scene {
                    id: "Camera".into(),
                    uuid: Some("scene-id".into()),
                }],
                inputs: vec![InputIdentity {
                    target: mic,
                    kind: "audio".into(),
                }],
                audio: vec![AudioInput::new("Mic".into(), false, 1.0, 0.0)
                    .with_identity(Some("mic-id".into()))],
                levels: InputLevelSnapshot::default(),
                start,
            };
            fixture.meter(-12.0, start);
            fixture
        }
        fn meter(&mut self, peak_db: f64, at: Instant) {
            self.levels.update_at(
                vec![InputLevels {
                    valid: true,
                    input: self.inputs[0].target.name.clone(),
                    channels: vec![ChannelLevel {
                        peak_db,
                        magnitude_db: peak_db,
                        input_peak_db: peak_db,
                    }],
                }],
                at,
            );
        }
        fn snapshot(&self) -> PreflightSnapshot<'_> {
            PreflightSnapshot {
                connected: true,
                fresh: true,
                collection: Some("Show"),
                scenes: &self.scenes,
                inputs: Some(&self.inputs),
                audio: &self.audio,
                levels: &self.levels,
                disk: Some((2048.0, self.start)),
            }
        }
        fn review(&self) -> PreflightReview {
            PreflightReview::new(&self.plan, self.start)
        }
    }
    #[test]
    fn requires_manual_confirmation_and_reset_requires_new_signal() {
        let fixture = Fixture::new();
        let mut review = fixture.review();
        let report = review.evaluate(&fixture.plan, fixture.snapshot(), fixture.start);
        assert_eq!(report.microphone, [CheckStatus::Passed; 3]);
        assert!(!review.ready(&report));
        review.checked[0] = true;
        assert!(review.ready(&report));
        let mut reset =
            PreflightReview::new(&fixture.plan, fixture.start + Duration::from_millis(1));
        let report = reset.evaluate(
            &fixture.plan,
            fixture.snapshot(),
            fixture.start + Duration::from_millis(1),
        );
        assert_eq!(report.microphone[2], CheckStatus::Unknown);
        assert_eq!(reset.checked, [false]);
    }
    #[test]
    fn wrong_collection_or_stale_workspace_never_checks_another_namespace() {
        let fixture = Fixture::new();
        let mut review = fixture.review();
        let mut data = fixture.snapshot();
        data.collection = Some("Other");
        let report = review.evaluate(&fixture.plan, data, fixture.start);
        assert_eq!(report.collection, CheckStatus::Failed);
        assert_eq!(report.scenes, [CheckStatus::Unknown]);
        assert_eq!(report.microphone, [CheckStatus::Unknown; 3]);
        for (connected, fresh) in [(false, true), (true, false)] {
            let mut data = fixture.snapshot();
            data.connected = connected;
            data.fresh = fresh;
            let report = review.evaluate(&fixture.plan, data, fixture.start);
            assert_eq!(report.collection, CheckStatus::Unknown);
            assert_eq!(report.microphone, [CheckStatus::Unknown; 3]);
            if !connected {
                assert_eq!(report.disk, CheckStatus::Unknown);
            }
        }
    }
    #[test]
    fn identities_survive_rename_but_not_name_reuse_or_ambiguity() {
        let mut fixture = Fixture::new();
        let mut review = fixture.review();
        fixture.scenes[0].id = "Renamed".into();
        fixture.inputs[0].target.name = "Renamed mic".into();
        fixture.audio[0].id = "Renamed mic".into();
        fixture.meter(-10.0, fixture.start);
        let report = review.evaluate(&fixture.plan, fixture.snapshot(), fixture.start);
        assert_eq!(report.scenes, [CheckStatus::Passed]);
        assert_eq!(report.microphone, [CheckStatus::Passed; 3]);
        assert_eq!(report.microphone_name.as_deref(), Some("Renamed mic"));
        fixture.scenes.push(fixture.scenes[0].clone());
        fixture.inputs.push(fixture.inputs[0].clone());
        let report = review.evaluate(&fixture.plan, fixture.snapshot(), fixture.start);
        assert_eq!(report.scenes, [CheckStatus::Unknown]);
        assert_eq!(report.microphone, [CheckStatus::Unknown; 3]);
        fixture.scenes.truncate(1);
        fixture.inputs.truncate(1);
        fixture.scenes[0].id = "Camera".into();
        fixture.scenes[0].uuid = Some("replacement".into());
        fixture.inputs[0].target.name = "Mic".into();
        fixture.inputs[0].target.uuid = Some("replacement".into());
        let report = review.evaluate(&fixture.plan, fixture.snapshot(), fixture.start);
        assert_eq!(report.scenes, [CheckStatus::Failed]);
        assert_eq!(report.microphone[0], CheckStatus::Failed);
    }
    #[test]
    fn silence_mute_and_missing_audio_reads_do_not_pass() {
        let mut fixture = Fixture::new();
        let mut review = fixture.review();
        fixture.meter(f64::NEG_INFINITY, fixture.start);
        assert_eq!(
            review
                .evaluate(&fixture.plan, fixture.snapshot(), fixture.start)
                .microphone[2],
            CheckStatus::Failed
        );
        fixture.meter(-10.0, fixture.start);
        fixture.audio[0].muted = true;
        assert_eq!(
            review
                .evaluate(&fixture.plan, fixture.snapshot(), fixture.start)
                .microphone[1..],
            [CheckStatus::Failed, CheckStatus::Unknown]
        );
        fixture.audio.clear();
        assert_eq!(
            review
                .evaluate(&fixture.plan, fixture.snapshot(), fixture.start)
                .microphone[1..],
            [CheckStatus::Unknown; 2]
        );
        let mut data = fixture.snapshot();
        data.inputs = None;
        assert_eq!(
            review
                .evaluate(&fixture.plan, data, fixture.start)
                .microphone,
            [CheckStatus::Unknown; 3]
        );
    }
    #[test]
    fn signal_latch_expires_and_needs_continuing_valid_meter_samples() {
        let mut fixture = Fixture::new();
        let mut review = fixture.review();
        review.evaluate(&fixture.plan, fixture.snapshot(), fixture.start);
        let later = fixture.start + Duration::from_secs(3);
        assert_eq!(
            review
                .evaluate(&fixture.plan, fixture.snapshot(), later)
                .microphone[2],
            CheckStatus::Unknown
        );
        fixture.meter(f64::NEG_INFINITY, later);
        assert_eq!(
            review
                .evaluate(&fixture.plan, fixture.snapshot(), later)
                .microphone[2],
            CheckStatus::Passed
        );
        let expired = fixture.start + Duration::from_secs(11);
        fixture.meter(f64::NEG_INFINITY, expired);
        assert_eq!(
            review
                .evaluate(&fixture.plan, fixture.snapshot(), expired)
                .microphone[2],
            CheckStatus::Failed
        );
        for invalid in [f64::NAN, f64::INFINITY] {
            fixture.meter(invalid, expired);
            assert_eq!(
                review
                    .evaluate(&fixture.plan, fixture.snapshot(), expired)
                    .microphone[2],
                CheckStatus::Unknown
            );
        }
        fixture.levels.clear();
        assert!(fixture.levels.observed_at.is_none());
        assert_eq!(
            review
                .evaluate(&fixture.plan, fixture.snapshot(), expired)
                .microphone[2],
            CheckStatus::Unknown
        );
    }
    #[test]
    fn disk_threshold_rejects_missing_stale_future_and_invalid_values() {
        let fixture = Fixture::new();
        let mut review = fixture.review();
        for (disk, expected) in [
            (None, CheckStatus::Unknown),
            (Some((0.0, fixture.start)), CheckStatus::Failed),
            (Some((1023.9, fixture.start)), CheckStatus::Failed),
            (Some((1024.0, fixture.start)), CheckStatus::Passed),
            (Some((f64::NAN, fixture.start)), CheckStatus::Unknown),
            (Some((f64::INFINITY, fixture.start)), CheckStatus::Unknown),
            (Some((-1.0, fixture.start)), CheckStatus::Unknown),
            (
                Some((2048.0, fixture.start - Duration::from_secs(6))),
                CheckStatus::Unknown,
            ),
            (
                Some((2048.0, fixture.start + Duration::from_secs(1))),
                CheckStatus::Unknown,
            ),
        ] {
            let mut data = fixture.snapshot();
            data.disk = disk;
            assert_eq!(
                review.evaluate(&fixture.plan, data, fixture.start).disk,
                expected
            );
        }
    }
    #[test]
    fn only_explicitly_disabled_checks_are_skipped() {
        let mut fixture = Fixture::new();
        let mut review = fixture.review();
        fixture.plan.minimum_disk_mib = Some(0);
        assert_eq!(
            review
                .evaluate(&fixture.plan, fixture.snapshot(), fixture.start)
                .disk,
            CheckStatus::Unknown
        );
        fixture.plan.minimum_disk_mib = None;
        fixture.plan.check_microphone = false;
        let report = review.evaluate(&fixture.plan, fixture.snapshot(), fixture.start);
        assert_eq!(report.disk, CheckStatus::Skipped);
        assert_eq!(report.microphone, [CheckStatus::Skipped; 3]);
        fixture.plan.check_microphone = true;
        fixture.plan.microphone = None;
        assert_eq!(
            review
                .evaluate(&fixture.plan, fixture.snapshot(), fixture.start)
                .microphone,
            [CheckStatus::Unknown; 3]
        );
    }
}
