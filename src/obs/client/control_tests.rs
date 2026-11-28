use super::*;
use crate::domain::control::{ControlAction, MediaAction, WorkspaceWarning};
use crate::obs::testing::scripted::{mock_client, ScriptedObs};
use crate::obs::testing::wire::{request_data, Reply};
use serde_json::json;

#[tokio::test]
async fn directional_actions_preserve_scene_item_owner_and_explicit_state() {
    let (client, server) = mock_client(vec![
        ("SetSceneItemEnabled", json!({})),
        ("PauseRecord", json!({})),
        ("ResumeRecord", json!({})),
        ("StartReplayBuffer", json!({})),
        ("TriggerMediaInputAction", json!({})),
    ])
    .await;
    client
        .execute_control(&ControlAction::SetItemEnabled {
            owner: "Nested group".into(),
            item_id: 42,
            enabled: false,
        })
        .await
        .unwrap();
    client
        .execute_control(&ControlAction::SetRecordingPaused(true))
        .await
        .unwrap();
    client
        .execute_control(&ControlAction::SetRecordingPaused(false))
        .await
        .unwrap();
    client
        .execute_control(&ControlAction::SetReplayActive(true))
        .await
        .unwrap();
    client
        .execute_control(&ControlAction::Media {
            input: "Clip".into(),
            action: MediaAction::Pause,
        })
        .await
        .unwrap();
    let requests = server.await.unwrap();
    assert_eq!(
        *request_data(&requests[1]),
        json!({"sceneName":"Nested group","sceneItemId":42,"sceneItemEnabled":false})
    );
    assert_eq!(
        request_data(&requests[5])["mediaAction"],
        "OBS_WEBSOCKET_MEDIA_INPUT_ACTION_PAUSE"
    );
}

#[tokio::test]
async fn control_snapshot_lists_scenes_in_switcher_order() {
    let scene = |index: u64, name: &str| {
        let uuid = format!("00000000-0000-0000-0000-00000000000{index}");
        json!({"sceneIndex": index, "sceneName": name, "sceneUuid": uuid})
    };
    let (client, _server) = ScriptedObs::by_type([
        (
            "GetSceneList",
            Reply::Ok(json!({
                "scenes": [scene(0, "Bottom"), scene(1, "Middle"), scene(2, "Top")],
                "currentProgramSceneName": "Top",
                "currentProgramSceneUuid": "00000000-0000-0000-0000-000000000002",
                "currentPreviewSceneName": "Middle",
                "currentPreviewSceneUuid": "00000000-0000-0000-0000-000000000001"
            })),
        ),
        ("GetStudioModeEnabled", Reply::err(604)),
        ("GetSceneTransitionList", Reply::err(604)),
        ("GetCurrentSceneTransition", Reply::err(604)),
        ("GetReplayBufferStatus", Reply::err(604)),
        ("GetLastReplayBufferReplay", Reply::err(604)),
        ("GetVirtualCamStatus", Reply::err(604)),
    ])
    .connect()
    .await;

    let snapshot = client.get_control_snapshot(None, false).await.unwrap();
    let inventory = client.get_scene_inventory().await.unwrap();

    let names: Vec<_> = snapshot.scenes.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(names, ["Top", "Middle", "Bottom"]);
    assert_eq!(snapshot.scenes, inventory.scenes);
    assert_eq!(snapshot.studio.program.as_deref(), Some("Top"));
    assert_eq!(snapshot.studio.preview.as_deref(), Some("Middle"));
    assert_eq!(snapshot.scene.as_deref(), Some("Top"));
}

