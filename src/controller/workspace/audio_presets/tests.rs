use super::*;
use crate::{domain::operator_action::ActionTarget, obs::client::control_tests::mock_client};
use serde_json::{json, Value};

const UUID: &str = "00000000-0000-0000-0000-000000000001";
fn setting(muted: bool, gain: f64) -> PresetInput {
    PresetInput {
        target: ActionTarget {
            name: "Old microphone name".into(),
            uuid: Some(UUID.into()),
        },
        muted,
        gain: PresetGain::try_from(gain).unwrap(),
    }
}
fn collection() -> (&'static str, Value) {
    (
        "GetSceneCollectionList",
        json!({"currentSceneCollectionName":"Show","sceneCollections":["Show"]}),
    )
}
fn catalog(uuid: &str) -> (&'static str, Value) {
    (
        "GetInputList",
        json!({"inputs":[{"inputName":"Renamed microphone","inputUuid":uuid,"inputKind":"audio","unversionedInputKind":"audio"}]}),
    )
}
fn audio(muted: bool, gain: f64) -> Vec<(&'static str, Value)> {
    vec![
        ("GetInputMute", json!({"inputMuted":muted})),
        (
            "GetInputVolume",
            json!({"inputVolumeMul":gain,"inputVolumeDb":if gain == 0.0 { -100.0 } else {20.0 * gain.log10()}}),
        ),
    ]
}
fn initial() -> Vec<(&'static str, Value)> {
    let mut script = vec![collection(), catalog(UUID)];
    script.extend(audio(true, 1.0));
    script.push(collection());
    script
}
fn plan(client: &ObsClient) -> PresetPlan {
    PresetPlan {
        id: 1,
        epoch: 2,
        name: "Interview".into(),
        scope: ScopeBinding {
            endpoint: client.endpoint().into(),
            collection: "Show".into(),
        },
        reviewed: Instant::now(),
        inputs: vec![PlannedInput {
            desired: setting(false, 2.0),
            before: Some(setting(true, 1.0)),
        }],
    }
}
async fn run(
    script: Vec<(&'static str, Value)>,
    edit: impl FnOnce(&mut PresetPlan),
) -> (Vec<PresetOutcome>, Vec<Value>) {
    let (client, server) = mock_client(script).await;
    let (tx, rx) = std::sync::mpsc::sync_channel(128);
    let mut controller = WorkspaceController::new(
        Handle::current(),
        tx,
        Arc::new(Mutex::new(Some(client.clone()))),
    );
    let mut plan = plan(&client);
    edit(&mut plan);
    controller.preset(plan);
    tokio::time::timeout(
        std::time::Duration::from_secs(3),
        controller.preset_task.take().unwrap(),
    )
    .await
    .unwrap()
    .unwrap();
    let outcomes = rx
        .try_iter()
        .map(|event| match event {
            AppEvent::PresetProgress { outcome, .. } => outcome,
            other => panic!("unexpected event {other:?}"),
        })
        .collect();
    (outcomes, server.await.unwrap())
}

#[tokio::test]
async fn preset_gain_is_read_back_before_unmute_and_missing_inputs_are_reported() {
    let mut script = initial();
    script.extend([collection(), ("SetInputVolume", json!({}))]);
    script.extend(audio(true, 2.0));
    script.extend([collection(), collection(), ("SetInputMute", json!({}))]);
    script.extend(audio(false, 2.0));
    script.push(collection());
    let (outcomes, requests) = run(script, |plan| {
        let mut missing = setting(false, 0.5);
        missing.target.name = "Missing".into();
        missing.target.uuid = Some("00000000-0000-0000-0000-000000000002".into());
        plan.inputs.push(PlannedInput {
            desired: missing,
            before: None,
        });
    })
    .await;
    let confirmed = outcomes
        .iter()
        .find(|row| row.phase == PresetPhase::Observed)
        .unwrap();
    assert!(confirmed.gain_ack && confirmed.mute_ack);
    assert_eq!(
        confirmed.observed.as_ref().unwrap().target.name,
        "Renamed microphone"
    );
    assert_eq!(outcomes.last().unwrap().phase, PresetPhase::Missing);
    let mutations: Vec<_> = requests
        .iter()
        .filter(|request| {
            request["d"]["requestType"]
                .as_str()
                .unwrap()
                .starts_with("SetInput")
        })
        .collect();
    assert_eq!(mutations.len(), 2);
    assert_eq!(mutations[0]["d"]["requestData"]["inputVolumeMul"], 2.0);
    assert_eq!(mutations[1]["d"]["requestData"]["inputMuted"], false);
    for request in mutations {
        assert_eq!(request["d"]["requestData"]["inputUuid"], UUID);
        assert!(request["d"]["requestData"].get("inputName").is_none());
    }
    assert!(outcomes
        .iter()
        .any(|row| row.phase == PresetPhase::Running && row.gain_ack && !row.mute_ack));
}

#[tokio::test]
async fn preset_failure_changed_values_and_reused_names_never_unmute() {
    let mut failed = initial();
    failed.extend([
        collection(),
        ("SetInputVolume", json!({"__error":true,"__code":702})),
    ]);
    let (outcomes, requests) = run(failed, |_| {}).await;
    assert_eq!(outcomes.last().unwrap().phase, PresetPhase::Failed);
    assert!(!outcomes.last().unwrap().gain_ack);
    assert!(requests
        .iter()
        .all(|request| request["d"]["requestType"] != "SetInputMute"));
    let mut no_readback = initial();
    no_readback.extend([
        collection(),
        ("SetInputVolume", json!({})),
        ("GetInputMute", json!({"__error":true})),
    ]);
    let (outcomes, requests) = run(no_readback, |_| {}).await;
    assert_eq!(outcomes.last().unwrap().phase, PresetPhase::Unconfirmed);
    assert!(outcomes.last().unwrap().gain_ack && !outcomes.last().unwrap().mute_ack);
    assert!(requests
        .iter()
        .all(|request| request["d"]["requestType"] != "SetInputMute"));
    let (outcomes, requests) = run(initial(), |plan| {
        plan.inputs[0].before.as_mut().unwrap().muted = false
    })
    .await;
    assert_eq!(outcomes.last().unwrap().phase, PresetPhase::Changed);
    assert!(requests.iter().all(|request| !request["d"]["requestType"]
        .as_str()
        .unwrap()
        .starts_with("Set")));
    let (outcomes, requests) = run(
        vec![
            collection(),
            catalog("00000000-0000-0000-0000-000000000002"),
        ],
        |_| {},
    )
    .await;
    assert_eq!(outcomes.last().unwrap().phase, PresetPhase::Missing);
    assert_eq!(requests.len(), 3);
}

#[tokio::test]
async fn a_failed_input_does_not_hide_another_inputs_confirmed_result() {
    let mut script = initial();
    script.extend([
        collection(),
        ("SetInputVolume", json!({"__error":true,"__code":702})),
        collection(),
        catalog("00000000-0000-0000-0000-000000000002"),
    ]);
    script.extend(audio(false, 1.0));
    script.extend([collection(), collection(), ("SetInputMute", json!({}))]);
    script.extend(audio(true, 1.0));
    script.push(collection());
    let (outcomes, requests) = run(script, |plan| {
        let mut before = setting(false, 1.0);
        before.target.name = "Other input".into();
        before.target.uuid = Some("00000000-0000-0000-0000-000000000002".into());
        let desired = PresetInput {
            muted: true,
            ..before.clone()
        };
        plan.inputs.push(PlannedInput {
            desired,
            before: Some(before),
        });
    })
    .await;
    assert!(outcomes
        .iter()
        .any(|outcome| outcome.phase == PresetPhase::Failed));
    assert_eq!(outcomes.last().unwrap().phase, PresetPhase::Observed);
    assert!(outcomes.last().unwrap().mute_ack && !outcomes.last().unwrap().gain_ack);
    assert_eq!(
        requests
            .iter()
            .filter(|r| r["d"]["requestType"] == "SetInputMute")
            .count(),
        1
    );
}

struct Gated {
    client: ObsClient,
    server: tokio::task::JoinHandle<Vec<Value>>,
    started: tokio::sync::oneshot::Receiver<()>,
    release: tokio::sync::oneshot::Sender<()>,
}
async fn gated(script: Vec<(&'static str, Value)>, gate: usize) -> Gated {
    use futures_util::{SinkExt, StreamExt};
    use tokio_websockets::Message;
    let (listener, port) = crate::obs::obs_fixture::listen().await;
    let (started, observed) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let mut socket = crate::obs::obs_fixture::accept(listener).await;
        let mut started = Some(started);
        let mut released = Some(released);
        let mut requests = Vec::new();
        for (index, (expected, data)) in
            std::iter::once(("GetVersion", crate::obs::obs_fixture::version(&[])))
                .chain(script)
                .enumerate()
        {
            let request: Value =
                serde_json::from_str(socket.next().await.unwrap().unwrap().as_text().unwrap())
                    .unwrap();
            assert_eq!(request["d"]["requestType"], expected);
            if index == gate + 1 {
                started.take().unwrap().send(()).unwrap();
                released.take().unwrap().await.unwrap();
            }
            let mut response = json!({"op":7,"d":{"requestId":request["d"]["requestId"],"requestType":expected,"requestStatus":{"result":true,"code":100}}});
            if data != json!({}) {
                response["d"]["responseData"] = data;
            }
            socket
                .send(Message::text(response.to_string()))
                .await
                .unwrap();
            requests.push(request);
        }
        requests
    });
    let (client, _events) = ObsClient::connect("127.0.0.1", port, None).await.unwrap();
    Gated {
        client,
        server,
        started: observed,
        release,
    }
}

