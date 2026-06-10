//! Scene dependency graph validation.
//!
//! Pure functions only — no OBS or GTK types.  The graph itself is built in
//! `obs::client::get_scene_graph`; this module classifies its edges against
//! the role rules in `RuleConfig`.

use crate::domain::graph::EdgeStatus;
use crate::domain::role::SceneRole;
use crate::storage::registry::RuleConfig;

/// Classify a single dependency edge `from → to` against the rules.
///
/// Resolution order:
/// 1. An unassigned endpoint can't be checked → `Ok`.
/// 2. An explicit `forbidden_edges` pair → `Forbidden`.
/// 3. A `*_can_depend_on` allow-list that exists but doesn't list the target
///    role → `Warning`.
/// 4. Otherwise → `Ok`.
pub fn classify_edge(
    from: Option<SceneRole>,
    to: Option<SceneRole>,
    rules: &RuleConfig,
) -> EdgeStatus {
    let (from, to) = match (from, to) {
        (Some(f), Some(t)) => (f, t),
        // Can't judge an edge touching an unassigned scene.
        _ => return EdgeStatus::Ok,
    };

    let from_key = from.rule_key();
    let to_key = to.rule_key();

    // 2. Explicit forbidden pairs take priority.
    let forbidden = rules
        .forbidden_edges
        .iter()
        .any(|pair| pair[0] == from_key && pair[1] == to_key);
    if forbidden {
        return EdgeStatus::Forbidden;
    }

    // 3. Allow-lists: if one is defined for this source role, the target must
    //    be in it.  An empty list means "no constraint configured".
    let allow_list = match from {
        SceneRole::Primary => Some(&rules.primary_can_depend_on),
        SceneRole::Module => Some(&rules.module_can_depend_on),
        _ => None,
    };
    if let Some(list) = allow_list {
        if !list.is_empty() && !list.iter().any(|r| r == to_key) {
            return EdgeStatus::Warning;
        }
    }

    EdgeStatus::Ok
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> RuleConfig {
        RuleConfig {
            primary_can_depend_on: vec!["module".into(), "secondary".into()],
            module_can_depend_on: vec!["module".into(), "raw".into()],
            forbidden_edges: vec![
                ["primary".into(), "debug".into()],
                ["module".into(), "primary".into()],
            ],
        }
    }

    #[test]
    fn forbidden_pair_is_forbidden() {
        let s = classify_edge(Some(SceneRole::Module), Some(SceneRole::Primary), &rules());
        assert_eq!(s, EdgeStatus::Forbidden);
    }

    #[test]
    fn allowed_dependency_is_ok() {
        let s = classify_edge(Some(SceneRole::Primary), Some(SceneRole::Module), &rules());
        assert_eq!(s, EdgeStatus::Ok);
    }

    #[test]
    fn out_of_allow_list_is_warning() {
        // primary may depend on module/secondary, but not raw
        let s = classify_edge(Some(SceneRole::Primary), Some(SceneRole::Raw), &rules());
        assert_eq!(s, EdgeStatus::Warning);
    }

    #[test]
    fn unassigned_endpoint_is_ok() {
        let s = classify_edge(None, Some(SceneRole::Raw), &rules());
        assert_eq!(s, EdgeStatus::Ok);
    }
}
