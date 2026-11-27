//! Checked-in wire fixtures. Parse, serialize, and compare as `serde_json::Value`.
//!
//! `read_registry_yaml_from_path` clears scene UUIDs and disables imported
//! controls. That is import policy, not the file format, so the YAML fixture
//! is read with `serde_saphyr` on `SceneRegistry`.
//!
//! A fixture holds one value per enum field, so `enum-spellings.json` pins
//! every variant's persisted spelling.

#![cfg(test)]

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::domain::appearance::{Language, MotionLevel, ThemeMode, UiDensity};
use crate::domain::hotkey::{LeaderKey, SceneHotkeyStyle};
use crate::domain::mixer::{MixerGrouping, MixerMode};
use crate::domain::output::OutputKind;
use crate::domain::output_result::{OutputResultKind, OutputResultStatus};
use crate::domain::role::SceneRole;
use crate::domain::workflow::ActivityOrigin;

/// Fail when `fixture` is not the JSON this type writes back.
pub(super) fn json_round_trip<T>(fixture: &str)
where
    T: DeserializeOwned + Serialize,
{
    let parsed: T = serde_json::from_str(fixture).expect("golden fixture parses");
    let again = serde_json::to_value(&parsed).expect("golden fixture serializes");
    let original: serde_json::Value =
        serde_json::from_str(fixture).expect("golden fixture is json");
    assert_eq!(again, original);
}

/// Fail when `fixture` is not the YAML this type writes back.
///
/// Compared as `serde_json::Value`, so `HashMap` key order does not matter.
/// One entry per map keeps the file readable. The comparison is what fails a rename.
pub(super) fn yaml_round_trip<T>(fixture: &str)
where
    T: DeserializeOwned + Serialize,
{
    let original: serde_json::Value =
        serde_saphyr::from_str(fixture).expect("golden yaml fixture parses");
    let parsed: T = serde_saphyr::from_str(fixture).expect("golden yaml fixture matches the type");
    let text = serde_saphyr::to_string(&parsed).expect("golden yaml fixture serializes");
    let again: serde_json::Value =
        serde_saphyr::from_str(&text).expect("serialized golden yaml parses");
    assert_eq!(original, again);
}

/// Fail when `variants` stop writing, or reading back from, the spellings
/// listed under `name`. Removes the entry so unchecked leftovers show up.
fn assert_spellings<T>(spellings: &mut Map<String, Value>, name: &str, variants: &[T])
where
    T: DeserializeOwned + Serialize + PartialEq + std::fmt::Debug,
{
    let listed = spellings
        .remove(name)
        .unwrap_or_else(|| panic!("enum-spellings.json has no {name}"));
    let written = serde_json::to_value(variants).expect("variants serialize");
    assert_eq!(written, listed, "{name} writes a different spelling");
    let read: Vec<T> = serde_json::from_value(listed).expect("listed spellings parse");
    assert_eq!(
        read, variants,
        "{name} reads a listed spelling as another variant"
    );
}

#[test]
fn persisted_enum_variants_keep_their_spelling() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/enum-spellings.json"))
        .expect("enum spelling fixture is json");
    let Value::Object(mut spellings) = fixture else {
        panic!("enum spelling fixture is an object");
    };

    assert_spellings(
        &mut spellings,
        "ActivityOrigin",
        &[
            ActivityOrigin::Requested,
            ActivityOrigin::Observed,
            ActivityOrigin::Failed,
            ActivityOrigin::Connection,
        ],
    );
    assert_spellings(&mut spellings, "Language", &Language::ALL);
    assert_spellings(&mut spellings, "LeaderKey", &LeaderKey::ALL);
    assert_spellings(&mut spellings, "MixerGrouping", &MixerGrouping::ALL);
    assert_spellings(&mut spellings, "MixerMode", &MixerMode::ALL);
    assert_spellings(&mut spellings, "MotionLevel", &MotionLevel::ALL);
    assert_spellings(
        &mut spellings,
        "OutputKind",
        &[OutputKind::Stream, OutputKind::Record],
    );
    assert_spellings(
        &mut spellings,
        "OutputResultKind",
        &[
            OutputResultKind::ReplaySave,
            OutputResultKind::RecordingStop,
            OutputResultKind::RecordingSplit,
            OutputResultKind::RecordingChapter,
        ],
    );
    assert_spellings(
        &mut spellings,
        "OutputResultStatus",
        &[
            OutputResultStatus::Requested,
            OutputResultStatus::Acknowledged,
            OutputResultStatus::Observed,
            OutputResultStatus::Failed,
            OutputResultStatus::Unconfirmed,
        ],
    );
    assert_spellings(&mut spellings, "SceneHotkeyStyle", &SceneHotkeyStyle::ALL);
    assert_spellings(&mut spellings, "SceneRole", &SceneRole::ALL);
    assert_spellings(
        &mut spellings,
        "ThemeMode",
        &[ThemeMode::System, ThemeMode::Light, ThemeMode::Dark],
    );
    assert_spellings(
        &mut spellings,
        "UiDensity",
        &[UiDensity::Compact, UiDensity::Comfortable],
    );

    assert!(
        spellings.is_empty(),
        "enum-spellings.json lists unchecked enums {:?}",
        spellings.keys().collect::<Vec<_>>()
    );
}
