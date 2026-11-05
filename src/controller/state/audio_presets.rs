use super::*;
use crate::domain::{audio_preset::*, workflow::ActivityOrigin};

impl AppState {
    pub(crate) fn start_preset(&mut self, mut plan: PresetPlan) -> Option<PresetPlan> {
        if !plan.valid()
            || plan.epoch != self.workspace.epoch
            || plan.scope != self.workflow_scope()
            || !matches!(self.obs_status, ObsStatus::Connected { .. })
            || self.preset_run.as_ref().is_some_and(PresetRun::active)
        {
            return None;
        }
        self.preset_serial = self.preset_serial.checked_add(1)?;
        plan.id = self.preset_serial;
        self.preset_run = Some(PresetRun {
            outcomes: vec![PresetOutcome::default(); plan.inputs.len()],
            plan: plan.clone(),
        });
        self.workspace.mark_dirty();
        self.workspace.log(
            ActivityOrigin::Requested,
            format!("{}: {}", fl!(LANGUAGE_LOADER, "preset-apply"), plan.name),
        );
        Some(plan)
    }
    pub(crate) fn preset_progress(
        &mut self,
        epoch: u64,
        id: u64,
        index: usize,
        outcome: PresetOutcome,
    ) {
        let Some(run) = self.preset_run.as_mut().filter(|run| {
            run.plan.id == id && run.plan.epoch == epoch && epoch == self.workspace.epoch
        }) else {
            return;
        };
        let Some(previous) = run.outcomes.get_mut(index) else {
            return;
        };
        if previous.phase.done() || *previous == outcome {
            return;
        }
        let description = format!(
            "{} · {}: {}",
            run.plan.name,
            run.plan.inputs[index].desired.target.name,
            outcome.description()
        );
        let origin = if matches!(
            outcome.phase,
            PresetPhase::Failed | PresetPhase::Unconfirmed
        ) {
            ActivityOrigin::Failed
        } else {
            ActivityOrigin::Observed
        };
        *previous = outcome;
        self.workspace.log(origin, description);
    }
    pub(crate) fn reject_preset(&mut self, epoch: u64, id: u64) {
        let count = self.preset_run.as_ref().map_or(0, |run| run.outcomes.len());
        for index in 0..count {
            self.preset_progress(
                epoch,
                id,
                index,
                PresetOutcome {
                    phase: PresetPhase::Failed,
                    detail: Some(fl!(LANGUAGE_LOADER, "preset-unavailable")),
                    ..Default::default()
                },
            );
        }
    }
    pub(crate) fn retire_preset(&mut self) {
        let Some(run) = self.preset_run.as_ref() else {
            return;
        };
        let (epoch, id) = (run.plan.epoch, run.plan.id);
        let outcomes = run.outcomes.clone();
        for (index, mut outcome) in outcomes.into_iter().enumerate() {
            if outcome.phase.done() {
                continue;
            }
            outcome.phase = if outcome.phase == PresetPhase::Pending {
                PresetPhase::Cancelled
            } else {
                PresetPhase::Unconfirmed
            };
            self.preset_progress(epoch, id, index, outcome);
        }
    }
    pub(crate) fn preset_terminal(
        &mut self,
        operation: &super::super::lifecycle::OperationEvidence,
    ) {
        use super::super::lifecycle::OperationKind;
        let OperationKind::AudioPreset {
            epoch,
            run: id,
            index,
            property,
            ..
        } = operation.kind
        else {
            return;
        };
        let Some(run) = self
            .preset_run
            .as_mut()
            .filter(|run| run.plan.id == id && run.plan.epoch == epoch)
        else {
            return;
        };
        let Some(outcome) = run.outcomes.get_mut(index) else {
            return;
        };
        if operation.acknowledged {
            match property {
                PresetProperty::Gain => outcome.gain_ack = true,
                PresetProperty::Mute => outcome.mute_ack = true,
            }
        }
        if !outcome.phase.done()
            || matches!(
                outcome.phase,
                PresetPhase::Cancelled | PresetPhase::Unconfirmed
            )
        {
            outcome.phase = if operation.dispatched
                || outcome.gain_ack
                || outcome.mute_ack
                || outcome.phase == PresetPhase::Unconfirmed
            {
                PresetPhase::Unconfirmed
            } else {
                PresetPhase::Cancelled
            };
        }
        // The session terminal audit records this evidence with its original report.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        controller::lifecycle::{OperationEvidence, OperationKind},
        domain::{operator_action::ActionTarget, workflow::ScopeBinding},
    };
    fn fixture() -> (AppState, PresetPlan) {
        let mut state = AppState::new(Default::default(), Default::default(), None, None);
        state.obs_status = ObsStatus::Connected {
            obs_version: "fixture".into(),
        };
        state.connected_endpoint = Some("fixture:4455".into());
        state.scene_collections.current = Some("Show".into());
        let input = PresetInput {
            target: ActionTarget {
                name: "Mic".into(),
                uuid: Some("one".into()),
            },
            gain: PresetGain::try_from(0.5).unwrap(),
            muted: false,
        };
        let plan = PresetPlan {
            id: 0,
            epoch: state.workspace.epoch,
            scope: ScopeBinding {
                endpoint: "fixture:4455".into(),
                collection: "Show".into(),
            },
            name: "Interview".into(),
            reviewed: Instant::now(),
            inputs: vec![PlannedInput {
                desired: input.clone(),
                before: Some(input),
            }],
        };
        (state, plan)
    }
    #[test]
    fn retirement_retains_partial_acknowledgements_without_replaying_or_crossing_runs() {
        let (mut state, plan) = fixture();
        let plan = state.start_preset(plan).unwrap();
        assert!(state.start_preset(plan.clone()).is_none());
        state.preset_progress(
            plan.epoch,
            plan.id,
            0,
            PresetOutcome {
                phase: PresetPhase::Running,
                ..Default::default()
            },
        );
        state.invalidate_collection_runtime();
        assert_eq!(
            state.preset_run.as_ref().unwrap().outcomes[0].phase,
            PresetPhase::Unconfirmed
        );
        state.preset_progress(
            plan.epoch,
            plan.id,
            0,
            PresetOutcome {
                phase: PresetPhase::Observed,
                ..Default::default()
            },
        );
        assert_eq!(
            state.preset_run.as_ref().unwrap().outcomes[0].phase,
            PresetPhase::Unconfirmed
        );
        let mut evidence = OperationEvidence {
            id: 1,
            kind: OperationKind::AudioPreset {
                epoch: plan.epoch,
                run: plan.id,
                index: 0,
                property: PresetProperty::Gain,
                input: plan.inputs[0].desired.target.clone(),
            },
            endpoint: plan.scope.endpoint.clone(),
            collection: 0,
            dispatched: true,
            acknowledged: true,
            uncertain: false,
            completed: true,
            failure: None,
        };
        state.preset_terminal(&evidence);
        assert!(state.preset_run.as_ref().unwrap().outcomes[0].gain_ack);
        evidence.dispatched = false;
        evidence.acknowledged = false;
        state.preset_terminal(&evidence);
        assert_eq!(
            state.preset_run.as_ref().unwrap().outcomes[0].phase,
            PresetPhase::Unconfirmed
        );
        state.scene_collections.current = Some("Show".into());
        let mut next = plan.clone();
        next.epoch = state.workspace.epoch;
        let next = state.start_preset(next).unwrap();
        assert_ne!(next.id, plan.id);
        state.preset_terminal(&evidence);
        assert_eq!(
            state.preset_run.as_ref().unwrap().outcomes[0],
            PresetOutcome::default()
        );
    }
    #[test]
    fn duplicate_or_out_of_order_completions_cannot_erase_confirmed_results() {
        let (mut state, plan) = fixture();
        let plan = state.start_preset(plan).unwrap();
        let observed = PresetOutcome {
            phase: PresetPhase::Observed,
            gain_ack: true,
            observed: Some(plan.inputs[0].desired.clone()),
            ..Default::default()
        };
        state.preset_progress(plan.epoch, plan.id, 0, observed.clone());
        state.preset_progress(plan.epoch, plan.id, 0, PresetOutcome::default());
        state.reject_preset(plan.epoch, plan.id);
        assert_eq!(state.preset_run.as_ref().unwrap().outcomes[0], observed);
        let mut old = plan;
        old.reviewed = Instant::now() - std::time::Duration::from_secs(31);
        assert!(state.start_preset(old).is_none());
    }
}
