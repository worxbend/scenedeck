//! OBS connection-session lifecycle orchestration.

use futures_util::future::BoxFuture;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

use crate::controller::dependencies::ControllerDependencies;
use crate::controller::event::AppEvent;
use crate::controller::event_sender::EventSender;
use crate::controller::lifecycle::SessionScope;
use crate::controller::output_controller::{
    stop_active_outputs_before_disconnect, OutputCommandClient, OutputController,
};
use crate::controller::refresh_controller::{run_event_loop, BitrateSample};
use crate::infra::error::AppError;
use crate::obs::client::ObsClient;

pub(crate) const REQUEST_BUDGET: Duration = Duration::from_secs(10);
const DISCONNECT_BUDGET: Duration = Duration::from_secs(3);

struct RetiringOutputClient(OutputCommandClient);

impl Drop for RetiringOutputClient {
    fn drop(&mut self) {
        self.0.retire();
    }
}

struct DisconnectSession {
    client: Option<RetiringOutputClient>,
    scope: SessionScope,
    previous: Option<JoinHandle<()>>,
}

impl Drop for DisconnectSession {
    fn drop(&mut self) {
        // Fence the socket before cancelling tasks that own admitted requests.
        if let Some(client) = &self.client {
            client.0.retire();
        }
        self.scope.retire();
        if let Some(previous) = &self.previous {
            previous.abort();
        }
    }
}

pub(crate) struct SessionController {
    runtime: Handle,
    event_tx: EventSender,
    runner: Arc<dyn SessionRunner>,
    scope: SessionScope,
    bitrate_sample: BitrateSample,
    task: Option<JoinHandle<()>>,
    disconnecting: bool,
    terminal_publisher: JoinHandle<()>,
}

impl SessionController {
    pub(crate) fn new(
        runtime: Handle,
        event_tx: EventSender,
        dependencies: ControllerDependencies,
        scope: SessionScope,
        bitrate_sample: BitrateSample,
    ) -> Self {
        let runner = Arc::new(ObsSessionRunner {
            dependencies,
            bitrate_sample: bitrate_sample.clone(),
        });
        Self::with_runner(runtime, event_tx, runner, scope, bitrate_sample)
    }

    pub(crate) fn with_runner(
        runtime: Handle,
        event_tx: EventSender,
        runner: Arc<dyn SessionRunner>,
        scope: SessionScope,
        bitrate_sample: BitrateSample,
    ) -> Self {
        let publication_scope = scope.clone();
        let publication_tx = event_tx.clone();
        let terminal_publisher = runtime
            .spawn(async move { publication_scope.publish_retirements(publication_tx).await });
        Self {
            terminal_publisher,
            runtime,
            event_tx,
            runner,
            scope,
            bitrate_sample,
            task: None,
            disconnecting: false,
        }
    }

    pub(crate) fn scope(&self) -> SessionScope {
        self.scope.clone()
    }

    pub(crate) fn connect(&mut self) {
        let retired = self.scope.clone();
        self.scope = self.scope.next();
        self.clear_bitrate_sample();
        self.disconnecting = false;
        let previous = self.task.take();
        let scope = self.scope.clone();
        let tx = self.event_tx.scoped(&scope);
        let runner = self.runner.clone();
        self.task = Some(self.runtime.spawn(async move {
            if let Some(previous) = previous {
                previous.abort();
                let _ = previous.await;
            }
            retired.quiesce().await;
            if tx.send(AppEvent::Connecting).await.is_err() {
                return;
            }
            tokio::select! {
                biased;
                () = scope.cancelled() => {}
                () = runner.run(tx.clone(), scope.clone()) => {}
            }
            scope.retire();
            scope.quiesce().await;
            // Preserve an establishment/transport error as the terminal UI
            // status; a following Disconnected would erase its Retry context.
            if !scope.failed() && !scope.has_terminal_reason() {
                let _ = tx.send(AppEvent::Disconnected).await;
            }
        }));
    }

