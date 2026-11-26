//! Checked-in wire fixtures. Parse, serialize, and compare as `serde_json::Value`.
//!
//! `read_registry_yaml_from_path` clears scene UUIDs and disables imported
//! controls. That is import policy, not the file format, so the YAML fixture
//! is read with `serde_saphyr` on `SceneRegistry`.

#![cfg(test)]

use serde::de::DeserializeOwned;
use serde::Serialize;

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
