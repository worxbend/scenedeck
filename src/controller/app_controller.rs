//! Application controller — owns the async OBS session and routes commands.
//!
//! Lives on the GTK main thread inside an `Rc<RefCell<AppController>>`.
//! Spawns tokio tasks for all async OBS work; results come back through the
//! `std::sync::mpsc` event channel and are processed by the GTK polling timer
//! in `ui::window`.
//!
//! ## Client lifecycle
//!
//! `ObsClient::connect` is async so the client is built inside a tokio task.
//! We share it back to the GTK-side controller via `Arc<Mutex<Option<ObsClient>>>`:
//!
//! ```text
//! connect() called on GTK thread
//!   → clears client_slot
//!   → spawns tokio session task
//!       → ObsClient::connect().await
//!       → *client_slot.lock() = Some(client.clone())   ← task writes once
//!       → run_event_loop(...)                           ← keeps running
//!       → *client_slot.lock() = None                   ← task clears on exit
//! switch_scene() / set_mute() etc. on GTK thread
//!   → client_slot.lock() → clone ObsClient             ← GTK reads
//!   → spawns new tokio task with cloned client
//! ```
//!
//! `std::sync::Mutex` (not tokio) is intentional: the lock is never held
//! across an `.await` point, so a synchronous mutex is both correct and cheaper.

#[cfg(test)]
use futures_util::future::BoxFuture;
use i18n_embed_fl::fl;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
#[cfg(test)]
use std::time::Instant;
use tokio::runtime::Handle;

use crate::controller::command::AppCommand;
use crate::controller::dependencies::ControllerDependencies;
use crate::controller::event::AppEvent;
use crate::controller::output_controller::{refresh_output_statuses, OutputController};
#[cfg(test)]
use crate::controller::output_controller::{FakeOutputCommandClient, OutputCommandClient};
#[cfg(test)]
use crate::controller::refresh_controller::bitrate_kbps_since_last_sample;
use crate::controller::refresh_controller::{
    publish_profiles_after, refresh_after_collection_change, refresh_live_data, refresh_obs_stats,
    refresh_profile_and_collection_lists, BitrateSample,
};
use crate::controller::session_controller::SessionController;
#[cfg(test)]
use crate::domain::output::{OutputRunState, OutputStatus};
#[cfg(test)]
use crate::infra::error::AppError;
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::obs::client::ObsClient;

pub struct AppController {
    event_tx: SyncSender<AppEvent>,
    runtime: Handle,
    dependencies: ControllerDependencies,
    /// Shared slot written by the session task once connected, read by
    /// per-command tasks.  `None` while disconnected.
    client_slot: Arc<Mutex<Option<ObsClient>>>,
    outputs: OutputController,
    session: SessionController,
    bitrate_sample: BitrateSample,
}

impl AppController {
    pub fn new(runtime: Handle, event_tx: SyncSender<AppEvent>) -> Self {
        Self::with_dependencies(runtime, event_tx, ControllerDependencies::default())
    }

    pub fn with_dependencies(
        runtime: Handle,
        event_tx: SyncSender<AppEvent>,
        dependencies: ControllerDependencies,
    ) -> Self {
        let client_slot = Arc::new(Mutex::new(None));
        let bitrate_sample = Arc::new(Mutex::new(None));
        let outputs =
            OutputController::new(runtime.clone(), event_tx.clone(), Arc::clone(&client_slot));
        let session = SessionController::new(
            runtime.clone(),
            event_tx.clone(),
            dependencies.clone(),
            Arc::clone(&client_slot),
            Arc::clone(&bitrate_sample),
        );
        Self {
            event_tx,
            runtime,
            dependencies,
            client_slot,
            outputs,
            session,
            bitrate_sample,
        }
    }

