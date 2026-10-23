//! Merge transport echoes over reads started before them, including bootstrap.
use std::collections::BTreeMap;

use super::event::AppEvent;
use crate::domain::audio::AudioInput;

const MAX_ECHO_INPUTS: usize = 4096;

#[derive(Debug, Clone, Default)]
struct InputEcho {
    muted: Option<(u64, bool)>,
    volume: Option<(u64, f64, f64)>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Observations {
    inputs: BTreeMap<String, InputEcho>,
    discarded_before: u64,
    program: Option<(u64, String)>,
    stream_revision: u64,
    record_revision: u64,
}

impl Observations {
    pub(crate) fn reconcile(&mut self, revision: u64, mut event: AppEvent) -> Option<AppEvent> {
        match &mut event {
            AppEvent::StreamStatusUpdated(_) => {
                if revision < self.stream_revision {
                    return None;
                }
                self.stream_revision = revision;
            }
            AppEvent::RecordStatusUpdated(_) => {
                if revision < self.record_revision {
                    return None;
                }
                self.record_revision = revision;
            }
            AppEvent::CurrentSceneChanged(scene) => self.program = Some((revision, scene.clone())),
            AppEvent::SceneInventoryUpdated(inventory) => {
                if let Some((echo_revision, scene)) = &self.program {
                    if *echo_revision > revision {
                        inventory.current_id = Some(scene.clone());
                    }
                }
            }
            AppEvent::InputMuteChanged { input, muted } => {
                self.admit(input, revision).muted = Some((revision, *muted));
            }
            AppEvent::InputVolumeChanged {
                input,
                volume_mul,
                volume_db,
            } => {
                self.admit(input, revision).volume = Some((revision, *volume_mul, *volume_db));
            }
            AppEvent::ActiveAudioObserved {
                result: Ok(inputs), ..
            }
            | AppEvent::MixerAudioInputsUpdated { inputs, .. } => {
                if revision < self.discarded_before {
                    // Never publish an old success if an adversarial input burst
                    // exceeded the bounded echo journal's coverage.
                    return Some(match event {
                        AppEvent::ActiveAudioObserved { scene, .. } => {
                            AppEvent::ActiveAudioObserved {
                                scene,
                                result: Err(
                                    "Audio changed during observation; refresh required".into()
                                ),
                            }
                        }
                        AppEvent::MixerAudioInputsUpdated { request, .. } => {
                            AppEvent::MixerAudioInputsFailed {
                                request,
                                message: "Audio changed during observation; refresh required"
                                    .into(),
                            }
                        }
                        _ => unreachable!(),
                    });
                }
                self.merge(revision, inputs);
            }
            _ => {}
        }
        Some(event)
    }

    fn admit(&mut self, input: &str, revision: u64) -> &mut InputEcho {
        if self.inputs.len() == MAX_ECHO_INPUTS && !self.inputs.contains_key(input) {
            self.inputs.clear();
            self.discarded_before = revision;
        }
        self.inputs.entry(input.to_owned()).or_default()
    }

    fn merge(&self, revision: u64, inputs: &mut [AudioInput]) {
        for input in inputs {
            let Some(echo) = self.inputs.get(&input.id) else {
                continue;
            };
            if let Some((echo_revision, muted)) = echo.muted {
                if echo_revision > revision {
                    input.muted = muted;
                }
            }
            if let Some((echo_revision, mul, db)) = echo.volume {
                if echo_revision > revision {
                    input.volume_mul = mul;
                    input.volume_db = db;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echo_budget_exhaustion_fails_old_observations_instead_of_publishing_partial_merge() {
        let mut observations = Observations::default();
        for input in 0..=MAX_ECHO_INPUTS {
            observations.reconcile(
                input as u64 + 2,
                AppEvent::InputMuteChanged {
                    input: format!("Mic{input}"),
                    muted: true,
                },
            );
        }
        assert!(observations.inputs.len() <= MAX_ECHO_INPUTS);
        let old = observations.reconcile(
            1,
            AppEvent::ActiveAudioObserved {
                scene: Some("A".into()),
                result: Ok(vec![AudioInput::new("Mic0".into(), false, 0.5, -6.0)]),
            },
        );
        assert!(matches!(
            old,
            Some(AppEvent::ActiveAudioObserved { result: Err(_), .. })
        ));
    }

    #[test]
    fn later_read_is_not_overwritten_by_an_echo_that_predates_its_start() {
        let mut observations = Observations::default();
        observations.reconcile(
            1,
            AppEvent::InputMuteChanged {
                input: "Mic".into(),
                muted: true,
            },
        );
        let later = observations.reconcile(
            2,
            AppEvent::ActiveAudioObserved {
                scene: Some("A".into()),
                result: Ok(vec![AudioInput::new("Mic".into(), false, 0.5, -6.0)]),
            },
        );
        assert!(
            matches!(later, Some(AppEvent::ActiveAudioObserved { result: Ok(inputs), .. }) if !inputs[0].muted)
        );
    }
}
