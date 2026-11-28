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
pub(super) mod tests;
