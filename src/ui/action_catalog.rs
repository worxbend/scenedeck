//! One observed-state catalog for operator actions and application navigation.

use crate::controller::state::{AppState, ObsStatus, Page, RegistryStatus};
use crate::domain::control::{ControlAction, MediaAction};
use crate::domain::operator_action::{ActionTarget, OperatorAction};
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::ui::navigation::NavigationContext;
use i18n_embed_fl::fl;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum CatalogAction {
    Page(Page),
    Bookmarks,
    Reports,
    Operator(OperatorAction),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Entry {
    pub action: CatalogAction,
    pub label: String,
    pub keywords: String,
    pub available: bool,
}

impl Entry {
    pub fn matches(&self, query: &str) -> bool {
        let haystack = format!("{} {}", self.label, self.keywords).to_lowercase();
        query
            .split_whitespace()
            .all(|word| haystack.contains(&word.to_lowercase()))
    }
}

fn inputs(state: &AppState) -> Vec<ActionTarget> {
    state
        .workspace
        .input_catalog
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .map(|inventory| inventory.iter().map(|input| input.target.clone()).collect())
        .unwrap_or_default()
}

fn observed_input<'a>(
    state: &'a AppState,
    target: &ActionTarget,
) -> Option<&'a crate::domain::operator_input::InputObservation> {
    state
        .workspace
        .operator_inputs
        .iter()
        .find(|input| target.matches(&input.target.name, input.target.uuid.as_deref()))
}

fn observed_mute(state: &AppState, target: &ActionTarget) -> Option<bool> {
    state
        .workspace
        .audio
        .iter()
        .find(|audio| target.matches(&audio.id, audio.uuid.as_deref()))
        .map(|audio| audio.muted)
}

/// Actual current state, independent of the pin's requested direction.
pub(super) fn observed_label(state: &AppState, action: &OperatorAction) -> Option<String> {
    if !matches!(state.obs_status, ObsStatus::Connected { .. })
        || !state.workspace.can_control()
        || state.workflow_scope().collection.is_empty()
        || state.show_registry.status != RegistryStatus::Ready
    {
        return None;
    }
    match action {
        OperatorAction::SaveReplay => Some(if state.workspace.snapshot.as_ref()?.replay_active? {
            fl!(LANGUAGE_LOADER, "output-state-active")
        } else {
            fl!(LANGUAGE_LOADER, "output-state-inactive")
        }),
        OperatorAction::Scene { target } => {
            let mut scenes = state
                .scene_inventory
                .scenes
                .iter()
                .filter(|scene| target.matches(&scene.id, scene.uuid.as_deref()));
            let scene = scenes.next()?;
            if scenes.next().is_some() {
                return None;
            }
            let studio = &state.workspace.snapshot.as_ref()?.studio;
            Some(if studio.program.as_deref() == Some(&scene.id) {
                fl!(LANGUAGE_LOADER, "scene-card-program-label")
            } else if studio.preview.as_deref() == Some(&scene.id) {
                fl!(LANGUAGE_LOADER, "scene-card-preview-label")
            } else {
                fl!(LANGUAGE_LOADER, "quick-ready")
            })
        }
        OperatorAction::OpenSource { target }
        | OperatorAction::SetMuted { target, .. }
        | OperatorAction::ToggleMuted { target }
        | OperatorAction::RestartMedia { target }
        | OperatorAction::SetFilterEnabled { target, .. }
        | OperatorAction::ToggleFilter { target, .. } => {
            let inventory = state.workspace.input_catalog.as_ref()?.as_ref().ok()?;
            let input = crate::domain::operator_input::resolve_input(inventory, target)?;
            match action {
                OperatorAction::OpenSource { .. } => Some(fl!(LANGUAGE_LOADER, "quick-ready")),
                OperatorAction::SetMuted { .. } | OperatorAction::ToggleMuted { .. } => {
                    Some(if observed_mute(state, &input.target)? {
                        fl!(LANGUAGE_LOADER, "quick-muted")
                    } else {
                        fl!(LANGUAGE_LOADER, "quick-unmuted")
                    })
                }
                OperatorAction::RestartMedia { .. } => Some(
                    observed_input(state, &input.target)?
                        .media
                        .as_ref()?
                        .as_ref()
                        .ok()?
                        .state
                        .label(),
                ),
                OperatorAction::SetFilterEnabled { filter, .. }
                | OperatorAction::ToggleFilter { filter, .. } => {
                    let enabled = observed_input(state, &input.target)?
                        .filters
                        .as_ref()?
                        .as_ref()
                        .ok()?
                        .iter()
                        .find(|value| &value.name == filter)?
                        .enabled;
                    Some(if enabled {
                        fl!(LANGUAGE_LOADER, "quick-filter-enabled")
                    } else {
                        fl!(LANGUAGE_LOADER, "quick-filter-disabled")
                    })
                }
                _ => None,
            }
        }
    }
}

