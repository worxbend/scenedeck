//! Lightweight OBS list state used by selector widgets.

/// Named OBS resources plus the currently selected item.
#[derive(Debug, Default, Clone)]
pub struct ObsNamedList {
    /// Available resource names in OBS order.
    pub items: Vec<String>,
    /// Current resource name, if OBS reported one.
    pub current: Option<String>,
}

impl ObsNamedList {
    /// Return a copy with `current` replaced by the newly reported OBS item.
    pub fn with_current(mut self, current: String) -> Self {
        self.current = Some(current);
        self
    }

    /// Return a copy using `fallback_items` only when OBS returned an empty
    /// list.
    pub fn with_fallback_items(mut self, fallback_items: Vec<String>) -> Self {
        if self.items.is_empty() {
            self.items = fallback_items;
        }
        self
    }

    /// Position of the current item inside `items`, if both are known.
    pub fn current_index(&self) -> Option<usize> {
        self.current
            .as_ref()
            .and_then(|current| self.items.iter().position(|item| item == current))
    }

    /// Whether the list contains any selectable items.
    pub fn has_items(&self) -> bool {
        !self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_index_finds_reported_item() {
        let list = ObsNamedList {
            items: vec!["One".to_string(), "Two".to_string()],
            current: Some("Two".to_string()),
        };

        assert_eq!(list.current_index(), Some(1));
        assert!(list.has_items());
    }

    #[test]
    fn current_index_is_absent_when_current_is_missing_or_unknown() {
        let missing = ObsNamedList {
            items: vec!["One".to_string()],
            current: Some("Two".to_string()),
        };
        let unknown = ObsNamedList {
            items: vec!["One".to_string()],
            current: None,
        };

        assert_eq!(missing.current_index(), None);
        assert_eq!(unknown.current_index(), None);
        assert!(!ObsNamedList::default().has_items());
    }

    #[test]
    fn with_current_replaces_reported_current_item() {
        let list = ObsNamedList {
            items: vec!["One".to_string(), "Two".to_string()],
            current: Some("One".to_string()),
        };

        assert_eq!(
            list.with_current("Two".to_string()).current,
            Some("Two".to_string())
        );
    }

    #[test]
    fn fallback_items_are_used_only_when_list_is_empty() {
        let empty = ObsNamedList::default().with_fallback_items(vec!["One".to_string()]);
        let populated = ObsNamedList {
            items: vec!["Existing".to_string()],
            current: None,
        }
        .with_fallback_items(vec!["Fallback".to_string()]);

        assert_eq!(empty.items, ["One"]);
        assert_eq!(populated.items, ["Existing"]);
    }
}
