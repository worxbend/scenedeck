//! Independent bounded scene stills. A single task never blocks workspace mutations.
use crate::{controller::event::AppEvent, domain::workflow::ScopeBinding, obs::client::ObsClient};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::SyncSender,
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::{runtime::Handle, task::JoinHandle};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ThumbnailRequest {
    pub(crate) epoch: u64,
    pub(crate) generation: u64,
    pub(crate) scope: ScopeBinding,
    pub(crate) scene: String,
}

pub(crate) struct ThumbnailController {
    runtime: Handle,
    tx: SyncSender<AppEvent>,
    slot: Arc<Mutex<Option<ObsClient>>>,
    task: Option<JoinHandle<()>>,
    cancelled: Arc<AtomicBool>,
    worker_client: Option<ObsClient>,
}
impl ThumbnailController {
    pub(crate) fn new(
        runtime: Handle,
        tx: SyncSender<AppEvent>,
        slot: Arc<Mutex<Option<ObsClient>>>,
    ) -> Self {
        Self {
            runtime,
            tx,
            slot,
            task: None,
            cancelled: Arc::new(AtomicBool::new(false)),
            worker_client: None,
        }
    }
    pub(crate) fn cancel(&mut self) {
        // obws retains unanswered request IDs even when their futures are dropped.
        self.cancelled.store(true, Ordering::Relaxed);
    }
    pub(crate) fn request(&mut self, request: ThumbnailRequest) {
        let client = self.slot.lock().ok().and_then(|slot| slot.clone());
        if self.task.as_ref().is_some_and(|task| !task.is_finished()) {
            if self
                .worker_client
                .as_ref()
                .zip(client.as_ref())
                .is_some_and(|(old, current)| old.same_connection(current))
            {
                let _ = self.tx.try_send(AppEvent::ThumbnailLoaded {
                    request,
                    result: Err("A previous thumbnail is still awaiting OBS".into()),
                });
                return;
            }
            // The retired receiver list belongs to another socket, never the active controls.
            self.cancel();
            if let Some(task) = self.task.take() {
                task.abort();
            }
        }
        self.worker_client = client.clone();
        let tx = self.tx.clone();
        self.cancelled = Arc::new(AtomicBool::new(false));
        let cancelled = self.cancelled.clone();
        self.task = Some(self.runtime.spawn(async move {
            let capture = async {
                if cancelled.load(Ordering::Relaxed) { return Err("Thumbnail cancelled".into()); }
                let client = client.ok_or_else(|| "OBS is disconnected".to_string())?;
                if client.endpoint() != request.scope.endpoint {
                    return Err("Thumbnail endpoint changed".to_string());
                }
                check_collection(&client, &request.scope).await?;
                if cancelled.load(Ordering::Relaxed) { return Err("Thumbnail cancelled".into()); }
                let image = client.get_scene_thumbnail(&request.scene, &cancelled).await.map_err(|error| error.to_string())?;
                if cancelled.load(Ordering::Relaxed) { return Err("Thumbnail cancelled".into()); }
                check_collection(&client, &request.scope).await?;
                Ok(image)
            };
            tokio::pin!(capture);
            tokio::select! {
                result = &mut capture => {
                    if !cancelled.load(Ordering::Relaxed) {
                        let _ = tx.try_send(AppEvent::ThumbnailLoaded { request: request.clone(), result });
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(2)) => {
                    if !cancelled.swap(true, Ordering::Relaxed) {
                        let _ = tx.try_send(AppEvent::ThumbnailLoaded {
                            request: request.clone(), result: Err("Thumbnail timed out".into()),
                        });
                    }
                    // Drain this one transaction; retries must not accumulate obws receivers.
                    let _ = capture.await;
                }
            }
        }));
    }
}
impl Drop for ThumbnailController {
    fn drop(&mut self) {
        self.cancel();
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

async fn check_collection(client: &ObsClient, scope: &ScopeBinding) -> Result<(), String> {
    let collection = client
        .get_thumbnail_collection()
        .await
        .map_err(|error| error.to_string())?;
    if collection != scope.collection {
        return Err("Thumbnail collection changed".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use serde_json::{json, Value};
    use std::sync::mpsc;
    use tokio::{net::TcpListener, sync::oneshot, time::timeout};
    use tokio_websockets::{Message, ServerBuilder};

    struct MockObs {
        client: ObsClient,
        server: JoinHandle<Vec<String>>,
        capture_started: oneshot::Receiver<()>,
        release_capture: oneshot::Sender<()>,
        finish: oneshot::Sender<()>,
    }

    async fn mock_obs(before: &str, after: &str, hold_video: bool) -> MockObs {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        let before = before.to_owned();
        let after = after.to_owned();
        let (started, capture_started) = oneshot::channel();
        let (release_capture, release) = oneshot::channel();
        let (finish, mut finished) = oneshot::channel();
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
            let mut requests = Vec::new();
            let mut capture_started = Some(started);
            let mut release = release;
            let mut released = false;
            let mut held_capture = None;
            let mut captured = false;
            loop {
                let message = tokio::select! {
                    _ = &mut finished => break,
                    _ = &mut release, if !released && held_capture.is_some() => {
                        captured = true;
                        released = true;
                        socket.send(Message::text(held_capture.take().unwrap())).await.unwrap();
                        continue;
                    }
                    message = socket.next() => message.unwrap().unwrap(),
                };
                let request: Value = serde_json::from_str(message.as_text().unwrap()).unwrap();
                let name = request["d"]["requestType"].as_str().unwrap();
                requests.push(name.to_owned());
                let mut hold = false;
                let data = match name {
                    "GetVersion" => {
                        json!({"obsStudioVersion":"31.0.0","obsWebSocketVersion":"5.5.0","rpcVersion":1,"availableRequests":[],"supportedImageFormats":["png"],"platform":"mock","platformDescription":"mock"})
                    }
                    "GetVideoSettings" => {
                        if hold_video {
                            if let Some(started) = capture_started.take() {
                                started.send(()).unwrap();
                                hold = true;
                            }
                        }
                        json!({"fpsNumerator":30,"fpsDenominator":1,"baseWidth":1920,"baseHeight":1080,"outputWidth":1920,"outputHeight":1080})
                    }
                    "GetSceneCollectionList" => {
                        let collection = if captured { &after } else { &before };
                        json!({"currentSceneCollectionName":collection,"sceneCollections":[collection]})
                    }
                    "GetSourceScreenshot" => {
                        assert_eq!(request["d"]["requestData"]["imageWidth"], 384);
                        assert_eq!(request["d"]["requestData"]["imageHeight"], 216);
                        if let Some(started) = capture_started.take() {
                            started.send(()).unwrap();
                            hold = true;
                        } else {
                            captured = true;
                        }
                        json!({"imageData":"fixture image"})
                    }
                    other => panic!("unexpected thumbnail request: {other}"),
                };
                let response = json!({"op":7,"d":{"requestId":request["d"]["requestId"],"requestType":name,"requestStatus":{"result":true,"code":100},"responseData":data}}).to_string();
                if hold {
                    held_capture = Some(response);
                } else {
                    socket.send(Message::text(response)).await.unwrap();
                }
            }
            requests
        });
        let (client, _events) = ObsClient::connect("127.0.0.1", port, None).await.unwrap();
        MockObs {
            client,
            server,
            capture_started,
            release_capture,
            finish,
        }
    }

    fn controller(client: &ObsClient) -> (ThumbnailController, mpsc::Receiver<AppEvent>) {
        let (tx, rx) = mpsc::sync_channel(8);
        (
            ThumbnailController::new(
                Handle::current(),
                tx,
                Arc::new(Mutex::new(Some(client.clone()))),
            ),
            rx,
        )
    }

    fn request(client: &ObsClient) -> ThumbnailRequest {
        ThumbnailRequest {
            epoch: 7,
            generation: 3,
            scope: ScopeBinding {
                endpoint: client.endpoint().into(),
                collection: "Expected".into(),
            },
            scene: "Program".into(),
        }
    }

    fn result(events: &mpsc::Receiver<AppEvent>) -> (ThumbnailRequest, Result<String, String>) {
        match events.try_recv().unwrap() {
            AppEvent::ThumbnailLoaded { request, result } => (request, result),
            event => panic!("unexpected event: {event:?}"),
        }
    }

    #[tokio::test]
    async fn a_new_connection_can_capture_while_a_retired_connection_has_no_image_reply() {
        let old = mock_obs("Expected", "Expected", false).await;
        let current = mock_obs("Expected", "Expected", false).await;
        assert!(old.client.same_connection(&old.client.clone()));
        assert!(!old.client.same_connection(&current.client));
        let (mut controller, events) = controller(&old.client);
        controller.request(request(&old.client));
        old.capture_started.await.unwrap();
        controller.cancel();
        *controller.slot.lock().unwrap() = Some(current.client.clone());
        controller.request(request(&current.client));
        timeout(Duration::from_secs(1), current.capture_started)
            .await
            .unwrap()
            .unwrap();
        current.release_capture.send(()).unwrap();
        timeout(Duration::from_secs(3), controller.task.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        let (request, image) = result(&events);
        assert_eq!(request.scope.endpoint, current.client.endpoint());
        assert_eq!(image.unwrap(), "fixture image");
        old.finish.send(()).unwrap();
        let old_requests = old.server.await.unwrap();
        assert_eq!(
            old_requests
                .iter()
                .filter(|name| name.as_str() == "GetSourceScreenshot")
                .count(),
            1
        );
        current.finish.send(()).unwrap();
        current.server.await.unwrap();
        assert!(events.try_recv().is_err());
    }

    #[tokio::test]
    async fn cancelling_before_worker_runs_does_not_begin_scope_or_capture_reads() {
        let mock = mock_obs("Expected", "Expected", false).await;
        let (mut controller, events) = controller(&mock.client);
        controller.request(request(&mock.client));
        controller.cancel();
        timeout(Duration::from_secs(3), controller.task.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        assert!(events.try_recv().is_err());
        mock.finish.send(()).unwrap();
        assert_eq!(mock.server.await.unwrap(), ["GetVersion"]);
    }

    #[tokio::test]
    async fn cancelling_during_canvas_read_does_not_start_an_image_capture() {
        let mock = mock_obs("Expected", "Expected", true).await;
        let (mut controller, events) = controller(&mock.client);
        controller.request(request(&mock.client));
        mock.capture_started.await.unwrap(); // The fixture holds GetVideoSettings here.
        controller.cancel();
        mock.release_capture.send(()).unwrap();
        timeout(Duration::from_secs(3), controller.task.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        assert!(events.try_recv().is_err());
        mock.finish.send(()).unwrap();
        assert_eq!(
            mock.server.await.unwrap(),
            ["GetVersion", "GetSceneCollectionList", "GetVideoSettings"]
        );
    }

    #[tokio::test]
    async fn thumbnail_rejects_a_new_endpoint_before_requesting_pixels() {
        let mock = mock_obs("Expected", "Expected", false).await;
        let (mut controller, events) = controller(&mock.client);
        let mut request = request(&mock.client);
        request.scope.endpoint = "other:4455".into();
        controller.request(request);
        timeout(Duration::from_secs(3), controller.task.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        assert!(result(&events).1.unwrap_err().contains("endpoint changed"));
        mock.finish.send(()).unwrap();
        assert_eq!(mock.server.await.unwrap(), ["GetVersion"]);
    }

    #[tokio::test]
    async fn thumbnail_rejects_a_changed_collection_before_requesting_pixels() {
        let mock = mock_obs("Other", "Other", false).await;
        let (mut controller, events) = controller(&mock.client);
        controller.request(request(&mock.client));
        timeout(Duration::from_secs(3), controller.task.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        assert!(result(&events)
            .1
            .unwrap_err()
            .contains("collection changed"));
        mock.finish.send(()).unwrap();
        assert_eq!(
            mock.server.await.unwrap(),
            ["GetVersion", "GetSceneCollectionList"]
        );
    }

    #[tokio::test]
    async fn late_image_is_discarded_when_collection_changes_during_capture() {
        let mock = mock_obs("Expected", "Other", false).await;
        let (mut controller, events) = controller(&mock.client);
        controller.request(request(&mock.client));
        mock.capture_started.await.unwrap();
        mock.release_capture.send(()).unwrap();
        timeout(Duration::from_secs(3), controller.task.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        let (request, image) = result(&events);
        assert_eq!((request.epoch, request.generation), (7, 3));
        assert!(image.unwrap_err().contains("collection changed"));
        mock.finish.send(()).unwrap();
        assert_eq!(mock.server.await.unwrap().len(), 5);
    }

    #[tokio::test]
    async fn cancelled_capture_publishes_no_late_image() {
        let mock = mock_obs("Expected", "Expected", false).await;
        let (mut controller, events) = controller(&mock.client);
        controller.request(request(&mock.client));
        mock.capture_started.await.unwrap();
        controller.cancel();
        mock.release_capture.send(()).unwrap();
        timeout(Duration::from_secs(3), controller.task.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        timeout(Duration::from_secs(3), mock.client.get_version())
            .await
            .unwrap()
            .unwrap();
        assert!(events.try_recv().is_err());
        mock.finish.send(()).unwrap();
        assert_eq!(
            mock.server
                .await
                .unwrap()
                .iter()
                .filter(|name| name.as_str() == "GetSceneCollectionList")
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn a_stalled_capture_times_out_without_waiting_for_the_image() {
        let mock = mock_obs("Expected", "Expected", false).await;
        let (mut controller, events) = controller(&mock.client);
        controller.request(request(&mock.client));
        mock.capture_started.await.unwrap();
        let timeout_event = timeout(Duration::from_secs(3), async {
            loop {
                if let Ok(event) = events.try_recv() {
                    break event;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        assert!(
            matches!(timeout_event, AppEvent::ThumbnailLoaded {result: Err(error), ..} if error.contains("timed out"))
        );
        assert!(!controller.task.as_ref().unwrap().is_finished());
        // Repeated timeouts/page hides cannot enqueue new protocol transactions.
        for generation in 4..68 {
            controller.cancel();
            let mut retry = request(&mock.client);
            retry.generation = generation;
            controller.request(retry);
            assert!(result(&events)
                .1
                .unwrap_err()
                .contains("still awaiting OBS"));
        }
        // The image remains unanswered beyond the ordinary request deadline.
        // Optional work must neither retire controls nor reopen image admission.
        tokio::time::sleep(Duration::from_secs(9)).await;
        assert!(!controller.task.as_ref().unwrap().is_finished());
        controller.request(request(&mock.client));
        assert!(result(&events)
            .1
            .unwrap_err()
            .contains("still awaiting OBS"));
        // An independent control read can finish while the image reply is held.
        timeout(Duration::from_secs(3), mock.client.get_version())
            .await
            .unwrap()
            .unwrap();
        mock.release_capture.send(()).unwrap();
        timeout(Duration::from_secs(3), controller.task.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        assert!(events.try_recv().is_err());
        controller.request(request(&mock.client));
        timeout(Duration::from_secs(3), controller.task.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result(&events).1.unwrap(), "fixture image");
        mock.finish.send(()).unwrap();
        let requests = mock.server.await.unwrap();
        assert_eq!(
            requests
                .iter()
                .filter(|name| name.as_str() == "GetSourceScreenshot")
                .count(),
            2,
            "one stalled request and one request after draining; retries cannot add protocol IDs"
        );
        assert_eq!(
            requests
                .iter()
                .filter(|name| name.as_str() == "GetSceneCollectionList")
                .count(),
            3,
            "cancelled/timed-out captures do not start a follow-up collection read"
        );
    }
}
