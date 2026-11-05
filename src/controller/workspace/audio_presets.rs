//! One sequential operation; no coalescing, reconnect replay, or invented success.
use super::*;
use crate::{
    controller::lifecycle::OperationKind,
    domain::{audio_preset::*, operator_input::resolve_input},
};
use std::sync::atomic::{AtomicBool, Ordering};

impl WorkspaceController {
    pub(crate) fn cancel_preset(&mut self) {
        self.preset_cancellation.fetch_add(1, Ordering::AcqRel);
    }
    pub(crate) fn preset(&mut self, plan: PresetPlan) {
        let client = self.slot.lock().ok().and_then(|slot| slot.clone());
        let tx = self.tx.scoped(&self.scope);
        let busy = self
            .preset_task
            .as_ref()
            .is_some_and(|task| !task.is_finished());
        if !plan.valid() || client.is_none() || busy || !self.scope.is_active() {
            // No protocol work is admitted. This result is fenced by run/epoch on GTK.
            let event = AppEvent::PresetRejected {
                epoch: plan.epoch,
                run: plan.id,
            };
            self.runtime.spawn(async move {
                let _ = tx.send(event).await;
            });
            return;
        }
        let client = client.unwrap();
        let cancellation = self.preset_cancellation.clone();
        let ticket = cancellation.load(Ordering::Acquire);
        let scope = self.scope.clone();
        self.preset_task = self.scope.spawn_reserved(&self.runtime, async move {
            for (index, input) in plan.inputs.iter().enumerate() {
                let topology = scope.graph_revision();
                let valid = || {
                    cancellation.load(Ordering::Acquire) == ticket
                        && tx.accepts_current_scope()
                        && scope.graph_revision() == topology
                        && client.retirement_reason().is_none()
                };
                let mut outcome = PresetOutcome::default();
                if !valid() {
                    outcome.phase = PresetPhase::Cancelled;
                } else if input.before.is_none() {
                    outcome.phase = PresetPhase::Missing;
                } else if !plan.valid() {
                    outcome.phase = PresetPhase::Changed;
                } else {
                    outcome.phase = PresetPhase::Running;
                    if publish(&tx, &plan, index, &outcome).await.is_err() {
                        return;
                    }
                    let result =
                        apply_input(&client, &tx, &plan, index, input, &valid, &mut outcome).await;
                    if let Err(error) = result {
                        outcome.phase = if client.retirement_reason().is_some()
                            || outcome.gain_ack
                            || outcome.mute_ack
                        {
                            PresetPhase::Unconfirmed
                        } else if !valid() {
                            PresetPhase::Cancelled
                        } else {
                            PresetPhase::Failed
                        };
                        outcome.detail = Some(error);
                    }
                }
                if publish(&tx, &plan, index, &outcome).await.is_err() {
                    return;
                }
            }
        });
    }
}

async fn publish(
    tx: &crate::controller::event_sender::EventSender,
    plan: &PresetPlan,
    index: usize,
    outcome: &PresetOutcome,
) -> Result<(), ()> {
    tx.send(AppEvent::PresetProgress {
        epoch: plan.epoch,
        run: plan.id,
        index,
        outcome: outcome.clone(),
    })
    .await
}