#[derive(Debug, PartialEq)]
enum Resolved {
    Control(ControlAction),
    OpenSource(String),
}

/// Re-evaluate identity, capability, scope readiness and freshness together.
fn resolve(state: &AppState, action: &OperatorAction) -> Option<Resolved> {
    resolve_with_inputs(state, action, &inputs(state))
}

fn resolve_with_inputs(
    state: &AppState,
    action: &OperatorAction,
    targets: &[ActionTarget],
) -> Option<Resolved> {
    if !matches!(state.obs_status, ObsStatus::Connected { .. })
        || state.workflow_scope().collection.is_empty()
    {
        return None;
    }
    let supports = |request: &str| state.available_requests.iter().any(|r| r == request);
    if let OperatorAction::Scene { target } = action {
        if state.show_registry.status != RegistryStatus::Ready {
            return None;
        }
        let mut matches = state
            .scene_inventory
            .scenes
            .iter()
            .filter(|scene| target.matches(&scene.id, scene.uuid.as_deref()));
        let scene = matches.next()?;
        if matches.next().is_some()
            || !state
                .registry
                .scenes
                .get(&scene.id)?
                .role?
                .is_live_switchable()
        {
            return None;
        }
        let action = state.workspace.scene_action(&scene.id)?;
        let request = match action {
            ControlAction::ActivateScene {
                studio_enabled: true,
                ..
            } => "SetCurrentPreviewScene",
            _ => "SetCurrentProgramScene",
        };
        return supports(request).then_some(Resolved::Control(action));
    }
    if !state.workspace.can_control() {
        return None;
    }
    if *action == OperatorAction::SaveReplay {
        return (state.workspace.snapshot.as_ref()?.replay_active == Some(true)
            && supports("SaveReplayBuffer"))
        .then_some(Resolved::Control(ControlAction::SaveReplay));
    }
    let target = match action {
        OperatorAction::OpenSource { target }
        | OperatorAction::SetMuted { target, .. }
        | OperatorAction::ToggleMuted { target }
        | OperatorAction::RestartMedia { target }
        | OperatorAction::SetFilterEnabled { target, .. }
        | OperatorAction::ToggleFilter { target, .. } => target,
        _ => return None,
    };
    let mut matches = targets
        .iter()
        .filter(|input| target.matches(&input.name, input.uuid.as_deref()));
    let input = matches.next()?;
    if matches.next().is_some()
        || targets
            .iter()
            .filter(|other| other.name == input.name)
            .count()
            != 1
    {
        return None;
    }
    if matches!(action, OperatorAction::OpenSource { .. }) {
        return Some(Resolved::OpenSource(input.name.clone()));
    }
    let observed = observed_input(state, input);
    let control = match action {
        OperatorAction::SetMuted { muted, .. } if supports("SetInputMute") => {
            observed_mute(state, input)?;
            ControlAction::SetInputMuted {
                input: input.name.clone(),
                muted: *muted,
            }
        }
        OperatorAction::ToggleMuted { .. } if supports("SetInputMute") => {
            ControlAction::SetInputMuted {
                input: input.name.clone(),
                muted: !observed_mute(state, input)?,
            }
        }
        OperatorAction::ToggleFilter { filter, .. } if supports("SetSourceFilterEnabled") => {
            ControlAction::SetFilterEnabled {
                input: input.name.clone(),
                filter: filter.clone(),
                enabled: !observed?
                    .filters
                    .as_ref()?
                    .as_ref()
                    .ok()?
                    .iter()
                    .find(|observed| observed.name == *filter)?
                    .enabled,
            }
        }
        OperatorAction::RestartMedia { .. } if supports("TriggerMediaInputAction") => {
            observed?.media.as_ref()?.as_ref().ok()?;
            ControlAction::Media {
                input: input.name.clone(),
                action: MediaAction::Restart,
            }
        }
        OperatorAction::SetFilterEnabled {
            filter, enabled, ..
        } if supports("SetSourceFilterEnabled") => {
            observed?
                .filters
                .as_ref()?
                .as_ref()
                .ok()?
                .iter()
                .find(|observed| observed.name == *filter)?;
            ControlAction::SetFilterEnabled {
                input: input.name.clone(),
                filter: filter.clone(),
                enabled: *enabled,
            }
        }
        _ => return None,
    };
    Some(Resolved::Control(control))
}

