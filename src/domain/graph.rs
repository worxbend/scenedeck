use std::collections::HashMap;

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

/// Directed dependency graph of OBS scenes.
///
/// An edge `A → [B, C]` means scene A contains B and C as nested scene
/// sources.  Used by the Graph page and the Doctor rule engine.
#[derive(Debug, Default, Clone)]
pub struct SceneGraph {
    pub edges: HashMap<SceneId, Vec<SceneId>>,
}

impl SceneGraph {
    /// Direct children of `scene`.
    pub fn children(&self, scene: &str) -> &[SceneId] {
        self.edges.get(scene).map(Vec::as_slice).unwrap_or(&[])
    }

    /// `true` if `from` depends on `to` through any path.
    pub fn is_reachable(&self, from: &str, to: &str) -> bool {
        let mut visited = std::collections::HashSet::new();
        self.dfs(from, to, &mut visited)
    }

    fn dfs(
        &self,
        current: &str,
        target: &str,
        visited: &mut std::collections::HashSet<String>,
    ) -> bool {
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
