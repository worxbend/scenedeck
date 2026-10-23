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
mod tests {
    use super::super::tests::*;
    use super::*;

    #[test]
    fn mixer_refresh_loading_tracks_request_and_preserves_snapshot() {
        let mut state = MixerAudioRefreshState {
            loaded: Some(MixerAudioSnapshot {
                scene: "Loaded".to_string(),
                inputs: vec![input("Mic")],
            }),
            error: Some(MixerAudioError {
                scene: "Target".to_string(),
                message: "previous failure".to_string(),
            }),
            ..Default::default()
        };

        let request = state.loading("Target".to_string());

        assert_eq!(request.scene(), "Target");
        assert_eq!(state.requested.as_ref(), Some(&request));
        assert_eq!(state.requested_scene(), Some("Target"));
        assert_eq!(state.loaded.as_ref().unwrap().scene, "Loaded");
        assert!(state.error.is_none());
    }

    #[test]
    fn mixer_refresh_success_replaces_loaded_snapshot_for_current_request() {
        let mut state = MixerAudioRefreshState::default();
        let request = state.loading("Scene A".to_string());

        let transition = state.success(request.clone(), vec![input("Mic A")]);

        assert_eq!(transition, MixerAudioRefreshTransition::Success);
        assert_eq!(state.requested_scene(), None);
        let loaded = state.loaded.as_ref().expect("loaded mixer snapshot");
        assert_eq!(loaded.scene, "Scene A");
        assert_eq!(loaded.inputs.len(), 1);
        assert_eq!(loaded.inputs[0].id, "Mic A");
        assert!(state.error.is_none());
        assert_eq!(
            state.failure(request, "duplicate completion".into()),
            MixerAudioRefreshTransition::StaleFailure
        );
    }

    #[test]
    fn mixer_refresh_failure_sets_visible_error_for_current_request() {
        let mut state = MixerAudioRefreshState {
            loaded: Some(MixerAudioSnapshot {
                scene: "Previous".to_string(),
                inputs: vec![input("Previous Mic")],
            }),
            ..Default::default()
        };
        let request = state.loading("Scene A".to_string());

        let transition = state.failure(request.clone(), "OBS failed".to_string());

        assert_eq!(transition, MixerAudioRefreshTransition::Failure);
        assert_eq!(state.requested_scene(), None);
        assert_eq!(state.loaded.as_ref().unwrap().scene, "Previous");
        assert_eq!(state.error.as_ref().unwrap().scene, "Scene A");
        assert_eq!(state.error.as_ref().unwrap().message, "OBS failed");
        assert_eq!(
            state.success(request, vec![input("duplicate completion")]),
            MixerAudioRefreshTransition::StaleSuccess
        );
    }

    #[test]
    fn mixer_refresh_aba_accepts_only_newest_a_in_either_completion_order() {
        for newest_first in [false, true] {
            for old_succeeds in [false, true] {
                for new_succeeds in [false, true] {
                    let mut state = MixerAudioRefreshState::default();
                    let old_a = state.loading("A".into());
                    let old_b = state.loading("B".into());
                    let new_a = state.loading("A".into());
                    assert!(old_a.id < old_b.id && old_b.id < new_a.id);
                    let stale = if old_succeeds {
                        MixerAudioRefreshTransition::StaleSuccess
                    } else {
                        MixerAudioRefreshTransition::StaleFailure
                    };
                    let current = if new_succeeds {
                        MixerAudioRefreshTransition::Success
                    } else {
                        MixerAudioRefreshTransition::Failure
                    };

                    if newest_first {
                        assert_eq!(
                            complete_mixer_request(&mut state, new_a, new_succeeds),
                            current
                        );
                        assert_eq!(
                            complete_mixer_request(&mut state, old_a, old_succeeds),
                            stale
                        );
                    } else {
                        assert_eq!(
                            complete_mixer_request(&mut state, old_a, old_succeeds),
                            stale
                        );
                        assert_eq!(state.requested.as_ref(), Some(&new_a));
                        assert!(state.loaded.is_none() && state.error.is_none());
                        assert_eq!(
                            complete_mixer_request(&mut state, new_a, new_succeeds),
                            current
                        );
                    }
                    assert_eq!(state.requested_scene(), None);
                    assert_eq!(state.loaded.is_some(), new_succeeds);
                    assert_eq!(state.error.is_some(), !new_succeeds);
                    if let Some(loaded) = &state.loaded {
                        assert_eq!(loaded.scene, "A");
                        assert_eq!(loaded.inputs[0].id, "Newest Mic");
                    }
                    if let Some(error) = &state.error {
                        assert_eq!(error.scene, "A");
                        assert_eq!(error.message, "newest failure");
                    }
                    assert!(state.success(old_b, Vec::new()).is_stale());
                }
            }
        }
    }

