//! Bounded container enumeration: structural graphs flatten groups, while
//! usage inspection retains every path and its observed visibility.

use std::collections::{HashMap, HashSet};

use obws::requests::scenes::SceneId;
use obws::responses::scene_items::{SceneItem, SourceType};

use crate::domain::control::SourceItem;
use crate::domain::graph::SceneGraph;
use crate::infra::error::AppError;

use super::client::{collect_bounded_ordered, ObsClient, MAX_CONCURRENT_OBS_REQUESTS};

pub(super) const MAX_INSPECTED_ITEMS: usize = 4096;
const MAX_ROOT_SCENES: usize = 64;
const MAX_PATH_DEPTH: usize = 32;
const MAX_WARNING_SAMPLES: usize = 32;

/// Work is charged before a request/inspection, including failed observations.
pub(super) struct ScanBudget {
    remaining: usize,
    containers: usize,
}

impl ScanBudget {
    pub(super) fn new(items: usize) -> Self {
        Self {
            remaining: items,
            containers: items.saturating_add(MAX_ROOT_SCENES),
        }
    }
    pub(super) fn inspect(&mut self, count: usize) -> Result<(), AppError> {
        self.remaining = self
            .remaining
            .checked_sub(count)
            .ok_or_else(|| AppError::request("Source inspection budget exhausted"))?;
        Ok(())
    }
    fn container(&mut self) -> Result<(), AppError> {
        self.containers = self
            .containers
            .checked_sub(1)
            .ok_or_else(|| AppError::request("Container inspection budget exhausted"))?;
        Ok(())
    }
}

#[derive(Default)]
struct ScanWarnings {
    samples: Vec<String>,
    omitted: usize,
}
impl ScanWarnings {
    fn push(&mut self, warning: String) {
        if self.samples.len() < MAX_WARNING_SAMPLES {
            self.samples.push(warning);
        } else {
            self.omitted += 1;
        }
    }
    fn finish(mut self) -> Vec<String> {
        if self.omitted > 0 {
            self.samples.push(format!(
                "{} additional source scan warnings omitted",
                self.omitted
            ));
        }
        self.samples
    }
}

impl ObsClient {
    pub(super) async fn list_container(
        &self,
        owner: &str,
        group: bool,
    ) -> Result<Vec<SceneItem>, AppError> {
        let items = self.inner.scene_items();
        if group {
            self.request(items.list_group(SceneId::Name(owner))).await
        } else {
            self.request(items.list(SceneId::Name(owner))).await
        }
        .map_err(|error| AppError::request(format!("{owner}: {error}")))
    }

    /// Structural scene edges include disabled groups, flattened into the owning
    /// scene. Groups are containers, not scene-role endpoints. Any failed or
    /// over-budget read fails the graph rather than publishing a partial graph.
    pub async fn get_scene_graph(&self, scene_names: &[String]) -> Result<SceneGraph, AppError> {
        let mut graph = SceneGraph::default();
        let mut budget = ScanBudget::new(MAX_INSPECTED_ITEMS);
        for scene in scene_names {
            let mut groups = HashSet::new();
            let mut children = Vec::new();
            let mut seen_children = HashSet::new();
            let mut pending = vec![(scene.clone(), false)];
            while let Some((owner, group)) = pending.pop() {
                if !groups.insert((owner.clone(), group)) {
                    continue;
                }
                budget.container()?;
                let items = self.list_container(&owner, group).await?;
                budget.inspect(items.len())?;
                for item in items {
                    if item.source_type != SourceType::Scene {
                        continue;
                    }
                    if item.is_group.unwrap_or(false) {
                        pending.push((item.source_name, true));
                    } else if seen_children.insert(item.source_name.clone()) {
                        children.push(item.source_name);
                    }
                }
            }
            if !children.is_empty() {
                graph.edges.insert(scene.clone(), children);
            }
        }
        Ok(graph)
    }

    pub(super) async fn scan_source_items(
        &self,
        scene: &str,
        limit: usize,
    ) -> (Vec<SourceItem>, Vec<String>) {
        let mut budget = ScanBudget::new(limit);
        let mut warnings = ScanWarnings::default();
        let identities = self.input_identities(&mut warnings).await;
        let items = self
            .scan_source_paths(scene, &identities, &mut budget, &mut warnings)
            .await;
        (items, warnings.finish())
    }

