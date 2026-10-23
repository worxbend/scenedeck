//! Mock-wire regressions for incomplete observations and bounded work.
#![cfg(test)]

use super::control_tests::mock_client;
use super::*;
use crate::domain::control::ControlAction;
use serde_json::{json, Value};

fn input(name: &str) -> Value {
    json!({"inputName":name,"inputUuid":"00000000-0000-0000-0000-000000000001","inputKind":"mock","unversionedInputKind":"mock"})
}
fn scene_item(id: i64, name: &str) -> Value {
    json!({"sceneItemId":id,"sceneItemIndex":0,"sourceName":name,"sourceType":"OBS_SOURCE_TYPE_INPUT","inputKind":"mock"})
}
fn scene(name: &str) -> Value {
    json!({"sceneName":name,"sceneUuid":"00000000-0000-0000-0000-000000000001","sceneIndex":0})
}

#[tokio::test]
async fn definite_non_audio_is_omitted_but_complete_audio_is_retained() {
    let (client, server) = mock_client(vec![
        ("GetInputList", json!({"inputs":[input("Image"),input("Mic")]})),
        ("GetInputMute", json!({"__error":true,"__code":604,"__comment":"The specified input does not support audio."})),
        ("GetInputMute", json!({"inputMuted":false})),
        ("GetInputVolume", json!({"inputVolumeMul":0.5,"inputVolumeDb":-6.0206})),
    ]).await;
    let inputs = client.get_all_audio_inputs().await.unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].id, "Mic");
    assert!(!inputs[0].muted);
    assert_eq!(inputs[0].volume_mul, 0.5);
    assert_eq!(server.await.unwrap().len(), 5);
}

#[tokio::test]
async fn transient_and_ambiguous_audio_rejections_never_become_empty_success() {
    for code in [600, 604, 702] {
        let (client, server) = mock_client(vec![
            ("GetInputList", json!({"inputs":[input("Failed mic")]})),
            ("GetInputMute", json!({"__error":true,"__code":code})),
        ])
        .await;
        let error = client.get_all_audio_inputs().await.unwrap_err().to_string();
        assert!(error.contains("Failed mic mute unavailable"));
        assert_eq!(server.await.unwrap().len(), 3);
    }
    let (client, server) = mock_client(vec![("GetInputList", json!({"inputs":[]}))]).await;
    assert!(client.get_all_audio_inputs().await.unwrap().is_empty());
    assert_eq!(server.await.unwrap().len(), 2);
}

#[tokio::test]
async fn volume_failure_invalidates_audio_snapshot_instead_of_losing_the_row() {
    let (client, server) = mock_client(vec![
        ("GetInputList", json!({"inputs":[input("Mic")]})),
        ("GetInputMute", json!({"inputMuted":true})),
        ("GetInputVolume", json!({"__error":true,"__code":702})),
    ])
    .await;
    assert!(client
        .get_all_audio_inputs()
        .await
        .unwrap_err()
        .to_string()
        .contains("Mic volume unavailable"));
    assert_eq!(server.await.unwrap().len(), 4);
}

#[tokio::test]
async fn unknown_active_scene_visibility_fails_closed() {
    let (client, server) = mock_client(vec![
        ("GetSpecialInputs", json!({"desktop1":null,"desktop2":null,"mic1":null,"mic2":null,"mic3":null,"mic4":null})),
        (
            "GetSceneItemList",
            json!({"sceneItems":[scene_item(1,"Mic")]}),
        ),
        ("GetSceneItemEnabled", json!({"__error":true,"__code":702})),
    ])
    .await;
    assert!(client
        .get_scene_audio_inputs("Program", &[])
        .await
        .unwrap_err()
        .to_string()
        .contains("visibility unavailable"));
    assert_eq!(server.await.unwrap().len(), 4);
}