#[tokio::test]
async fn cancellation_and_collection_changes_fence_mutations_before_and_after_gain_ack() {
    for after_gain in [false, true] {
        for collection_change in [false, true] {
            let (script, gate) = if after_gain {
                let mut script = initial();
                script.extend([collection(), ("SetInputVolume", json!({}))]);
                let gate = script.len();
                script.extend(audio(true, 2.0));
                script.push(collection());
                (script, gate)
            } else {
                (vec![collection()], 0)
            };
            let mock = tokio::time::timeout(std::time::Duration::from_secs(3), gated(script, gate))
                .await
                .expect("gated connection deadline");
            let (tx, rx) = std::sync::mpsc::sync_channel(128);
            let mut controller = WorkspaceController::new(
                Handle::current(),
                tx,
                Arc::new(Mutex::new(Some(mock.client.clone()))),
            );
            controller.preset(plan(&mock.client));
            assert!(controller.preset_task.is_some(), "preset was not admitted");
            tokio::time::timeout(std::time::Duration::from_secs(3), mock.started)
                .await
                .expect("request did not reach gate")
                .unwrap();
            if collection_change {
                controller.scope.advance_collection();
            } else {
                controller.cancel_preset();
            }
            mock.release.send(()).unwrap();
            tokio::time::timeout(
                std::time::Duration::from_secs(3),
                controller.preset_task.take().unwrap(),
            )
            .await
            .unwrap()
            .unwrap();
            let requests = tokio::time::timeout(std::time::Duration::from_secs(3), mock.server)
                .await
                .expect("server script did not finish")
                .unwrap();
            assert!(requests
                .iter()
                .all(|request| request["d"]["requestType"] != "SetInputMute"));
            assert_eq!(
                requests
                    .iter()
                    .filter(|request| request["d"]["requestType"] == "SetInputVolume")
                    .count(),
                usize::from(after_gain)
            );
            let outcomes: Vec<_> = rx
                .try_iter()
                .filter_map(|event| {
                    if let AppEvent::PresetProgress { outcome, .. } = event {
                        Some(outcome)
                    } else {
                        None
                    }
                })
                .collect();
            assert!(outcomes
                .iter()
                .all(|outcome| outcome.phase != PresetPhase::Observed));
            if after_gain {
                assert!(outcomes.iter().any(|outcome| outcome.gain_ack));
            }
            if !collection_change {
                assert_eq!(
                    outcomes.last().unwrap().phase,
                    if after_gain {
                        PresetPhase::Unconfirmed
                    } else {
                        PresetPhase::Cancelled
                    }
                );
            }
        }
    }
}

