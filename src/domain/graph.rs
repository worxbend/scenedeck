//! Directed scene dependency graph types.

use std::collections::{HashMap, HashSet};

use i18n_embed_fl::fl;

use crate::domain::scene::SceneId;
use crate::infra::i18n::LANGUAGE_LOADER;

/// Result of validating a single dependency edge against the role rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeStatus {
    /// Edge satisfies the rules.
    Ok,
    /// Edge is allowed but discouraged (e.g. `primary → raw`).
    Warning,
    /// Edge is explicitly forbidden (e.g. `module → primary`).
    Forbidden,
}

impl EdgeStatus {
    /// Display order for graph summary chips.
    pub const SUMMARY_ORDER: [Self; 3] = [Self::Ok, Self::Warning, Self::Forbidden];

    /// Short label used in the graph edge summary.
    pub fn summary_label(self) -> String {
        match self {
            Self::Ok => fl!(LANGUAGE_LOADER, "edge-status-ok-label"),
            Self::Warning => fl!(LANGUAGE_LOADER, "edge-status-warning-label"),
            Self::Forbidden => fl!(LANGUAGE_LOADER, "edge-status-forbidden-label"),
        }
    }

    /// Tooltip used to explain this edge status in the graph summary.
    pub fn summary_tooltip(self) -> String {
        match self {
            Self::Ok => fl!(LANGUAGE_LOADER, "edge-status-ok-tooltip"),
            Self::Warning => fl!(LANGUAGE_LOADER, "edge-status-warning-tooltip"),
            Self::Forbidden => fl!(LANGUAGE_LOADER, "edge-status-forbidden-tooltip"),
        }
    }

    /// Symbolic icon name for displaying this status in the UI.
    pub const fn icon_name(self) -> &'static str {
        match self {
            Self::Ok => "object-select-symbolic",
            Self::Warning => "dialog-warning-symbolic",
            Self::Forbidden => "dialog-error-symbolic",
        }
    }

    /// CSS class used to colour the status icon.
    pub const fn css_class(self) -> &'static str {
        match self {
            Self::Ok => "diag-ok",
            Self::Warning => "diag-warning",
            Self::Forbidden => "diag-error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_status_summary_order_matches_graph_legend() {
        assert_eq!(
            EdgeStatus::SUMMARY_ORDER,
            [EdgeStatus::Ok, EdgeStatus::Warning, EdgeStatus::Forbidden]
        );
    }

    #[test]
    fn edge_status_summary_text_explains_policy_result() {
        assert_eq!(EdgeStatus::Ok.summary_label(), "OK");
        assert_eq!(
            EdgeStatus::Ok.summary_tooltip(),
            "Edges that match the graph policy"
        );
        assert_eq!(EdgeStatus::Warning.summary_label(), "Warning");
        assert_eq!(
            EdgeStatus::Warning.summary_tooltip(),
            "Edges outside an allow-list"
        );
        assert_eq!(EdgeStatus::Forbidden.summary_label(), "Forbidden");
        assert_eq!(
            EdgeStatus::Forbidden.summary_tooltip(),
            "Edges forbidden by graph policy"
        );
    }
    #[test]
    fn iterative_components_handle_long_chains_and_disconnected_cycles() {
        let mut graph = SceneGraph::default();
        for index in 0..20_000 {
            graph
                .edges
                .insert(index.to_string(), vec![(index + 1).to_string()]);
        }
        assert!(graph.cycle_edges().is_empty());
        graph.edges.insert("20000".into(), vec!["19999".into()]);
        graph.edges.insert("Self".into(), vec!["Self".into()]);
        graph.edges.insert("A".into(), vec!["B".into(), "B".into()]);
        graph.edges.insert("B".into(), vec!["A".into()]);
        assert_eq!(
            graph.cycle_edges(),
            vec![
                ("19999", "20000"),
                ("20000", "19999"),
                ("A", "B"),
                ("B", "A"),
                ("Self", "Self")
            ]
        );
    }

    #[test]
    fn components_match_reachability_reference_for_every_three_node_graph() {
        for mask in 0..(1 << 9) {
            let mut graph = SceneGraph::default();
            for parent in 0..3 {
                for child in 0..3 {
                    if mask & (1 << (parent * 3 + child)) != 0 {
                        graph
                            .edges
                            .entry(parent.to_string())
                            .or_default()
                            .push(child.to_string());
                    }
                }
            }
            let expected: HashSet<&str> = graph
                .edges
                .iter()
                .filter_map(|(parent, children)| {
                    children
                        .iter()
                        .any(|child| {
                            let mut visited = HashSet::new();
                            let mut pending = vec![child.as_str()];
                            while let Some(node) = pending.pop() {
                                if node == parent {
                                    return true;
                                }
                                if visited.insert(node) {
                                    pending.extend(graph.children(node).iter().map(String::as_str));
                                }
                            }
                            false
                        })
                        .then_some(parent.as_str())
                })
                .collect();
            let actual: HashSet<_> = graph
                .cycle_edges()
                .into_iter()
                .map(|(parent, _)| parent)
                .collect();
            assert_eq!(actual, expected, "adjacency mask {mask}");
        }
    }
}

/// Directed dependency graph of OBS scenes.
///
/// An edge `A → [B, C]` means scene A contains B and C as nested scene
/// sources.  Used by the Graph page and the Doctor rule engine.
#[derive(Debug, Default, Clone)]
pub struct SceneGraph {
    /// Adjacency list keyed by parent scene id.
    pub edges: HashMap<SceneId, Vec<SceneId>>,
}

impl SceneGraph {
    /// Direct children of `scene`.
    pub fn children(&self, scene: &str) -> &[SceneId] {
        self.edges.get(scene).map(Vec::as_slice).unwrap_or(&[])
    }

    /// One representative cyclic edge per scene, using a linear iterative SCC
    /// pass followed by deterministic report sorting. No scene depth uses call stack.
    pub fn cycle_edges(&self) -> Vec<(&str, &str)> {
        let mut reverse: HashMap<&str, Vec<&str>> = HashMap::new();
        for (parent, children) in &self.edges {
            reverse.entry(parent).or_default();
            for child in children {
                reverse.entry(child).or_default().push(parent);
            }
        }
        let mut visited = HashSet::new();
        let mut order = Vec::new();
        for &root in reverse.keys() {
            let mut stack = vec![(root, false)];
            while let Some((node, finished)) = stack.pop() {
                if finished {
                    order.push(node);
                    continue;
                }
                if !visited.insert(node) {
                    continue;
                }
                stack.push((node, true));
                stack.extend(
                    self.children(node)
                        .iter()
                        .rev()
                        .map(|child| (child.as_str(), false)),
                );
            }
        }
        let mut component = HashMap::new();
        let mut sizes = Vec::new();
        for root in order.into_iter().rev() {
            if component.contains_key(root) {
                continue;
            }
            let id = sizes.len();
            let mut size = 0;
            let mut stack = vec![root];
            while let Some(node) = stack.pop() {
                if component.contains_key(node) {
                    continue;
                }
                component.insert(node, id);
                size += 1;
                stack.extend(reverse[node].iter().copied());
            }
            sizes.push(size);
        }
        let mut cycles = Vec::new();
        for (parent, children) in &self.edges {
            let id = component[parent.as_str()];
            if let Some(child) = children.iter().find(|child| {
                component[child.as_str()] == id && (sizes[id] > 1 || child.as_str() == parent)
            }) {
                cycles.push((parent.as_str(), child.as_str()));
            }
        }
        cycles.sort_unstable();
        cycles
    }
}
