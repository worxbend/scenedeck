//! Loopback protocol fixtures for workspace lifecycle invariants.

use super::*;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::sync::{oneshot, Notify};
use tokio_websockets::{Message, ServerBuilder};

struct MockConnection {
    client: ObsClient,
    _events: crate::obs::event::ObsEventStream,
    entered: Arc<Notify>,
    release: Arc<Notify>,
    finish: oneshot::Sender<()>,
    emit: oneshot::Sender<()>,
    server: JoinHandle<Vec<String>>,
}

async fn mock_connection(held_request: &'static str) -> MockConnection {
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let (finish, mut finished) = oneshot::channel();
    let (emit, mut emission) = oneshot::channel();
    let started = entered.clone();
    let resume = release.clone();
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
        let mut held = None;
        let mut held_once = false;
        let mut emitted = false;
        let mut requests = Vec::new();
        loop {
            let message = tokio::select! {
                _ = &mut finished => break,
                _ = &mut emission, if !emitted => {
                    emitted = true;
                    socket.send(Message::text(json!({"op":5,"d":{"eventType":"InputMuteStateChanged","eventIntent":8,"eventData":{"inputName":"Mic","inputUuid":"00000000-0000-0000-0000-000000000001","inputMuted":true}}}).to_string())).await.unwrap();
                    continue;
                }
                _ = resume.notified(), if held.is_some() => {
                    socket.send(Message::text(held.take().unwrap())).await.unwrap();
                    continue;
                }
                message = socket.next() => message.unwrap().unwrap(),
            };
            let request: Value = serde_json::from_str(message.as_text().unwrap()).unwrap();
            let name = request["d"]["requestType"].as_str().unwrap();
            requests.push(name.to_owned());
            let response = match name {
                "GetVersion" => Some(
                    json!({"obsStudioVersion":"31.0.0","obsWebSocketVersion":"5.5.0","rpcVersion":1,"availableRequests":[],"supportedImageFormats":[],"platform":"mock","platformDescription":"mock"}),
                ),
                "GetSceneCollectionList" => {
                    Some(json!({"currentSceneCollectionName":"Show","sceneCollections":["Show"]}))
                }
                "GetSceneList" => Some(json!({"currentProgramSceneName":"Program","scenes":[]})),
                "GetInputList" => Some(json!({"inputs":[]})),
                "SetSceneItemEnabled" => Some(json!({})),
                _ => None,
            };
            let mut reply = json!({"op":7,"d":{"requestId":request["d"]["requestId"],"requestType":name,"requestStatus":{"result":response.is_some(),"code":if response.is_some() {100} else {204}}}});
            if let Some(response) = response {
                if response != json!({}) {
                    reply["d"]["responseData"] = response;
                }
            }
            if name == held_request && !held_once {
                held_once = true;
                held = Some(reply.to_string());
                started.notify_one();
            } else {
                socket.send(Message::text(reply.to_string())).await.unwrap();
            }
        }
        requests
    });
    let (client, events) = ObsClient::connect("127.0.0.1", port, None).await.unwrap();
    MockConnection {
        client,
        _events: events,
        entered,
        release,
        finish,
        emit,
        server,
    }
}

fn request(client: &ObsClient, sequence: u64) -> WorkspaceRequest {
    WorkspaceRequest {
        epoch: 5,
        sequence,
        scope: ScopeBinding {
            endpoint: client.endpoint().into(),
            collection: "Show".into(),
        },
        scene: None,
        input: None,
        include_sources: false,
        screenshot: false,
        result_id: Some(11),
    }
}

#[tokio::test]
async fn bootstrap_scan_does_not_stop_the_obs_event_pump() {
    let mock = mock_connection("GetSceneList").await;
    let (tx, rx) = std::sync::mpsc::sync_channel(8);
    let pump = tokio::spawn(crate::controller::refresh_controller::run_event_loop(
        mock.client.clone(),
        mock._events,
        crate::controller::event_sender::EventSender::new(tx),
        Vec::new(),
        Arc::new(Mutex::new(None)),
    ));
    tokio::time::timeout(Duration::from_secs(2), mock.entered.notified())
        .await
        .unwrap();
    mock.emit.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match rx.try_recv() {
                Ok(AppEvent::Observation { event, .. }) if matches!(*event, AppEvent::InputMuteChanged { ref input, muted: true } if input == "Mic") => break,
                Ok(AppEvent::Disconnected | AppEvent::Error(_)) => {
                    panic!("held bootstrap must not disconnect a healthy session")
                }
                _ => tokio::task::yield_now().await,
            }
        }
    })
    .await
    .expect("event pump must run while bootstrap response is held");
    mock.client.get_version().await.unwrap();
    mock.release.notify_one();
    mock.client.retire();
    tokio::time::timeout(Duration::from_secs(2), pump)
        .await
        .unwrap()
        .unwrap();
    mock.finish.send(()).unwrap();
    mock.server.await.unwrap();
}