#[tokio::test]
async fn readback_timeout_keeps_gain_ack_and_cancels_unsent_inputs_without_replay() {
    let mut script = initial();
    script.extend([collection(), ("SetInputVolume", json!({}))]);
    let gate = script.len();
    script.push(("GetInputMute", json!({"inputMuted":true})));
    let mock = gated(script, gate).await;
    let (tx, rx) = std::sync::mpsc::sync_channel(128);
    let mut controller = WorkspaceController::new(
        Handle::current(),
        tx,
        Arc::new(Mutex::new(Some(mock.client.clone()))),
    );
    let mut plan = plan(&mock.client);
    let mut next = plan.inputs[0].clone();
    next.desired.target.name = "Next".into();
    next.desired.target.uuid = Some("00000000-0000-0000-0000-000000000002".into());
    next.before.as_mut().unwrap().target = next.desired.target.clone();
    plan.inputs.push(next);
    controller.preset(plan);
    tokio::time::timeout(std::time::Duration::from_secs(3), mock.started)
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(
        super::super::super::session_controller::REQUEST_BUDGET + std::time::Duration::from_secs(3),
        controller.preset_task.take().unwrap(),
    )
    .await
    .unwrap()
    .unwrap();
    let outcomes: Vec<_> = rx
        .try_iter()
        .filter_map(|event| {
            if let AppEvent::PresetProgress { index, outcome, .. } = event {
                Some((index, outcome))
            } else {
                None
            }
        })
        .collect();
    let first = outcomes
        .iter()
        .rev()
        .find(|(index, _)| *index == 0)
        .unwrap();
    assert_eq!(first.1.phase, PresetPhase::Unconfirmed);
    assert!(first.1.gain_ack && !first.1.mute_ack);
    assert_eq!(outcomes.last().unwrap().1.phase, PresetPhase::Cancelled);
    assert!(mock.client.retirement_reason().is_some());
    mock.release.send(()).unwrap();
    let requests = tokio::time::timeout(std::time::Duration::from_secs(3), mock.server)
        .await
        .unwrap()
        .unwrap();
    assert!(requests
        .iter()
        .all(|request| request["d"]["requestType"] != "SetInputMute"));
    assert_eq!(
        requests
            .iter()
            .filter(|request| request["d"]["requestType"] == "SetInputVolume")
            .count(),
        1
    );
}

#[tokio::test]
async fn all_64_selected_inputs_receive_results_without_the_single_input_task_limit() {
    let mut script = Vec::new();
    let mut inputs = Vec::new();
    for index in 1..=INPUT_LIMIT {
        let uuid = format!("00000000-0000-0000-0000-{index:012}");
        script.extend([collection(), catalog(&uuid)]);
        script.extend(audio(true, 1.0));
        script.push(collection());
        let mut input = setting(true, 1.0);
        input.target.name = format!("Input {index}");
        input.target.uuid = Some(uuid);
        inputs.push(PlannedInput {
            desired: input.clone(),
            before: Some(input),
        });
    }
    let (outcomes, requests) = run(script, |plan| plan.inputs = inputs).await;
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| outcome.phase == PresetPhase::Unchanged)
            .count(),
        INPUT_LIMIT
    );
    assert_eq!(requests.len(), 1 + INPUT_LIMIT * 5);
    assert!(requests.iter().all(|request| !request["d"]["requestType"]
        .as_str()
        .unwrap()
        .starts_with("Set")));
}
