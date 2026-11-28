use crate::obs::testing::scripted::{mock_client, ScriptedObs};
use crate::obs::testing::wire::{input, scene_item, Reply};
use serde_json::json;

#[tokio::test]
async fn definite_non_audio_is_omitted_but_complete_audio_is_retained() {
    let (client, server) = ScriptedObs::ordered(vec![
        (
            "GetInputList",
            Reply::Ok(json!({"inputs":[input("Image"),input("Mic")]})),
        ),
        (
            "GetInputMute",
            Reply::Err {
                code: 604,
                comment: "The specified input does not support audio.".into(),
            },
        ),
        ("GetInputMute", Reply::Ok(json!({"inputMuted":false}))),
        (
            "GetInputVolume",
            Reply::Ok(json!({"inputVolumeMul":0.5,"inputVolumeDb":-6.0206})),
        ),
    ])
    .connect()
    .await;
    let inputs = client.get_all_audio_inputs().await.unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].id, "Mic");
    assert_eq!(
        inputs[0].uuid.as_deref(),
        Some("00000000-0000-0000-0000-000000000001")
    );
    assert!(!inputs[0].muted);
    assert_eq!(inputs[0].volume_mul, 0.5);
    assert_eq!(server.await.unwrap().len(), 5);
}

#[tokio::test]
async fn transient_and_ambiguous_audio_rejections_never_become_empty_success() {
    for code in [600, 604, 702] {
        let (client, server) = ScriptedObs::ordered(vec![
            (
                "GetInputList",
                Reply::Ok(json!({"inputs":[input("Failed mic")]})),
            ),
            ("GetInputMute", Reply::err(code)),
        ])
        .connect()
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
    let (client, server) = ScriptedObs::ordered(vec![
        ("GetInputList", Reply::Ok(json!({"inputs":[input("Mic")]}))),
        ("GetInputMute", Reply::Ok(json!({"inputMuted":true}))),
        ("GetInputVolume", Reply::err(702)),
    ])
    .connect()
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
    let (client, server) = ScriptedObs::ordered(vec![
        ("GetSpecialInputs", Reply::Ok(json!({"desktop1":null,"desktop2":null,"mic1":null,"mic2":null,"mic3":null,"mic4":null}))),
        (
            "GetSceneItemList",
            Reply::Ok(json!({"sceneItems":[scene_item(1,"Mic")]})),
        ),
        ("GetSceneItemEnabled", Reply::err(702)),
    ])
    .connect()
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
async fn partial_audio_failure_is_explicit_and_drains_other_admitted_reads() {
    let (client, server) = ScriptedObs::ordered(vec![
        (
            "GetInputList",
            Reply::Ok(json!({"inputs":[input("Good"),input("Failed")]})),
        ),
        ("GetInputMute", Reply::Ok(json!({"inputMuted":false}))),
        ("GetInputMute", Reply::err(702)),
        (
            "GetInputVolume",
            Reply::Ok(json!({"inputVolumeMul":1.0,"inputVolumeDb":0.0})),
        ),
        ("SetCurrentProgramScene", Reply::Ok(json!({}))),
    ])
    .connect()
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
async fn audio_nil_identity_stays_unknown_without_an_extra_read() {
    let mut microphone = input("Mic");
    microphone["inputUuid"] = json!("00000000-0000-0000-0000-000000000000");
    let (client, server) = mock_client(vec![
        ("GetInputList", json!({"inputs":[microphone]})),
        ("GetInputMute", json!({"inputMuted":false})),
        (
            "GetInputVolume",
            json!({"inputVolumeMul":1.0,"inputVolumeDb":0.0}),
        ),
    ])
    .await;
    let inputs = client.get_all_audio_inputs().await.unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].uuid, None);
    assert_eq!(server.await.unwrap().len(), 4);
}
