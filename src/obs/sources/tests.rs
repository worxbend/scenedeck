use super::*;
use crate::obs::testing::scripted::{mock_client, ScriptedObs};
use crate::obs::testing::wire::{scene, scene_item, Reply};
use serde_json::json;

#[test]
fn warnings_keep_a_bounded_sample_and_count_the_rest() {
    let mut warnings = ScanWarnings::default();
    for _ in 0..MAX_WARNING_SAMPLES + 3 {
        warnings.push(WorkspaceWarning::InventoryTruncated);
    }
    let finished = warnings.finish();
    assert_eq!(finished.len(), MAX_WARNING_SAMPLES + 1);
    assert_eq!(finished.last(), Some(&WorkspaceWarning::Omitted(3)));
    assert!(ScanWarnings::default().finish().is_empty());
}

#[test]
fn budget_charges_items_and_containers_separately() {
    let mut budget = ScanBudget::new(2);
    assert!(budget.inspect(2).is_ok());
    assert!(budget.inspect(1).is_err());
    for _ in 0..2 + MAX_ROOT_SCENES {
        assert!(budget.container().is_ok());
    }
    assert!(budget.container().is_err());
}

#[tokio::test]
async fn recursive_inventory_keeps_disabled_group_owner_and_input_identity() {
    let input = json!({"inputName":"Mic", "inputUuid":"00000000-0000-0000-0000-000000000001", "inputKind":"pulse_input_capture", "unversionedInputKind":"pulse_input_capture"});
    let (client, server) = mock_client(vec![
        ("GetInputList", json!({"inputs":[input]})),
        ("GetSceneItemList", json!({"sceneItems":[{"sceneItemId":7,"sceneItemIndex":0,"sourceName":"Group","sourceType":"OBS_SOURCE_TYPE_SCENE","isGroup":true}]})),
        ("GetSceneItemEnabled", json!({"sceneItemEnabled":false})),
        ("GetGroupSceneItemList", json!({"sceneItems":[{"sceneItemId":9,"sceneItemIndex":0,"sourceName":"Mic","sourceType":"OBS_SOURCE_TYPE_INPUT","inputKind":"pulse_input_capture"}]})),
        ("GetSceneItemEnabled", json!({"sceneItemEnabled":true})),
    ]).await;
    let (items, warnings) = client.scan_source_items("Program", 4096).await;
    assert!(warnings.is_empty());
    assert_eq!(items.len(), 2);
    assert_eq!(items[1].owner, "Group");
    assert_eq!(items[1].item_id, 9);
    assert_eq!(items[1].path, ["Program", "Group"]);
    assert!(items[1].enabled);
    assert!(!items[1].ancestor_enabled);
    assert_eq!(items[1].input_uuid, "00000000-0000-0000-0000-000000000001");
    assert_eq!(server.await.unwrap().len(), 6);
}

#[tokio::test]
async fn source_inventory_budget_stops_extra_visibility_requests_and_reports_partial() {
    let (client, server) = mock_client(vec![
        ("GetInputList", json!({"inputs":[]})),
        ("GetSceneItemList", json!({"sceneItems":[
            {"sceneItemId":1,"sceneItemIndex":0,"sourceName":"First","sourceType":"OBS_SOURCE_TYPE_INPUT","inputKind":"image_source"},
            {"sceneItemId":2,"sceneItemIndex":1,"sourceName":"Second","sourceType":"OBS_SOURCE_TYPE_INPUT","inputKind":"image_source"}
        ]})),
        ("GetSceneItemEnabled", json!({"sceneItemEnabled":true})),
    ]).await;
    let (items, warnings) = client.scan_source_items("Program", 1).await;
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].input, "First");
    assert!(warnings.contains(&WorkspaceWarning::InventoryTruncated));
    assert_eq!(server.await.unwrap().len(), 4);
}

#[tokio::test]
async fn missing_nested_scene_reports_partial_coverage_without_losing_other_items() {
    let (client, server) = ScriptedObs::ordered(vec![
        ("GetInputList", Reply::Ok(json!({"inputs":[]}))),
        ("GetSceneItemList", Reply::Ok(json!({"sceneItems":[
            {"sceneItemId":1,"sceneItemIndex":0,"sourceName":"Missing nested scene","sourceType":"OBS_SOURCE_TYPE_SCENE","isGroup":false},
            {"sceneItemId":2,"sceneItemIndex":1,"sourceName":"Sibling","sourceType":"OBS_SOURCE_TYPE_INPUT","inputKind":"image_source"}
        ]}))),
        ("GetSceneItemEnabled",Reply::Ok(json!({"sceneItemEnabled":true}))),
        ("GetSceneItemEnabled",Reply::Ok(json!({"sceneItemEnabled":true}))),
        ("GetSceneItemList",Reply::err(600)),
    ])
    .connect()
    .await;
    let (items, warnings) = client.scan_source_items("Program", 4096).await;
    assert_eq!(items.len(), 2);
    assert!(items.iter().any(|item| item.input == "Sibling"));
    assert_eq!(warnings.len(), 1);
    assert!(matches!(
        &warnings[0],
        WorkspaceWarning::ContainerUnavailable { path, .. } if path.contains("Missing nested scene")
    ));
    server.await.unwrap();
}