    pub fn handle(&mut self, cmd: AppCommand) {
        match cmd {
            AppCommand::Connect => self.session.connect(),
            AppCommand::Disconnect => self.session.disconnect(&self.outputs),
            AppCommand::RefreshAll => self.session.connect(), // reconnect = full refresh

            AppCommand::SwitchPrimaryScene(id) => {
                self.with_client(|c, tx, rt| {
                    rt.spawn(async move {
                        if let Err(e) = c.set_current_program_scene(&id).await {
                            let _ = tx.send(AppEvent::Error(e));
                        }
                        // OBS emits CurrentProgramSceneChanged via the event loop;
                        // no manual refresh needed.
                    });
                });
            }

            AppCommand::SetCurrentProfile(name) => {
                self.with_client(|c, tx, rt| {
                    rt.spawn(async move {
                        let result = c.set_current_profile(&name).await;
                        publish_profiles_after(result, &c, &tx).await;
                    });
                });
            }

            AppCommand::CreateProfile(name) => {
                self.with_client(|c, tx, rt| {
                    rt.spawn(async move {
                        let result = c.create_profile(&name).await;
                        publish_profiles_after(result, &c, &tx).await;
                    });
                });
            }

            AppCommand::RemoveProfile(name) => {
                self.with_client(|c, tx, rt| {
                    rt.spawn(async move {
                        let result = c.remove_profile(&name).await;
                        publish_profiles_after(result, &c, &tx).await;
                    });
                });
            }

            AppCommand::SetCurrentSceneCollection(name) => {
                let dependencies = self.dependencies.clone();
                self.with_client(|c, tx, rt| {
                    rt.spawn(async move {
                        let audio_filter =
                            load_config_blocking(dependencies).await.live.audio_inputs;
                        let result = c.set_current_scene_collection(&name).await;
                        refresh_after_collection_change(result, &c, &tx, &audio_filter).await;
                    });
                });
            }

            AppCommand::CreateSceneCollection(name) => {
                let dependencies = self.dependencies.clone();
                self.with_client(|c, tx, rt| {
                    rt.spawn(async move {
                        let audio_filter =
                            load_config_blocking(dependencies).await.live.audio_inputs;
                        let result = c.create_scene_collection(&name).await;
                        refresh_after_collection_change(result, &c, &tx, &audio_filter).await;
                    });
                });
            }

            AppCommand::SetInputMute { input, muted } => {
                self.with_client(|c, tx, rt| {
                    rt.spawn(async move {
                        if let Err(e) = c.set_input_mute(&input, muted).await {
                            let _ = tx.send(AppEvent::Error(e));
                        }
                    });
                });
            }

            AppCommand::ToggleInputMute { input } => {
                self.with_client(|c, tx, rt| {
                    rt.spawn(async move {
                        // Use the native OBS toggle — avoids a read+write round-trip
                        if let Err(e) = c.toggle_input_mute(&input).await {
                            let _ = tx.send(AppEvent::Error(e));
                        }
                    });
                });
            }

            AppCommand::SetInputVolume { input, volume_mul } => {
                self.with_client(|c, tx, rt| {
                    rt.spawn(async move {
                        if let Err(e) = c.set_input_volume(&input, volume_mul).await {
                            let _ = tx.send(AppEvent::Error(e));
                        }
                    });
                });
            }
            AppCommand::RefreshMixerSceneAudio(scene) => self.refresh_mixer_scene_audio(scene),

            AppCommand::StartStreaming => self.outputs.set_streaming(true),
            AppCommand::StopStreaming => self.outputs.set_streaming(false),
            AppCommand::SetStreaming(active) => self.outputs.set_streaming(active),

            AppCommand::StartRecording => self.outputs.set_recording(true),
            AppCommand::StopRecording => self.outputs.set_recording(false),
            AppCommand::SetRecording(active) => self.outputs.set_recording(active),
            AppCommand::RefreshOutputStatus => self.refresh_output_status(),
            AppCommand::RefreshStats => self.refresh_stats(),

            AppCommand::RefreshData => self.refresh_data(),
        }
    }

    // ── Private ───────────────────────────────────────────────────────────────

    fn refresh_data(&self) {
        let dependencies = self.dependencies.clone();
        self.with_client(|c, tx, rt| {
            rt.spawn(async move {
                let config = load_config_blocking(dependencies).await;
                refresh_profile_and_collection_lists(&c, &tx).await;
                refresh_output_statuses(&c, &tx).await;
                refresh_live_data(&c, &tx, &config.live.audio_inputs).await;
            });
        });
    }