pub(super) fn entries(state: &AppState) -> Vec<Entry> {
    let targets = inputs(state);
    let mut entries: Vec<_> = [
        Page::Live,
        Page::Sources,
        Page::Mixer,
        Page::Inventory,
        Page::Graph,
        Page::Doctor,
        Page::Stats,
        Page::Settings,
        Page::Help,
    ]
    .into_iter()
    .map(|page| Entry {
        action: CatalogAction::Page(page),
        label: fl!(LANGUAGE_LOADER, "palette-open-page", page = page.title()),
        keywords: String::new(),
        available: true,
    })
    .collect();
    entries.push(Entry {
        action: CatalogAction::Bookmarks,
        label: fl!(LANGUAGE_LOADER, "bookmarks-title"),
        keywords: String::new(),
        available: true,
    });
    entries.push(Entry {
        action: CatalogAction::Reports,
        label: fl!(LANGUAGE_LOADER, "reports-title"),
        keywords: String::new(),
        available: true,
    });
    let mut add = |action: OperatorAction, label: String, keywords: String| {
        let available = resolve_with_inputs(state, &action, &targets).is_some();
        entries.push(Entry {
            action: CatalogAction::Operator(action),
            label,
            keywords,
            available,
        });
    };
    for scene in &state.scene_inventory.scenes {
        let metadata = state.registry.scenes.get(&scene.id);
        if !metadata
            .and_then(|m| m.role)
            .is_some_and(|role| role.is_live_switchable())
        {
            continue;
        }
        let studio = matches!(
            state.workspace.scene_action(&scene.id),
            Some(ControlAction::ActivateScene {
                studio_enabled: true,
                ..
            })
        );
        let label = if studio {
            fl!(
                LANGUAGE_LOADER,
                "palette-stage-scene",
                name = scene.id.as_str()
            )
        } else {
            fl!(
                LANGUAGE_LOADER,
                "palette-switch-scene",
                name = scene.id.as_str()
            )
        };
        add(
            OperatorAction::Scene {
                target: ActionTarget {
                    name: scene.id.clone(),
                    uuid: scene.uuid.clone(),
                },
            },
            label,
            metadata.map(|m| m.tags.join(" ")).unwrap_or_default(),
        );
    }
    add(
        OperatorAction::SaveReplay,
        fl!(LANGUAGE_LOADER, "control-replay-save"),
        String::new(),
    );
    for target in &targets {
        add(
            OperatorAction::OpenSource {
                target: target.clone(),
            },
            fl!(
                LANGUAGE_LOADER,
                "palette-open-source",
                name = target.name.as_str()
            ),
            String::new(),
        );
        let observed = observed_input(state, target);
        let muted = observed_mute(state, target);
        if let Some(muted) = muted {
            let label = if muted {
                fl!(
                    LANGUAGE_LOADER,
                    "palette-unmute",
                    name = target.name.as_str()
                )
            } else {
                fl!(
                    LANGUAGE_LOADER,
                    "audio-source-mute",
                    source = target.name.as_str()
                )
            };
            add(
                OperatorAction::SetMuted {
                    target: target.clone(),
                    muted: !muted,
                },
                label,
                String::new(),
            );
        }
        if let Some(observed) = observed {
            if observed.media.as_ref().is_some_and(Result::is_ok) {
                add(
                    OperatorAction::RestartMedia {
                        target: target.clone(),
                    },
                    fl!(
                        LANGUAGE_LOADER,
                        "palette-restart",
                        name = target.name.as_str()
                    ),
                    String::new(),
                );
            }
            for filter in observed
                .filters
                .as_ref()
                .and_then(|result| result.as_ref().ok())
                .into_iter()
                .flatten()
            {
                let label = if filter.enabled {
                    fl!(
                        LANGUAGE_LOADER,
                        "palette-disable-filter",
                        name = target.name.as_str(),
                        filter = filter.name.as_str()
                    )
                } else {
                    fl!(
                        LANGUAGE_LOADER,
                        "palette-enable-filter",
                        name = target.name.as_str(),
                        filter = filter.name.as_str()
                    )
                };
                add(
                    OperatorAction::SetFilterEnabled {
                        target: target.clone(),
                        filter: filter.name.clone(),
                        enabled: !filter.enabled,
                    },
                    label,
                    String::new(),
                );
            }
        }
    }
    entries
}

