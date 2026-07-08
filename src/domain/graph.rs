//! Directed scene dependency graph types.

use std::collections::{HashMap, HashSet};

use crate::domain::scene::SceneId;

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
    pub const fn summary_label(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Warning => "Warning",
            Self::Forbidden => "Forbidden",
        }
    }

    /// Tooltip used to explain this edge status in the graph summary.
    pub const fn summary_tooltip(self) -> &'static str {
        match self {
            Self::Ok => "Edges that match the graph policy",
            Self::Warning => "Edges outside an allow-list",
            Self::Forbidden => "Edges forbidden by graph policy",
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

    /// `true` if `from` depends on `to` through any path.
    pub fn is_reachable(&self, from: &str, to: &str) -> bool {
        let mut visited = HashSet::new();
        self.dfs(from, to, &mut visited)
    }

    fn dfs(&self, current: &str, target: &str, visited: &mut HashSet<String>) -> bool {
        if current == target {
            return true;
        }
        if !visited.insert(current.to_string()) {
            return false; // already explored
        }
        self.children(current)
            .iter()
            .any(|child| self.dfs(child, target, visited))
    }
}
