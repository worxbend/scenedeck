//! Portable scene views stored alongside each workspace's curation.

use serde::{Deserialize, Serialize};

use super::{SceneEntry, SceneRegistry};

pub(super) fn deserialize_decks<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<std::collections::BTreeMap<String, SceneFilter>, D::Error> {
    let decks = std::collections::BTreeMap::<String, SceneFilter>::deserialize(deserializer)?;
    let mut seen = std::collections::HashSet::new();
    if decks
        .keys()
        .any(|name| name.trim().is_empty() || !seen.insert(name.trim().to_lowercase()))
    {
        return Err(serde::de::Error::custom(
            "deck names must be nonempty and unique",
        ));
    }
    Ok(decks)
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SceneFilter {
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub favorites_only: bool,
}

impl SceneFilter {
    pub(crate) fn is_active(&self) -> bool {
        !self.query.trim().is_empty() || self.favorites_only
    }

    fn matches(&self, name: &str, entry: Option<&SceneEntry>) -> bool {
        if self.favorites_only && !entry.is_some_and(|entry| entry.favorite) {
            return false;
        }
        let mut haystack = name.to_lowercase();
        if let Some(entry) = entry {
            for tag in &entry.tags {
                haystack.push('\n');
                haystack.push_str(&tag.to_lowercase());
            }
        }
        self.query
            .to_lowercase()
            .split_whitespace()
            .all(|term| haystack.contains(term))
    }
}

impl SceneRegistry {
    pub(crate) fn scene_matches_filter(&self, name: &str) -> bool {
        self.scene_filter.matches(name, self.scenes.get(name))
    }

    /// Save a uniquely named view; duplicate names require an explicit removal.
    pub(crate) fn save_deck(&mut self, name: &str, filter: SceneFilter) -> bool {
        if !self.can_save_deck(name) {
            return false;
        }
        self.decks.insert(name.trim().to_owned(), filter);
        true
    }

    pub(crate) fn can_save_deck(&self, name: &str) -> bool {
        let name = name.trim().to_lowercase();
        !name.is_empty()
            && !self
                .decks
                .keys()
                .any(|existing| existing.trim().to_lowercase() == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ambiguous_or_empty_imported_deck_names_are_rejected() {
        for raw in [
            r#"{"decks":{"":{}}}"#,
            r#"{"decks":{"Podcast":{}," podcast ":{}}}"#,
        ] {
            assert!(serde_json::from_str::<SceneRegistry>(raw).is_err());
        }
        let registry: SceneRegistry =
            serde_json::from_str(r#"{"decks":{" Podcast ":{}}}"#).unwrap();
        assert!(!registry.can_save_deck("podcast"));
    }

    #[test]
    fn saved_decks_reject_ambiguous_names_and_round_trip_without_changing_current_view() {
        let mut registry = SceneRegistry::default();
        let filter = SceneFilter {
            query: "interview guest".into(),
            favorites_only: true,
        };
        assert!(!registry.save_deck("  ", filter.clone()));
        assert!(registry.save_deck(" Podcast ", filter.clone()));
        assert!(!registry.save_deck("PODCAST", SceneFilter::default()));
        assert_eq!(registry.scene_filter, SceneFilter::default());
        assert_eq!(registry.decks["Podcast"], filter);
        let raw = serde_saphyr::to_string(&registry).unwrap();
        let restored: SceneRegistry = serde_saphyr::from_str(&raw).unwrap();
        assert_eq!(restored, registry);
        let legacy: SceneRegistry = serde_json::from_str("{}").unwrap();
        assert!(legacy.decks.is_empty());
        assert!(!legacy.scene_filter.is_active());
    }

    #[test]
    fn filter_matches_names_and_tags_without_mutating_order_or_metadata() {
        let mut registry = SceneRegistry::default();
        registry.set_scene_tags("Guest Camera", ["Interview".into(), "Київ".into()]);
        registry.set_scene_favorite("Guest Camera", true);
        registry.scene_order = vec!["Intro".into(), "Guest Camera".into()];
        registry.scene_filter = SceneFilter {
            query: "  guest ІНТЕРVIEW  ".into(),
            favorites_only: false,
        };
        assert!(!registry.scene_matches_filter("Guest Camera"));
        registry.scene_filter.query = "GUEST Київ".into();
        assert!(registry.scene_matches_filter("Guest Camera"));
        assert!(!registry.scene_matches_filter("Intro"));
        registry.scene_filter = SceneFilter {
            query: String::new(),
            favorites_only: true,
        };
        assert!(registry.scene_matches_filter("Guest Camera"));
        assert!(!registry.scene_matches_filter("Intro"));
        registry.scene_filter = SceneFilter::default();
        assert!(registry.scene_matches_filter("Intro"));
        assert_eq!(registry.scene_order, ["Intro", "Guest Camera"]);
        assert_eq!(registry.scenes["Guest Camera"].tags, ["Interview", "Київ"]);
    }
}