    pub(crate) fn disconnect(&mut self, outputs: &OutputController) {
        if self.disconnecting {
            return;
        }
        self.disconnecting = true;
        let client = outputs.client().map(RetiringOutputClient);
        // A disconnect before the first connection still publishes a scoped
        // lifecycle event; no real request owner exists to drain in that case.
        if self.scope.generation() == 0
            && !matches!(
                &client,
                Some(RetiringOutputClient(OutputCommandClient::Obs(_)))
            )
        {
            self.scope = self.scope.next();
        }
        if let Some(RetiringOutputClient(OutputCommandClient::Obs(client))) = &client {
            client.begin_disconnect();
        }
        // Keep admitted requests alive until drainage/stop completes. Closing
        // admission already prevents delayed commands from reaching the socket.
        let mut teardown = DisconnectSession {
            client,
            scope: self.scope.clone(),
            previous: self.task.take(),
        };
        let tx = self.event_tx.scoped(&self.scope);
        self.clear_bitrate_sample();
        outputs.clear_pending();
        self.task = Some(self.runtime.spawn(async move {
            let stopped = tokio::time::timeout(DISCONNECT_BUDGET, async {
                match teardown.client.as_ref().map(|client| &client.0) {
                    Some(OutputCommandClient::Obs(client)) => {
                        let shutdown = client.disconnect_client().await?;
                        stop_active_outputs_before_disconnect(&OutputCommandClient::Obs(shutdown.clone())).await;
                        let stream = shutdown.get_stream_status().await?;
                        let record = shutdown.get_record_status().await?;
                        if stream.active || record.active {
                            return Err(AppError::request("OBS outputs remain active after stop"));
                        }
                        Ok(())
                    }
                    #[cfg(test)]
                    Some(client @ OutputCommandClient::Fake(_)) => {
                        stop_active_outputs_before_disconnect(client).await;
                        Ok(())
                    }
                    None => Ok(()),
                }
            }).await;
            let failure = match stopped {
                Ok(Ok(())) => None,
                Ok(Err(error)) => Some(format!("Disconnected; output shutdown unconfirmed: {error}; commands were not retried")),
                Err(_) => Some("Disconnected; output shutdown unconfirmed: graceful stop deadline exceeded; commands were not retried".into()),
            };
            if let Some(reason) = &failure {
                teardown.scope.set_terminal_reason(reason.clone());
            }
            if let Some(client) = &teardown.client { client.0.retire(); }
            teardown.scope.retire();
            if let Some(previous) = teardown.previous.take() {
                previous.abort();
                let _ = previous.await;
            }
            teardown.scope.quiesce().await;
            let event = failure.map_or(AppEvent::Disconnected, |reason| AppEvent::Error(AppError::Connection(reason)));
            let _ = tx.send(event).await;
        }));
    }

    pub(crate) fn shutdown(&mut self) -> JoinHandle<()> {
        self.scope.retire();
        let previous = self.task.take();
        let scope = self.scope.clone();
        self.runtime.spawn(async move {
            if let Some(previous) = previous {
                previous.abort();
                let _ = previous.await;
            }
            scope.quiesce().await;
        })
    }

    fn clear_bitrate_sample(&self) {
        if let Ok(mut sample) = self.bitrate_sample.lock() {
            *sample = None;
        }
    }
}

