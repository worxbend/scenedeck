//! Exact linear gain and directional mute for identity-bound preset steps.
use super::client::ObsClient;
use crate::{
    domain::{
        audio_preset::{PresetGain, PresetInput, PresetProperty},
        operator_input::InputIdentity,
    },
    infra::error::AppError,
};
use obws::requests::inputs::{InputId, Volume};

impl ObsClient {
    pub(crate) async fn read_preset_input(
        &self,
        identity: &InputIdentity,
    ) -> Result<Option<PresetInput>, String> {
        let inputs = self
            .get_audio_for_catalog(std::slice::from_ref(identity))
            .await
            .map_err(|error| error.user_message())?;
        inputs
            .first()
            .map(|input| {
                Ok(PresetInput {
                    target: identity.target.clone(),
                    muted: input.muted,
                    gain: PresetGain::try_from(input.volume_mul).map_err(str::to_owned)?,
                })
            })
            .transpose()
    }
    pub(crate) async fn apply_preset_property(
        &self,
        input: &PresetInput,
        property: PresetProperty,
        valid: &(impl Fn() -> bool + Sync),
    ) -> Result<(), String> {
        let id = input
            .target
            .uuid
            .as_deref()
            .map(|value| value.parse().map(InputId::Uuid))
            .transpose()
            .map_err(|error| AppError::request(error).user_message())?
            .unwrap_or(InputId::Name(&input.target.name));
        match property {
            PresetProperty::Gain => {
                self.mutation(
                    valid,
                    self.inner
                        .inputs()
                        .set_volume(id, Volume::Mul(f64::from(input.gain) as f32)),
                )
                .await
            }
            PresetProperty::Mute => {
                self.mutation(valid, self.inner.inputs().set_muted(id, input.muted))
                    .await
            }
        }
        .map_err(|error| AppError::request(error).user_message())
    }
}