    fn refresh_output_status(&self) {
        self.with_client(|c, tx, rt| {
            rt.spawn(async move {
                refresh_output_statuses(&c, &tx).await;
            });
        });
    }

    fn refresh_stats(&self) {
        let bitrate_sample = Arc::clone(&self.bitrate_sample);
        self.with_client(|c, tx, rt| {
            rt.spawn(async move {
                refresh_obs_stats(&c, &tx, &bitrate_sample).await;
            });
        });
    }

    fn refresh_mixer_scene_audio(&self, scene: String) {
        let dependencies = self.dependencies.clone();
        let tx = self.event_tx.clone();
        let _ = tx.send(AppEvent::MixerAudioInputsLoading {
            scene: scene.clone(),
        });

        let Some(client) = self.client_slot.lock().ok().and_then(|s| s.clone()) else {
            tracing::warn!("mixer scene audio refresh ignored — not connected to OBS");
            let _ = tx.send(AppEvent::MixerAudioInputsFailed {
                scene,
                message: fl!(LANGUAGE_LOADER, "controller-not-connected"),
            });
            return;
        };

        self.runtime.spawn(async move {
            let config = load_config_blocking(dependencies).await;
            match client
                .get_scene_audio_inputs(&scene, &config.live.audio_inputs)
                .await
            {
                Ok(inputs) => {
                    let _ = tx.send(AppEvent::MixerAudioInputsUpdated { scene, inputs });
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::MixerAudioInputsFailed {
                        scene,
                        message: e.to_string(),
                    });
                }
            }
        });
    }

    /// Clone the current client if connected, then call `f` with it.
    fn with_client<F>(&self, f: F)
    where
        F: FnOnce(ObsClient, SyncSender<AppEvent>, Handle),
    {
        match self.client_slot.lock().ok().and_then(|s| s.clone()) {
            Some(client) => f(client, self.event_tx.clone(), self.runtime.clone()),
            None => tracing::warn!("command ignored — not connected to OBS"),
        }
    }

    #[cfg(test)]
    fn set_output_client_override(&self, client: OutputCommandClient) {
        self.outputs.set_client_override(client);
    }
}