impl Drop for SessionController {
    fn drop(&mut self) {
        self.terminal_publisher.abort();
        self.scope.retire();
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

pub(crate) trait SessionRunner: Send + Sync {
    fn run(&self, tx: EventSender, scope: SessionScope) -> BoxFuture<'static, ()>;
}

struct ObsSessionRunner {
    dependencies: ControllerDependencies,
    bitrate_sample: BitrateSample,
}

impl SessionRunner for ObsSessionRunner {
    fn run(&self, tx: EventSender, scope: SessionScope) -> BoxFuture<'static, ()> {
        let dependencies = self.dependencies.clone();
        let bitrate_sample = self.bitrate_sample.clone();
        Box::pin(async move {
            let settings = tokio::task::spawn_blocking(move || {
                let config = dependencies.load_config();
                let password = dependencies.obs_password()?;
                Ok::<_, AppError>((config, password))
            })
            .await;
            let (config, password) = match settings {
                Ok(Ok(settings)) => settings,
                Ok(Err(error)) => {
                    let _ = tx.send(AppEvent::Error(error)).await;
                    return;
                }
                Err(error) => {
                    let _ = tx.send(AppEvent::Error(AppError::storage(error))).await;
                    return;
                }
            };
            let connected = tokio::time::timeout(
                REQUEST_BUDGET,
                ObsClient::connect(&config.obs.host, config.obs.port, password.as_deref()),
            )
            .await;
            let (client, events) = match connected {
                Ok(Ok(session)) => session,
                Ok(Err(error)) => {
                    let _ = tx.send(AppEvent::Error(error)).await;
                    return;
                }
                Err(_) => {
                    let _ = tx
                        .send(AppEvent::Error(AppError::Connection(
                            "OBS connection deadline exceeded".into(),
                        )))
                        .await;
                    return;
                }
            };
            if !scope.install(client.clone()) {
                return;
            }
            // Event ingress starts before version/bootstrap requests can block.
            run_event_loop(client, events, tx, config.live.audio_inputs, bitrate_sample).await;
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{mpsc, Mutex};
    use std::time::Duration;

    use crate::controller::event::ConnectionInfo;

    struct FakeSessionRunner {
        runs: Arc<AtomicUsize>,
    }

    impl SessionRunner for FakeSessionRunner {
        fn run(&self, tx: EventSender, _scope: SessionScope) -> BoxFuture<'static, ()> {
            let runs = Arc::clone(&self.runs);
            Box::pin(async move {
                runs.fetch_add(1, Ordering::SeqCst);
                let _ = tx
                    .send(AppEvent::Connected(ConnectionInfo {
                        endpoint: "test:4455".to_string(),
                        obs_version: "fake-obs".to_string(),
                        websocket_version: "fake-websocket".to_string(),
                        available_requests: Vec::new(),
                        supported_image_formats: Vec::new(),
                    }))
                    .await;
                std::future::pending::<()>().await;
            })
        }
    }

    fn receive(rx: &mpsc::Receiver<AppEvent>, scope: &SessionScope) -> AppEvent {
        loop {
            match rx
                .recv_timeout(Duration::from_secs(1))
                .expect("session lifecycle event")
            {
                AppEvent::SessionEvent {
                    generation, event, ..
                } => {
                    if scope.owns_generation(generation) {
                        return *event;
                    }
                }
                event => return event,
            }
        }
    }

    struct FailedSessionRunner;

    impl SessionRunner for FailedSessionRunner {
        fn run(&self, tx: EventSender, _: SessionScope) -> BoxFuture<'static, ()> {
            Box::pin(async move {
                let _ = tx
                    .send(AppEvent::Error(AppError::Connection(
                        "fixture refused".into(),
                    )))
                    .await;
            })
        }
    }

    #[tokio::test]
    async fn establishment_error_is_not_overwritten_by_disconnected_cleanup() {
        let (tx, rx) = mpsc::sync_channel(8);
        let scope = SessionScope::initial(Arc::new(Mutex::new(None)));
        let mut session = SessionController::with_runner(
            Handle::current(),
            EventSender::new(tx),
            Arc::new(FailedSessionRunner),
            scope,
            Arc::new(Mutex::new(None)),
        );
        session.connect();
        session.task.take().unwrap().await.unwrap();
        assert!(matches!(receive(&rx, &session.scope), AppEvent::Connecting));
        assert!(matches!(receive(&rx, &session.scope), AppEvent::Error(_)));
        assert!(rx.try_recv().is_err());
        assert!(!session.scope.is_active());
    }

    struct GatedShutdown {
        started: tokio::sync::Notify,
        release: tokio::sync::Notify,
        writes: AtomicUsize,
    }

    impl crate::controller::output_controller::FakeOutputCommandClient for GatedShutdown {
        fn set_streaming(&self, _: bool) -> BoxFuture<'_, Result<(), AppError>> {
            Box::pin(async move {
                self.writes.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        }
        fn set_recording(&self, _: bool) -> BoxFuture<'_, Result<Option<String>, AppError>> {
            Box::pin(async { Ok(None) })
        }
        fn get_stream_status(
            &self,
        ) -> BoxFuture<'_, Result<crate::domain::output::OutputStatus, AppError>> {
            Box::pin(async move {
                self.started.notify_one();
                self.release.notified().await;
                Ok(crate::domain::output::OutputStatus::new(
                    true,
                    crate::domain::output::OutputRunState::Active,
                ))
            })
        }
        fn get_record_status(
            &self,
        ) -> BoxFuture<'_, Result<crate::domain::output::OutputStatus, AppError>> {
            Box::pin(async { Ok(Default::default()) })
        }
    }

