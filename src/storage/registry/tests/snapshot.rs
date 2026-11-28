//! The domain snapshot and the golden wire fixtures.

use super::scratch_file;
use crate::domain::registry::RoleDependency;
use crate::storage::registry::*;

#[test]
fn registry_snapshot_converts_string_rules_to_typed_policy() {
    let mut registry = SceneRegistry::default();
    registry
        .rules
        .primary_can_depend_on
        .push("module".to_string());
    registry
        .rules
        .forbidden_edges
        .push(["primary".to_string(), "debug".to_string()]);

    let snapshot = registry.snapshot();

    assert_eq!(
        snapshot.graph_policy().primary_can_depend_on,
        vec![SceneRole::Module]
    );
    assert!(snapshot
        .graph_policy()
        .forbidden_edges
        .contains(&RoleDependency::new(SceneRole::Primary, SceneRole::Debug)));
    assert!(snapshot
        .graph_policy()
        .forbidden_edges
        .contains(&RoleDependency::new(SceneRole::Raw, SceneRole::Module)));
}

#[test]
fn registry_json_fixture_round_trips_through_the_file_writer() {
    let fixture = include_str!("../../fixtures/registry.json");
    let (_dir, path) = scratch_file("registry-golden", "json");
    let registry: SceneRegistry = serde_json::from_str(fixture).unwrap();

    write_registry_to_path(&path, &registry).unwrap();

    let written: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let original: serde_json::Value = serde_json::from_str(fixture).unwrap();
    assert_eq!(written, original);
}

#[test]
fn registry_yaml_fixture_round_trips() {
    // Not `read_registry_yaml_from_path`: that import clears identities.
    crate::storage::golden::yaml_round_trip::<SceneRegistry>(include_str!(
        "../../fixtures/registry.yaml"
    ));
}

#[test]
fn registry_fixture_names_every_collection_field() {
    let names = collection_serde_names();
    for anchor in ["scenes", "decks", "audio_presets"] {
        assert!(
            names.iter().any(|name| name == anchor),
            "{anchor} not seen as a collection; got {names:?}"
        );
    }
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/registry.json")).unwrap();
    let object = fixture.as_object().unwrap();
    let missing: Vec<_> = names
        .iter()
        .filter(|name| !object.contains_key(name.as_str()))
        .cloned()
        .collect();
    assert!(
        missing.is_empty(),
        "registry fixture is missing collection fields {missing:?}"
    );
}

fn collection_serde_names() -> Vec<String> {
    let file = syn::parse_file(include_str!("../../registry.rs")).unwrap();
    let fields = file
        .items
        .into_iter()
        .find_map(|item| {
            let syn::Item::Struct(item) = item else {
                return None;
            };
            (item.ident == "SceneRegistry").then_some(item.fields)
        })
        .unwrap();
    let syn::Fields::Named(fields) = fields else {
        panic!("SceneRegistry fields are named");
    };
    let mut names: Vec<String> = fields
        .named
        .into_iter()
        .filter(|field| is_collection(&field.ty))
        .map(|field| {
            serde_name(&field).unwrap_or_else(|| field.ident.expect("named field").to_string())
        })
        .collect();
    names.sort();
    names
}

fn is_collection(ty: &syn::Type) -> bool {
    // shortcut: direct Vec/HashMap/BTreeMap fields only, upgrade when a collection is a type alias
    let syn::Type::Path(path) = ty else {
        return false;
    };
    matches!(
        path.path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .as_deref(),
        Some("Vec" | "HashMap" | "BTreeMap")
    )
}

fn serde_name(field: &syn::Field) -> Option<String> {
    let mut renamed = None;
    for attr in &field.attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let value = meta.value()?;
                let literal: syn::LitStr = value.parse()?;
                renamed = Some(literal.value());
            } else if meta.input.peek(syn::Token![=]) {
                let value = meta.value()?;
                let _: syn::Expr = value.parse()?;
            }
            Ok(())
        })
        .expect("serde attribute parses");
    }
    renamed
}
