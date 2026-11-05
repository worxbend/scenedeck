//! A reviewed intent and factual per-input results, independent of the editor.
use super::*;
use crate::domain::workflow::ScopeBinding;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedInput {
    pub desired: PresetInput,
    pub before: Option<PresetInput>,
}

#[derive(Debug, Clone)]
pub(crate) struct PresetPlan {
    pub id: u64,
    pub epoch: u64,
    pub scope: ScopeBinding,
    pub name: String,
    pub reviewed: Instant,
    pub inputs: Vec<PlannedInput>,
}
impl PresetPlan {
    pub fn valid(&self) -> bool {
        valid_name(&self.name)
            && !self.scope.collection.is_empty()
            && self.reviewed.elapsed() <= Duration::from_secs(30)
            && AudioPreset {
                inputs: self.inputs.iter().map(|row| row.desired.clone()).collect(),
            }
            .valid()
            && self.inputs.iter().all(|row| {
                row.before.as_ref().is_none_or(|before| {
                    row.desired
                        .target
                        .matches(&before.target.name, before.target.uuid.as_deref())
                })
            })
    }
}
impl PresetInput {
    /// OBS and obws expose audio gain as f32, including values imported as f64.
    pub fn same_values(&self, other: &Self) -> bool {
        self.target
            .matches(&other.target.name, other.target.uuid.as_deref())
            && self.muted == other.muted
            && f64::from(self.gain) as f32 == f64::from(other.gain) as f32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PresetProperty {
    Gain,
    Mute,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum PresetPhase {
    #[default]
    Pending,
    Running,
    Observed,
    Unchanged,
    Missing,
    Changed,
    Failed,
    Unconfirmed,
    Cancelled,
}
impl PresetPhase {
    pub fn done(self) -> bool {
        !matches!(self, Self::Pending | Self::Running)
    }
    pub fn label(self) -> String {
        use crate::infra::i18n::LANGUAGE_LOADER;
        use i18n_embed_fl::fl;
        match self {
            Self::Pending => fl!(LANGUAGE_LOADER, "preset-pending"),
            Self::Running => fl!(LANGUAGE_LOADER, "preset-running"),
            Self::Observed => fl!(LANGUAGE_LOADER, "preset-observed"),
            Self::Unchanged => fl!(LANGUAGE_LOADER, "preset-unchanged"),
            Self::Missing => fl!(LANGUAGE_LOADER, "preset-missing"),
            Self::Changed => fl!(LANGUAGE_LOADER, "preset-changed"),
            Self::Failed => fl!(LANGUAGE_LOADER, "preset-failed"),
            Self::Unconfirmed => fl!(LANGUAGE_LOADER, "preset-unconfirmed"),
            Self::Cancelled => fl!(LANGUAGE_LOADER, "preset-cancelled"),
        }
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PresetOutcome {
    pub phase: PresetPhase,
    pub gain_ack: bool,
    pub mute_ack: bool,
    pub observed: Option<PresetInput>,
    pub detail: Option<String>,
}
impl PresetOutcome {
    pub fn description(&self) -> String {
        use crate::infra::i18n::LANGUAGE_LOADER;
        use i18n_embed_fl::fl;
        let acks = fl!(
            LANGUAGE_LOADER,
            "preset-acknowledgements",
            gain = if self.gain_ack { "✓" } else { "—" },
            mute = if self.mute_ack { "✓" } else { "—" }
        );
        format!(
            "{} · {acks}{}",
            self.phase.label(),
            self.detail
                .as_ref()
                .map(|text| format!(" · {text}"))
                .unwrap_or_default()
        )
    }
}
#[derive(Debug, Clone)]
pub(crate) struct PresetRun {
    pub plan: PresetPlan,
    pub outcomes: Vec<PresetOutcome>,
}
impl PresetRun {
    pub fn active(&self) -> bool {
        self.outcomes.iter().any(|row| !row.phase.done())
    }
}