/// Configure explicit states even when the input/filter already has that state.
pub(super) fn binding_entries(state: &AppState) -> Vec<Entry> {
    let mut choices: Vec<_> = entries(state)
        .into_iter()
        .filter(|entry| matches!(entry.action, CatalogAction::Operator(_)))
        .collect();
    for entry in choices.clone() {
        let toggle = match &entry.action {
            CatalogAction::Operator(OperatorAction::SetMuted { target, .. }) => {
                Some(OperatorAction::ToggleMuted {
                    target: target.clone(),
                })
            }
            CatalogAction::Operator(OperatorAction::SetFilterEnabled {
                target, filter, ..
            }) => Some(OperatorAction::ToggleFilter {
                target: target.clone(),
                filter: filter.clone(),
            }),
            _ => None,
        };
        if let Some(toggle) = toggle {
            choices.push(binding_entry(state, &toggle));
        }
        let opposite = match entry.action {
            CatalogAction::Operator(OperatorAction::SetMuted { target, muted }) => {
                Some(OperatorAction::SetMuted {
                    target,
                    muted: !muted,
                })
            }
            CatalogAction::Operator(OperatorAction::SetFilterEnabled {
                target,
                filter,
                enabled,
            }) => Some(OperatorAction::SetFilterEnabled {
                target,
                filter,
                enabled: !enabled,
            }),
            _ => None,
        };
        if let Some(action) = opposite {
            choices.push(binding_entry(state, &action));
        }
    }
    choices
}

/// Resolve the current display name while retaining the saved identity/intent.
pub(super) fn binding_entry(state: &AppState, action: &OperatorAction) -> Entry {
    let name = |target: &ActionTarget, scene: bool| {
        let names: Vec<_> = if scene {
            state
                .scene_inventory
                .scenes
                .iter()
                .filter(|s| target.matches(&s.id, s.uuid.as_deref()))
                .map(|s| s.id.clone())
                .collect()
        } else {
            inputs(state)
                .into_iter()
                .filter(|input| target.matches(&input.name, input.uuid.as_deref()))
                .map(|input| input.name)
                .collect()
        };
        if names.len() == 1 {
            names[0].clone()
        } else {
            target.name.clone()
        }
    };
    let label = match action {
        OperatorAction::Scene { target } => {
            let name = name(target, true);
            if matches!(
                state.workspace.scene_action(&name),
                Some(ControlAction::ActivateScene {
                    studio_enabled: true,
                    ..
                })
            ) {
                fl!(LANGUAGE_LOADER, "palette-stage-scene", name = name)
            } else {
                fl!(LANGUAGE_LOADER, "palette-switch-scene", name = name)
            }
        }
        OperatorAction::OpenSource { target } => fl!(
            LANGUAGE_LOADER,
            "palette-open-source",
            name = name(target, false)
        ),
        OperatorAction::SetMuted {
            target,
            muted: false,
        } => fl!(
            LANGUAGE_LOADER,
            "palette-unmute",
            name = name(target, false)
        ),
        OperatorAction::SetMuted {
            target,
            muted: true,
        } => fl!(
            LANGUAGE_LOADER,
            "audio-source-mute",
            source = name(target, false)
        ),
        OperatorAction::ToggleMuted { target } => fl!(
            LANGUAGE_LOADER,
            "quick-toggle-mute",
            name = name(target, false)
        ),
        OperatorAction::ToggleFilter { target, filter } => fl!(
            LANGUAGE_LOADER,
            "quick-toggle-filter",
            name = name(target, false),
            filter = filter.as_str()
        ),
        OperatorAction::SaveReplay => fl!(LANGUAGE_LOADER, "control-replay-save"),
        OperatorAction::RestartMedia { target } => fl!(
            LANGUAGE_LOADER,
            "palette-restart",
            name = name(target, false)
        ),
        OperatorAction::SetFilterEnabled {
            target,
            filter,
            enabled: true,
        } => fl!(
            LANGUAGE_LOADER,
            "palette-enable-filter",
            name = name(target, false),
            filter = filter.as_str()
        ),
        OperatorAction::SetFilterEnabled {
            target,
            filter,
            enabled: false,
        } => fl!(
            LANGUAGE_LOADER,
            "palette-disable-filter",
            name = name(target, false),
            filter = filter.as_str()
        ),
    };
    Entry {
        action: CatalogAction::Operator(action.clone()),
        label,
        keywords: String::new(),
        available: resolve(state, action).is_some(),
    }
}