    #[test]
    fn reconnect_owns_and_cancels_old_teardown_before_starting_the_next_runner() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let (tx, rx) = mpsc::sync_channel(16);
        let slot = Arc::new(Mutex::new(None));
        let mut sessions = SessionController::with_runner(
            runtime.handle().clone(),
            EventSender::new(tx.clone()),
            Arc::new(FakeSessionRunner {
                runs: Arc::new(AtomicUsize::new(0)),
            }),
            SessionScope::initial(slot.clone()),
            Arc::new(Mutex::new(None)),
        );
        let outputs = OutputController::new(runtime.handle().clone(), tx, slot);
        let gated = Arc::new(GatedShutdown {
            started: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
            writes: AtomicUsize::new(0),
        });
        outputs.set_client_override(OutputCommandClient::Fake(gated.clone()));
        sessions.connect();
        assert!(matches!(
            receive(&rx, &sessions.scope()),
            AppEvent::Connecting
        ));
        assert!(matches!(
            receive(&rx, &sessions.scope()),
            AppEvent::Connected(_)
        ));
        sessions.disconnect(&outputs);
        sessions.disconnect(&outputs);
        runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(1), gated.started.notified())
                .await
                .unwrap();
        });
        let retired = sessions.scope();
        sessions.connect();
        assert!(matches!(
            receive(&rx, &sessions.scope()),
            AppEvent::Connecting
        ));
        assert!(matches!(
            receive(&rx, &sessions.scope()),
            AppEvent::Connected(_)
        ));
        gated.release.notify_one();
        runtime.block_on(retired.quiesce());
        assert!(sessions.scope().is_active());
        assert!(!retired.is_current());
        assert_eq!(
            gated.writes.load(Ordering::SeqCst),
            0,
            "cancelled teardown cannot begin a delayed stop"
        );
        assert!(
            rx.try_recv().is_err(),
            "no retired Disconnected may follow the replacement Connected"
        );
        runtime.block_on(sessions.shutdown()).unwrap();
    }

    #[test]
    fn lifecycle_handlers_return_while_the_gtk_queue_is_full() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let (tx, _rx) = mpsc::sync_channel(1);
        tx.send(AppEvent::Disconnected).unwrap();
        let slot = Arc::new(Mutex::new(None));
        let mut sessions = SessionController::with_runner(
            runtime.handle().clone(),
            EventSender::new(tx.clone()),
            Arc::new(FakeSessionRunner {
                runs: Arc::new(AtomicUsize::new(0)),
            }),
            SessionScope::initial(slot.clone()),
            Arc::new(Mutex::new(None)),
        );
        let outputs = OutputController::new(runtime.handle().clone(), tx, slot);
        let (returned, observed) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            sessions.connect();
            sessions.disconnect(&outputs);
            sessions.disconnect(&outputs);
            sessions.connect();
            returned.send(()).unwrap();
            sessions
        });
        observed
            .recv_timeout(Duration::from_secs(1))
            .expect("GTK handlers must not wait for queue capacity");
        let mut sessions = worker.join().unwrap();
        runtime.block_on(sessions.shutdown()).unwrap();
    }

    #[test]
    fn fake_session_connect_and_reconnect_publish_lifecycle_events() {
        let runtime = tokio::runtime::Runtime::new().expect("test runtime");
        let (tx, rx) = mpsc::sync_channel(16);
        let client_slot = Arc::new(Mutex::new(None));
        let sample = Arc::new(Mutex::new(None));
        let runs = Arc::new(AtomicUsize::new(0));
        let mut sessions = SessionController::with_runner(
            runtime.handle().clone(),
            EventSender::new(tx),
            Arc::new(FakeSessionRunner {
                runs: Arc::clone(&runs),
            }),
            SessionScope::initial(client_slot),
            sample,
        );

        sessions.connect();
        assert!(matches!(
            receive(&rx, &sessions.scope()),
            AppEvent::Connecting
        ));
        assert!(matches!(
            receive(&rx, &sessions.scope()),
            AppEvent::Connected(_)
        ));

        sessions.connect();
        assert!(matches!(
            receive(&rx, &sessions.scope()),
            AppEvent::Connecting
        ));
        assert!(matches!(
            receive(&rx, &sessions.scope()),
            AppEvent::Connected(_)
        ));
        assert_eq!(runs.load(Ordering::SeqCst), 2);
    }

    struct WireSessionRunner(ObsClient, bool);

    impl SessionRunner for WireSessionRunner {
        fn run(&self, _: EventSender, scope: SessionScope) -> BoxFuture<'static, ()> {
            let client = self.0.clone();
            let mutation = self.1;
            Box::pin(async move {
                assert!(scope.install(client.clone()));
                if mutation {
                    let _ = client.set_current_program_scene("Pending scene").await;
                } else {
                    let _ = client.get_version().await;
                }
                // A real refresh pipeline may request its next stage as soon
                // as an admitted read completes, then retire on transport error.
                if client.get_version().await.is_err() {
                    client.retire();
                }
                std::future::pending::<()>().await;
            })
        }
    }

    struct DisconnectWire {
        client: ObsClient,
        entered: tokio::sync::oneshot::Receiver<()>,
        release: tokio::sync::oneshot::Sender<()>,
        finish: tokio::sync::oneshot::Sender<()>,
        server: JoinHandle<Vec<String>>,
    }

    async fn disconnect_wire(hold: &'static str) -> DisconnectWire {
        use futures_util::{SinkExt, StreamExt};
        use serde_json::{json, Value};
        use tokio::sync::oneshot;
        use tokio_websockets::{Message, ServerBuilder};

        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        let (started, entered) = oneshot::channel();
        let (release, mut released) = oneshot::channel();
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
            let mut names = Vec::new();
            let mut started = Some(started);
            let mut held = None;
            let mut stream_active = true;
            let mut record_active = true;
            loop {
                let message = tokio::select! {
                    _ = &mut finished => break,
                    _ = &mut released, if held.is_some() => {
                        socket.send(Message::text(held.take().unwrap())).await.unwrap();
                        continue;
                    }
                    message = socket.next() => match message {
                        Some(Ok(message)) if message.is_text() => message,
                        _ => break,
                    },
                };
                let frame: Value = serde_json::from_str(message.as_text().unwrap()).unwrap();
                let name = frame["d"]["requestType"].as_str().unwrap();
                names.push(name.to_owned());
                let data = match name {
                    "GetVersion" => Some(
                        json!({"obsStudioVersion":"31.0.0","obsWebSocketVersion":"5.5.0","rpcVersion":1,"availableRequests":[],"supportedImageFormats":[],"platform":"mock","platformDescription":"mock"}),
                    ),
                    "GetStreamStatus" => Some(
                        json!({"outputActive":stream_active,"outputReconnecting":false,"outputTimecode":"00:00:00.000","outputDuration":0,"outputCongestion":0.0,"outputBytes":0,"outputSkippedFrames":0,"outputTotalFrames":0}),
                    ),
                    "GetRecordStatus" => Some(
                        json!({"outputActive":record_active,"outputPaused":false,"outputTimecode":"00:00:00.000","outputDuration":0,"outputBytes":0}),
                    ),
                    "SetCurrentProgramScene" => None,
                    "StopStream" => {
                        stream_active = false;
                        None
                    }
                    "StopRecord" => {
                        record_active = false;
                        Some(json!({"outputPath":"fixture.mkv"}))
                    }
                    other => panic!("unexpected shutdown request: {other}"),
                };
                let mut response = json!({"op":7,"d":{"requestId":frame["d"]["requestId"],"requestType":name,"requestStatus":{"result":true,"code":100}}});
                if let Some(data) = data {
                    response["d"]["responseData"] = data;
                }
                if name == hold && names.len() > 1 && started.is_some() {
                    held = Some(response.to_string());
                    started.take().unwrap().send(()).unwrap();
                } else {
                    socket
                        .send(Message::text(response.to_string()))
                        .await
                        .unwrap();
                }
            }
            names
        });
        let (client, _events) = ObsClient::connect("127.0.0.1", port, None).await.unwrap();
        DisconnectWire {
            client,
            entered,
            release,
            finish,
            server,
        }
    }

    #[tokio::test]
    async fn disconnect_drains_a_real_request_before_stopping_outputs_and_retiring() {
        let wire = disconnect_wire("GetVersion").await;
        let (tx, rx) = mpsc::sync_channel(32);
        let slot = Arc::new(Mutex::new(None));
        let mut session = SessionController::with_runner(
            Handle::current(),
            EventSender::new(tx.clone()),
            Arc::new(WireSessionRunner(wire.client.clone(), false)),
            SessionScope::initial(slot.clone()),
            Arc::new(Mutex::new(None)),
        );
        let outputs = OutputController::new(Handle::current(), tx, slot.clone());
        session.connect();
        tokio::time::timeout(Duration::from_secs(1), wire.entered)
            .await
            .unwrap()
            .unwrap();
        session.disconnect(&outputs);
        tokio::task::yield_now().await;
        wire.release.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(4), session.task.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        assert!(slot.lock().unwrap().is_none());
        wire.finish.send(()).unwrap();
        let names = wire.server.await.unwrap();
        assert_eq!(
            names.iter().filter(|name| *name == "StopStream").count(),
            1,
            "{names:?}"
        );
        assert_eq!(
            names.iter().filter(|name| *name == "StopRecord").count(),
            1,
            "{names:?}"
        );
        assert!(wire.client.retirement_reason().is_some());
        let events: Vec<_> = rx.try_iter().collect();
        assert!(events.iter().any(|event| matches!(event, AppEvent::SessionEvent { event, .. } if matches!(**event, AppEvent::Disconnected))));
    }

    #[tokio::test]
    async fn undrained_disconnect_is_bounded_and_reports_unconfirmed_shutdown() {
        let wire = disconnect_wire("SetCurrentProgramScene").await;
        let (tx, rx) = mpsc::sync_channel(32);
        let slot = Arc::new(Mutex::new(None));
        let mut session = SessionController::with_runner(
            Handle::current(),
            EventSender::new(tx.clone()),
            Arc::new(WireSessionRunner(wire.client.clone(), true)),
            SessionScope::initial(slot.clone()),
            Arc::new(Mutex::new(None)),
        );
        let outputs = OutputController::new(Handle::current(), tx, slot.clone());
        session.connect();
        tokio::time::timeout(Duration::from_secs(1), wire.entered)
            .await
            .unwrap()
            .unwrap();
        session.disconnect(&outputs);
        // Admission closes immediately, before the async teardown gets polled.
        assert!(wire
            .client
            .set_current_program_scene("Must not send")
            .await
            .is_err());
        tokio::time::timeout(Duration::from_secs(4), session.task.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        assert!(slot.lock().unwrap().is_none());
        assert!(wire.client.retirement_reason().is_some());
        let events: Vec<_> = rx.try_iter().collect();
        assert!(events.iter().any(|event| matches!(event,
            AppEvent::SessionEvent { event, .. }
                if matches!(&**event, AppEvent::Error(error) if error.to_string().contains("output shutdown unconfirmed"))
        )), "{events:?}");
        // A late reply cannot restart cleanup after the bounded teardown.
        wire.release.send(()).unwrap();
        wire.finish.send(()).unwrap();
        let names = wire.server.await.unwrap();
        assert_eq!(names, ["GetVersion", "SetCurrentProgramScene"]);
    }

    #[test]
    fn disconnect_after_fake_session_emits_disconnected() {
        let runtime = tokio::runtime::Runtime::new().expect("test runtime");
        let (tx, rx) = mpsc::sync_channel(16);
        let client_slot = Arc::new(Mutex::new(None));
        let sample = Arc::new(Mutex::new(None));
        let mut sessions = SessionController::with_runner(
            runtime.handle().clone(),
            EventSender::new(tx.clone()),
            Arc::new(FakeSessionRunner {
                runs: Arc::new(AtomicUsize::new(0)),
            }),
            SessionScope::initial(Arc::clone(&client_slot)),
            sample,
        );
        let outputs = OutputController::new(runtime.handle().clone(), tx, client_slot);

        sessions.connect();
        let _ = receive(&rx, &sessions.scope());
        let _ = receive(&rx, &sessions.scope());
        sessions.disconnect(&outputs);

        assert!(matches!(
            receive(&rx, &sessions.scope()),
            AppEvent::Disconnected
        ));
    }
}