#[tokio::test]
async fn source_usage_shares_attempts_across_failure_heavy_roots() {
    let (client, server) = ScriptedObs::ordered(vec![
        ("GetSceneList", Reply::Ok(json!({"scenes":[scene("A"),scene("B"),scene("C")],"currentProgramSceneName":"A","currentProgramSceneUuid":"00000000-0000-0000-0000-000000000001"}))),
        ("GetInputList", Reply::Ok(json!({"inputs":[]}))),
        ("GetSceneItemList", Reply::Ok(json!({"sceneItems":[scene_item(1,"Mic")]}))),
        ("GetSceneItemEnabled", Reply::err(702)),
        ("GetSceneItemList", Reply::Ok(json!({"sceneItems":[scene_item(2,"Mic")]}))),
        ("GetSceneItemEnabled", Reply::err(702)),
    ])
    .connect()
    .await;
    let (items, warnings) = client.source_usage_with_limit("Mic", 2).await.unwrap();
    assert!(items.is_empty());
    assert_eq!(
        warnings
            .iter()
            .filter(|w| matches!(w, WorkspaceWarning::VisibilityUnavailable { .. }))
            .count(),
        2
    );
    assert!(warnings.contains(&WorkspaceWarning::UsagePartial {
        scenes: 64,
        items: 2
    }));
    let requests = server.await.unwrap();
    assert_eq!(
        requests
            .iter()
            .filter(|r| r["d"]["requestType"] == "GetSceneItemEnabled")
            .count(),
        2
    );
}

#[tokio::test]
async fn source_budget_counts_failed_visibility_before_nested_cycle_scan() {
    let nested = json!({"sceneItemId":1,"sceneItemIndex":0,"sourceName":"Nested","sourceType":"OBS_SOURCE_TYPE_SCENE","isGroup":false});
    let (client, server) = ScriptedObs::ordered(vec![
        ("GetInputList", Reply::Ok(json!({"inputs":[]}))),
        (
            "GetSceneItemList",
            Reply::Ok(json!({"sceneItems":[nested,scene_item(2,"Fail")]})),
        ),
        (
            "GetSceneItemEnabled",
            Reply::Ok(json!({"sceneItemEnabled":true})),
        ),
        ("GetSceneItemEnabled", Reply::err(702)),
    ])
    .connect()
    .await;
    let (items, warnings) = client.scan_source_items("Program", 2).await;
    assert_eq!(items.len(), 1);
    assert!(warnings.contains(&WorkspaceWarning::InventoryTruncated));
    assert_eq!(server.await.unwrap().len(), 5);
}

#[tokio::test]
async fn graph_flattens_groups_including_disabled_and_repeated_membership() {
    let group = json!({"sceneItemId":1,"sceneItemIndex":0,"sourceName":"Group","sourceType":"OBS_SOURCE_TYPE_SCENE","isGroup":true});
    let nested = json!({"sceneItemId":2,"sceneItemIndex":0,"sourceName":"Debug","sourceType":"OBS_SOURCE_TYPE_SCENE","isGroup":false});
    let group_cycle = json!({"sceneItemId":3,"sceneItemIndex":0,"sourceName":"Group","sourceType":"OBS_SOURCE_TYPE_SCENE","isGroup":true});
    let (client, server) = mock_client(vec![
        (
            "GetSceneItemList",
            json!({"sceneItems":[group.clone(),group]}),
        ),
        (
            "GetGroupSceneItemList",
            json!({"sceneItems":[nested.clone(),nested,group_cycle]}),
        ),
        ("GetSceneItemList", json!({"sceneItems":[]})),
    ])
    .await;
    let graph = client
        .get_scene_graph(&["Program".into(), "Debug".into()])
        .await
        .unwrap();
    assert_eq!(graph.children("Program"), &["Debug"]);
    assert!(!graph.edges.contains_key("Group"));
    assert_eq!(server.await.unwrap().len(), 4); // no visibility reads: topology includes disabled groups
}

#[tokio::test]
async fn failed_group_lookup_never_publishes_complete_looking_graph() {
    let group = json!({"sceneItemId":1,"sceneItemIndex":0,"sourceName":"Group","sourceType":"OBS_SOURCE_TYPE_SCENE","isGroup":true});
    let (client, server) = ScriptedObs::ordered(vec![
        ("GetSceneItemList", Reply::Ok(json!({"sceneItems":[group]}))),
        ("GetGroupSceneItemList", Reply::err(702)),
    ])
    .connect()
    .await;
    assert!(client
        .get_scene_graph(&["Program".into()])
        .await
        .unwrap_err()
        .to_string()
        .contains("Group"));
    assert_eq!(server.await.unwrap().len(), 3);
}

#[tokio::test]
async fn failure_warning_samples_are_bounded_without_losing_omitted_count() {
    let items: Vec<_> = (0..40).map(|id| scene_item(id, "Failed")).collect();
    let mut script = vec![
        ("GetInputList", Reply::Ok(json!({"inputs":[]}))),
        ("GetSceneItemList", Reply::Ok(json!({"sceneItems":items}))),
    ];
    script.extend((0..40).map(|_| ("GetSceneItemEnabled", Reply::err(702))));
    let (client, server) = ScriptedObs::ordered(script).connect().await;
    let (items, warnings) = client.scan_source_items("Program", 40).await;
    assert!(items.is_empty());
    assert_eq!(warnings.len(), 33);
    assert_eq!(warnings.last(), Some(&WorkspaceWarning::Omitted(8)));
    assert_eq!(server.await.unwrap().len(), 43);
}
