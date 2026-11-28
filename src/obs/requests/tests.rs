use super::MAX_CONCURRENT_REQUESTS;
use crate::domain::control::ControlAction;
use crate::obs::testing::scripted::{mock_client, unresponsive_client, ScriptedObs};
use crate::obs::testing::stateful::{Request, StatefulObs, When};
use crate::obs::testing::wire::Reply;
use serde_json::json;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::oneshot;

#[tokio::test]
async fn cancelled_requests_retire_admission_and_bound_unanswered_wire_work() {
    let (client, mut seen, server) = unresponsive_client().await;
    let mut tasks = Vec::new();
    for _ in 0..32 {
        let client = client.clone();
        tasks.push(tokio::spawn(async move { client.get_version().await }));
    }
    for _ in 0..MAX_CONCURRENT_REQUESTS {
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
    assert_eq!(requests.len(), MAX_CONCURRENT_REQUESTS);
}

#[tokio::test]
async fn retiring_connection_prevents_queued_mutation_from_reaching_wire() {
    let (client, mut seen, server) = unresponsive_client().await;
    let mut tasks = Vec::new();
    for _ in 0..MAX_CONCURRENT_REQUESTS {
        let client = client.clone();
        tasks.push(tokio::spawn(async move { client.get_version().await }));
    }
    for _ in 0..MAX_CONCURRENT_REQUESTS {
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
    assert_eq!(requests.len(), MAX_CONCURRENT_REQUESTS);
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
    let (client, server) = ScriptedObs::ordered(vec![
        ("SetCurrentProgramScene", Reply::err(600)),
        ("SetCurrentProgramScene", Reply::Ok(json!({}))),
    ])
    .connect()
    .await;
    assert!(client.set_current_program_scene("Missing").await.is_err());
    client.set_current_program_scene("Program").await.unwrap();
    assert_eq!(server.await.unwrap().len(), 3);
}

#[tokio::test]
async fn cancellation_while_waiting_for_connection_admission_does_not_poll_a_mutation() {
    let mut mock = StatefulObs::default()
        .hold(Request::GetVersion, When::AfterFirst)
        .connect()
        .await;
    let mut reads = Vec::new();
    for _ in 0..8 {
        let client = mock.client.clone();
        reads.push(tokio::spawn(async move { client.get_version().await }));
    }
    mock.wait(Request::GetVersion, 9).await;
    let valid = Arc::new(AtomicBool::new(true));
    let lease = valid.clone();
    let client = mock.client.clone();
    let (polled, queued) = oneshot::channel();
    let mutation = tokio::spawn(async move {
        let action = ControlAction::SetInputMuted {
            input: "MicA".into(),
            muted: true,
        };
        let check = || lease.load(Ordering::Acquire);
        let write = client.execute_control_if(&action, &check);
        tokio::pin!(write);
        let mut polled = Some(polled);
        std::future::poll_fn(|context| {
            if let Some(polled) = polled.take() {
                let _ = polled.send(());
            }
            write.as_mut().poll(context)
        })
        .await
    });
    queued.await.unwrap();
    valid.store(false, Ordering::Release);
    mock.release_all().await;
    for read in reads {
        read.await.unwrap().unwrap();
    }
    assert!(mutation.await.unwrap().is_err());
    assert!(
        mock.client.retirement_reason().is_none(),
        "a skipped, unpolled request does not poison admission"
    );
    let frames = mock.stop().await;
    assert_eq!(frames.count(Request::GetVersion), 9);
    assert_eq!(frames.count(Request::SetInputMute), 0);
}