#[tokio::test]
async fn source_usage_shares_attempts_across_failure_heavy_roots() {
    let (client, server) = mock_client(vec![
        ("GetSceneList", json!({"scenes":[scene("A"),scene("B"),scene("C")],"currentProgramSceneName":"A","currentProgramSceneUuid":"00000000-0000-0000-0000-000000000001"})),
        ("GetInputList", json!({"inputs":[]})),
        ("GetSceneItemList", json!({"sceneItems":[scene_item(1,"Mic")]})),
        ("GetSceneItemEnabled", json!({"__error":true,"__code":702})),
        ("GetSceneItemList", json!({"sceneItems":[scene_item(2,"Mic")]})),
        ("GetSceneItemEnabled", json!({"__error":true,"__code":702})),
    ]).await;
    let (items, warnings) = client.source_usage_with_limit("Mic", 2).await.unwrap();
    assert!(items.is_empty());
    assert_eq!(
        warnings
            .iter()
            .filter(|w| w.contains("visibility unavailable"))
            .count(),
        2
    );
    assert!(warnings.iter().any(|w| w.contains("partial")));
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
    let (client, server) = mock_client(vec![
        ("GetInputList", json!({"inputs":[]})),
        (
            "GetSceneItemList",
            json!({"sceneItems":[nested,scene_item(2,"Fail")]}),
        ),
        ("GetSceneItemEnabled", json!({"sceneItemEnabled":true})),
        ("GetSceneItemEnabled", json!({"__error":true,"__code":702})),
    ])
    .await;
    let (items, warnings) = client.scan_source_items("Program", 2).await;
    assert_eq!(items.len(), 1);
    assert!(warnings.iter().any(|w| w.contains("truncated")));
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
    use crate::domain::registry::{SceneMetadata, SceneRegistrySnapshot};
    use crate::domain::role::SceneRole;
    use crate::domain::scene::{Scene, SceneInventory};
    let registry = SceneRegistrySnapshot::new(
        [("Program", SceneRole::Primary), ("Debug", SceneRole::Debug)]
            .into_iter()
            .map(|(name, role)| {
                (
                    name.to_owned(),
                    SceneMetadata {
                        role,
                        tags: Vec::new(),
                        protected: false,
                    },
                )
            })
            .collect(),
        crate::domain::registry::GraphDependencyPolicy::default()
            .forbid(SceneRole::Primary, SceneRole::Debug),
    );
    let inventory = SceneInventory {
        scenes: vec![
            Scene {
                id: "Program".into(),
            },
            Scene { id: "Debug".into() },
        ],
        ..Default::default()
    };
    let diagnostics =
        crate::services::doctor_service::DoctorService::run(&inventory, &registry, &graph);
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("Debug")));
    assert!(!diagnostics
        .iter()
        .any(|diagnostic| diagnostic.scene.as_deref() == Some("Group")));
    assert_eq!(server.await.unwrap().len(), 4); // no visibility reads: topology includes disabled groups
}

#[tokio::test]
async fn failed_group_lookup_never_publishes_complete_looking_graph() {
    let group = json!({"sceneItemId":1,"sceneItemIndex":0,"sourceName":"Group","sourceType":"OBS_SOURCE_TYPE_SCENE","isGroup":true});
    let (client, server) = mock_client(vec![
        ("GetSceneItemList", json!({"sceneItems":[group]})),
        (
            "GetGroupSceneItemList",
            json!({"__error":true,"__code":702}),
        ),
    ])
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
async fn fader_silence_unity_and_finite_levels_reach_wire_canonically() {
    let (client, server) = mock_client(vec![
        ("SetInputVolume", json!({})),
        ("SetInputVolume", json!({})),
        ("SetInputVolume", json!({})),
        ("SetInputVolume", json!({})),
    ])
    .await;
    for db in [MIN_VOLUME_DB, MAX_VOLUME_DB, -6.0] {
        client
            .execute_control(&ControlAction::SetInputVolumeDb {
                input: "Mic".into(),
                db,
            })
            .await
            .unwrap();
    }
    client.set_input_volume("Mic", 0.0).await.unwrap();
    let requests = server.await.unwrap();
    assert_eq!(requests[1]["d"]["requestData"]["inputVolumeMul"], 0.0);
    assert_eq!(requests[2]["d"]["requestData"]["inputVolumeMul"], 1.0);
    let finite = requests[3]["d"]["requestData"]["inputVolumeMul"]
        .as_f64()
        .unwrap();
    assert!((finite - volume_db_to_mul(-6.0)).abs() < 0.000001);
    assert_eq!(requests[4]["d"]["requestData"]["inputVolumeMul"], 0.0);
}

#[tokio::test]
async fn invalid_gain_commands_are_rejected_before_network_work() {
    let (client, server) = mock_client(vec![]).await;
    for db in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -100.1, 0.1] {
        assert!(client
            .execute_control(&ControlAction::SetInputVolumeDb {
                input: "Mic".into(),
                db
            })
            .await
            .is_err());
    }
    for mul in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.1, 1.1] {
        assert!(client.set_input_volume("Mic", mul).await.is_err());
    }
    assert_eq!(server.await.unwrap().len(), 1);
}

