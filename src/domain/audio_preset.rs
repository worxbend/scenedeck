//! Saved audio values retain exact observed gain and stable input identity.

use super::operator_action::ActionTarget;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod run;
pub(crate) use run::*;

pub(crate) const PRESET_LIMIT: usize = 64;
pub(crate) const INPUT_LIMIT: usize = 64;

/// Finite OBS linear gain, including digital silence and positive dB levels.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub(crate) struct PresetGain(f64);

// Construction and deserialization reject NaN, making equality reflexive.
impl Eq for PresetGain {}

impl TryFrom<f64> for PresetGain {
    type Error = &'static str;
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.is_finite() && (0.0..=20.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err("Audio preset gain must be finite and between 0 and 20")
        }
    }
}
impl From<PresetGain> for f64 {
    fn from(value: PresetGain) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PresetInput {
    pub target: ActionTarget,
    pub muted: bool,
    pub gain: PresetGain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AudioPreset {
    pub inputs: Vec<PresetInput>,
}

pub(crate) fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.trim() == name
        && name.chars().count() <= 80
        && !name.chars().any(char::is_control)
}

impl AudioPreset {
    pub(crate) fn valid(&self) -> bool {
        !self.inputs.is_empty()
            && self.inputs.len() <= INPUT_LIMIT
            && self.inputs.iter().all(|input| {
                !input.target.name.is_empty()
                    && input.target.uuid.as_ref().is_none_or(|id| !id.is_empty())
                    && self
                        .inputs
                        .iter()
                        .filter(|other| {
                            other.target.name == input.target.name
                                || input
                                    .target
                                    .matches(&other.target.name, other.target.uuid.as_deref())
                        })
                        .count()
                        == 1
            })
    }
}

pub(crate) fn deserialize_presets<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, AudioPreset>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let presets = BTreeMap::<String, AudioPreset>::deserialize(deserializer)?;
    if presets.len() > PRESET_LIMIT
        || presets
            .iter()
            .any(|(name, preset)| !valid_name(name) || !preset.valid())
    {
        return Err(serde::de::Error::custom(
            "Invalid or excessive audio presets",
        ));
    }
    Ok(presets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::registry::SceneRegistry;

    #[test]
    fn old_registries_default_empty_and_presets_preserve_exact_gain_and_identity() {
        let mut registry: SceneRegistry = serde_json::from_str("{}").unwrap();
        assert!(registry.audio_presets.is_empty());
        for (name, gain) in [
            ("Silence", 0.0),
            ("Quiet", 0.0000001),
            ("Music", 0.25118863582611084),
            ("Boost", 20.0),
        ] {
            registry.audio_presets.insert(
                name.into(),
                AudioPreset {
                    inputs: vec![PresetInput {
                        target: ActionTarget {
                            name: "Microphone".into(),
                            uuid: Some("input-1".into()),
                        },
                        muted: false,
                        gain: PresetGain::try_from(gain).unwrap(),
                    }],
                },
            );
        }
        let loaded: SceneRegistry =
            serde_json::from_str(&serde_json::to_string(&registry).unwrap()).unwrap();
        assert_eq!(loaded, registry);
    }

    #[test]
    fn malformed_presets_never_silently_clamp_or_drop_targets() {
        for gain in [f64::NAN, f64::INFINITY, -1.0, 20.1] {
            assert!(PresetGain::try_from(gain).is_err());
        }
        let valid = serde_json::json!({"audio_presets":{"Show":{"inputs":[{"target":{"name":"Mic","uuid":"one"},"muted":true,"gain":0.5}]}}});
        for gain in [-1.0, 20.1] {
            let mut value = valid.clone();
            value["audio_presets"]["Show"]["inputs"][0]["gain"] = gain.into();
            assert!(serde_json::from_value::<SceneRegistry>(value).is_err());
        }
        let mut duplicate = valid.clone();
        let mut renamed = duplicate["audio_presets"]["Show"]["inputs"][0].clone();
        renamed["target"]["name"] = "Renamed".into();
        duplicate["audio_presets"]["Show"]["inputs"]
            .as_array_mut()
            .unwrap()
            .push(renamed);
        assert!(serde_json::from_value::<SceneRegistry>(duplicate).is_err());
        let mut empty = valid;
        empty["audio_presets"]["Show"]["inputs"] = serde_json::json!([]);
        assert!(serde_json::from_value::<SceneRegistry>(empty).is_err());
        assert!(!valid_name(" "));
        assert!(!valid_name("Show\nOther"));
    }
}