#[tokio::test]
async fn seeking_checks_fresh_duration_before_sending_cursor() {
    let (client, server) = mock_client(vec![(
        "GetMediaInputStatus",
        json!({"mediaState":"OBS_MEDIA_STATE_PLAYING","mediaDuration":5000,"mediaCursor":0}),
    )])
    .await;
    let error = client
        .execute_control(&ControlAction::SeekMedia {
            input: "Clip".into(),
            position_ms: 5001,
        })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("outside current duration"));
    assert_eq!(server.await.unwrap().len(), 2);
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
async fn text_edit_only_overlays_text_and_preserves_other_properties() {
    let (client, server) = mock_client(vec![
        ("GetInputSettings", json!({"inputKind":"text_ft2_source_v2", "inputSettings":{"text":"Old","color":123,"from_file":false}})),
        ("GetInputDefaultSettings", json!({"defaultInputSettings":{"from_file":false}})),
        ("SetInputSettings", json!({})),
    ]).await;
    client
        .execute_control(&ControlAction::SetText {
            input: "Caption".into(),
            text: "New".into(),
        })
        .await
        .unwrap();
    let requests = server.await.unwrap();
    assert_eq!(
        *request_data(&requests[3]),
        json!({"inputName":"Caption","inputSettings":{"text":"New"},"overlay":true})
    );
}

#[tokio::test]
async fn file_backed_text_never_sends_write() {
    let (client, server) = mock_client(vec![
        (
            "GetInputSettings",
            json!({"inputKind":"text_ft2_source_v2", "inputSettings":{}}),
        ),
        (
            "GetInputDefaultSettings",
            json!({"defaultInputSettings":{"from_file":true}}),
        ),
    ])
    .await;
    assert!(client
        .execute_control(&ControlAction::SetText {
            input: "Caption".into(),
            text: "New".into()
        })
        .await
        .is_err());
    assert_eq!(server.await.unwrap().len(), 3);
}

#[tokio::test]
async fn filter_enabled_action_retains_source_and_filter_identity() {
    let (client, server) = mock_client(vec![("SetSourceFilterEnabled", json!({}))]).await;
    client
        .execute_control(&ControlAction::SetFilterEnabled {
            input: "Mic".into(),
            filter: "Noise suppression".into(),
            enabled: false,
        })
        .await
        .unwrap();
    let requests = server.await.unwrap();
    assert_eq!(
        *request_data(&requests[1]),
        json!({"sourceName":"Mic","filterName":"Noise suppression","filterEnabled":false})
    );
}

#[test]
fn thumbnails_fit_canvas_without_stretching_or_unbounded_decode() {
    assert_eq!(thumbnail_dimensions(1920, 1080), (384, 216));
    assert_eq!(thumbnail_dimensions(1080, 1920), (122, 216));
    assert_eq!(thumbnail_dimensions(4000, 1000), (384, 96));
    assert_eq!(thumbnail_dimensions(0, 0), (1, 1));
    assert_eq!(thumbnail_dimensions(320, 180), (320, 180));
}
#[tokio::test]
async fn cancelled_thumbnail_does_not_request_canvas_settings() {
    let (client, server) = mock_client(vec![]).await;
    assert!(client
        .get_scene_thumbnail("Portrait", &std::sync::atomic::AtomicBool::new(true))
        .await
        .is_err());
    assert_eq!(server.await.unwrap().len(), 1);
}

#[tokio::test]
async fn thumbnail_request_is_bounded_and_preserves_portrait_canvas() {
    let (client,server)=mock_client(vec![
        ("GetVideoSettings",json!({"fpsNumerator":30,"fpsDenominator":1,"baseWidth":1080,"baseHeight":1920,"outputWidth":1080,"outputHeight":1920})),
        ("GetSourceScreenshot",json!({"imageData":"data:image/png;base64,fixture"})),
    ]).await;
    client
        .get_scene_thumbnail("Portrait", &std::sync::atomic::AtomicBool::new(false))
        .await
        .unwrap();
    let requests = server.await.unwrap();
    assert_eq!(
        *request_data(&requests[2]),
        json!({"sourceName":"Portrait","imageFormat":"png","imageWidth":122,"imageHeight":216,"imageCompressionQuality":-1})
    );
}

#[tokio::test]
async fn invalid_audio_values_never_reach_obs() {
    let (client, server) = mock_client(vec![]).await;
    assert!(client
        .execute_control(&ControlAction::SetAudioSync {
            input: "Mic".into(),
            offset_ms: -951
        })
        .await
        .is_err());
    assert!(client
        .execute_control(&ControlAction::SetAudioBalance {
            input: "Mic".into(),
            balance: f32::NAN
        })
        .await
        .is_err());
    assert_eq!(server.await.unwrap().len(), 1);
}