/// Return whether an action was admitted, never whether OBS completed it.
pub(super) fn execute(nav: &NavigationContext, action: &CatalogAction) -> bool {
    if matches!(action, CatalogAction::Reports) {
        let Some(parent) = nav.parent_window() else {
            return false;
        };
        super::session_reports::show(&parent, nav);
        return true;
    }
    if matches!(action, CatalogAction::Bookmarks) {
        let Some(parent) = nav.parent_window() else {
            return false;
        };
        super::bookmarks::show(&parent, nav);
        return true;
    }
    let CatalogAction::Operator(action) = action else {
        let CatalogAction::Page(page) = action else {
            unreachable!()
        };
        nav.switch_to_page(*page);
        return true;
    };
    if !nav.is_current() || !nav.registry_is_current() {
        return false;
    }
    let resolved = resolve(&nav.state.borrow(), action);
    match resolved {
        Some(Resolved::Control(control)) => {
            let uuid = match action {
                OperatorAction::Scene { target }
                | OperatorAction::SetMuted { target, .. }
                | OperatorAction::ToggleMuted { target }
                | OperatorAction::RestartMedia { target }
                | OperatorAction::SetFilterEnabled { target, .. }
                | OperatorAction::ToggleFilter { target, .. } => target.uuid.clone(),
                _ => None,
            };
            crate::ui::workspace::act_identified(nav, control, uuid);
        }
        Some(Resolved::OpenSource(input)) => {
            nav.switch_to_page(Page::Sources);
            crate::ui::workspace::inspect_input(nav, Some(input));
        }
        None => return false,
    }
    true
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::domain::{
        control::{ControlSnapshot, SourceDetail, SourceFilter, SourceItem, StudioState},
        role::SceneRole,
        scene::Scene,
    };
    use crate::storage::registry::SceneEntry;
    use std::time::{Duration, Instant};

    pub(crate) fn connected() -> AppState {
        let mut state = AppState::new(Default::default(), Default::default(), None, None);
        state.obs_status = ObsStatus::Connected {
            obs_version: "fixture".into(),
        };
        state.scene_collections.current = Some("Show".into());
        state.scene_inventory.scenes.push(Scene {
            id: "Camera <b>One</b>".into(),
            uuid: Some("scene-1".into()),
        });
        state.registry.scenes.insert(
            "Camera <b>One</b>".into(),
            SceneEntry {
                role: Some(SceneRole::Primary),
                tags: vec!["Interview".into()],
                ..Default::default()
            },
        );
        state.show_registry.status = RegistryStatus::Ready;
        state.workspace.snapshot = Some(ControlSnapshot {
            studio: StudioState {
                enabled: Some(true),
                ..Default::default()
            },
            replay_active: Some(true),
            items: vec![SourceItem {
                owner: "Camera <b>One</b>".into(),
                item_id: 1,
                input: "Microphone".into(),
                input_uuid: "input-1".into(),
                kind: None,
                path: vec![],
                enabled: true,
                ancestor_enabled: true,
                is_group: false,
                is_scene: false,
            }],
            ..Default::default()
        });
        state.workspace.observed_at = Some(Instant::now());
        state.workspace.detail = Some(SourceDetail {
            input: "Microphone".into(),
            muted: Some(false),
            filters: vec![SourceFilter {
                name: "Limiter".into(),
                kind: "limiter_filter".into(),
                enabled: false,
            }],
            ..Default::default()
        });
        state.workspace.input_catalog =
            Some(Ok(vec![crate::domain::operator_input::InputIdentity {
                target: ActionTarget {
                    name: "Microphone".into(),
                    uuid: Some("input-1".into()),
                },
                kind: "audio".into(),
            }]));
        state.workspace.audio =
            vec![
                crate::domain::audio::AudioInput::new("Microphone".into(), false, 1.0, 0.0)
                    .with_identity(Some("input-1".into())),
            ];
        state.workspace.operator_inputs = vec![crate::domain::operator_input::InputObservation {
            target: ActionTarget {
                name: "Microphone".into(),
                uuid: Some("input-1".into()),
            },
            media: None,
            filters: Some(Ok(state.workspace.detail.as_ref().unwrap().filters.clone())),
        }];
        state.available_requests = [
            "SetCurrentPreviewScene",
            "SetCurrentProgramScene",
            "SetInputMute",
            "SaveReplayBuffer",
            "SetSourceFilterEnabled",
        ]
        .map(str::to_owned)
        .into();
        state
    }

    fn scene() -> OperatorAction {
        OperatorAction::Scene {
            target: ActionTarget {
                name: "Camera <b>One</b>".into(),
                uuid: Some("scene-1".into()),
            },
        }
    }

    #[test]
    fn search_includes_tags_and_hidden_scenes_and_navigation_works_offline() {
        let mut state = connected();
        state.registry.scene_filter.query = "no match".into();
        let filtered: Vec<_> = entries(&state)
            .into_iter()
            .filter(|e| e.matches("INTERVIEW camera"))
            .collect();
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].available);
        assert!(filtered[0].label.contains("<b>One</b>"));
        assert!(!filtered[0].matches("interview missing"));
        state.obs_status = ObsStatus::Disconnected;
        for entry in entries(&state) {
            assert_eq!(
                entry.available,
                matches!(
                    entry.action,
                    CatalogAction::Page(_) | CatalogAction::Bookmarks | CatalogAction::Reports
                )
            );
        }
    }

    #[test]
    fn scene_resolution_retains_identity_and_requires_role_freshness_and_capability() {
        let mut state = connected();
        assert_eq!(
            resolve(&state, &scene()),
            Some(Resolved::Control(ControlAction::ActivateScene {
                scene: "Camera <b>One</b>".into(),
                studio_enabled: true
            }))
        );
        state.scene_inventory.scenes[0].id = "Renamed".into();
        let entry = state.registry.scenes.remove("Camera <b>One</b>").unwrap();
        state.registry.scenes.insert("Renamed".into(), entry);
        assert_eq!(
            resolve(&state, &scene()),
            Some(Resolved::Control(ControlAction::ActivateScene {
                scene: "Renamed".into(),
                studio_enabled: true
            }))
        );
        state.scene_inventory.scenes[0].uuid = Some("replacement".into());
        assert_eq!(resolve(&state, &scene()), None);
        state = connected();
        state
            .scene_inventory
            .scenes
            .push(state.scene_inventory.scenes[0].clone());
        assert_eq!(resolve(&state, &scene()), None);
        state = connected();
        state.workspace.observed_at = Some(Instant::now() - Duration::from_secs(31));
        assert_eq!(resolve(&state, &scene()), None);
        state = connected();
        state.available_requests.clear();
        assert_eq!(resolve(&state, &scene()), None);
        state = connected();
        state
            .registry
            .scenes
            .get_mut("Camera <b>One</b>")
            .unwrap()
            .role = Some(SceneRole::Module);
        assert_eq!(resolve(&state, &scene()), None);
        state = connected();
        state.show_registry.status = RegistryStatus::Loading;
        assert_eq!(resolve(&state, &scene()), None);
    }

    #[test]
    fn source_intents_never_invert_or_target_a_replacement() {
        let mut state = connected();
        let target = ActionTarget {
            name: "Microphone".into(),
            uuid: Some("input-1".into()),
        };
        let mute = OperatorAction::SetMuted {
            target: target.clone(),
            muted: true,
        };
        assert!(resolve(&state, &mute).is_some());
        state.workspace.audio[0].muted = true;
        assert_eq!(
            resolve(&state, &mute),
            Some(Resolved::Control(ControlAction::SetInputMuted {
                input: "Microphone".into(),
                muted: true
            }))
        );
        let filter = OperatorAction::SetFilterEnabled {
            target: target.clone(),
            filter: "Limiter".into(),
            enabled: true,
        };
        assert!(resolve(&state, &filter).is_some());
        state.workspace.operator_inputs[0]
            .filters
            .as_mut()
            .unwrap()
            .as_mut()
            .unwrap()[0]
            .enabled = true;
        assert_eq!(
            resolve(&state, &filter),
            Some(Resolved::Control(ControlAction::SetFilterEnabled {
                input: "Microphone".into(),
                filter: "Limiter".into(),
                enabled: true
            }))
        );
        let open = OperatorAction::OpenSource { target };
        assert!(resolve(&state, &open).is_some());
        let duplicate = state.workspace.snapshot.as_ref().unwrap().items[0].clone();
        state
            .workspace
            .snapshot
            .as_mut()
            .unwrap()
            .items
            .push(duplicate);
        assert!(
            resolve(&state, &open).is_some(),
            "repeated uses of one input are one target"
        );
        let mut conflicting = state
            .workspace
            .input_catalog
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()[0]
            .clone();
        conflicting.target.uuid = Some("different".into());
        state
            .workspace
            .input_catalog
            .as_mut()
            .unwrap()
            .as_mut()
            .unwrap()
            .push(conflicting);
        assert!(
            resolve(&state, &open).is_none(),
            "ambiguous names cannot be dispatched by name"
        );
        state = connected();
        state
            .workspace
            .input_catalog
            .as_mut()
            .unwrap()
            .as_mut()
            .unwrap()[0]
            .target
            .uuid = Some("replacement".into());
        assert!(resolve(&state, &open).is_none());
    }

    #[test]
    fn audio_actions_keep_identity_across_pages_and_renames() {
        use crate::domain::audio::AudioInput;
        let mut state = connected();
        state.current_page = Page::Settings;
        state.workspace.snapshot.as_mut().unwrap().items.clear();
        state.workspace.detail = None;
        state.workspace.audio = vec![AudioInput::new("Microphone".into(), false, 1.0, 0.0)
            .with_identity(Some("input-1".into()))];
        let action = OperatorAction::SetMuted {
            target: ActionTarget {
                name: "Microphone".into(),
                uuid: Some("input-1".into()),
            },
            muted: true,
        };
        assert!(entries(&state).iter().any(|entry| entry.action
            == CatalogAction::Operator(action.clone())
            && entry.available));
        state.workspace.audio[0].id = "Renamed".into();
        state
            .workspace
            .input_catalog
            .as_mut()
            .unwrap()
            .as_mut()
            .unwrap()[0]
            .target
            .name = "Renamed".into();
        assert_eq!(
            resolve(&state, &action),
            Some(Resolved::Control(ControlAction::SetInputMuted {
                input: "Renamed".into(),
                muted: true
            }))
        );
        state.workspace.audio[0].uuid = Some("replacement".into());
        assert_eq!(resolve(&state, &action), None);
        state.workspace.audio[0].uuid = Some("input-1".into());
        state.workspace.observed_at = None;
        assert_eq!(resolve(&state, &action), None);
    }

    #[test]
    fn replay_and_source_controls_require_current_observations() {
        let mut state = connected();
        assert!(resolve(&state, &OperatorAction::SaveReplay).is_some());
        for mutate in [
            (|s: &mut AppState| s.workspace.pending = true) as fn(&mut AppState),
            |s| s.workspace.dirty = true,
            |s| s.workspace.error = Some("failed".into()),
            |s| s.workspace.snapshot.as_mut().unwrap().replay_active = Some(false),
            |s| s.available_requests.clear(),
            |s| s.scene_collections.current = None,
        ] {
            state = connected();
            mutate(&mut state);
            assert!(resolve(&state, &OperatorAction::SaveReplay).is_none());
        }
        let target = ActionTarget {
            name: "Old".into(),
            uuid: Some("id".into()),
        };
        assert!(target.matches("Renamed", Some("id")));
        assert!(!target.matches("Old", None));
        assert!(!ActionTarget {
            name: "Old".into(),
            uuid: None
        }
        .matches("Old", Some("new")));
    }

    #[test]
    fn media_and_filters_use_targeted_reads_without_sources_selection() {
        use crate::domain::control::{MediaState, MediaStatus};
        let mut state = connected();
        state.current_page = Page::Mixer;
        state.workspace.selected_input = Some("Unrelated editor".into());
        state.workspace.detail = None;
        state.workspace.snapshot.as_mut().unwrap().items.clear();
        state
            .available_requests
            .push("TriggerMediaInputAction".into());
        let target = state.workspace.operator_inputs[0].target.clone();
        let media = OperatorAction::RestartMedia {
            target: target.clone(),
        };
        let filter = OperatorAction::SetFilterEnabled {
            target,
            filter: "Limiter".into(),
            enabled: true,
        };
        state.workspace.operator_inputs[0].media = Some(Ok(MediaStatus {
            state: MediaState::Playing,
            duration_ms: Some(5000),
            cursor_ms: Some(0),
        }));
        for action in [&media, &filter] {
            assert!(resolve(&state, action).is_some());
            assert!(entries(&state).iter().any(|entry| entry.action
                == CatalogAction::Operator(action.clone())
                && entry.available));
        }
        state.workspace.operator_inputs[0].filters = Some(Err("unavailable".into()));
        assert!(resolve(&state, &filter).is_none());
        assert!(resolve(&state, &media).is_some());
        state.workspace.input_catalog = Some(Err("inventory unavailable".into()));
        assert!(
            resolve(&state, &media).is_none(),
            "old targeted data cannot authorize a write without inventory"
        );
        state.workspace.invalidate();
        assert!(state.workspace.operator_inputs.is_empty());
        assert!(state.workspace.input_catalog.is_none());
    }

    #[test]
    fn pin_state_reports_observations_instead_of_requested_values() {
        let mut state = connected();
        let target = state.workspace.operator_inputs[0].target.clone();
        let mute = OperatorAction::SetMuted {
            target: target.clone(),
            muted: true,
        };
        let filter = OperatorAction::SetFilterEnabled {
            target,
            filter: "Limiter".into(),
            enabled: true,
        };
        assert_eq!(
            observed_label(&state, &mute),
            Some(fl!(LANGUAGE_LOADER, "quick-unmuted"))
        );
        assert_eq!(
            observed_label(&state, &filter),
            Some(fl!(LANGUAGE_LOADER, "quick-filter-disabled"))
        );
        state.workspace.audio[0].muted = true;
        assert_eq!(
            observed_label(&state, &mute),
            Some(fl!(LANGUAGE_LOADER, "quick-muted"))
        );
        state.workspace.snapshot.as_mut().unwrap().replay_active = Some(false);
        assert_eq!(
            observed_label(&state, &OperatorAction::SaveReplay),
            Some(fl!(LANGUAGE_LOADER, "output-state-inactive"))
        );
        assert!(!binding_entry(&state, &OperatorAction::SaveReplay).available);
        state.workspace.snapshot.as_mut().unwrap().studio.program =
            Some("Camera <b>One</b>".into());
        assert_eq!(
            observed_label(&state, &scene()),
            Some(fl!(LANGUAGE_LOADER, "scene-card-program-label"))
        );
        for invalidate in [
            (|s: &mut AppState| s.workspace.dirty = true) as fn(&mut AppState),
            |s| s.workspace.pending = true,
            |s| s.workspace.observed_at = None,
            |s| s.scene_collections.current = None,
            |s| s.show_registry.status = RegistryStatus::Loading,
            |s| s.workspace.input_catalog = Some(Err("unavailable".into())),
            |s| {
                s.workspace
                    .input_catalog
                    .as_mut()
                    .unwrap()
                    .as_mut()
                    .unwrap()[0]
                    .target
                    .uuid = Some("replacement".into())
            },
        ] {
            let mut state = connected();
            invalidate(&mut state);
            assert!(observed_label(&state, &mute).is_none());
        }
    }

    #[test]
    fn toggles_resolve_to_explicit_values_only_from_current_observations() {
        let mut state = connected();
        let target = state.workspace.operator_inputs[0].target.clone();
        let mute = OperatorAction::ToggleMuted {
            target: target.clone(),
        };
        let filter = OperatorAction::ToggleFilter {
            target,
            filter: "Limiter".into(),
        };
        for value in [false, true] {
            state.workspace.audio[0].muted = value;
            state.workspace.operator_inputs[0]
                .filters
                .as_mut()
                .unwrap()
                .as_mut()
                .unwrap()[0]
                .enabled = value;
            assert_eq!(
                resolve(&state, &mute),
                Some(Resolved::Control(ControlAction::SetInputMuted {
                    input: "Microphone".into(),
                    muted: !value
                }))
            );
            assert_eq!(
                resolve(&state, &filter),
                Some(Resolved::Control(ControlAction::SetFilterEnabled {
                    input: "Microphone".into(),
                    filter: "Limiter".into(),
                    enabled: !value
                }))
            );
            assert!(binding_entries(&state)
                .iter()
                .any(|entry| entry.action == CatalogAction::Operator(mute.clone())));
            assert!(binding_entries(&state)
                .iter()
                .any(|entry| entry.action == CatalogAction::Operator(filter.clone())));
        }
        state.workspace.operator_inputs[0].filters = Some(Err("unavailable".into()));
        assert!(resolve(&state, &filter).is_none());
        state.workspace.pending = true;
        assert!(resolve(&state, &mute).is_none());
    }
}
