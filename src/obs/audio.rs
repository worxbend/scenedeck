//! Fail-closed audio observations: definite non-audio inputs are omitted;
//! unavailable properties or membership fail the snapshot, never fake emptiness.

use std::collections::HashSet;

use obws::error::Error;
use obws::requests::{inputs::InputId, scenes::SceneId};
use obws::responses::{scene_items::SourceType, StatusCode};

use crate::domain::audio::{AudioInput, AudioSourceScope};
use crate::infra::error::AppError;

use super::client::{collect_bounded_ordered, ObsClient, MAX_CONCURRENT_OBS_REQUESTS};
use super::sources::{ScanBudget, MAX_INSPECTED_ITEMS};

/// OBS 5.5's GetInputMute/Volume reject non-audio with this precise combination.
/// 604 alone is ambiguous (blocked/unavailable resources use it too).
pub(super) fn is_non_audio(error: &Error) -> bool {
    matches!(error, Error::Api { code: StatusCode::InvalidResourceState, message: Some(message) }
        if message == "The specified input does not support audio.")
}

impl ObsClient {
    /// Return a complete usable audio snapshot, or an explicit observation error.
    /// Only a definite non-audio rejection can silently omit an input.
    pub async fn get_all_audio_inputs(&self) -> Result<Vec<AudioInput>, AppError> {
        let sources = self
            .request(self.inner.inputs().list(None))
            .await
            .map_err(AppError::request)?
            .into_iter()
            .map(|input| AudioInputSource {
                name: input.id.name,
                scope: AudioSourceScope::Global,
                parent_scene_path: Vec::new(),
            })
            .collect();
        self.get_audio_inputs_by_name(sources).await
    }

    /// Enabled scene audio plus global devices. Unlike full source inspection,
    /// disabled ancestors prune traversal; unknown membership fails closed.
    pub async fn get_scene_audio_inputs(
        &self,
        scene_name: &str,
        filter: &[String],
    ) -> Result<Vec<AudioInput>, AppError> {
        let filter: HashSet<&str> = filter.iter().map(String::as_str).collect();
        let mut seen_inputs = HashSet::new();
        let mut sources = Vec::new();
        for name in self.get_global_audio_input_names().await? {
            if seen_inputs.insert(name.clone()) {
                sources.push(AudioInputSource {
                    name,
                    scope: AudioSourceScope::Global,
                    parent_scene_path: Vec::new(),
                });
            }
        }
        let mut visited = HashSet::new();
        let mut pending = vec![(
            scene_name.to_owned(),
            false,
            vec![scene_name.to_owned()],
            AudioSourceScope::ActiveScene,
        )];
        let mut budget = ScanBudget::new(MAX_INSPECTED_ITEMS);
        while let Some((owner, group, path, scope)) = pending.pop() {
            if !visited.insert((owner.clone(), group)) {
                continue;
            }
            let items = self.list_container(&owner, group).await?;
            budget.inspect(items.len())?;
            let items = collect_bounded_ordered(items, MAX_CONCURRENT_OBS_REQUESTS, |item| async {
                let enabled = self
                    .request(
                        self.inner
                            .scene_items()
                            .enabled(SceneId::Name(&owner), item.id),
                    )
                    .await
                    .map_err(|error| {
                        AppError::request(format!(
                            "{owner} / {} visibility unavailable: {error}",
                            item.source_name
                        ))
                    })?;
                Ok::<_, AppError>((item, enabled))
            })
            .await?;
            for (item, enabled) in items {
                if !enabled {
                    continue;
                }
                match item.source_type {
                    SourceType::Input => {
                        if !filter.is_empty() && !filter.contains(item.source_name.as_str()) {
                            continue;
                        }
                        if seen_inputs.insert(item.source_name.clone()) {
                            sources.push(AudioInputSource {
                                name: item.source_name,
                                scope,
                                parent_scene_path: path.clone(),
                            });
                        }
                    }
                    SourceType::Scene => {
                        let group = item.is_group.unwrap_or(false);
                        let mut child_path = path.clone();
                        child_path.push(item.source_name.clone());
                        pending.push((
                            item.source_name,
                            group,
                            child_path,
                            if group {
                                AudioSourceScope::Group
                            } else {
                                AudioSourceScope::NestedScene
                            },
                        ));
                    }
                    _ => {}
                }
            }
        }
        self.get_audio_inputs_by_name(sources).await
    }

    async fn get_global_audio_input_names(&self) -> Result<Vec<String>, AppError> {
        let specials = self
            .request(self.inner.inputs().specials())
            .await
            .map_err(AppError::request)?;
        Ok([
            specials.desktop1,
            specials.desktop2,
            specials.mic1,
            specials.mic2,
            specials.mic3,
            specials.mic4,
        ]
        .into_iter()
        .flatten()
        .collect())
    }

    async fn get_audio_inputs_by_name(
        &self,
        sources: Vec<AudioInputSource>,
    ) -> Result<Vec<AudioInput>, AppError> {
        if sources.len() > MAX_INSPECTED_ITEMS {
            return Err(AppError::request(
                "Audio inventory exceeds inspection budget",
            ));
        }
        let inputs =
            collect_bounded_ordered(sources, MAX_CONCURRENT_OBS_REQUESTS, |source| async move {
                let muted = match self
                    .request(self.inner.inputs().muted(InputId::Name(&source.name)))
                    .await
                {
                    Ok(muted) => muted,
                    Err(error) if is_non_audio(&error) => return Ok(None),
                    Err(error) => {
                        return Err(AppError::request(format!(
                            "{} mute unavailable: {error}",
                            source.name
                        )))
                    }
                };
                let volume = self
                    .request(self.inner.inputs().volume(InputId::Name(&source.name)))
                    .await
                    .map_err(|error| {
                        AppError::request(format!("{} volume unavailable: {error}", source.name))
                    })?;
                if !volume.mul.is_finite() || volume.mul < 0.0 || !volume.db.is_finite() {
                    return Err(AppError::request(format!(
                        "{} returned invalid audio volume",
                        source.name
                    )));
                }
                Ok(Some(
                    AudioInput::new(
                        source.name,
                        muted,
                        f64::from(volume.mul),
                        f64::from(volume.db),
                    )
                    .with_source_context(source.scope, source.parent_scene_path),
                ))
            })
            .await?;
        Ok(inputs.into_iter().flatten().collect())
    }
}

struct AudioInputSource {
    name: String,
    scope: AudioSourceScope,
    parent_scene_path: Vec<String>,
}
