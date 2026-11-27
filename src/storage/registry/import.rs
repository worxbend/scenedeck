//! Limits on a YAML registry import, a file the user picked from anywhere.
//!
//! The parser budget bounds what one crafted file costs to read. The caps
//! bound what it can put in front of the operator. Both sit far above a real
//! production. `scene_uuids` and retained UUIDs are cleared on import, so
//! only the budget bounds them.

use std::iter;

use thiserror::Error;

use super::{RuleConfig, SceneEntry, SceneRegistry};
use crate::domain::operator_action::{ActionTarget, OperatorAction};

const SCENE_LIMIT: usize = 2_000;
const DECK_LIMIT: usize = 200;
const ACTION_LIMIT: usize = 200;
const AUDIO_INPUT_LIMIT: usize = 500;
const TAG_LIMIT: usize = 64;
const RULE_LIMIT: usize = 64;
const TEXT_LIMIT: usize = 512;

/// Why a YAML file was refused before it reached the registry.
#[derive(Debug, Error)]
pub(super) enum ImportError {
    /// Malformed YAML, a type mismatch, or a breached parser budget.
    #[error(transparent)]
    Yaml(#[from] serde_saphyr::Error),
    /// A collection holds more entries than an import accepts.
    #[error("{collection} has {count} entries; an import accepts at most {limit}")]
    TooMany {
        collection: &'static str,
        count: usize,
        limit: usize,
    },
    /// One value is longer than an import accepts.
    #[error("a value in {field} is {length} bytes; an import accepts at most {TEXT_LIMIT}")]
    TooLong { field: &'static str, length: usize },
}

/// Parse `raw` under the import budget, then apply the import caps.
pub(super) fn parse_import(raw: &str) -> Result<SceneRegistry, ImportError> {
    let registry = serde_saphyr::from_str_with_options(raw, import_options())?;
    validate_import(&registry)?;
    Ok(registry)
}

fn import_options() -> serde_saphyr::Options {
    serde_saphyr::options! {
        budget: serde_saphyr::budget! {
            max_aliases: 1_000,
            max_anchors: 1_000,
            max_recorded_anchor_bytes: 1 << 20,
            max_nodes: 100_000,
            max_merge_keys: 100,
        },
    }
}

fn validate_import(registry: &SceneRegistry) -> Result<(), ImportError> {
    check_counts(registry)?;
    check_scene_texts(registry)?;
    check_view_texts(registry)?;
    check_action_texts(registry)?;
    check_audio_texts(registry)?;
    check_rules(&registry.rules)
}

fn check_counts(registry: &SceneRegistry) -> Result<(), ImportError> {
    check_count("scenes", registry.scenes.len(), SCENE_LIMIT)?;
    check_count("scene_order", registry.scene_order.len(), SCENE_LIMIT)?;
    check_count(
        "retained_scenes",
        registry.retained_scenes.len(),
        SCENE_LIMIT,
    )?;
    check_count("decks", registry.decks.len(), DECK_LIMIT)?;
    check_count(
        "action_bindings",
        registry.action_bindings.len(),
        ACTION_LIMIT,
    )?;
    check_count("quick_actions", registry.quick_actions.len(), ACTION_LIMIT)?;
    check_count(
        "audio_alerts",
        registry.audio_alerts.len(),
        AUDIO_INPUT_LIMIT,
    )?;
    check_count("inputs", registry.inputs.len(), AUDIO_INPUT_LIMIT)
}

fn check_scene_texts(registry: &SceneRegistry) -> Result<(), ImportError> {
    check_texts("scenes", registry.scenes.keys().map(String::as_str))?;
    check_texts(
        "scene_order",
        registry.scene_order.iter().map(String::as_str),
    )?;
    let retained = registry.retained_scenes.iter();
    check_texts(
        "retained_scenes",
        retained.clone().map(|scene| scene.name.as_str()),
    )?;
    let retained_entries = retained.filter_map(|scene| scene.entry.as_ref());
    registry
        .scenes
        .values()
        .chain(retained_entries)
        .try_for_each(check_entry)
}

fn check_entry(entry: &SceneEntry) -> Result<(), ImportError> {
    check_count("tags", entry.tags.len(), TAG_LIMIT)?;
    check_texts("tags", entry.tags.iter().map(String::as_str))?;
    check_texts("accent_color", entry.accent_color.as_deref())?;
    check_texts("icon", entry.icon.as_deref())
}

fn check_view_texts(registry: &SceneRegistry) -> Result<(), ImportError> {
    check_texts("decks", registry.decks.keys().map(String::as_str))?;
    let filters = iter::once(&registry.scene_filter).chain(registry.decks.values());
    check_texts("query", filters.map(|filter| filter.query.as_str()))
}

fn check_action_texts(registry: &SceneRegistry) -> Result<(), ImportError> {
    let bindings = registry.action_bindings.iter();
    let keys = bindings
        .clone()
        .map(|binding| binding.shortcut.key.as_str());
    check_texts("action_bindings", keys)?;
    let pinned = registry.quick_actions.iter().map(|pin| &pin.action);
    let actions = bindings.map(|binding| &binding.action).chain(pinned);
    check_texts("operator actions", actions.flat_map(action_texts))
}

fn check_audio_texts(registry: &SceneRegistry) -> Result<(), ImportError> {
    let alerts = registry.audio_alerts.iter().map(|rule| &rule.target);
    let presets = registry.audio_presets.values();
    let preset_targets = presets.flat_map(|preset| preset.inputs.iter().map(|input| &input.target));
    check_texts(
        "audio targets",
        alerts.chain(preset_targets).flat_map(target_texts),
    )?;
    let inputs = registry.inputs.iter();
    check_texts(
        "inputs",
        inputs.flat_map(|(name, entry)| iter::once(name.as_str()).chain(entry.icon.as_deref())),
    )
}

fn check_rules(rules: &RuleConfig) -> Result<(), ImportError> {
    let primary = &rules.primary_can_depend_on;
    let module = &rules.module_can_depend_on;
    check_count("primary_can_depend_on", primary.len(), RULE_LIMIT)?;
    check_count("module_can_depend_on", module.len(), RULE_LIMIT)?;
    check_count("forbidden_edges", rules.forbidden_edges.len(), RULE_LIMIT)?;
    let edges = rules.forbidden_edges.iter().flatten();
    check_texts(
        "rules",
        primary
            .iter()
            .chain(module)
            .chain(edges)
            .map(String::as_str),
    )
}

/// Every string an action carries. The match is exhaustive, so a new
/// variant cannot slip past the length cap.
fn action_texts(action: &OperatorAction) -> impl Iterator<Item = &str> {
    let (target, filter) = match action {
        OperatorAction::SaveReplay => (None, None),
        OperatorAction::Scene { target }
        | OperatorAction::OpenSource { target }
        | OperatorAction::SetMuted { target, .. }
        | OperatorAction::ToggleMuted { target }
        | OperatorAction::RestartMedia { target } => (Some(target), None),
        OperatorAction::SetFilterEnabled { target, filter, .. }
        | OperatorAction::ToggleFilter { target, filter } => (Some(target), Some(filter.as_str())),
    };
    target.into_iter().flat_map(target_texts).chain(filter)
}

fn target_texts(target: &ActionTarget) -> impl Iterator<Item = &str> {
    iter::once(target.name.as_str()).chain(target.uuid.as_deref())
}

fn check_count(collection: &'static str, count: usize, limit: usize) -> Result<(), ImportError> {
    if count > limit {
        return Err(ImportError::TooMany {
            collection,
            count,
            limit,
        });
    }
    Ok(())
}

fn check_texts<'a>(
    field: &'static str,
    texts: impl IntoIterator<Item = &'a str>,
) -> Result<(), ImportError> {
    match texts.into_iter().find(|text| text.len() > TEXT_LIMIT) {
        Some(text) => Err(ImportError::TooLong {
            field,
            length: text.len(),
        }),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use serde_saphyr::budget::BudgetBreach;

    use super::*;
    use crate::storage::registry::read_registry_yaml_from_path;
    use crate::storage::test_support::TempDir;

    fn import(yaml: &str) -> io::Result<SceneRegistry> {
        let dir = TempDir::new("registry-import");
        let path = dir.path().join("import.yaml");
        std::fs::write(&path, yaml).unwrap();
        read_registry_yaml_from_path(&path)
    }

    /// The typed refusal behind the `InvalidData` error the import UI shows.
    fn refusal(error: &io::Error) -> &ImportError {
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        error
            .get_ref()
            .and_then(|source| source.downcast_ref::<ImportError>())
            .unwrap()
    }

    /// The breach behind snippet and alias-location wrappers, if any.
    fn budget_breach(refusal: &ImportError) -> Option<&BudgetBreach> {
        let ImportError::Yaml(outer) = refusal else {
            return None;
        };
        let mut error = outer;
        loop {
            match error {
                serde_saphyr::Error::WithSnippet { error: inner, .. }
                | serde_saphyr::Error::AliasError { error: inner, .. } => error = inner.as_ref(),
                serde_saphyr::Error::Budget { breach, .. } => return Some(breach),
                _ => return None,
            }
        }
    }

    /// 200 anchors and 1,100 aliases: under the parser's default alias and
    /// alias/anchor-ratio limits, over the import's alias budget.
    fn alias_heavy_yaml() -> String {
        let anchors = (0..200).map(|n| format!("&a{n} s{n}"));
        let aliases = (0..1_100).map(|n| format!("*a{}", n % 200));
        let order: Vec<String> = anchors.chain(aliases).collect();
        format!("scene_order: [{}]\n", order.join(", "))
    }

    #[test]
    fn aliases_over_the_import_budget_are_refused_as_invalid_data() {
        let yaml = alias_heavy_yaml();
        assert!(serde_saphyr::from_str::<SceneRegistry>(&yaml).is_ok());

        let error = import(&yaml).unwrap_err();

        assert!(matches!(
            budget_breach(refusal(&error)),
            Some(BudgetBreach::Aliases { .. })
        ));
    }

    #[test]
    fn nested_alias_bomb_is_refused_by_the_budget() {
        let mut yaml = String::from("a0: &a0 [x, x, x, x, x, x, x, x, x, x]\n");
        for level in 1..10 {
            let aliases = vec![format!("*a{}", level - 1); 10].join(", ");
            yaml.push_str(&format!("a{level}: &a{level} [{aliases}]\n"));
        }
        // An unknown key is skipped by walking the whole value, expanding it.
        yaml.push_str("bomb: *a9\n");

        let error = import(&yaml).unwrap_err();

        assert!(budget_breach(refusal(&error)).is_some());
    }

    #[test]
    fn deck_count_at_the_cap_imports_and_one_more_is_refused() {
        let decks = |count: usize| {
            let lines: String = (0..count).map(|n| format!("  deck-{n}: {{}}\n")).collect();
            format!("decks:\n{lines}")
        };

        assert_eq!(import(&decks(DECK_LIMIT)).unwrap().decks.len(), DECK_LIMIT);
        let error = import(&decks(DECK_LIMIT + 1)).unwrap_err();

        assert!(matches!(
            refusal(&error),
            ImportError::TooMany { collection: "decks", count, limit: DECK_LIMIT }
                if *count == DECK_LIMIT + 1
        ));
    }

    #[test]
    fn scene_count_over_the_cap_is_refused() {
        let scenes: String = (0..=SCENE_LIMIT)
            .map(|n| format!("  scene-{n}: {{protected: true}}\n"))
            .collect();

        let error = import(&format!("scenes:\n{scenes}")).unwrap_err();

        assert!(matches!(
            refusal(&error),
            ImportError::TooMany {
                collection: "scenes",
                ..
            }
        ));
    }

    #[test]
    fn text_at_the_length_cap_imports_and_one_byte_more_is_refused() {
        let tagged = |tag: &str| format!("scenes:\n  Main:\n    tags: [{tag}]\n");

        let imported = import(&tagged(&"t".repeat(TEXT_LIMIT))).unwrap();
        assert_eq!(imported.scenes["Main"].tags[0].len(), TEXT_LIMIT);
        let error = import(&tagged(&"t".repeat(TEXT_LIMIT + 1))).unwrap_err();

        assert!(matches!(
            refusal(&error),
            ImportError::TooLong { field: "tags", length } if *length == TEXT_LIMIT + 1
        ));
    }

    #[test]
    fn long_text_inside_an_operator_action_is_refused() {
        let filter = "f".repeat(TEXT_LIMIT + 1);
        let yaml = format!(
            "quick_actions:\n  - action:\n      action: toggle-filter\n      \
             target: {{name: Camera, uuid: null}}\n      filter: {filter}\n"
        );

        let error = import(&yaml).unwrap_err();

        assert!(matches!(
            refusal(&error),
            ImportError::TooLong {
                field: "operator actions",
                ..
            }
        ));
    }
}
