//! Resolve scene clicks against authoritative Studio state, without replay.

use super::client::ObsClient;
use crate::infra::error::AppError;
use crate::infra::i18n::LANGUAGE_LOADER;
use i18n_embed_fl::fl;
use obws::requests::scenes::SceneId;

impl ObsClient {
    /// Reject changed/unknown mode rather than silently rerouting an old click.
    /// `valid` is checked by mutation admission immediately before first poll;
    /// callers must retain their invalidation ticket across the mode read.
    pub(crate) async fn activate_scene_if(
        &self,
        scene: &str,
        studio_enabled: bool,
        valid: &(impl Fn() -> bool + Sync),
    ) -> Result<(), AppError> {
        let authoritative = self
            .request(self.inner.ui().studio_mode_enabled())
            .await
            .map_err(AppError::request)?;
        if authoritative != studio_enabled {
            return Err(AppError::request(fl!(
                LANGUAGE_LOADER,
                "control-scene-mode-changed"
            )));
        }
        if authoritative {
            self.mutation(
                valid,
                self.inner
                    .scenes()
                    .set_current_preview_scene(SceneId::Name(scene)),
            )
            .await
        } else {
            self.mutation(
                valid,
                self.inner
                    .scenes()
                    .set_current_program_scene(SceneId::Name(scene)),
            )
            .await
        }
        .map_err(AppError::request)
    }
}
