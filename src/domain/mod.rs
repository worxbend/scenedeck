//! Pure application concepts, free of GTK and of the OBS protocol.

pub mod appearance;
pub mod audio;
pub mod diagnostic;
pub mod graph;
pub mod hotkey;
pub mod icon;
pub mod meter;
pub mod mixer;
pub mod obs;
pub mod output;
pub mod registry;
pub mod role;
pub mod scene;
pub mod stats;

/// Derive `Serialize`/`Deserialize` for an enum stored as a lowercase string.
///
/// Several settings are closed sets of states that persist as a single word:
/// theme mode, language, motion level, mixer mode, hotkey style, and so on.
/// They all round-trip the same way — write `as_str()`, read it back through
/// `FromStr` — and each was carrying its own copy of two trait impls that
/// differed only in the type name. Eight copies of the same twenty lines is
/// eight chances for one of them to serialize the wrong thing without anyone
/// noticing.
///
/// The type must provide:
/// - `fn as_str(&self) -> &str`, giving the stored spelling, and
/// - `FromStr` with `Err = Infallible`, which is what makes reading total:
///   an unrecognized value falls back to the type's default rather than
///   failing the whole config load and losing every other setting with it.
macro_rules! string_enum_serde {
    ($ty:ty) => {
        impl serde::Serialize for $ty {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> serde::Deserialize<'de> for $ty {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                match value.parse() {
                    Ok(parsed) => Ok(parsed),
                    // `FromStr::Err` is `Infallible`, so this arm cannot be
                    // reached; matching on the empty type proves it.
                    Err(never) => match never {},
                }
            }
        }
    };
}

pub(crate) use string_enum_serde;

#[cfg(test)]
mod tests {
    use super::appearance::{Language, MotionLevel, ThemeMode, UiDensity};
    use super::hotkey::{LeaderKey, SceneHotkeyStyle};
    use super::mixer::{MixerGrouping, MixerMode};

    /// Round-trip a value through JSON and assert it comes back unchanged, and
    /// that what was written is the type's own `as_str` spelling.
    macro_rules! assert_round_trips {
        ($value:expr) => {{
            let value = $value;
            let json = serde_json::to_string(&value).expect("serialize");
            assert_eq!(json, format!("\"{}\"", value.as_str()));
            assert_eq!(serde_json::from_str(&json).ok(), Some(value));
        }};
    }

    #[test]
    fn every_string_enum_round_trips_through_its_own_spelling() {
        for mode in [ThemeMode::System, ThemeMode::Light, ThemeMode::Dark] {
            assert_round_trips!(mode);
        }
        for language in Language::ALL {
            assert_round_trips!(language);
        }
        for density in [UiDensity::Compact, UiDensity::Comfortable] {
            assert_round_trips!(density);
        }
        for motion in MotionLevel::ALL {
            assert_round_trips!(motion);
        }
        for style in SceneHotkeyStyle::ALL {
            assert_round_trips!(style);
        }
        for leader in LeaderKey::ALL {
            assert_round_trips!(leader);
        }
        for mode in [
            MixerMode::ActiveScene,
            MixerMode::SelectedScene,
            MixerMode::PinnedScene,
        ] {
            assert_round_trips!(mode);
        }
        for grouping in [
            MixerGrouping::Scope,
            MixerGrouping::ScenePath,
            MixerGrouping::None,
        ] {
            assert_round_trips!(grouping);
        }
    }

    #[test]
    fn an_unknown_spelling_falls_back_instead_of_failing_the_load() {
        // Reading is total on purpose: a value this build does not recognize —
        // a setting from a newer version, or a hand-edited config — must not
        // fail the whole config load and take every other setting with it.
        assert_eq!(
            serde_json::from_str::<ThemeMode>("\"chartreuse\"").unwrap(),
            ThemeMode::System
        );
        assert_eq!(
            serde_json::from_str::<MixerMode>("\"whatever\"").unwrap(),
            MixerMode::ActiveScene
        );
    }
}