/// Withhold every ordinary response while observing all requests until EOF.
async fn unresponsive_client() -> (
    ObsClient,
    tokio::sync::mpsc::UnboundedReceiver<Value>,
    tokio::task::JoinHandle<Vec<Value>>,
) {
    use futures_util::SinkExt;
    use tokio_websockets::{Message, ServerBuilder};
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    let (seen_tx, seen_rx) = tokio::sync::mpsc::unbounded_channel();
    let server = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.unwrap();
        let (_, mut socket) = ServerBuilder::new().accept(tcp).await.unwrap();
        socket
            .send(Message::text(
                json!({"op":0,"d":{"obsWebSocketVersion":"5.5.0","rpcVersion":1}}).to_string(),
            ))
            .await
            .unwrap();
        socket.next().await.unwrap().unwrap();
        socket
            .send(Message::text(
                json!({"op":2,"d":{"negotiatedRpcVersion":1}}).to_string(),
            ))
            .await
            .unwrap();
        let version: Value =
            serde_json::from_str(socket.next().await.unwrap().unwrap().as_text().unwrap()).unwrap();
        socket.send(Message::text(json!({"op":7,"d":{"requestId":version["d"]["requestId"],"requestType":"GetVersion","requestStatus":{"result":true,"code":100},"responseData":{"obsStudioVersion":"31.0.0","obsWebSocketVersion":"5.5.0","rpcVersion":1,"availableRequests":[],"supportedImageFormats":[],"platform":"mock","platformDescription":"mock"}}}).to_string())).await.unwrap();
        let mut requests = Vec::new();
        while let Some(Ok(message)) = socket.next().await {
            if let Some(text) = message.as_text() {
                let request: Value = serde_json::from_str(text).unwrap();
                let _ = seen_tx.send(request.clone());
                requests.push(request);
            }
        }
        requests
    });
    let (client, _events) = ObsClient::connect("127.0.0.1", port, None).await.unwrap();
    (client, seen_rx, server)
}

#[tokio::test]
async fn cancelled_requests_retire_admission_and_bound_unanswered_wire_work() {
    let (client, mut seen, server) = unresponsive_client().await;
    let mut tasks = Vec::new();
    for _ in 0..32 {
        let client = client.clone();
        tasks.push(tokio::spawn(async move { client.get_version().await }));
    }
    for _ in 0..MAX_CONCURRENT_OBS_REQUESTS {
        tokio::time::timeout(std::time::Duration::from_secs(2), seen.recv())
            .await
            .unwrap()
            .unwrap();
    }
    for task in &tasks {
        task.abort();
    }
    for task in tasks {
        let _ = task.await;
    }
    tokio::time::timeout(std::time::Duration::from_secs(2), client.wait_retired())
        .await
        .unwrap();
    for _ in 0..64 {
        assert!(client.get_version().await.is_err());
    }
    drop(client);
    let requests = tokio::time::timeout(std::time::Duration::from_secs(2), server)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(requests.len(), MAX_CONCURRENT_OBS_REQUESTS);
}

#[tokio::test]
async fn retiring_connection_prevents_queued_mutation_from_reaching_wire() {
    let (client, mut seen, server) = unresponsive_client().await;
    let mut tasks = Vec::new();
    for _ in 0..MAX_CONCURRENT_OBS_REQUESTS {
        let client = client.clone();
        tasks.push(tokio::spawn(async move { client.get_version().await }));
    }
    for _ in 0..MAX_CONCURRENT_OBS_REQUESTS {
        tokio::time::timeout(std::time::Duration::from_secs(2), seen.recv())
            .await
            .unwrap()
            .unwrap();
    }
    let command = {
        let client = client.clone();
        tokio::spawn(async move { client.set_current_program_scene("Must not send").await })
    };
    let retirement = {
        let client = client.clone();
        tokio::spawn(async move { client.wait_retired().await })
    };
    client.retire();
    retirement.await.unwrap();
    assert!(command.await.unwrap().is_err());
    for task in tasks {
        assert!(task.await.unwrap().is_err());
    }
    drop(client);
    let requests = tokio::time::timeout(std::time::Duration::from_secs(2), server)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(requests.len(), MAX_CONCURRENT_OBS_REQUESTS);
    assert!(requests
        .iter()
        .all(|r| r["d"]["requestType"] == "GetVersion"));
}