    async fn input_identities(&self, warnings: &mut ScanWarnings) -> HashMap<String, String> {
        match self.request(self.inner.inputs().list(None)).await {
            Ok(inputs) => inputs
                .into_iter()
                .map(|input| (input.id.name, input.id.uuid.to_string()))
                .collect(),
            Err(error) => {
                warnings.push(format!("Input identity lookup: {error}"));
                HashMap::new()
            }
        }
    }

    async fn scan_source_paths(
        &self,
        scene: &str,
        identities: &HashMap<String, String>,
        budget: &mut ScanBudget,
        warnings: &mut ScanWarnings,
    ) -> Vec<SourceItem> {
        let mut result = Vec::new();
        // Path-local ancestors preserve repeated usages without following cycles.
        let mut pending = vec![(scene.to_owned(), false, vec![scene.to_owned()], true)];
        while let Some((owner, group, path, ancestor_enabled)) = pending.pop() {
            if budget.remaining == 0 || budget.container().is_err() {
                warnings.push("Source inventory truncated: inspection budget exhausted".into());
                break;
            }
            let items = match self.list_container(&owner, group).await {
                Ok(items) => items,
                Err(error) => {
                    warnings.push(format!("{}: {error}", path.join(" / ")));
                    continue;
                }
            };
            let count = items.len().min(budget.remaining);
            if count < items.len() {
                warnings.push("Source inventory truncated: inspection budget exhausted".into());
            }
            budget
                .inspect(count)
                .expect("count limited to remaining work");
            let observed = collect_bounded_ordered(
                items.into_iter().take(count),
                MAX_CONCURRENT_OBS_REQUESTS,
                |item| async {
                    let enabled = self
                        .request(
                            self.inner
                                .scene_items()
                                .enabled(SceneId::Name(&owner), item.id),
                        )
                        .await;
                    Ok::<_, std::convert::Infallible>((item, enabled))
                },
            )
            .await
            .unwrap_or_else(|never| match never {});
            for (item, enabled) in observed {
                let enabled = match enabled {
                    Ok(enabled) => enabled,
                    Err(error) => {
                        warnings.push(format!(
                            "{owner} / {} visibility unavailable: {error}",
                            item.source_name
                        ));
                        continue;
                    }
                };
                let is_scene = item.source_type == SourceType::Scene;
                let is_group = item.is_group.unwrap_or(false);
                result.push(SourceItem {
                    owner: owner.clone(),
                    item_id: item.id,
                    input: item.source_name.clone(),
                    input_uuid: identities
                        .get(&item.source_name)
                        .cloned()
                        .unwrap_or_default(),
                    kind: item.input_kind,
                    path: path.clone(),
                    enabled,
                    ancestor_enabled,
                    is_group,
                    is_scene,
                });
                if is_scene {
                    if path.contains(&item.source_name) || path.len() >= MAX_PATH_DEPTH {
                        warnings.push(format!(
                            "Recursion stopped at {} / {}",
                            path.join(" / "),
                            item.source_name
                        ));
                    } else {
                        let mut child_path = path.clone();
                        child_path.push(item.source_name.clone());
                        pending.push((
                            item.source_name,
                            is_group,
                            child_path,
                            ancestor_enabled && enabled,
                        ));
                    }
                }
            }
        }
        result
    }

    /// All usage paths, including disabled ancestors. Failures and truncation
    /// remain visible; attempted work is shared across roots, not output length.
    pub async fn get_source_usage(
        &self,
        input: &str,
    ) -> Result<(Vec<SourceItem>, Vec<String>), AppError> {
        self.source_usage_with_limit(input, MAX_INSPECTED_ITEMS)
            .await
    }

    pub(super) async fn source_usage_with_limit(
        &self,
        input: &str,
        limit: usize,
    ) -> Result<(Vec<SourceItem>, Vec<String>), AppError> {
        let scenes = self
            .request(self.inner.scenes().list())
            .await
            .map_err(AppError::request)?;
        let mut result = Vec::new();
        let mut warnings = ScanWarnings::default();
        let identities = self.input_identities(&mut warnings).await;
        let mut budget = ScanBudget::new(limit);
        for (index, scene) in scenes.scenes.into_iter().enumerate() {
            if index >= MAX_ROOT_SCENES || budget.remaining == 0 {
                warnings.push(format!("Usage search is partial: limited to {MAX_ROOT_SCENES} scenes and {limit} inspected items"));
                break;
            }
            let items = self
                .scan_source_paths(&scene.id.name, &identities, &mut budget, &mut warnings)
                .await;
            result.extend(items.into_iter().filter(|item| item.input == input));
        }
        Ok((result, warnings.finish()))
    }
}
