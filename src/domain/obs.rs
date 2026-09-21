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
}