    #[test]
    fn clearing_pending_mixer_refresh_rejects_late_completion_but_keeps_in_scope_cache() {
        let mut state = MixerAudioRefreshState::default();
        let loaded = state.loading("A".into());
        state.success(loaded, vec![input("Kept")]);
        let cancelled = state.loading("A".into());
        state.clear_pending();
        assert!(state.success(cancelled.clone(), Vec::new()).is_stale());
        assert!(state
            .failure(cancelled, "cancelled failure".into())
            .is_stale());
        assert_eq!(state.loaded.as_ref().unwrap().inputs[0].id, "Kept");
    }

    #[test]
    fn mixer_refresh_invalidation_discards_loaded_error_and_pending_without_reusing_ids() {
        let mut state = MixerAudioRefreshState::default();
        let loaded = state.loading("A".into());
        state.success(loaded, vec![input("Old")]);
        let failed = state.loading("A".into());
        state.failure(failed, "old failure".into());
        let old = state.loading("B".into());
        assert!(state.loaded.is_some() && state.error.is_some());

        state.invalidate();

        assert!(state.loaded.is_none() && state.error.is_none());
        assert_eq!(state.requested_scene(), None);
        let new = state.loading("B".into());
        assert!(new.id > old.id);
        assert!(new.scope_epoch > old.scope_epoch);
        assert!(state.success(old.clone(), Vec::new()).is_stale());
        assert!(state.failure(old, "old scope".into()).is_stale());
        assert_eq!(state.requested.as_ref(), Some(&new));
    }

    #[test]
    #[should_panic(expected = "mixer request IDs exhausted")]
    fn mixer_request_ids_fail_closed_instead_of_wrapping() {
        let mut state = MixerAudioRefreshState {
            next_request_id: u64::MAX,
            ..Default::default()
        };
        state.loading("A".into());
    }

    #[test]
    #[should_panic(expected = "mixer scope epochs exhausted")]
    fn mixer_scope_epochs_fail_closed_instead_of_wrapping() {
        let mut state = MixerAudioRefreshState {
            scope_epoch: u64::MAX,
            ..Default::default()
        };
        state.invalidate();
    }

    #[test]
    fn visible_mixer_audio_status_reports_loading_for_target_scene() {
        let mut state = app_state();
        state.set_mixer_audio_loading("Scene A".to_string());

        assert!(matches!(
            state.visible_mixer_audio_status("Scene A"),
            MixerVisibleAudioStatus::Loading
        ));
    }

    #[test]
    fn visible_mixer_audio_status_reports_error_for_target_scene() {
        let mut state = app_state();
        let request = state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_failure(request.clone(), "OBS failed".to_string());

        let MixerVisibleAudioStatus::Error(error) = state.visible_mixer_audio_status("Scene A")
        else {
            panic!("expected visible mixer error");
        };

        assert_eq!(error.scene, "Scene A");
        assert_eq!(error.message, "OBS failed");
    }

