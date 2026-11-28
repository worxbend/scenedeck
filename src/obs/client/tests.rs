use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;

use super::{collect_bounded_ordered, ObsClient};
use crate::obs::event::ObsEvent;
use crate::obs::testing::wire::{self, Reply};

#[tokio::test]
async fn connect_negotiates_obs_websocket_and_requests_meter_events() {
    let (listener, port) = wire::listen().await;
    let (send_event, event_requested) = tokio::sync::oneshot::channel();
    let (release_server, hold_server) = tokio::sync::oneshot::channel();

    let server = tokio::spawn(async move {
        let (mut websocket, identify) = wire::accept_with_identify(listener).await;

        let version_request = wire::read_json(&mut websocket).await;
        assert_eq!(version_request["op"], 6);
        assert_eq!(version_request["d"]["requestType"], "GetVersion");
        let version = Reply::Ok(wire::version(&[]));
        wire::send_json(&mut websocket, &wire::response(&version_request, &version)).await;

        event_requested.await.expect("request mock OBS event");
        let scene_changed = wire::event(
            "CurrentProgramSceneChanged",
            obws::requests::EventSubscription::SCENES.bits(),
            json!({
                "sceneName": "Program",
                "sceneUuid": "00000000-0000-0000-0000-000000000001"
            }),
        );
        wire::send_json(&mut websocket, &scene_changed).await;

        let _ = hold_server.await;
        identify
    });

    let (client, mut events) = tokio::time::timeout(
        Duration::from_secs(2),
        ObsClient::connect("127.0.0.1", port, None),
    )
    .await
    .expect("OBS handshake timed out")
    .expect("OBS handshake failed");

    send_event.send(()).expect("request mock OBS event");
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(2), events.next())
            .await
            .expect("OBS event timed out"),
        Some(ObsEvent::CurrentProgramSceneChanged("Program".to_string()))
    );

    release_server.send(()).expect("release mock OBS server");
    let identify = server.await.expect("mock OBS task panicked");
    assert_eq!(identify["op"], 1);
    assert_eq!(identify["d"]["rpcVersion"], 1);
    let subscriptions = identify["d"]["eventSubscriptions"]
        .as_u64()
        .expect("Identify includes numeric event subscriptions");
    assert_ne!(
        subscriptions & u64::from(obws::requests::EventSubscription::INPUT_VOLUME_METERS.bits()),
        0,
        "volume-meter events must be requested during the handshake"
    );

    drop(events);
    drop(client);
}

#[tokio::test]
async fn bounded_collection_preserves_input_order_and_limit() {
    let active = Arc::new(AtomicUsize::new(0));
    let maximum = Arc::new(AtomicUsize::new(0));

    let values = collect_bounded_ordered(0..6, 2, |value| {
        let active = Arc::clone(&active);
        let maximum = Arc::clone(&maximum);
        async move {
            let now_active = active.fetch_add(1, Ordering::SeqCst) + 1;
            maximum.fetch_max(now_active, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis((6 - value) * 2)).await;
            active.fetch_sub(1, Ordering::SeqCst);
            Ok::<_, ()>(value)
        }
    })
    .await
    .unwrap();

    assert_eq!(values, vec![0, 1, 2, 3, 4, 5]);
    assert_eq!(maximum.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn bounded_collection_reports_the_first_error_in_input_order() {
    let error = collect_bounded_ordered(0..4, 4, |value| async move {
        tokio::time::sleep(Duration::from_millis(if value == 1 { 10 } else { 1 })).await;
        match value {
            1 => Err("first"),
            2 => Err("second"),
            _ => Ok(value),
        }
    })
    .await
    .unwrap_err();

    assert_eq!(error, "first");
}