#[tokio::test]
async fn response_deadline_retires_admission_instead_of_accumulating_registrations() {
    let (client, _seen, server) = unresponsive_client().await;
    let error = tokio::time::timeout(std::time::Duration::from_secs(12), client.get_version())
        .await
        .unwrap()
        .unwrap_err();
    assert!(error.to_string().contains("timeout"));
    for _ in 0..16 {
        assert!(client.get_version().await.is_err());
    }
    drop(client);
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(2), server)
            .await
            .unwrap()
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn cancelled_optional_image_lane_is_bounded_without_retiring_controls() {
    let (client, mut seen, server) = unresponsive_client().await;
    let image = {
        let client = client.clone();
        tokio::spawn(async move { client.get_thumbnail_collection().await })
    };
    let frame = tokio::time::timeout(std::time::Duration::from_secs(2), seen.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(frame["d"]["requestType"], "GetSceneCollectionList");
    image.abort();
    assert!(image.await.unwrap_err().is_cancelled());
    for _ in 0..64 {
        assert!(client.get_thumbnail_collection().await.is_err());
    }
    assert!(client.retirement_reason().is_none());
    // Control admission still reaches the actual socket, not a fake gate.
    let control = {
        let client = client.clone();
        tokio::spawn(async move { client.get_version().await })
    };
    let frame = tokio::time::timeout(std::time::Duration::from_secs(2), seen.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(frame["d"]["requestType"], "GetVersion");
    client.retire();
    assert!(control.await.unwrap().is_err());
    drop(client);
    let frames = tokio::time::timeout(std::time::Duration::from_secs(2), server)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(frames.len(), 2);
}

#[tokio::test]
async fn graceful_shutdown_handle_is_issued_once_and_never_reopens_controls() {
    let (client, server) = mock_client(vec![("GetRecordStatus", json!({"outputActive":false,"outputPaused":false,"outputTimecode":"00:00:00.000","outputDuration":0,"outputBytes":0}))]).await;
    client.begin_disconnect();
    let shutdown = client.disconnect_client().await.unwrap();
    assert!(client.same_connection(&shutdown));
    assert!(client.disconnect_client().await.is_err());
    shutdown.get_record_status().await.unwrap();
    // Retirement releases parked ordinary callers without polling a mutation.
    client.retire();
    assert!(client
        .set_current_program_scene("Must not send")
        .await
        .is_err());
    assert_eq!(server.await.unwrap().len(), 2);
}

#[tokio::test]
async fn ordinary_server_rejection_keeps_connection_admission_usable() {
    let (client, server) = mock_client(vec![
        (
            "SetCurrentProgramScene",
            json!({"__error":true,"__code":600}),
        ),
        ("SetCurrentProgramScene", json!({})),
    ])
    .await;
    assert!(client.set_current_program_scene("Missing").await.is_err());
    client.set_current_program_scene("Program").await.unwrap();
    assert_eq!(server.await.unwrap().len(), 3);
}

#[tokio::test]
async fn configured_fps_is_observed_separately_from_measured_jitter() {
    for (numerator, denominator, expected) in [
        (30, 1, Some(30.0)),
        (60000, 1001, Some(60000.0 / 1001.0)),
        (60, 1, Some(60.0)),
        (120, 1, Some(120.0)),
        (144, 1, Some(144.0)),
        (0, 1, None),
        (60, 0, None),
    ] {
        let (client,server)=mock_client(vec![
            ("GetStats",json!({"cpuUsage":1.0,"memoryUsage":2.0,"availableDiskSpace":3.0,"activeFps":60.01,"averageFrameRenderTime":1.0,"renderSkippedFrames":0,"renderTotalFrames":100,"outputSkippedFrames":0,"outputTotalFrames":100,"webSocketSessionIncomingMessages":0,"webSocketSessionOutgoingMessages":0})),
            ("GetVideoSettings",json!({"fpsNumerator":numerator,"fpsDenominator":denominator,"baseWidth":1920,"baseHeight":1080,"outputWidth":1920,"outputHeight":1080})),
        ]).await;
        let stats = client.get_obs_stats().await.unwrap();
        assert_eq!(stats.configured_fps, expected);
        assert_eq!(stats.active_fps, 60.01);
        assert_eq!(server.await.unwrap().len(), 3);
    }
}

#[tokio::test]
async fn partial_audio_failure_is_explicit_and_drains_other_admitted_reads() {
    let (client, server) = mock_client(vec![
        (
            "GetInputList",
            json!({"inputs":[input("Good"),input("Failed")]}),
        ),
        ("GetInputMute", json!({"inputMuted":false})),
        ("GetInputMute", json!({"__error":true,"__code":702})),
        (
            "GetInputVolume",
            json!({"inputVolumeMul":1.0,"inputVolumeDb":0.0}),
        ),
        ("SetCurrentProgramScene", json!({})),
    ])
    .await;
    assert!(client
        .get_all_audio_inputs()
        .await
        .unwrap_err()
        .to_string()
        .contains("Failed mute unavailable"));
    client.set_current_program_scene("Program").await.unwrap();
    assert_eq!(server.await.unwrap().len(), 6);
}

#[tokio::test]
async fn failure_warning_samples_are_bounded_without_losing_omitted_count() {
    let items: Vec<_> = (0..40).map(|id| scene_item(id, "Failed")).collect();
    let mut script = vec![
        ("GetInputList", json!({"inputs":[]})),
        ("GetSceneItemList", json!({"sceneItems":items})),
    ];
    script.extend((0..40).map(|_| ("GetSceneItemEnabled", json!({"__error":true,"__code":702}))));
    let (client, server) = mock_client(script).await;
    let (items, warnings) = client.scan_source_items("Program", 40).await;
    assert!(items.is_empty());
    assert_eq!(warnings.len(), 33);
    assert_eq!(
        warnings.last().unwrap(),
        "8 additional source scan warnings omitted"
    );
    assert_eq!(server.await.unwrap().len(), 43);
}