async fn apply_input(
    client: &ObsClient,
    tx: &crate::controller::event_sender::EventSender,
    plan: &PresetPlan,
    index: usize,
    input: &PlannedInput,
    valid: &(impl Fn() -> bool + Sync),
    outcome: &mut PresetOutcome,
) -> Result<(), String> {
    bounded_workspace_read(client, validate_scope(client, &plan.scope)).await?;
    if !valid() {
        outcome.phase = PresetPhase::Cancelled;
        return Ok(());
    }
    let inventory = bounded_workspace_read(client, async {
        client
            .get_input_catalog()
            .await
            .map_err(|error| error.user_message())
    })
    .await?;
    let Some(identity) = resolve_input(&inventory, &input.desired.target) else {
        outcome.phase = PresetPhase::Missing;
        return Ok(());
    };
    let Some(mut observed) =
        bounded_workspace_read(client, client.read_preset_input(identity)).await?
    else {
        outcome.phase = PresetPhase::Missing;
        return Ok(());
    };
    outcome.observed = Some(observed.clone());
    bounded_workspace_read(client, validate_scope(client, &plan.scope)).await?;
    if !plan.valid() {
        outcome.phase = PresetPhase::Changed;
        return Ok(());
    }
    if !valid() {
        outcome.phase = PresetPhase::Cancelled;
        return Ok(());
    }
    if input
        .before
        .as_ref()
        .is_none_or(|before| !before.same_values(&observed))
    {
        outcome.phase = PresetPhase::Changed;
        return Ok(());
    }
    if input.desired.same_values(&observed) {
        outcome.phase = PresetPhase::Unchanged;
        return Ok(());
    }
    let properties = if input.desired.muted {
        [PresetProperty::Mute, PresetProperty::Gain]
    } else {
        [PresetProperty::Gain, PresetProperty::Mute]
    };
    for property in properties {
        let changed = match property {
            PresetProperty::Gain => {
                f64::from(observed.gain) as f32 != f64::from(input.desired.gain) as f32
            }
            PresetProperty::Mute => observed.muted != input.desired.muted,
        };
        if !changed {
            continue;
        }
        bounded_workspace_read(client, validate_scope(client, &plan.scope)).await?;
        if !valid() {
            outcome.phase = PresetPhase::Cancelled;
            return Ok(());
        }
        let evidence = tx
            .operation(
                OperationKind::AudioPreset {
                    epoch: plan.epoch,
                    run: plan.id,
                    index,
                    property,
                    input: identity.target.clone(),
                },
                client.endpoint().into(),
            )
            .ok_or_else(|| fl!(LANGUAGE_LOADER, "controller-workspace-admission-full"))?;
        let dispatched = AtomicBool::new(false);
        let desired = PresetInput {
            target: identity.target.clone(),
            ..input.desired.clone()
        };
        let result = bounded_workspace_read(
            client,
            client.apply_preset_property(&desired, property, &|| {
                if !valid() || plan.reviewed.elapsed() > std::time::Duration::from_secs(30) {
                    return false;
                }
                dispatched.store(true, Ordering::Release);
                evidence.dispatch();
                true
            }),
        )
        .await;
        if result.is_ok() {
            evidence.acknowledge();
            match property {
                PresetProperty::Gain => outcome.gain_ack = true,
                PresetProperty::Mute => outcome.mute_ack = true,
            }
        }
        if result.is_err()
            && dispatched.load(Ordering::Acquire)
            && client.retirement_reason().is_some()
        {
            evidence.unknown(result.clone().unwrap_err());
        } else {
            evidence.finish(result.clone());
        }
        result?;
        publish(tx, plan, index, outcome)
            .await
            .map_err(|()| fl!(LANGUAGE_LOADER, "controller-workspace-action-cancelled"))?;
        let mut expected = observed.clone();
        match property {
            PresetProperty::Gain => expected.gain = desired.gain,
            PresetProperty::Mute => expected.muted = desired.muted,
        }
        let Some(readback) =
            bounded_workspace_read(client, client.read_preset_input(identity)).await?
        else {
            outcome.phase = PresetPhase::Unconfirmed;
            return Ok(());
        };
        outcome.observed = Some(readback.clone());
        bounded_workspace_read(client, validate_scope(client, &plan.scope)).await?;
        if !valid() {
            outcome.phase = PresetPhase::Unconfirmed;
            return Ok(());
        }
        if !expected.same_values(&readback) {
            outcome.phase = PresetPhase::Changed;
            return Ok(());
        }
        observed = readback;
    }
    outcome.phase = if input.desired.same_values(&observed) {
        PresetPhase::Observed
    } else {
        PresetPhase::Unconfirmed
    };
    Ok(())
}

#[cfg(test)]
mod tests;