    #[test]
    fn visible_mixer_audio_status_reports_loaded_inputs_for_target_scene() {
        let mut state = app_state();
        let request = state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success(request.clone(), vec![input("Mic"), input("Music")]);

        let MixerVisibleAudioStatus::Loaded(inputs) = state.visible_mixer_audio_status("Scene A")
        else {
            panic!("expected visible mixer inputs");
        };

        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0].id, "Mic");
        assert_eq!(inputs[1].id, "Music");
    }

    #[test]
    fn visible_mixer_audio_status_reports_missing_without_target_state() {
        let state = app_state();

        assert!(matches!(
            state.visible_mixer_audio_status("Scene A"),
            MixerVisibleAudioStatus::Missing
        ));
    }

    #[test]
    fn visible_mixer_audio_status_treats_other_scene_state_as_missing() {
        let mut state = app_state();
        let request = state.set_mixer_audio_loading("Scene A".to_string());
        assert!(matches!(
            state.visible_mixer_audio_status("Scene B"),
            MixerVisibleAudioStatus::Missing
        ));

        state.set_mixer_audio_failure(request.clone(), "OBS failed".to_string());
        assert!(matches!(
            state.visible_mixer_audio_status("Scene B"),
            MixerVisibleAudioStatus::Missing
        ));

        let request = state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success(request.clone(), vec![input("Mic")]);
        assert!(matches!(
            state.visible_mixer_audio_status("Scene B"),
            MixerVisibleAudioStatus::Missing
        ));
    }

    #[test]
    fn visible_mixer_render_source_active_reads_live_audio_inputs() {
        let mut state = app_state();
        state.active_audio_status = ActiveAudioStatus::Ready;
        state.mixer.mode = MixerMode::ActiveScene;
        state.audio_inputs = vec![input("Live Mic"), input("Live Music")];
        let request = state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_failure(request.clone(), "OBS failed".to_string());

        let MixerVisibleRenderSource::ActiveScene(inputs) = state.visible_mixer_render_source()
        else {
            panic!("expected active mixer render source");
        };

        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0].id, "Live Mic");
        assert_eq!(inputs[1].id, "Live Music");
    }

    #[test]
    fn visible_mixer_render_source_selected_uses_selected_scene_status() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;
        state.scene_inventory.current_id = Some("Active".to_string());
        state.mixer.selected_scene = Some("Selected".to_string());
        let request = state.set_mixer_audio_loading("Selected".to_string());
        state.set_mixer_audio_success(request.clone(), vec![input("Selected Mic")]);

        let MixerVisibleRenderSource::Scene { scene, status } = state.visible_mixer_render_source()
        else {
            panic!("expected scene mixer render source");
        };

        assert_eq!(scene, "Selected");
        let MixerVisibleAudioStatus::Loaded(inputs) = status else {
            panic!("expected selected mixer inputs");
        };
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].id, "Selected Mic");
    }

    #[test]
    fn visible_mixer_render_source_pinned_uses_pinned_scene_status() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::PinnedScene;
        state.scene_inventory.current_id = Some("Active".to_string());
        state.mixer.selected_scene = Some("Selected".to_string());
        state.mixer.pinned_scene = Some("Pinned".to_string());
        state.set_mixer_audio_loading("Pinned".to_string());

        let MixerVisibleRenderSource::Scene { scene, status } = state.visible_mixer_render_source()
        else {
            panic!("expected scene mixer render source");
        };

        assert_eq!(scene, "Pinned");
        assert!(matches!(status, MixerVisibleAudioStatus::Loading));
    }

    #[test]
    fn visible_mixer_render_source_selected_reports_missing_for_other_loaded_scene() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;
        state.mixer.selected_scene = Some("Selected".to_string());
        let request = state.set_mixer_audio_loading("Loaded".to_string());
        state.set_mixer_audio_success(request.clone(), vec![input("Other Mic")]);

        let MixerVisibleRenderSource::Scene { scene, status } = state.visible_mixer_render_source()
        else {
            panic!("expected scene mixer render source");
        };

        assert_eq!(scene, "Selected");
        assert!(matches!(status, MixerVisibleAudioStatus::Missing));
    }

    #[test]
    fn visible_mixer_render_source_pinned_reports_target_error_over_other_loaded_scene() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::PinnedScene;
        state.mixer.pinned_scene = Some("Pinned".to_string());
        let request = state.set_mixer_audio_loading("Loaded".to_string());
        state.set_mixer_audio_success(request.clone(), vec![input("Other Mic")]);
        let request = state.set_mixer_audio_loading("Pinned".to_string());
        state.set_mixer_audio_failure(request.clone(), "OBS failed".to_string());

        let MixerVisibleRenderSource::Scene { scene, status } = state.visible_mixer_render_source()
        else {
            panic!("expected scene mixer render source");
        };

        assert_eq!(scene, "Pinned");
        let MixerVisibleAudioStatus::Error(error) = status else {
            panic!("expected pinned mixer error");
        };
        assert_eq!(error.scene, "Pinned");
        assert_eq!(error.message, "OBS failed");
    }

    #[test]
    fn visible_mixer_render_source_scene_modes_report_missing_without_target_scene() {
        let mut state = app_state();

        state.mixer.mode = MixerMode::SelectedScene;
        assert!(matches!(
            state.visible_mixer_render_source(),
            MixerVisibleRenderSource::MissingScene
        ));

        state.mixer.mode = MixerMode::PinnedScene;
        assert!(matches!(
            state.visible_mixer_render_source(),
            MixerVisibleRenderSource::MissingScene
        ));
    }

    #[test]
    fn mixer_scene_refresh_target_active_mode_has_no_scene_specific_target() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::ActiveScene;
        state.scene_inventory.current_id = Some("Active".to_string());
        state.mixer.selected_scene = Some("Selected".to_string());
        state.mixer.pinned_scene = Some("Pinned".to_string());

        assert_eq!(state.mixer_scene_refresh_target(), None);
    }

    #[test]
    fn mixer_scene_refresh_target_selected_mode_uses_selected_scene() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;
        state.scene_inventory.current_id = Some("Active".to_string());
        state.mixer.selected_scene = Some("Selected".to_string());

        assert_eq!(state.mixer_scene_refresh_target(), Some("Selected"));
        assert_eq!(
            state.mixer_scene_refresh_target_details(),
            Some(MixerSceneRefreshTarget {
                scene: "Selected",
                reason: MixerSceneRefreshTargetReason::DirectSelectedScene,
            })
        );
    }

    #[test]
    fn mixer_scene_refresh_target_selected_mode_falls_back_to_current_scene() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;
        state.scene_inventory.current_id = Some("Active".to_string());

        assert_eq!(state.mixer_scene_refresh_target(), Some("Active"));
        assert_eq!(
            state.mixer_scene_refresh_target_details(),
            Some(MixerSceneRefreshTarget {
                scene: "Active",
                reason: MixerSceneRefreshTargetReason::SelectedModeCurrentSceneFallback,
            })
        );
    }

    #[test]
    fn mixer_scene_refresh_target_selected_mode_reports_none_without_selected_or_current_scene() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;

        assert_eq!(state.mixer_scene_refresh_target(), None);
        assert_eq!(state.mixer_scene_refresh_target_details(), None);
    }

    #[test]
    fn mixer_scene_refresh_target_pinned_mode_uses_pinned_scene() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::PinnedScene;
        state.scene_inventory.current_id = Some("Active".to_string());
        state.mixer.selected_scene = Some("Selected".to_string());
        state.mixer.pinned_scene = Some("Pinned".to_string());

        assert_eq!(state.mixer_scene_refresh_target(), Some("Pinned"));
        assert_eq!(
            state.mixer_scene_refresh_target_details(),
            Some(MixerSceneRefreshTarget {
                scene: "Pinned",
                reason: MixerSceneRefreshTargetReason::DirectPinnedScene,
            })
        );
    }

    #[test]
    fn mixer_scene_refresh_target_pinned_mode_falls_back_to_selected_scene() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::PinnedScene;
        state.scene_inventory.current_id = Some("Active".to_string());
        state.mixer.selected_scene = Some("Selected".to_string());

        assert_eq!(state.mixer_scene_refresh_target(), Some("Selected"));
        assert_eq!(
            state.mixer_scene_refresh_target_details(),
            Some(MixerSceneRefreshTarget {
                scene: "Selected",
                reason: MixerSceneRefreshTargetReason::PinnedModeSelectedSceneFallback,
            })
        );
    }

    #[test]
    fn mixer_scene_refresh_target_pinned_mode_falls_back_to_current_scene() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::PinnedScene;
        state.scene_inventory.current_id = Some("Active".to_string());

        assert_eq!(state.mixer_scene_refresh_target(), Some("Active"));
        assert_eq!(
            state.mixer_scene_refresh_target_details(),
            Some(MixerSceneRefreshTarget {
                scene: "Active",
                reason: MixerSceneRefreshTargetReason::PinnedModeCurrentSceneFallback,
            })
        );
    }

    #[test]
    fn mixer_scene_refresh_target_pinned_mode_reports_none_without_any_fallback_scene() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::PinnedScene;

        assert_eq!(state.mixer_scene_refresh_target(), None);
        assert_eq!(state.mixer_scene_refresh_target_details(), None);
    }

    #[test]
    fn mixer_inspection_snapshot_reports_active_render_source() {
        let mut state = app_state();
        state.active_audio_status = ActiveAudioStatus::Ready;
        state.mixer.mode = MixerMode::ActiveScene;
        state.scene_inventory.current_id = Some("Active".to_string());
        state.mixer.selected_scene = Some("Selected".to_string());
        state.mixer.pinned_scene = Some("Pinned".to_string());
        state.audio_inputs = vec![loud_muted_input("Live Mic")];
        state.set_mixer_audio_loading("Selected".to_string());

        let snapshot = state.mixer_inspection_snapshot();

        assert_eq!(snapshot.mode, MixerMode::ActiveScene);
        assert_eq!(snapshot.selected_scene, Some("Selected"));
        assert_eq!(snapshot.pinned_scene, Some("Pinned"));
        assert_eq!(snapshot.refresh_target, None);
        assert_eq!(
            snapshot.render_source_kind,
            MixerInspectionRenderSourceKind::ActiveScene
        );
        assert_eq!(snapshot.scene, Some("Active"));
        assert_eq!(
            snapshot.status,
            MixerInspectionStatus::LoadedWithVisibleInputCards
        );
        assert_eq!(snapshot.inputs.len(), 1);
        assert_snapshot_input(&snapshot.inputs[0], "Live Mic", true);
    }

    #[test]
    fn mixer_inspection_snapshot_reports_selected_direct_target() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;
        state.scene_inventory.current_id = Some("Active".to_string());
        state.mixer.selected_scene = Some("Selected".to_string());
        let request = state.set_mixer_audio_loading("Selected".to_string());
        state.set_mixer_audio_success(request.clone(), vec![loud_muted_input("Scene Mic")]);

        let snapshot = state.mixer_inspection_snapshot();

        assert_eq!(snapshot.mode, MixerMode::SelectedScene);
        assert_eq!(
            snapshot.refresh_target,
            Some(MixerSceneRefreshTarget {
                scene: "Selected",
                reason: MixerSceneRefreshTargetReason::DirectSelectedScene,
            })
        );
        assert_eq!(
            snapshot.render_source_kind,
            MixerInspectionRenderSourceKind::Scene
        );
        assert_eq!(snapshot.scene, Some("Selected"));
        assert_eq!(
            snapshot.status,
            MixerInspectionStatus::LoadedWithVisibleInputCards
        );
        assert_eq!(snapshot.inputs.len(), 1);
        assert_snapshot_input(&snapshot.inputs[0], "Scene Mic", true);
    }

    #[test]
    fn mixer_inspection_snapshot_reports_selected_fallback_target() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;
        state.scene_inventory.current_id = Some("Active".to_string());
        let request = state.set_mixer_audio_loading("Active".to_string());
        state.set_mixer_audio_success(request.clone(), vec![loud_muted_input("Active Mic")]);

        let snapshot = state.mixer_inspection_snapshot();

        assert_eq!(snapshot.selected_scene, None);
        assert_eq!(
            snapshot.refresh_target,
            Some(MixerSceneRefreshTarget {
                scene: "Active",
                reason: MixerSceneRefreshTargetReason::SelectedModeCurrentSceneFallback,
            })
        );
        assert_eq!(snapshot.scene, Some("Active"));
        assert_eq!(
            snapshot.status,
            MixerInspectionStatus::LoadedWithVisibleInputCards
        );
        assert_eq!(snapshot.inputs.len(), 1);
        assert_snapshot_input(&snapshot.inputs[0], "Active Mic", true);
    }

    #[test]
    fn mixer_inspection_snapshot_reports_pinned_direct_target() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::PinnedScene;
        state.scene_inventory.current_id = Some("Active".to_string());
        state.mixer.selected_scene = Some("Selected".to_string());
        state.mixer.pinned_scene = Some("Pinned".to_string());
        let request = state.set_mixer_audio_loading("Pinned".to_string());
        state.set_mixer_audio_success(request.clone(), vec![loud_muted_input("Pinned Mic")]);

        let snapshot = state.mixer_inspection_snapshot();

        assert_eq!(snapshot.mode, MixerMode::PinnedScene);
        assert_eq!(snapshot.selected_scene, Some("Selected"));
        assert_eq!(snapshot.pinned_scene, Some("Pinned"));
        assert_eq!(
            snapshot.refresh_target,
            Some(MixerSceneRefreshTarget {
                scene: "Pinned",
                reason: MixerSceneRefreshTargetReason::DirectPinnedScene,
            })
        );
        assert_eq!(snapshot.scene, Some("Pinned"));
        assert_eq!(
            snapshot.status,
            MixerInspectionStatus::LoadedWithVisibleInputCards
        );
        assert_eq!(snapshot.inputs.len(), 1);
        assert_snapshot_input(&snapshot.inputs[0], "Pinned Mic", true);
    }

    #[test]
    fn mixer_inspection_snapshot_reports_pinned_fallback_target() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::PinnedScene;
        state.scene_inventory.current_id = Some("Active".to_string());
        state.mixer.selected_scene = Some("Selected".to_string());
        let request = state.set_mixer_audio_loading("Selected".to_string());
        state.set_mixer_audio_success(request.clone(), vec![loud_muted_input("Selected Mic")]);

        let snapshot = state.mixer_inspection_snapshot();

        assert_eq!(snapshot.pinned_scene, None);
        assert_eq!(
            snapshot.refresh_target,
            Some(MixerSceneRefreshTarget {
                scene: "Selected",
                reason: MixerSceneRefreshTargetReason::PinnedModeSelectedSceneFallback,
            })
        );
        assert_eq!(snapshot.scene, Some("Selected"));
        assert_eq!(
            snapshot.status,
            MixerInspectionStatus::LoadedWithVisibleInputCards
        );
        assert_eq!(snapshot.inputs.len(), 1);
        assert_snapshot_input(&snapshot.inputs[0], "Selected Mic", true);
    }

    #[test]
    fn mixer_inspection_snapshot_reports_loading_status() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;
        state.mixer.selected_scene = Some("Selected".to_string());
        state.set_mixer_audio_loading("Selected".to_string());

        let snapshot = state.mixer_inspection_snapshot();

        assert_eq!(snapshot.scene, Some("Selected"));
        assert_eq!(
            snapshot.status,
            MixerInspectionStatus::LoadingPlaceholderShown
        );
        assert!(snapshot.inputs.is_empty());
    }

    #[test]
    fn mixer_inspection_snapshot_reports_error_status() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::PinnedScene;
        state.mixer.pinned_scene = Some("Pinned".to_string());
        let request = state.set_mixer_audio_loading("Pinned".to_string());
        state.set_mixer_audio_failure(request.clone(), "OBS failed".to_string());

        let snapshot = state.mixer_inspection_snapshot();

        assert_eq!(snapshot.scene, Some("Pinned"));
        assert_eq!(
            snapshot.status,
            MixerInspectionStatus::ErrorPlaceholderShown("OBS failed")
        );
        assert!(snapshot.inputs.is_empty());
    }

    #[test]
    fn mixer_inspection_snapshot_reports_missing_status() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;
        state.mixer.selected_scene = Some("Selected".to_string());
        let request = state.set_mixer_audio_loading("Other".to_string());
        state.set_mixer_audio_success(request.clone(), vec![loud_muted_input("Other Mic")]);

        let snapshot = state.mixer_inspection_snapshot();

        assert_eq!(
            snapshot.render_source_kind,
            MixerInspectionRenderSourceKind::Scene
        );
        assert_eq!(snapshot.scene, Some("Selected"));
        assert_eq!(snapshot.status, MixerInspectionStatus::MissingNoTarget);
        assert!(snapshot.inputs.is_empty());
    }

    #[test]
    fn mixer_inspection_snapshot_reports_active_loaded_empty_status() {
        let mut state = app_state();
        state.active_audio_status = ActiveAudioStatus::Ready;
        state.mixer.mode = MixerMode::ActiveScene;
        state.scene_inventory.current_id = Some("Active".to_string());

        let snapshot = state.mixer_inspection_snapshot();

        assert_eq!(
            snapshot.render_source_kind,
            MixerInspectionRenderSourceKind::ActiveScene
        );
        assert_eq!(snapshot.scene, Some("Active"));
        assert_eq!(snapshot.status, MixerInspectionStatus::LoadedNoAudioSources);
        assert!(snapshot.inputs.is_empty());
    }

    #[test]
    fn mixer_inspection_snapshot_reports_scene_loaded_empty_status() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;
        state.mixer.selected_scene = Some("Selected".to_string());
        let request = state.set_mixer_audio_loading("Selected".to_string());
        state.set_mixer_audio_success(request.clone(), Vec::new());

        let snapshot = state.mixer_inspection_snapshot();

        assert_eq!(
            snapshot.render_source_kind,
            MixerInspectionRenderSourceKind::Scene
        );
        assert_eq!(snapshot.scene, Some("Selected"));
        assert_eq!(snapshot.status, MixerInspectionStatus::LoadedNoAudioSources);
        assert!(snapshot.inputs.is_empty());
    }

    #[test]
    fn mixer_inspection_status_can_represent_filtered_loaded_empty_status() {
        assert_ne!(
            MixerInspectionStatus::LoadedNoAudioSources,
            MixerInspectionStatus::LoadedNoMatchingAudioSourcesAfterFiltering
        );
    }

    #[test]
    fn mixer_input_mute_update_changes_visible_loaded_snapshot() {
        let mut state = app_state();
        let request = state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success(request.clone(), vec![input("Mic"), input("Music")]);

        assert!(state.update_mixer_input_mute("Mic", true));

        assert!(visible_loaded_input(&state, "Scene A", "Mic").muted);
        assert!(!visible_loaded_input(&state, "Scene A", "Music").muted);
    }

    #[test]
    fn mixer_input_volume_update_changes_visible_loaded_snapshot() {
        let mut state = app_state();
        let request = state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success(request.clone(), vec![input("Mic"), input("Music")]);

        assert!(state.update_mixer_input_volume("Music", 0.42, -7.5));

        assert_eq!(
            visible_loaded_input(&state, "Scene A", "Music").volume_mul,
            0.42
        );
        assert_eq!(
            visible_loaded_input(&state, "Scene A", "Music").volume_db,
            -7.5
        );
        assert_eq!(
            visible_loaded_input(&state, "Scene A", "Mic").volume_mul,
            1.0
        );
    }

    #[test]
    fn mixer_input_mute_update_preserves_hidden_loaded_snapshot_invariant() {
        let mut state = app_state();
        let request = state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success(request.clone(), vec![input("Mic"), input("Music")]);

        assert!(state.update_mixer_input_mute("Mic", true));

        let request = state.set_mixer_audio_loading("Scene A".to_string());
        assert_eq!(state.mixer_audio_refresh.requested.as_ref(), Some(&request));
        assert!(hidden_loaded_input(&state, "Scene A", "Mic").muted);
        assert!(!hidden_loaded_input(&state, "Scene A", "Music").muted);

        let transition = state.set_mixer_audio_failure(request.clone(), "OBS failed".to_string());
        assert_eq!(transition, MixerAudioRefreshTransition::Failure);
        assert!(hidden_loaded_input(&state, "Scene A", "Mic").muted);
        assert!(!hidden_loaded_input(&state, "Scene A", "Music").muted);
    }

    #[test]
    fn mixer_input_volume_update_preserves_hidden_loaded_snapshot_invariant() {
        let mut state = app_state();
        let request = state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success(request.clone(), vec![input("Mic"), input("Music")]);

        assert!(state.update_mixer_input_volume("Music", 0.42, -7.5));

        let request = state.set_mixer_audio_loading("Scene A".to_string());
        assert_eq!(state.mixer_audio_refresh.requested.as_ref(), Some(&request));
        assert_eq!(
            hidden_loaded_input(&state, "Scene A", "Music").volume_mul,
            0.42
        );
        assert_eq!(
            hidden_loaded_input(&state, "Scene A", "Music").volume_db,
            -7.5
        );
        assert_eq!(
            hidden_loaded_input(&state, "Scene A", "Mic").volume_mul,
            1.0
        );

        let transition = state.set_mixer_audio_failure(request.clone(), "OBS failed".to_string());
        assert_eq!(transition, MixerAudioRefreshTransition::Failure);
        assert_eq!(
            hidden_loaded_input(&state, "Scene A", "Music").volume_mul,
            0.42
        );
        assert_eq!(
            hidden_loaded_input(&state, "Scene A", "Music").volume_db,
            -7.5
        );
        assert_eq!(
            hidden_loaded_input(&state, "Scene A", "Mic").volume_mul,
            1.0
        );
    }

    #[test]
    fn mixer_input_mute_update_is_hidden_by_same_scene_loading_and_failure_status() {
        let mut state = app_state();
        let request = state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success(request.clone(), vec![input("Mic"), input("Music")]);

        assert!(state.update_mixer_input_mute("Mic", true));

        let request = state.set_mixer_audio_loading("Scene A".to_string());
        assert_eq!(state.mixer_audio_refresh.requested.as_ref(), Some(&request));
        assert!(matches!(
            state.visible_mixer_audio_status("Scene A"),
            MixerVisibleAudioStatus::Loading
        ));

        let transition = state.set_mixer_audio_failure(request.clone(), "OBS failed".to_string());
        assert_eq!(transition, MixerAudioRefreshTransition::Failure);
        let MixerVisibleAudioStatus::Error(error) = state.visible_mixer_audio_status("Scene A")
        else {
            panic!("expected visible mixer error");
        };
        assert_eq!(error.message, "OBS failed");

        let request = state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success(request.clone(), vec![input("Mic"), input("Music")]);
        assert!(!visible_loaded_input(&state, "Scene A", "Mic").muted);
    }

    #[test]
    fn mixer_input_volume_update_is_hidden_by_same_scene_loading_and_failure_status() {
        let mut state = app_state();
        let request = state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success(request.clone(), vec![input("Mic"), input("Music")]);

        assert!(state.update_mixer_input_volume("Music", 0.42, -7.5));

        let request = state.set_mixer_audio_loading("Scene A".to_string());
        assert_eq!(state.mixer_audio_refresh.requested.as_ref(), Some(&request));
        assert!(matches!(
            state.visible_mixer_audio_status("Scene A"),
            MixerVisibleAudioStatus::Loading
        ));

        let transition = state.set_mixer_audio_failure(request.clone(), "OBS failed".to_string());
        assert_eq!(transition, MixerAudioRefreshTransition::Failure);
        let MixerVisibleAudioStatus::Error(error) = state.visible_mixer_audio_status("Scene A")
        else {
            panic!("expected visible mixer error");
        };
        assert_eq!(error.message, "OBS failed");

        let request = state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success(request.clone(), vec![input("Mic"), input("Music")]);
        assert_eq!(
            visible_loaded_input(&state, "Scene A", "Music").volume_mul,
            1.0
        );
        assert_eq!(
            visible_loaded_input(&state, "Scene A", "Music").volume_db,
            0.0
        );
    }

    #[test]
    fn completed_mixer_cache_and_error_are_retired_before_same_name_scope_is_reused() {
        for retire_session in [false, true] {
            for failed in [false, true] {
                let mut state = app_state();
                seed_session_observations(&mut state);
                let old = state.set_mixer_audio_loading("Common".into());
                state.set_mixer_audio_success(old.clone(), vec![input("Old Mic")]);
                if failed {
                    let retry = state.set_mixer_audio_loading("Common".into());
                    state.set_mixer_audio_failure(retry, "old error".into());
                }
                assert_eq!(state.mixer_audio_refresh.requested_scene(), None);

                if retire_session {
                    state.reset_obs_session(ObsStatus::Disconnected);
                } else {
                    state.invalidate_collection_runtime();
                }

                assert!(state.mixer_audio_refresh.loaded.is_none());
                assert!(state.mixer_audio_refresh.error.is_none());
                assert!(matches!(
                    state.visible_mixer_audio_status("Common"),
                    MixerVisibleAudioStatus::Missing
                ));
                seed_session_observations(&mut state);
                let current = state.set_mixer_audio_loading("Common".into());
                assert!(current.id > old.id && current.scope_epoch > old.scope_epoch);
                assert!(state
                    .set_mixer_audio_success(old.clone(), vec![input("Late Mic")])
                    .is_stale());
                assert!(state
                    .set_mixer_audio_failure(old, "late failure".into())
                    .is_stale());
                assert!(matches!(
                    state.visible_mixer_audio_status("Common"),
                    MixerVisibleAudioStatus::Loading
                ));
                assert_eq!(
                    state.set_mixer_audio_success(current, vec![input("Current Mic")]),
                    MixerAudioRefreshTransition::Success
                );
                assert_eq!(
                    visible_loaded_input(&state, "Common", "Current Mic").id,
                    "Current Mic"
                );
            }
        }
    }
}