#[tokio::test]
async fn profile_collection_rejections_and_profile_readback_errors_are_localized_events() {
    let mock = mock_connection("NeverHeld").await;
    let (tx, rx) = std::sync::mpsc::sync_channel(8);
    let sender = crate::controller::event_sender::EventSender::new(tx);
    crate::controller::refresh_controller::publish_profiles_after(
        Err(crate::infra::error::AppError::Request(
            "profile rejected".into(),
        )),
        &mock.client,
        &sender,
    )
    .await;
    crate::controller::refresh_controller::refresh_after_collection_change(
        Err(crate::infra::error::AppError::Request(
            "collection rejected".into(),
        )),
        &mock.client,
        &sender,
        &[],
    )
    .await;
    crate::controller::refresh_controller::publish_profiles_after(Ok(()), &mock.client, &sender)
        .await;
    use crate::controller::event::FailedOperation;
    for expected in [
        FailedOperation::Profile,
        FailedOperation::Collection,
        FailedOperation::ProfileReadback,
    ] {
        assert!(
            matches!(rx.try_recv(), Ok(AppEvent::CommandFailed { operation, .. }) if operation == expected)
        );
    }
    assert!(
        rx.try_recv().is_err(),
        "command rejection must not emit connection Error/Disconnected"
    );
    mock.client
        .get_version()
        .await
        .expect("local failure preserves the connection");
    mock.finish.send(()).unwrap();
    assert_eq!(
        mock.server.await.unwrap(),
        ["GetVersion", "GetProfileList", "GetVersion"]
    );
}

#[tokio::test]
async fn replacing_many_reads_drains_one_transaction_then_runs_only_the_latest_request() {
    let mock = mock_connection("GetSceneCollectionList").await;
    let (tx, rx) = std::sync::mpsc::sync_channel(8);
    let mut workspace = WorkspaceController::new(
        Handle::current(),
        tx,
        Arc::new(Mutex::new(Some(mock.client.clone()))),
    );
    workspace.request(request(&mock.client, 1), None, None);
    if tokio::time::timeout(Duration::from_secs(1), mock.entered.notified())
        .await
        .is_err()
    {
        panic!("request did not reach gated read: {:?}", rx.try_recv());
    }
    for sequence in 2..=129 {
        workspace.request(request(&mock.client, sequence), None, None);
    }
    mock.release.notify_one();
    tokio::time::timeout(Duration::from_secs(2), workspace.task.take().unwrap())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        rx.try_recv(),
        Ok(AppEvent::WorkspaceLoaded {
            sequence: 129,
            result: Ok(_),
            ..
        })
    ));
    assert!(
        rx.try_recv().is_err(),
        "no superseded completion may escape"
    );
    mock.client
        .get_version()
        .await
        .expect("coalescing must not retire the active socket");
    mock.finish.send(()).unwrap();
    let requests = mock.server.await.unwrap();
    assert_eq!(
        requests
            .iter()
            .filter(|name| *name == "GetSceneList")
            .count(),
        1
    );
    assert_eq!(
        requests
            .iter()
            .filter(|name| *name == "GetSceneCollectionList")
            .count(),
        3
    );
}

#[tokio::test]
async fn acknowledged_write_is_published_before_gated_readback_and_deadline_never_replays_it() {
    let mock = mock_connection("GetSceneList").await;
    let (tx, rx) = std::sync::mpsc::sync_channel(8);
    let mut workspace = WorkspaceController::new(
        Handle::current(),
        tx,
        Arc::new(Mutex::new(Some(mock.client.clone()))),
    );
    workspace.request(
        request(&mock.client, 1),
        Some(ControlAction::SetItemEnabled {
            owner: "Program".into(),
            item_id: 1,
            enabled: true,
        }),
        None,
    );
    if tokio::time::timeout(Duration::from_secs(1), mock.entered.notified())
        .await
        .is_err()
    {
        panic!("request did not reach gated read: {:?}", rx.try_recv());
    }
    assert!(matches!(
        rx.try_recv(),
        Ok(AppEvent::WorkspaceAcknowledged {
            sequence: 1,
            result_id: Some(11),
            ..
        })
    ));
    assert!(
        rx.try_recv().is_err(),
        "readback is still gated after the acknowledgement"
    );
    tokio::time::timeout(
        crate::controller::session_controller::REQUEST_BUDGET + Duration::from_secs(2),
        workspace.task.take().unwrap(),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(matches!(
        rx.try_recv(),
        Ok(AppEvent::WorkspaceLoaded {
            acknowledged: true,
            result: Err(_),
            ..
        })
    ));
    tokio::time::timeout(Duration::from_secs(1), mock.client.wait_retired())
        .await
        .expect("deadline must retire admission, not accumulate abandoned requests");
    mock.finish.send(()).unwrap();
    let requests = mock.server.await.unwrap();
    assert_eq!(
        requests
            .iter()
            .filter(|name| *name == "SetSceneItemEnabled")
            .count(),
        1
    );
    assert_eq!(
        requests
            .iter()
            .filter(|name| *name == "GetSceneList")
            .count(),
        1
    );
}
