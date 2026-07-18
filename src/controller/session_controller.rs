//! OBS connection-session lifecycle orchestration.

use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};

use futures_util::future::BoxFuture;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

use crate::controller::dependencies::ControllerDependencies;
use crate::controller::event::AppEvent;
use crate::controller::output_controller::{
    refresh_output_statuses, stop_active_outputs_before_disconnect, OutputController,
};
use crate::controller::refresh_controller::{
    refresh_live_data, refresh_profile_and_collection_lists, run_event_loop, BitrateSample,
};
use crate::obs::client::ObsClient;

pub(crate) struct SessionController {
    runtime: Handle,
    event_tx: SyncSender<AppEvent>,
    runner: Arc<dyn SessionRunner>,
    client_slot: Arc<Mutex<Option<ObsClient>>>,
    bitrate_sample: BitrateSample,
    task: Option<JoinHandle<()>>,
}

impl SessionController {
    pub(crate) fn new(
        runtime: Handle,
        event_tx: SyncSender<AppEvent>,
        dependencies: ControllerDependencies,
        client_slot: Arc<Mutex<Option<ObsClient>>>,
        bitrate_sample: BitrateSample,
    ) -> Self {
        Self::with_runner(
            runtime,
            event_tx,
            Arc::new(ObsSessionRunner { dependencies }),
            client_slot,
            bitrate_sample,
        )
    }

    pub(crate) fn with_runner(
        runtime: Handle,
        event_tx: SyncSender<AppEvent>,
        runner: Arc<dyn SessionRunner>,
        client_slot: Arc<Mutex<Option<ObsClient>>>,
        bitrate_sample: BitrateSample,
    ) -> Self {
        Self {
            runtime,
            event_tx,
            runner,
            client_slot,
            bitrate_sample,
            task: None,
        }
    }

    pub(crate) fn connect(&mut self) {
        self.abort_task();
        self.clear_runtime_state();

        let tx = self.event_tx.clone();
        let runner = Arc::clone(&self.runner);
        let client_slot = Arc::clone(&self.client_slot);
        let _ = tx.send(AppEvent::Connecting);

        self.task = Some(self.runtime.spawn(async move {
            runner.run(tx, client_slot).await;
        }));
    }

    pub(crate) fn disconnect(&mut self, outputs: &OutputController) {
        let task = self.task.take();
        let client = outputs.client();
        let client_slot = Arc::clone(&self.client_slot);
        let tx = self.event_tx.clone();
        self.clear_bitrate_sample();
        outputs.clear_pending();

        self.runtime.spawn(async move {
            if let Some(client) = client {
                stop_active_outputs_before_disconnect(&client).await;
            }
            if let Some(task) = task {
                task.abort();
            }
            if let Ok(mut slot) = client_slot.lock() {
                *slot = None;
            }
            let _ = tx.send(AppEvent::Disconnected);
        });
    }

    fn abort_task(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }

    fn clear_runtime_state(&self) {
        if let Ok(mut slot) = self.client_slot.lock() {
            *slot = None;
        }
        self.clear_bitrate_sample();
    }

    fn clear_bitrate_sample(&self) {
        if let Ok(mut sample) = self.bitrate_sample.lock() {
            *sample = None;
        }
    }
}

pub(crate) trait SessionRunner: Send + Sync {
    fn run(
        &self,
        tx: SyncSender<AppEvent>,
        client_slot: Arc<Mutex<Option<ObsClient>>>,
    ) -> BoxFuture<'static, ()>;
}

struct ObsSessionRunner {
    dependencies: ControllerDependencies,
}

impl SessionRunner for ObsSessionRunner {
    fn run(
        &self,
        tx: SyncSender<AppEvent>,
        client_slot: Arc<Mutex<Option<ObsClient>>>,
    ) -> BoxFuture<'static, ()> {
        let dependencies = self.dependencies.clone();
        Box::pin(async move {
            let (config, password) = tokio::task::spawn_blocking(move || {
                let config = dependencies.load_config();
                let password = dependencies.obs_password().unwrap_or_else(|error| {
                    tracing::warn!(%error, "could not read OBS password from keyring");
                    None
                });
                (config, password)
            })
            .await
            .unwrap_or_else(|error| {
                tracing::error!(%error, "storage worker failed while loading OBS connection settings");
                (Default::default(), None)
            });

            let (client, events) =
                match ObsClient::connect(&config.obs.host, config.obs.port, password.as_deref())
                    .await
                {
                    Ok(session) => session,
                    Err(error) => {
                        let _ = tx.send(AppEvent::Error(error));
                        return;
                    }
                };
            match client.get_version().await {
                Ok(info) => {
                    let _ = tx.send(AppEvent::Connected(info));
                }
                Err(error) => {
                    let _ = tx.send(AppEvent::Error(error));
                    return;
                }
            }
            if let Ok(mut slot) = client_slot.lock() {
                *slot = Some(client.clone());
            }
            refresh_profile_and_collection_lists(&client, &tx).await;
            refresh_output_statuses(&client, &tx).await;
            refresh_live_data(&client, &tx, &config.live.audio_inputs).await;
            run_event_loop(client, events, tx, config.live.audio_inputs).await;
            if let Ok(mut slot) = client_slot.lock() {
                *slot = None;
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::time::Duration;

    use crate::controller::event::ConnectionInfo;

    struct FakeSessionRunner {
        runs: Arc<AtomicUsize>,
    }

    impl SessionRunner for FakeSessionRunner {
        fn run(
            &self,
            tx: SyncSender<AppEvent>,
            _client_slot: Arc<Mutex<Option<ObsClient>>>,
        ) -> BoxFuture<'static, ()> {
            let runs = Arc::clone(&self.runs);
            Box::pin(async move {
                runs.fetch_add(1, Ordering::SeqCst);
                let _ = tx.send(AppEvent::Connected(ConnectionInfo {
                    obs_version: "fake-obs".to_string(),
                    websocket_version: "fake-websocket".to_string(),
                }));
                std::future::pending::<()>().await;
            })
        }
    }

    fn receive(rx: &mpsc::Receiver<AppEvent>) -> AppEvent {
        rx.recv_timeout(Duration::from_secs(1))
            .expect("session lifecycle event")
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
            tx,
            Arc::new(FakeSessionRunner {
                runs: Arc::clone(&runs),
            }),
            client_slot,
            sample,
        );

        sessions.connect();
        assert!(matches!(receive(&rx), AppEvent::Connecting));
        assert!(matches!(receive(&rx), AppEvent::Connected(_)));

        sessions.connect();
        assert!(matches!(receive(&rx), AppEvent::Connecting));
        assert!(matches!(receive(&rx), AppEvent::Connected(_)));
        assert_eq!(runs.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn disconnect_after_fake_session_emits_disconnected() {
        let runtime = tokio::runtime::Runtime::new().expect("test runtime");
        let (tx, rx) = mpsc::sync_channel(16);
        let client_slot = Arc::new(Mutex::new(None));
        let sample = Arc::new(Mutex::new(None));
        let mut sessions = SessionController::with_runner(
            runtime.handle().clone(),
            tx.clone(),
            Arc::new(FakeSessionRunner {
                runs: Arc::new(AtomicUsize::new(0)),
            }),
            Arc::clone(&client_slot),
            sample,
        );
        let outputs = OutputController::new(runtime.handle().clone(), tx, client_slot);

        sessions.connect();
        let _ = receive(&rx);
        let _ = receive(&rx);
        sessions.disconnect(&outputs);

        assert!(matches!(receive(&rx), AppEvent::Disconnected));
    }
}
