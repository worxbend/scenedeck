//! Application controller — routes commands into connection-owned tasks.
//!
//! GTK owns this controller; Tokio owns protocol work. The shared client slot
//! is installed and cleared only by its session generation. SessionScope owns
//! command cancellation, and EventSender fences both session and collection
//! identity before GTK applies a publication. No mutex guard crosses an await.

#[cfg(test)]
use futures_util::future::BoxFuture;
use i18n_embed_fl::fl;
use std::future::Future;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
#[cfg(test)]
use std::time::Instant;
use tokio::runtime::Handle;

use crate::controller::command::AppCommand;
use crate::controller::dependencies::ControllerDependencies;
use crate::controller::event::{AppEvent, FailedOperation};
use crate::controller::event_sender::EventSender;
use crate::controller::lifecycle::SessionScope;
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
use crate::infra::error::AppError;
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::obs::client::ObsClient;

#[derive(Default)]
struct RefreshQueue {
    running: bool,
    pending: bool,
}

#[derive(Default)]
struct MixerQueue {
    running: bool,
    pending: Option<(crate::controller::state::MixerAudioRequest, EventSender)>,
}

pub struct AppController {
    event_tx: EventSender,
    runtime: Handle,
    dependencies: ControllerDependencies,
    /// Shared slot written by the session task once connected, read by
    /// per-command tasks.  `None` while disconnected.
    client_slot: Arc<Mutex<Option<ObsClient>>>,
    outputs: OutputController,
    session: SessionController,
    bitrate_sample: BitrateSample,
    workspace: super::workspace::WorkspaceController,
    thumbnails: super::thumbnails::ThumbnailController,
    mixer_queue: Arc<Mutex<MixerQueue>>,
    refresh_queue: Arc<Mutex<RefreshQueue>>,
    refresh_task: Option<tokio::task::JoinHandle<()>>,
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
        let scope = SessionScope::initial(client_slot.clone());
        let mut outputs =
            OutputController::new(runtime.clone(), event_tx.clone(), Arc::clone(&client_slot));
        outputs.set_scope(scope.clone());
        let session = SessionController::new(
            runtime.clone(),
            EventSender::new(event_tx.clone()),
            dependencies.clone(),
            scope.clone(),
            Arc::clone(&bitrate_sample),
        );
        let mut workspace = super::workspace::WorkspaceController::new(
            runtime.clone(),
            event_tx.clone(),
            Arc::clone(&client_slot),
        );
        workspace.set_scope(scope);
        let thumbnails = super::thumbnails::ThumbnailController::new(
            runtime.clone(),
            event_tx.clone(),
            client_slot.clone(),
        );
        Self {
            thumbnails,
            mixer_queue: Arc::new(Mutex::new(MixerQueue::default())),
            refresh_queue: Arc::new(Mutex::new(RefreshQueue::default())),
            refresh_task: None,
            event_tx: EventSender::new(event_tx),
            runtime,
            dependencies,
            client_slot,
            outputs,
            session,
            bitrate_sample,
            workspace,
        }
    }

    pub fn handle(&mut self, cmd: AppCommand) {
        match cmd {
            AppCommand::CaptureThumbnail(request) => self.thumbnails.request(request),
            AppCommand::CancelThumbnails => self.thumbnails.cancel(),
            AppCommand::ReadWorkspace(request) => self.workspace.request(request, None, None),
            AppCommand::WorkspaceAction {
                request,
                action,
                expected_text,
            } => self.workspace.request(request, Some(action), expected_text),
            AppCommand::CancelWorkspace => self.workspace.cancel(),
            AppCommand::RefreshWorkspaceMedia(request) => self.workspace.media(request),
            AppCommand::Connect => {
                self.thumbnails.cancel();
                self.workspace.cancel();
                if let Some(client) = self.connected_client() {
                    client.retire();
                }
                self.session.connect();
                self.sync_session_scope();
            }
            AppCommand::Disconnect => {
                self.thumbnails.cancel();
                self.workspace.cancel();
                self.session.disconnect(&self.outputs);
                self.sync_session_scope();
            }

            AppCommand::SwitchPrimaryScene(id) => {
                self.spawn_with_client(|c, tx| async move {
                    let action = crate::domain::control::ControlAction::SetProgram(id);
                    let result = c
                        .execute_control_if(&action, &|| tx.accepts_current_scope())
                        .await;
                    report_err(result, &tx).await;
                    // OBS emits CurrentProgramSceneChanged via the event loop;
                    // no manual refresh needed.
                });
            }

            AppCommand::SetCurrentProfile(name) => {
                self.spawn_with_client(|c, tx| async move {
                    let result = c.set_current_profile(&name).await;
                    publish_profiles_after(result, &c, &tx).await;
                });
            }

            AppCommand::SetCurrentSceneCollection(name) => {
                self.thumbnails.cancel();
                self.workspace.cancel();
                let dependencies = self.dependencies.clone();
                self.spawn_with_client(|c, tx| async move {
                    let result = c.set_current_scene_collection(&name).await;
                    let audio_filter = load_config_blocking(dependencies).await.live.audio_inputs;
                    refresh_after_collection_change(result, &c, &tx, &audio_filter).await;
                });
            }

            AppCommand::SetInputMute {
                scope,
                input,
                muted,
            } => {
                self.workspace.audio(
                    scope,
                    crate::domain::control::ControlAction::SetInputMuted { input, muted },
                );
            }

            AppCommand::SetInputVolume {
                scope,
                input,
                volume_mul,
            } => {
                let db = if volume_mul > 0.0 {
                    20.0 * volume_mul.log10()
                } else {
                    -100.0
                };
                self.workspace.audio(
                    scope,
                    crate::domain::control::ControlAction::SetInputVolumeDb {
                        input,
                        db: db.clamp(-100.0, 0.0),
                    },
                );
            }
            AppCommand::RefreshMixerSceneAudio(scene) => self.refresh_mixer_scene_audio(scene),

            AppCommand::StartStreaming => self.outputs.set_streaming(true),
            AppCommand::StopStreaming => self.outputs.set_streaming(false),

            AppCommand::StartRecording => self.outputs.set_recording(true),
            AppCommand::StopRecording => self.outputs.set_recording(false),
            AppCommand::RefreshStats => self.refresh_stats(),

            AppCommand::RefreshData => self.refresh_data(),
        }
    }

    // ── Private ───────────────────────────────────────────────────────────────

    fn refresh_data(&mut self) {
        {
            let mut queue = self.refresh_queue.lock().expect("refresh queue");
            queue.pending = true;
            if queue.running {
                return;
            }
            queue.running = true;
        }
        let queue = self.refresh_queue.clone();
        let dependencies = self.dependencies.clone();
        let client = self.connected_client();
        let scope = self.session.scope();
        let tx = self.event_tx.scoped(&scope);
        // This is one controller-owned worker, not optional command admission.
        // Even a disconnected/retired scope must finish Doctor's loading state.
        let worker_scope = scope.clone();
        let worker = async move {
            let scope = worker_scope;
            loop {
                {
                    let mut queue = queue.lock().expect("refresh queue");
                    if !std::mem::take(&mut queue.pending) {
                        queue.running = false;
                        return;
                    }
                }
                let Some(client) = client.as_ref().filter(|_| scope.is_active()) else {
                    let _ = tx
                        .send(AppEvent::GraphRefreshRejected(AppError::Connection(fl!(
                            LANGUAGE_LOADER,
                            "controller-not-connected"
                        ))))
                        .await;
                    queue.lock().expect("refresh queue").running = false;
                    return;
                };
                let scan = async {
                    let config = load_config_blocking(dependencies.clone()).await;
                    let tx = tx.fresh();
                    refresh_profile_and_collection_lists(client, &tx).await;
                    refresh_output_statuses(client, &tx).await;
                    refresh_live_data(client, &tx, &config.live.audio_inputs).await;
                };
                tokio::select! {
                    biased;
                    () = scope.cancelled() => {
                        let _ = tx.send(AppEvent::GraphRefreshRejected(AppError::Connection(fl!(LANGUAGE_LOADER, "controller-not-connected")))).await;
                        let mut queue = queue.lock().expect("refresh queue");
                        queue.running = false;
                        queue.pending = false;
                        return;
                    },
                    () = scan => {}
                }
            }
        };
        self.refresh_task = if scope.is_active() {
            // A reserved single worker is retained by the session until after
            // disconnect drainage. Scope retirement then cancels and joins it.
            scope.spawn_reserved(&self.runtime, worker)
        } else {
            // No request can be admitted here; still finish Doctor's loading.
            Some(self.runtime.spawn(worker))
        };
    }

    fn refresh_stats(&self) {
        let bitrate_sample = Arc::clone(&self.bitrate_sample);
        self.spawn_with_client(|c, tx| async move {
            refresh_obs_stats(&c, &tx, &bitrate_sample).await;
        });
    }

    fn refresh_mixer_scene_audio(&self, request: crate::controller::state::MixerAudioRequest) {
        let tx = self.event_tx.scoped(&self.session.scope());
        {
            let mut queued = self.mixer_queue.lock().expect("mixer queue");
            queued.pending = Some((request, tx));
            if queued.running {
                return;
            }
            queued.running = true;
        }
        let dependencies = self.dependencies.clone();
        let client = self.connected_client();
        let queued = self.mixer_queue.clone();
        self.session
            .scope()
            .spawn_reserved(&self.runtime, async move {
                loop {
                    let job = {
                        let mut queued = queued.lock().expect("mixer queue");
                        let job = queued.pending.take();
                        if job.is_none() {
                            queued.running = false;
                        }
                        job
                    };
                    let Some((request, tx)) = job else {
                        break;
                    };
                    let Some(client) = &client else {
                        let _ = tx
                            .send(AppEvent::MixerAudioInputsFailed {
                                request,
                                message: fl!(LANGUAGE_LOADER, "controller-not-connected"),
                            })
                            .await;
                        continue;
                    };
                    let config = load_config_blocking(dependencies.clone()).await;
                    let revision = client.next_observation();
                    match client
                        .get_scene_audio_inputs(request.scene(), &config.live.audio_inputs)
                        .await
                    {
                        Ok(inputs) => {
                            let _ = tx
                                .send(AppEvent::Observation {
                                    revision,
                                    event: Box::new(AppEvent::MixerAudioInputsUpdated {
                                        request,
                                        inputs,
                                    }),
                                })
                                .await;
                        }
                        Err(error) => {
                            let _ = tx
                                .send(AppEvent::MixerAudioInputsFailed {
                                    request,
                                    message: error.user_message(),
                                })
                                .await;
                        }
                    }
                }
            });
    }

    fn sync_session_scope(&mut self) {
        // Active refresh workers belong to their SessionScope. Disconnect must
        // retain admitted reads until drainage, not abort them while rebinding
        // controller queues. Connect already retires the old scope first.
        self.refresh_task.take();
        self.refresh_queue = Arc::new(Mutex::new(RefreshQueue::default()));
        self.mixer_queue = Arc::new(Mutex::new(MixerQueue::default()));
        let scope = self.session.scope();
        self.outputs.set_scope(scope.clone());
        self.workspace.set_scope(scope);
    }

    pub(crate) fn accepts_scope(&self, generation: u64, collection: Option<u64>) -> bool {
        let scope = self.session.scope();
        scope.owns_generation(generation)
            && collection.is_none_or(|epoch| scope.is_active() && scope.collection() == epoch)
    }

    pub(crate) fn shutdown(&mut self) -> tokio::task::JoinHandle<()> {
        if let Some(task) = self.refresh_task.take() {
            task.abort();
        }
        self.thumbnails.cancel();
        self.workspace.cancel();
        if let Some(client) = self.connected_client() {
            client.retire();
        }
        self.session.shutdown()
    }

    /// Clone the current client if connected, then call `f` with it.
    /// The live OBS client, if a session is currently connected.
    ///
    /// The lock is taken and released here; the returned client is a cheap
    /// handle, so nothing holds the mutex across an await.
    fn connected_client(&self) -> Option<ObsClient> {
        self.client_slot.lock().ok().and_then(|slot| slot.clone())
    }

    /// Run `f` against the connected client and spawn the future it builds.
    ///
    /// `f` itself runs here on the GTK thread and only builds the future; the
    /// future is what crosses to the Tokio runtime, which is why `Send` is
    /// required of `Fut` and not of `f`. Commands arriving while disconnected
    /// are dropped with a warning.
    fn spawn_with_client<F, Fut>(&self, f: F)
    where
        F: FnOnce(ObsClient, EventSender) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        match self.connected_client() {
            Some(client) => {
                let scope = self.session.scope();
                scope.spawn(&self.runtime, f(client, self.event_tx.scoped(&scope)));
            }
            None => tracing::warn!("command ignored — not connected to OBS"),
        }
    }

    #[cfg(test)]
    fn set_output_client_override(&self, client: OutputCommandClient) {
        self.outputs.set_client_override(client);
    }
}

impl Drop for AppController {
    fn drop(&mut self) {
        if let Some(task) = self.refresh_task.take() {
            task.abort();
        }
    }
}

/// Forward a failed OBS call to the UI as an error event.
///
/// Generic over the success type on purpose: some of these calls return a
/// value the caller does not need (`toggle_input_mute` reports the resulting
/// mute state), and a `Result<(), _>` signature would not accept them.
async fn report_err<T>(result: Result<T, AppError>, tx: &EventSender) {
    if let Err(error) = result {
        let _ = tx
            .send(AppEvent::CommandFailed {
                operation: FailedOperation::Command,
                error,
            })
            .await;
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
#[path = "final_regression_tests.rs"]
mod final_regression_tests;

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

        let event = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("stream failure event");
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

        let event = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("record failure event");
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
        assert!(
            matches!(event, AppEvent::SessionEvent { event, .. } if matches!(*event, AppEvent::Disconnected))
        );
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