async fn load_config_blocking(
    dependencies: ControllerDependencies,
) -> crate::storage::config::AppConfig {
    tokio::task::spawn_blocking(move || dependencies.load_config())
        .await
        .unwrap_or_else(|e| {
            tracing::error!(%e, "storage worker failed while loading configuration");
            Default::default()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread::ThreadId;
    use std::time::Duration;
    use tokio::runtime::Runtime;

    fn controller_with_receiver() -> (Runtime, AppController, mpsc::Receiver<AppEvent>) {
        let runtime = Runtime::new().expect("test runtime");
        let handle = runtime.handle().clone();
        let (tx, rx) = mpsc::sync_channel(16);
        (runtime, AppController::new(handle, tx), rx)
    }

    struct ThreadRecordingConfigProvider {
        caller: ThreadId,
        observed: Arc<Mutex<Option<ThreadId>>>,
    }

    impl crate::controller::dependencies::AppConfigProvider for ThreadRecordingConfigProvider {
        fn load_config(&self) -> crate::storage::config::AppConfig {
            let worker = std::thread::current().id();
            assert_ne!(worker, self.caller);
            *self.observed.lock().expect("observed worker thread") = Some(worker);
            Default::default()
        }
    }

    struct EmptyPasswordProvider;

    impl crate::controller::dependencies::ObsPasswordProvider for EmptyPasswordProvider {
        fn obs_password(&self) -> Result<Option<String>, AppError> {
            Ok(None)
        }
    }

    #[test]
    fn config_loading_runs_on_blocking_worker() {
        let runtime = Runtime::new().expect("test runtime");
        let caller = std::thread::current().id();
        let observed = Arc::new(Mutex::new(None));
        let dependencies = ControllerDependencies::new(
            Arc::new(ThreadRecordingConfigProvider {
                caller,
                observed: Arc::clone(&observed),
            }),
            Arc::new(EmptyPasswordProvider),
        );

        runtime.block_on(load_config_blocking(dependencies));

        assert!(observed.lock().expect("observed worker thread").is_some());
    }

    #[derive(Debug)]
    struct FakeOutputClient {
        stream_result: Result<(), AppError>,
        record_result: Result<Option<String>, AppError>,
        stream_status: Result<OutputStatus, AppError>,
        record_status: Result<OutputStatus, AppError>,
        stream_commands: Arc<Mutex<Vec<bool>>>,
        record_commands: Arc<Mutex<Vec<bool>>>,
        stream_status_calls: Arc<Mutex<usize>>,
        record_status_calls: Arc<Mutex<usize>>,
    }

    impl FakeOutputClient {
        fn with_failures() -> Arc<Self> {
            Arc::new(Self {
                stream_result: Err(AppError::Request("stream backend refused".to_string())),
                record_result: Err(AppError::Request("record backend refused".to_string())),
                stream_status: Ok(OutputStatus {
                    active: false,
                    state: OutputRunState::Inactive,
                    detail: None,
                }),
                record_status: Ok(OutputStatus {
                    active: true,
                    state: OutputRunState::Active,
                    detail: Some("/tmp/current-recording.mkv".to_string()),
                }),
                stream_commands: Arc::new(Mutex::new(Vec::new())),
                record_commands: Arc::new(Mutex::new(Vec::new())),
                stream_status_calls: Arc::new(Mutex::new(0)),
                record_status_calls: Arc::new(Mutex::new(0)),
            })
        }

        fn with_status_results(
            stream_status: Result<OutputStatus, AppError>,
            record_status: Result<OutputStatus, AppError>,
        ) -> Arc<Self> {
            Arc::new(Self {
                stream_result: Err(AppError::Request("stream backend refused".to_string())),
                record_result: Err(AppError::Request("record backend refused".to_string())),
                stream_status,
                record_status,
                stream_commands: Arc::new(Mutex::new(Vec::new())),
                record_commands: Arc::new(Mutex::new(Vec::new())),
                stream_status_calls: Arc::new(Mutex::new(0)),
                record_status_calls: Arc::new(Mutex::new(0)),
            })
        }

        fn with_successful_commands(
            stream_status: OutputStatus,
            record_status: OutputStatus,
        ) -> Arc<Self> {
            Arc::new(Self {
                stream_result: Ok(()),
                record_result: Ok(None),
                stream_status: Ok(stream_status),
                record_status: Ok(record_status),
                stream_commands: Arc::new(Mutex::new(Vec::new())),
                record_commands: Arc::new(Mutex::new(Vec::new())),
                stream_status_calls: Arc::new(Mutex::new(0)),
                record_status_calls: Arc::new(Mutex::new(0)),
            })
        }

        fn stream_status_call_count(&self) -> usize {
            *self
                .stream_status_calls
                .lock()
                .expect("stream status calls")
        }

        fn record_status_call_count(&self) -> usize {
            *self
                .record_status_calls
                .lock()
                .expect("record status calls")
        }

        fn stream_commands(&self) -> Vec<bool> {
            self.stream_commands
                .lock()
                .expect("stream commands")
                .clone()
        }

        fn record_commands(&self) -> Vec<bool> {
            self.record_commands
                .lock()
                .expect("record commands")
                .clone()
        }
    }

    impl FakeOutputCommandClient for FakeOutputClient {
        fn set_streaming(&self, active: bool) -> BoxFuture<'_, Result<(), AppError>> {
            let result = self.stream_result.clone();
            let commands = Arc::clone(&self.stream_commands);
            Box::pin(async move {
                commands.lock().expect("stream commands").push(active);
                result
            })
        }

        fn set_recording(&self, active: bool) -> BoxFuture<'_, Result<Option<String>, AppError>> {
            let result = self.record_result.clone();
            let commands = Arc::clone(&self.record_commands);
            Box::pin(async move {
                commands.lock().expect("record commands").push(active);
                result
            })
        }

        fn get_stream_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>> {
            let status = self.stream_status.clone();
            let calls = Arc::clone(&self.stream_status_calls);
            Box::pin(async move {
                *calls.lock().expect("stream status calls") += 1;
                status
            })
        }

        fn get_record_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>> {
            let status = self.record_status.clone();
            let calls = Arc::clone(&self.record_status_calls);
            Box::pin(async move {
                *calls.lock().expect("record status calls") += 1;
                status
            })
        }
    }

    #[test]
    fn bitrate_sample_is_none_until_a_second_sample_arrives() {
        let sample: BitrateSample = Arc::new(Mutex::new(None));

        assert_eq!(bitrate_kbps_since_last_sample(&sample, 1_000), None);
        assert_eq!(sample.lock().unwrap().map(|(_, bytes)| bytes), Some(1_000));
    }

    #[test]
    fn bitrate_sample_returns_none_when_byte_counter_goes_backwards() {
        let sample: BitrateSample = Arc::new(Mutex::new(Some((Instant::now(), 5_000))));

        assert_eq!(bitrate_kbps_since_last_sample(&sample, 1_000), None);
        assert_eq!(sample.lock().unwrap().map(|(_, bytes)| bytes), Some(1_000));
    }

    #[test]
    fn bitrate_sample_computes_kbps_from_byte_delta_over_elapsed_time() {
        let previous_time = Instant::now() - Duration::from_secs(1);
        let sample: BitrateSample = Arc::new(Mutex::new(Some((previous_time, 0))));

        // 125_000 bytes/sec == 1_000_000 bits/sec == 1000 kbps.
        let kbps = bitrate_kbps_since_last_sample(&sample, 125_000)
            .expect("expected a bitrate sample after a prior reading");

        assert!(
            (900.0..=1100.0).contains(&kbps),
            "expected ~1000 kbps, got {kbps}"
        );
    }

    fn receive_events(rx: &mpsc::Receiver<AppEvent>, count: usize, context: &str) -> Vec<AppEvent> {
        (0..count)
            .map(|_| {
                rx.recv_timeout(Duration::from_secs(2))
                    .unwrap_or_else(|_| panic!("timed out waiting for {context} event"))
            })
            .collect()
    }

    fn inactive_status() -> OutputStatus {
        OutputStatus {
            active: false,
            state: OutputRunState::Inactive,
            detail: None,
        }
    }

    fn active_status() -> OutputStatus {
        OutputStatus {
            active: true,
            state: OutputRunState::Active,
            detail: Some("/tmp/current-recording.mkv".to_string()),
        }
    }

    fn app_state() -> crate::controller::state::AppState {
        crate::controller::state::AppState::new(
            crate::storage::config::AppConfig::default(),
            crate::storage::registry::SceneRegistry::default(),
            None,
            None,
        )
    }

    fn mark_obs_connected(state: &mut crate::controller::state::AppState) {
        state.set_obs_status(crate::controller::state::ObsStatus::Connected {
            obs_version: "32.1.2".to_string(),
        });
    }

    fn assert_obs_connected(state: &crate::controller::state::AppState) {
        assert_eq!(
            state.obs_status,
            crate::controller::state::ObsStatus::Connected {
                obs_version: "32.1.2".to_string()
            }
        );
    }

    fn apply_output_event(state: &mut crate::controller::state::AppState, event: &AppEvent) {
        match event {
            AppEvent::StreamStatusUpdated(status) => state.set_stream_status(status.clone()),
            AppEvent::RecordStatusUpdated(status) => state.set_record_status(status.clone()),
            AppEvent::StreamCommandPending(status) => {
                state.set_stream_command_pending(status.clone());
            }
            AppEvent::RecordCommandPending(status) => {
                state.set_record_command_pending(status.clone());
            }
            AppEvent::StreamCommandFailed(failure) => {
                state.set_stream_command_failure_with_recovery(failure.clone());
            }
            AppEvent::RecordCommandFailed(failure) => {
                state.set_record_command_failure_with_recovery(failure.clone());
            }
            AppEvent::StreamCommandSucceeded => state.set_stream_command_success(),
            AppEvent::RecordCommandSucceeded => state.set_record_command_success(),
            _ => {}
        }
    }

    fn assert_non_transitioning(status: &OutputStatus) {
        assert!(
            !status.state.is_transitioning(),
            "expected non-transitioning output status, got {status:?}"
        );
    }

    fn wait_for_status_calls(fake: &FakeOutputClient, stream_calls: usize, record_calls: usize) {
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            if fake.stream_status_call_count() >= stream_calls
                && fake.record_status_call_count() >= record_calls
            {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(fake.stream_status_call_count(), stream_calls);
        assert_eq!(fake.record_status_call_count(), record_calls);
    }

    #[test]
    fn start_streaming_without_client_emits_stream_command_failure() {
        let (_runtime, mut controller, rx) = controller_with_receiver();

        controller.handle(AppCommand::StartStreaming);

        let event = rx.try_recv().expect("stream failure event");
        let AppEvent::StreamCommandFailed(failure) = event else {
            panic!("expected stream command failure event");
        };
        assert_eq!(failure.message(), "Not connected to OBS");
        assert_eq!(failure.fallback_status(), &inactive_status());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn start_recording_without_client_emits_record_command_failure() {
        let (_runtime, mut controller, rx) = controller_with_receiver();

        controller.handle(AppCommand::StartRecording);

        let event = rx.try_recv().expect("record failure event");
        let AppEvent::RecordCommandFailed(failure) = event else {
            panic!("expected record command failure event");
        };
        assert_eq!(failure.message(), "Not connected to OBS");
        assert_eq!(failure.fallback_status(), &inactive_status());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn disconnect_stops_active_outputs_before_disconnected_event() {
        let (_runtime, mut controller, rx) = controller_with_receiver();
        let fake = FakeOutputClient::with_successful_commands(active_status(), active_status());
        controller.set_output_client_override(OutputCommandClient::Fake(fake.clone()));

        controller.handle(AppCommand::Disconnect);

        let event = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("disconnected event");
        assert!(matches!(event, AppEvent::Disconnected));
        assert_eq!(fake.stream_status_call_count(), 1);
        assert_eq!(fake.record_status_call_count(), 1);
        assert_eq!(fake.stream_commands(), vec![false]);
        assert_eq!(fake.record_commands(), vec![false]);
    }

    #[test]
    fn async_stream_command_failure_emits_only_stream_failure_and_refreshes_status() {
        let (_runtime, mut controller, rx) = controller_with_receiver();
        let fake = FakeOutputClient::with_failures();
        controller.set_output_client_override(OutputCommandClient::Fake(fake.clone()));

        controller.handle(AppCommand::StartStreaming);

        let events = receive_events(&rx, 4, "stream failure");
        assert!(matches!(
            events[0],
            AppEvent::StreamCommandPending(OutputStatus {
                active: false,
                state: OutputRunState::Starting,
                detail: None
            })
        ));

        let AppEvent::StreamCommandFailed(failure) = &events[1] else {
            panic!("expected stream command failure after pending event");
        };
        assert!(failure.message().contains("stream backend refused"));
        assert_eq!(failure.fallback_status(), &inactive_status());

        assert!(matches!(
            events[2],
            AppEvent::StreamStatusUpdated(OutputStatus {
                active: false,
                state: OutputRunState::Inactive,
                detail: None
            })
        ));
        assert!(matches!(
            events[3],
            AppEvent::RecordStatusUpdated(OutputStatus {
                active: true,
                state: OutputRunState::Active,
                detail: Some(_)
            })
        ));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, AppEvent::Error(_))),
            "output command failure should not emit generic AppEvent::Error"
        );
        assert_eq!(fake.stream_status_call_count(), 1);
        assert_eq!(fake.record_status_call_count(), 1);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn async_record_command_failure_emits_only_record_failure_and_refreshes_status() {
        let (_runtime, mut controller, rx) = controller_with_receiver();
        let fake = FakeOutputClient::with_failures();
        controller.set_output_client_override(OutputCommandClient::Fake(fake.clone()));

        controller.handle(AppCommand::StartRecording);

        let events = receive_events(&rx, 4, "record failure");
        assert!(matches!(
            events[0],
            AppEvent::RecordCommandPending(OutputStatus {
                active: false,
                state: OutputRunState::Starting,
                detail: None
            })
        ));

        let AppEvent::RecordCommandFailed(failure) = &events[1] else {
            panic!("expected record command failure after pending event");
        };
        assert!(failure.message().contains("record backend refused"));
        assert_eq!(failure.fallback_status(), &inactive_status());

        assert!(matches!(
            events[2],
            AppEvent::StreamStatusUpdated(OutputStatus {
                active: false,
                state: OutputRunState::Inactive,
                detail: None
            })
        ));
        assert!(matches!(
            events[3],
            AppEvent::RecordStatusUpdated(OutputStatus {
                active: true,
                state: OutputRunState::Active,
                detail: Some(_)
            })
        ));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, AppEvent::Error(_))),
            "output command failure should not emit generic AppEvent::Error"
        );
        assert_eq!(fake.stream_status_call_count(), 1);
        assert_eq!(fake.record_status_call_count(), 1);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn stream_command_failure_with_stream_status_refresh_failure_unblocks_stream() {
        let (_runtime, mut controller, rx) = controller_with_receiver();
        let fake = FakeOutputClient::with_status_results(
            Err(AppError::Request("stream status unavailable".to_string())),
            Ok(active_status()),
        );
        controller.set_output_client_override(OutputCommandClient::Fake(fake.clone()));

        controller.handle(AppCommand::StartStreaming);

        let events = receive_events(&rx, 3, "stream command failure with stream refresh failure");
        wait_for_status_calls(&fake, 1, 1);

        assert!(matches!(
            events[0],
            AppEvent::StreamCommandPending(OutputStatus {
                active: false,
                state: OutputRunState::Starting,
                detail: None
            })
        ));
        let AppEvent::StreamCommandFailed(failure) = &events[1] else {
            panic!("expected stream command failure after pending event");
        };
        assert!(failure.message().contains("stream backend refused"));
        assert_eq!(failure.fallback_status(), &inactive_status());
        assert!(matches!(events[2], AppEvent::RecordStatusUpdated(_)));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, AppEvent::Error(_))),
            "localized stream/status failure should not emit generic AppEvent::Error"
        );

        let mut state = app_state();
        mark_obs_connected(&mut state);
        state.set_record_command_failure("existing record error".to_string());
        for event in &events {
            apply_output_event(&mut state, event);
        }
        assert_obs_connected(&state);
        assert_eq!(state.stream_status, inactive_status());
        assert_non_transitioning(&state.stream_status);
        assert!(state
            .last_stream_command_error
            .as_deref()
            .is_some_and(|message| message.contains("stream backend refused")));
        assert_eq!(
            state.last_record_command_error.as_deref(),
            Some("existing record error")
        );
        assert_eq!(fake.stream_status_call_count(), 1);
        assert_eq!(fake.record_status_call_count(), 1);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn record_command_failure_with_record_status_refresh_failure_unblocks_record() {
        let (_runtime, mut controller, rx) = controller_with_receiver();
        let fake = FakeOutputClient::with_status_results(
            Ok(inactive_status()),
            Err(AppError::Request("record status unavailable".to_string())),
        );
        controller.set_output_client_override(OutputCommandClient::Fake(fake.clone()));

        controller.handle(AppCommand::StartRecording);

        let events = receive_events(&rx, 3, "record command failure with record refresh failure");
        wait_for_status_calls(&fake, 1, 1);

        assert!(matches!(
            events[0],
            AppEvent::RecordCommandPending(OutputStatus {
                active: false,
                state: OutputRunState::Starting,
                detail: None
            })
        ));
        let AppEvent::RecordCommandFailed(failure) = &events[1] else {
            panic!("expected record command failure after pending event");
        };
        assert!(failure.message().contains("record backend refused"));
        assert_eq!(failure.fallback_status(), &inactive_status());
        assert!(matches!(events[2], AppEvent::StreamStatusUpdated(_)));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, AppEvent::Error(_))),
            "localized record/status failure should not emit generic AppEvent::Error"
        );

        let mut state = app_state();
        mark_obs_connected(&mut state);
        state.set_stream_command_failure("existing stream error".to_string());
        for event in &events {
            apply_output_event(&mut state, event);
        }
        assert_obs_connected(&state);
        assert_eq!(state.record_status, inactive_status());
        assert_non_transitioning(&state.record_status);
        assert_eq!(
            state.last_stream_command_error.as_deref(),
            Some("existing stream error")
        );
        assert!(state
            .last_record_command_error
            .as_deref()
            .is_some_and(|message| message.contains("record backend refused")));
        assert_eq!(fake.stream_status_call_count(), 1);
        assert_eq!(fake.record_status_call_count(), 1);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn stream_command_failure_with_both_status_refreshes_failing_unblocks_stream_stop() {
        let (_runtime, mut controller, rx) = controller_with_receiver();
        let fake = FakeOutputClient::with_status_results(
            Err(AppError::Request("stream status unavailable".to_string())),
            Err(AppError::Request("record status unavailable".to_string())),
        );
        controller.set_output_client_override(OutputCommandClient::Fake(fake.clone()));

        controller.handle(AppCommand::StopStreaming);

        let events = receive_events(&rx, 2, "stream command failure with both refreshes failing");
        wait_for_status_calls(&fake, 1, 1);

        assert!(matches!(
            events[0],
            AppEvent::StreamCommandPending(OutputStatus {
                active: true,
                state: OutputRunState::Stopping,
                detail: None
            })
        ));
        let AppEvent::StreamCommandFailed(failure) = &events[1] else {
            panic!("expected stream command failure after pending event");
        };
        assert!(failure.message().contains("stream backend refused"));
        assert_eq!(
            failure.fallback_status(),
            &OutputStatus {
                active: true,
                state: OutputRunState::Active,
                detail: None
            }
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, AppEvent::Error(_))),
            "localized stream/status failure should not emit generic AppEvent::Error"
        );

        let mut state = app_state();
        mark_obs_connected(&mut state);
        state.set_record_command_failure("existing record error".to_string());
        for event in &events {
            apply_output_event(&mut state, event);
        }
        assert_obs_connected(&state);
        assert_eq!(
            state.stream_status,
            OutputStatus {
                active: true,
                state: OutputRunState::Active,
                detail: None
            }
        );
        assert_non_transitioning(&state.stream_status);
        assert_eq!(
            state.last_record_command_error.as_deref(),
            Some("existing record error")
        );
        assert_eq!(fake.stream_status_call_count(), 1);
        assert_eq!(fake.record_status_call_count(), 1);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn record_command_failure_with_both_status_refreshes_failing_unblocks_record_stop() {
        let (_runtime, mut controller, rx) = controller_with_receiver();
        let fake = FakeOutputClient::with_status_results(
            Err(AppError::Request("stream status unavailable".to_string())),
            Err(AppError::Request("record status unavailable".to_string())),
        );
        controller.set_output_client_override(OutputCommandClient::Fake(fake.clone()));

        controller.handle(AppCommand::StopRecording);

        let events = receive_events(&rx, 2, "record command failure with both refreshes failing");
        wait_for_status_calls(&fake, 1, 1);

        assert!(matches!(
            events[0],
            AppEvent::RecordCommandPending(OutputStatus {
                active: true,
                state: OutputRunState::Stopping,
                detail: None
            })
        ));
        let AppEvent::RecordCommandFailed(failure) = &events[1] else {
            panic!("expected record command failure after pending event");
        };
        assert!(failure.message().contains("record backend refused"));
        assert_eq!(
            failure.fallback_status(),
            &OutputStatus {
                active: true,
                state: OutputRunState::Active,
                detail: None
            }
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, AppEvent::Error(_))),
            "localized record/status failure should not emit generic AppEvent::Error"
        );

        let mut state = app_state();
        mark_obs_connected(&mut state);
        state.set_stream_command_failure("existing stream error".to_string());
        for event in &events {
            apply_output_event(&mut state, event);
        }
        assert_obs_connected(&state);
        assert_eq!(
            state.record_status,
            OutputStatus {
                active: true,
                state: OutputRunState::Active,
                detail: None
            }
        );
        assert_non_transitioning(&state.record_status);
        assert_eq!(
            state.last_stream_command_error.as_deref(),
            Some("existing stream error")
        );
        assert_eq!(fake.stream_status_call_count(), 1);
        assert_eq!(fake.record_status_call_count(), 1);
        assert!(rx.try_recv().is_err());
    }
}
