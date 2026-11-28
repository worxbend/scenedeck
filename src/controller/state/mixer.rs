//! Mixer audio view model: per-scene audio snapshots, request fencing and
//! the inspection/render decisions the Mixer page reads.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixerAudioError {
    pub scene: SceneId,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct MixerAudioSnapshot {
    pub scene: SceneId,
    pub inputs: Vec<AudioInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixerAudioRefreshTransition {
    Success,
    Failure,
    StaleSuccess,
    StaleFailure,
}

impl MixerAudioRefreshTransition {
    /// A superseded request must not change the current snapshot or loading UI,
    /// even when its scene name matches the current target again.
    pub const fn is_stale(self) -> bool {
        matches!(self, Self::StaleSuccess | Self::StaleFailure)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MixerVisibleAudioStatus<'a> {
    Loading,
    Error(&'a MixerAudioError),
    Loaded(&'a [AudioInput]),
    Missing,
}

#[derive(Debug, Clone, Copy)]
pub enum MixerVisibleRenderSource<'a> {
    ActiveScene(&'a [AudioInput]),
    Scene {
        scene: &'a str,
        status: MixerVisibleAudioStatus<'a>,
    },
    MissingScene,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixerSceneRefreshTargetReason {
    DirectSelectedScene,
    DirectPinnedScene,
    SelectedModeCurrentSceneFallback,
    PinnedModeSelectedSceneFallback,
    PinnedModeCurrentSceneFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MixerSceneRefreshTarget<'a> {
    pub scene: &'a str,
    pub reason: MixerSceneRefreshTargetReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixerInspectionRenderSourceKind {
    ActiveScene,
    Scene,
    MissingScene,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixerInspectionStatus<'a> {
    LoadingPlaceholderShown,
    ErrorPlaceholderShown(&'a str),
    MissingNoTarget,
    LoadedWithVisibleInputCards,
    LoadedNoAudioSources,
    LoadedNoMatchingAudioSourcesAfterFiltering,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MixerInspectionInput<'a> {
    pub name: &'a str,
    pub display_name: &'a str,
    pub muted: bool,
    pub volume_mul: f64,
    pub volume_db: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MixerInspectionSnapshot<'a> {
    pub mode: MixerMode,
    pub selected_scene: Option<&'a str>,
    pub pinned_scene: Option<&'a str>,
    pub refresh_target: Option<MixerSceneRefreshTarget<'a>>,
    pub render_source_kind: MixerInspectionRenderSourceKind,
    pub scene: Option<&'a str>,
    pub status: MixerInspectionStatus<'a>,
    pub inputs: Vec<MixerInspectionInput<'a>>,
}

/// Issued on the state-owning thread before dispatch, then echoed unchanged by
/// the controller. Scene names cannot identify retries or session/collection ABA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixerAudioRequest {
    pub(super) id: u64,
    pub(super) scope_epoch: u64,
    pub(super) scene: SceneId,
}

impl MixerAudioRequest {
    pub fn scene(&self) -> &str {
        &self.scene
    }
}

#[derive(Debug, Default, Clone)]
pub struct MixerAudioRefreshState {
    pub(super) next_request_id: u64,
    pub(super) scope_epoch: u64,
    pub(super) requested: Option<MixerAudioRequest>,
    pub loaded: Option<MixerAudioSnapshot>,
    pub error: Option<MixerAudioError>,
}

impl MixerAudioRefreshState {
    pub fn loading(&mut self, scene: SceneId) -> MixerAudioRequest {
        self.next_request_id = self
            .next_request_id
            .checked_add(1)
            .expect("mixer request IDs exhausted");
        let request = MixerAudioRequest {
            id: self.next_request_id,
            scope_epoch: self.scope_epoch,
            scene,
        };
        if self.error.as_ref().map(|err| &err.scene) == Some(&request.scene) {
            self.error = None;
        }
        self.requested = Some(request.clone());
        request
    }

    pub fn success(
        &mut self,
        request: MixerAudioRequest,
        inputs: Vec<AudioInput>,
    ) -> MixerAudioRefreshTransition {
        if self.requested.as_ref() != Some(&request) {
            return MixerAudioRefreshTransition::StaleSuccess;
        }

        self.requested = None;
        self.error = None;
        self.loaded = Some(MixerAudioSnapshot {
            scene: request.scene,
            inputs,
        });
        MixerAudioRefreshTransition::Success
    }

    pub fn failure(
        &mut self,
        request: MixerAudioRequest,
        message: String,
    ) -> MixerAudioRefreshTransition {
        if self.requested.as_ref() != Some(&request) {
            return MixerAudioRefreshTransition::StaleFailure;
        }

        self.requested = None;
        self.error = Some(MixerAudioError {
            scene: request.scene,
            message,
        });
        MixerAudioRefreshTransition::Failure
    }

    pub fn requested_scene(&self) -> Option<&str> {
        self.requested.as_ref().map(MixerAudioRequest::scene)
    }

    pub fn clear_pending(&mut self) {
        self.requested = None;
    }

    pub fn invalidate(&mut self) {
        self.scope_epoch = self
            .scope_epoch
            .checked_add(1)
            .expect("mixer scope epochs exhausted");
        self.clear_pending();
        self.loaded = None;
        self.error = None;
    }
}

impl AppState {
    pub fn set_mixer_audio_loading(&mut self, scene: SceneId) -> MixerAudioRequest {
        self.mixer_audio_refresh.loading(scene)
    }

    pub fn set_mixer_audio_success(
        &mut self,
        request: MixerAudioRequest,
        inputs: Vec<AudioInput>,
    ) -> MixerAudioRefreshTransition {
        self.mixer_audio_refresh.success(request, inputs)
    }

    pub fn set_mixer_audio_failure(
        &mut self,
        request: MixerAudioRequest,
        message: String,
    ) -> MixerAudioRefreshTransition {
        self.mixer_audio_refresh.failure(request, message)
    }

    pub fn visible_mixer_audio_status(&self, scene: &str) -> MixerVisibleAudioStatus<'_> {
        if self.mixer_audio_refresh.requested_scene() == Some(scene) {
            return MixerVisibleAudioStatus::Loading;
        }

        if let Some(error) = self.mixer_audio_refresh.error.as_ref() {
            if error.scene == scene {
                return MixerVisibleAudioStatus::Error(error);
            }
        }

        if let Some(snapshot) = self.mixer_audio_refresh.loaded.as_ref() {
            if snapshot.scene == scene {
                return MixerVisibleAudioStatus::Loaded(&snapshot.inputs);
            }
        }

        MixerVisibleAudioStatus::Missing
    }

    pub fn visible_mixer_render_source(&self) -> MixerVisibleRenderSource<'_> {
        match self.mixer.mode {
            MixerMode::ActiveScene => match &self.active_audio_status {
                ActiveAudioStatus::Ready => {
                    MixerVisibleRenderSource::ActiveScene(&self.audio_inputs)
                }
                status => MixerVisibleRenderSource::Scene {
                    scene: self
                        .scene_inventory
                        .current_id
                        .as_deref()
                        .unwrap_or_default(),
                    status: match status {
                        ActiveAudioStatus::Failed(error) => MixerVisibleAudioStatus::Error(error),
                        _ => MixerVisibleAudioStatus::Loading,
                    },
                },
            },
            MixerMode::SelectedScene | MixerMode::PinnedScene => {
                let Some(scene) = self.mixer_scene_refresh_target() else {
                    return MixerVisibleRenderSource::MissingScene;
                };

                MixerVisibleRenderSource::Scene {
                    scene,
                    status: self.visible_mixer_audio_status(scene),
                }
            }
        }
    }

    pub fn mixer_inspection_snapshot(&self) -> MixerInspectionSnapshot<'_> {
        let refresh_target = self.mixer_scene_refresh_target_details();
        let (render_source_kind, scene, status, inputs) = match self.visible_mixer_render_source() {
            MixerVisibleRenderSource::ActiveScene(inputs) => (
                MixerInspectionRenderSourceKind::ActiveScene,
                self.scene_inventory.current_id.as_deref(),
                mixer_inspection_loaded_status(inputs),
                mixer_inspection_inputs(inputs),
            ),
            MixerVisibleRenderSource::Scene { scene, status } => {
                let (status, inputs) = mixer_inspection_scene_status(status);
                (
                    if self.mixer.mode == MixerMode::ActiveScene {
                        MixerInspectionRenderSourceKind::ActiveScene
                    } else {
                        MixerInspectionRenderSourceKind::Scene
                    },
                    Some(scene),
                    status,
                    inputs,
                )
            }
            MixerVisibleRenderSource::MissingScene => (
                MixerInspectionRenderSourceKind::MissingScene,
                None,
                MixerInspectionStatus::MissingNoTarget,
                Vec::new(),
            ),
        };

        MixerInspectionSnapshot {
            mode: self.mixer.mode,
            selected_scene: self.mixer.selected_scene.as_deref(),
            pinned_scene: self.mixer.pinned_scene.as_deref(),
            refresh_target,
            render_source_kind,
            scene,
            status,
            inputs,
        }
    }

    /// Scene target for OBS scene-specific Mixer refresh requests.
    ///
    /// Active mode renders live active-scene audio and must not dispatch a
    /// scene-specific Mixer refresh, so it intentionally has no target here.
    pub fn mixer_scene_refresh_target(&self) -> Option<&str> {
        self.mixer_scene_refresh_target_details()
            .map(|target| target.scene)
    }

    /// Scene target for OBS scene-specific Mixer refresh requests, annotated
    /// with the selected/pinned/fallback rule that produced it.
    pub fn mixer_scene_refresh_target_details(&self) -> Option<MixerSceneRefreshTarget<'_>> {
        match self.mixer.mode {
            MixerMode::ActiveScene => None,
            MixerMode::SelectedScene => {
                if let Some(scene) = self.mixer.selected_scene.as_deref() {
                    Some(MixerSceneRefreshTarget {
                        scene,
                        reason: MixerSceneRefreshTargetReason::DirectSelectedScene,
                    })
                } else {
                    self.scene_inventory.current_id.as_deref().map(|scene| {
                        MixerSceneRefreshTarget {
                            scene,
                            reason: MixerSceneRefreshTargetReason::SelectedModeCurrentSceneFallback,
                        }
                    })
                }
            }
            MixerMode::PinnedScene => {
                if let Some(scene) = self.mixer.pinned_scene.as_deref() {
                    Some(MixerSceneRefreshTarget {
                        scene,
                        reason: MixerSceneRefreshTargetReason::DirectPinnedScene,
                    })
                } else if let Some(scene) = self.mixer.selected_scene.as_deref() {
                    Some(MixerSceneRefreshTarget {
                        scene,
                        reason: MixerSceneRefreshTargetReason::PinnedModeSelectedSceneFallback,
                    })
                } else {
                    self.scene_inventory.current_id.as_deref().map(|scene| {
                        MixerSceneRefreshTarget {
                            scene,
                            reason: MixerSceneRefreshTargetReason::PinnedModeCurrentSceneFallback,
                        }
                    })
                }
            }
        }
    }

    /// One input inside the currently loaded mixer snapshot, if it is there.
    ///
    /// The lookup has two steps that are easy to get subtly different — the
    /// snapshot may not be loaded, and the input may not be in it — so it is
    /// written once here rather than at each mutator, mirroring
    /// `audio_input_mut` above.
    pub(super) fn mixer_input_mut(&mut self, input_id: &str) -> Option<&mut AudioInput> {
        self.mixer_audio_refresh
            .loaded
            .as_mut()?
            .inputs
            .iter_mut()
            .find(|input| input.id == input_id)
    }

    /// Returns whether the input was present to update.
    pub(super) fn update_mixer_input_mute(&mut self, input_id: &str, muted: bool) -> bool {
        match self.mixer_input_mut(input_id) {
            Some(input) => {
                input.muted = muted;
                true
            }
            None => false,
        }
    }

    /// Returns whether the input was present to update.
    pub(super) fn update_mixer_input_volume(
        &mut self,
        input_id: &str,
        volume_mul: f64,
        volume_db: f64,
    ) -> bool {
        match self.mixer_input_mut(input_id) {
            Some(input) => {
                input.volume_mul = volume_mul;
                input.volume_db = volume_db;
                true
            }
            None => false,
        }
    }
}

pub(super) fn mixer_inspection_scene_status(
    status: MixerVisibleAudioStatus<'_>,
) -> (MixerInspectionStatus<'_>, Vec<MixerInspectionInput<'_>>) {
    match status {
        MixerVisibleAudioStatus::Loading => {
            (MixerInspectionStatus::LoadingPlaceholderShown, Vec::new())
        }
        MixerVisibleAudioStatus::Error(error) => (
            MixerInspectionStatus::ErrorPlaceholderShown(error.message.as_str()),
            Vec::new(),
        ),
        MixerVisibleAudioStatus::Loaded(inputs) => (
            mixer_inspection_loaded_status(inputs),
            mixer_inspection_inputs(inputs),
        ),
        MixerVisibleAudioStatus::Missing => (MixerInspectionStatus::MissingNoTarget, Vec::new()),
    }
}

pub(super) fn mixer_inspection_loaded_status(
    inputs: &[AudioInput],
) -> MixerInspectionStatus<'static> {
    if inputs.is_empty() {
        MixerInspectionStatus::LoadedNoAudioSources
    } else {
        MixerInspectionStatus::LoadedWithVisibleInputCards
    }
}

pub(super) fn mixer_inspection_inputs(inputs: &[AudioInput]) -> Vec<MixerInspectionInput<'_>> {
    inputs
        .iter()
        .map(|input| MixerInspectionInput {
            name: input.name.as_str(),
            display_name: input.display_name.as_str(),
            muted: input.muted,
            volume_mul: input.volume_mul,
            volume_db: input.volume_db,
        })
        .collect()
}

#[cfg(test)]
mod tests;
