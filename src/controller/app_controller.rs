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

use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use futures_util::future::BoxFuture;
use futures_util::StreamExt;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

use crate::controller::command::AppCommand;
use crate::controller::dependencies::ControllerDependencies;
use crate::controller::event::{
    fallback_status_after_failed_output_command, AppEvent, OutputCommandFailureRecovery,
};
use crate::domain::output::{OutputRunState, OutputStatus};
use crate::infra::error::AppError;
use crate::obs::client::ObsClient;

/// Last (timestamp, cumulative bytes) sample used to derive stream bitrate
/// between successive `GetStats`/`GetStreamStatus` polls.
type BitrateSample = Arc<Mutex<Option<(Instant, u64)>>>;

pub struct AppController {
    event_tx: SyncSender<AppEvent>,
    runtime: Handle,
    dependencies: ControllerDependencies,
    /// Session task handle — aborted on disconnect or reconnect.
    session: Option<JoinHandle<()>>,
    /// Shared slot written by the session task once connected, read by
    /// per-command tasks.  `None` while disconnected.
    client_slot: Arc<Mutex<Option<ObsClient>>>,
    #[cfg(test)]
    output_client_override: Arc<Mutex<Option<OutputCommandClient>>>,
    output_pending: Arc<Mutex<OutputPending>>,
    bitrate_sample: BitrateSample,
}

#[derive(Debug, Default)]
struct OutputPending {
    stream: bool,
    record: bool,
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
        Self {
            event_tx,
            runtime,
            dependencies,
            session: None,
            client_slot: Arc::new(Mutex::new(None)),
            #[cfg(test)]
            output_client_override: Arc::new(Mutex::new(None)),
            output_pending: Arc::new(Mutex::new(OutputPending::default())),
            bitrate_sample: Arc::new(Mutex::new(None)),
        }
    }

    pub fn handle(&mut self, cmd: AppCommand) {
        match cmd {
            AppCommand::Connect => self.connect(),
            AppCommand::Disconnect => self.disconnect(),
            AppCommand::RefreshAll => self.connect(), // reconnect = full refresh

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
                let audio_filter = self.dependencies.load_config().live.audio_inputs;
                self.with_client(|c, tx, rt| {
                    rt.spawn(async move {
                        let result = c.set_current_scene_collection(&name).await;
                        refresh_after_collection_change(result, &c, &tx, &audio_filter).await;
                    });
                });
            }

            AppCommand::CreateSceneCollection(name) => {
                let audio_filter = self.dependencies.load_config().live.audio_inputs;
                self.with_client(|c, tx, rt| {
                    rt.spawn(async move {
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

            AppCommand::StartStreaming => self.set_streaming(true),
            AppCommand::StopStreaming => self.set_streaming(false),
            AppCommand::ToggleStreaming => self.with_client(|_c, _tx, _rt| {
                // Kept for future state-aware controller routing. The current
                // Live page dispatches explicit start/stop based on AppState.
                tracing::warn!("toggle streaming command ignored without controller-side state");
            }),
            AppCommand::SetStreaming(active) => self.set_streaming(active),

            AppCommand::StartRecording => self.set_recording(true),
            AppCommand::StopRecording => self.set_recording(false),
            AppCommand::ToggleRecording => self.with_client(|_c, _tx, _rt| {
                tracing::warn!("toggle recording command ignored without controller-side state");
            }),
            AppCommand::SetRecording(active) => self.set_recording(active),
            AppCommand::RefreshOutputStatus => self.refresh_output_status(),
            AppCommand::RefreshStats => self.refresh_stats(),

            AppCommand::RefreshData => self.refresh_data(),

            AppCommand::RunDoctor | AppCommand::SetSceneRole { .. } => {
                tracing::debug!(?cmd, "command not yet implemented");
            }
        }
    }

    // ── Private ───────────────────────────────────────────────────────────────

    fn connect(&mut self) {
        if let Some(h) = self.session.take() {
            h.abort();
        }
        // Clear any stale client handle from the previous session
        if let Ok(mut slot) = self.client_slot.lock() {
            *slot = None;
        }
        if let Ok(mut sample) = self.bitrate_sample.lock() {
            *sample = None;
        }

        let tx = self.event_tx.clone();
        let config = self.dependencies.load_config();
        let client_slot = Arc::clone(&self.client_slot);

        let _ = tx.send(AppEvent::Connecting);

        // Read the password synchronously on the GTK thread before spawning;
        // the keyring API is blocking and this avoids it on the async runtime.
        let password = self.dependencies.obs_password().unwrap_or_else(|e| {
            tracing::warn!(%e, "could not read OBS password from keyring");
            None
        });

        let task = self.runtime.spawn(async move {
            match ObsClient::connect(&config.obs.host, config.obs.port, password.as_deref()).await {
                Err(e) => {
                    let _ = tx.send(AppEvent::Error(e));
                }
                Ok((client, events)) => {
                    // Confirm auth by requesting the version
                    match client.get_version().await {
                        Ok(info) => {
                            let _ = tx.send(AppEvent::Connected(info));
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::Error(e));
                            return;
                        }
                    }

                    // Publish the client so GTK-side per-command tasks can use it
                    if let Ok(mut slot) = client_slot.lock() {
                        *slot = Some(client.clone());
                    }

                    refresh_profile_and_collection_lists(&client, &tx).await;
                    refresh_output_statuses(&client, &tx).await;

                    // Initial scene inventory
                    refresh_live_data(&client, &tx, &config.live.audio_inputs).await;

                    // Block here until OBS disconnects or the task is aborted
                    run_event_loop(client, events, tx, config.live.audio_inputs).await;

                    // Clear the shared client on clean exit
                    if let Ok(mut slot) = client_slot.lock() {
                        *slot = None;
                    }
                }
            }
        });

        self.session = Some(task);
    }

    fn refresh_data(&self) {
        let config = self.dependencies.load_config();
        self.with_client(|c, tx, rt| {
            rt.spawn(async move {
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
        let config = self.dependencies.load_config();
        let tx = self.event_tx.clone();
        let _ = tx.send(AppEvent::MixerAudioInputsLoading {
            scene: scene.clone(),
        });

        let Some(client) = self.client_slot.lock().ok().and_then(|s| s.clone()) else {
            tracing::warn!("mixer scene audio refresh ignored — not connected to OBS");
            let _ = tx.send(AppEvent::MixerAudioInputsFailed {
                scene,
                message: "Not connected to OBS".to_string(),
            });
            return;
        };

        self.runtime.spawn(async move {
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

    fn set_streaming(&self, active: bool) {
        if !mark_stream_pending(&self.output_pending) {
            tracing::warn!(
                active,
                "stream command ignored while previous operation is pending"
            );
            return;
        }

        let Some(client) = self.output_command_client() else {
            clear_stream_pending(&self.output_pending);
            tracing::warn!("stream command ignored — not connected to OBS");
            let failure = OutputCommandFailureRecovery::with_failed_command_fallback_status(
                "Not connected to OBS".to_string(),
                OutputStatus {
                    active: false,
                    state: OutputRunState::Inactive,
                    detail: None,
                },
            );
            let _ = self.event_tx.send(AppEvent::StreamCommandFailed(failure));
            return;
        };

        let tx = self.event_tx.clone();
        let pending = Arc::clone(&self.output_pending);
        let transition_state = if active {
            OutputRunState::Starting
        } else {
            OutputRunState::Stopping
        };
        let pending_status = OutputStatus {
            active: !active,
            state: transition_state,
            detail: None,
        };
        let fallback_failure_status = fallback_status_after_failed_output_command(&pending_status);
        let _ = tx.send(AppEvent::StreamCommandPending(pending_status));

        self.runtime.spawn(async move {
            match client.set_streaming(active).await {
                Ok(()) => {
                    let _ = tx.send(AppEvent::StreamCommandSucceeded);
                    refresh_output_statuses(&client, &tx).await;
                }
                Err(e) => {
                    let message = e.to_string();
                    tracing::warn!(%e, active, "stream command failed");
                    let failure = OutputCommandFailureRecovery::with_failed_command_fallback_status(
                        message,
                        fallback_failure_status,
                    );
                    let _ = tx.send(AppEvent::StreamCommandFailed(failure));
                    refresh_output_statuses(&client, &tx).await;
                }
            }
            clear_stream_pending(&pending);
        });
    }

    fn set_recording(&self, active: bool) {
        if !mark_record_pending(&self.output_pending) {
            tracing::warn!(
                active,
                "record command ignored while previous operation is pending"
            );
            return;
        }

        let Some(client) = self.output_command_client() else {
            clear_record_pending(&self.output_pending);
            tracing::warn!("record command ignored — not connected to OBS");
            let failure = OutputCommandFailureRecovery::with_failed_command_fallback_status(
                "Not connected to OBS".to_string(),
                OutputStatus {
                    active: false,
                    state: OutputRunState::Inactive,
                    detail: None,
                },
            );
            let _ = self.event_tx.send(AppEvent::RecordCommandFailed(failure));
            return;
        };

        let tx = self.event_tx.clone();
        let pending = Arc::clone(&self.output_pending);
        let transition_state = if active {
            OutputRunState::Starting
        } else {
            OutputRunState::Stopping
        };
        let pending_status = OutputStatus {
            active: !active,
            state: transition_state,
            detail: None,
        };
        let fallback_failure_status = fallback_status_after_failed_output_command(&pending_status);
        let _ = tx.send(AppEvent::RecordCommandPending(pending_status));

        self.runtime.spawn(async move {
            match client.set_recording(active).await {
                Ok(path) => {
                    let _ = tx.send(AppEvent::RecordCommandSucceeded);
                    refresh_output_statuses(&client, &tx).await;
                    if let Some(path) = path {
                        let _ = tx.send(AppEvent::RecordStatusUpdated(OutputStatus {
                            active: false,
                            state: OutputRunState::Inactive,
                            detail: Some(path),
                        }));
                    }
                }
                Err(e) => {
                    let message = e.to_string();
                    tracing::warn!(%e, active, "record command failed");
                    let failure = OutputCommandFailureRecovery::with_failed_command_fallback_status(
                        message,
                        fallback_failure_status,
                    );
                    let _ = tx.send(AppEvent::RecordCommandFailed(failure));
                    refresh_output_statuses(&client, &tx).await;
                }
            }
            clear_record_pending(&pending);
        });
    }

    fn disconnect(&mut self) {
        let session = self.session.take();
        let client = self.output_command_client();
        let client_slot = Arc::clone(&self.client_slot);
        let output_pending = Arc::clone(&self.output_pending);
        let tx = self.event_tx.clone();

        if let Ok(mut sample) = self.bitrate_sample.lock() {
            *sample = None;
        }

        self.runtime.spawn(async move {
            if let Some(client) = client {
                stop_active_outputs_before_disconnect(&client).await;
            }

            if let Some(session) = session {
                session.abort();
            }
            if let Ok(mut slot) = client_slot.lock() {
                *slot = None;
            }
            if let Ok(mut pending) = output_pending.lock() {
                pending.stream = false;
                pending.record = false;
            }
            let _ = tx.send(AppEvent::Disconnected);
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

    fn output_command_client(&self) -> Option<OutputCommandClient> {
        #[cfg(test)]
        if let Some(client) = self
            .output_client_override
            .lock()
            .ok()
            .and_then(|s| s.clone())
        {
            return Some(client);
        }

        self.client_slot
            .lock()
            .ok()
            .and_then(|s| s.clone())
            .map(OutputCommandClient::Obs)
    }

    #[cfg(test)]
    fn set_output_client_override(&self, client: OutputCommandClient) {
        if let Ok(mut slot) = self.output_client_override.lock() {
            *slot = Some(client);
        }
    }
}

#[derive(Clone)]
enum OutputCommandClient {
    Obs(ObsClient),
    #[cfg(test)]
    Fake(Arc<dyn FakeOutputCommandClient>),
}

impl OutputCommandClient {
    async fn set_streaming(&self, active: bool) -> Result<(), AppError> {
        match self {
            Self::Obs(client) => client.set_streaming(active).await,
            #[cfg(test)]
            Self::Fake(client) => client.set_streaming(active).await,
        }
    }

    async fn set_recording(&self, active: bool) -> Result<Option<String>, AppError> {
        match self {
            Self::Obs(client) => client.set_recording(active).await,
            #[cfg(test)]
            Self::Fake(client) => client.set_recording(active).await,
        }
    }

    async fn get_stream_status(&self) -> Result<OutputStatus, AppError> {
        match self {
            Self::Obs(client) => client.get_stream_status().await,
            #[cfg(test)]
            Self::Fake(client) => client.get_stream_status().await,
        }
    }

    async fn get_record_status(&self) -> Result<OutputStatus, AppError> {
        match self {
            Self::Obs(client) => client.get_record_status().await,
            #[cfg(test)]
            Self::Fake(client) => client.get_record_status().await,
        }
    }
}

trait OutputStatusReader {
    fn read_stream_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>>;
    fn read_record_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>>;
}

impl OutputStatusReader for ObsClient {
    fn read_stream_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>> {
        Box::pin(async move { self.get_stream_status().await })
    }

    fn read_record_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>> {
        Box::pin(async move { self.get_record_status().await })
    }
}

impl OutputStatusReader for OutputCommandClient {
    fn read_stream_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>> {
        Box::pin(async move { self.get_stream_status().await })
    }

    fn read_record_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>> {
        Box::pin(async move { self.get_record_status().await })
    }
}

#[cfg(test)]
trait FakeOutputCommandClient: Send + Sync {
    fn set_streaming(&self, active: bool) -> BoxFuture<'_, Result<(), AppError>>;
    fn set_recording(&self, active: bool) -> BoxFuture<'_, Result<Option<String>, AppError>>;
    fn get_stream_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>>;
    fn get_record_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>>;
}

fn mark_stream_pending(pending: &Arc<Mutex<OutputPending>>) -> bool {
    let Ok(mut pending) = pending.lock() else {
        return false;
    };
    if pending.stream {
        return false;
    }
    pending.stream = true;
    true
}

fn clear_stream_pending(pending: &Arc<Mutex<OutputPending>>) {
    if let Ok(mut pending) = pending.lock() {
        pending.stream = false;
    }
}

fn mark_record_pending(pending: &Arc<Mutex<OutputPending>>) -> bool {
    let Ok(mut pending) = pending.lock() else {
        return false;
    };
    if pending.record {
        return false;
    }
    pending.record = true;
    true
}

fn clear_record_pending(pending: &Arc<Mutex<OutputPending>>) {
    if let Ok(mut pending) = pending.lock() {
        pending.record = false;
    }
}

async fn stop_active_outputs_before_disconnect(client: &OutputCommandClient) {
    match client.get_stream_status().await {
        Ok(status) if status.active => {
            if let Err(e) = client.set_streaming(false).await {
                tracing::warn!(%e, "failed to stop active stream before disconnect");
            }
        }
        Ok(_) => {}
        Err(e) => tracing::warn!(%e, "could not read stream status before disconnect"),
    }

    match client.get_record_status().await {
        Ok(status) if status.active => {
            if let Err(e) = client.set_recording(false).await {
                tracing::warn!(%e, "failed to stop active recording before disconnect");
            }
        }
        Ok(_) => {}
        Err(e) => tracing::warn!(%e, "could not read recording status before disconnect"),
    }
}

// ── Shared OBS refresh helpers ────────────────────────────────────────────────

async fn publish_profiles_after(
    operation: Result<(), AppError>,
    client: &ObsClient,
    tx: &SyncSender<AppEvent>,
) {
    match operation {
        Ok(()) => match client.get_profiles().await {
            Ok(profiles) => {
                let _ = tx.send(AppEvent::ProfilesUpdated(profiles));
            }
            Err(e) => {
                let _ = tx.send(AppEvent::Error(e));
            }
        },
        Err(e) => {
            let _ = tx.send(AppEvent::Error(e));
        }
    }
}

async fn refresh_after_collection_change(
    operation: Result<(), AppError>,
    client: &ObsClient,
    tx: &SyncSender<AppEvent>,
    audio_filter: &[String],
) {
    match operation {
        Ok(()) => {
            refresh_profile_and_collection_lists(client, tx).await;
            refresh_live_data(client, tx, audio_filter).await;
        }
        Err(e) => {
            let _ = tx.send(AppEvent::Error(e));
        }
    }
}

async fn refresh_profile_and_collection_lists(client: &ObsClient, tx: &SyncSender<AppEvent>) {
    match client.get_profiles().await {
        Ok(profiles) => {
            let _ = tx.send(AppEvent::ProfilesUpdated(profiles));
        }
        Err(e) => tracing::warn!(%e, "profile list refresh failed"),
    }

    match client.get_scene_collections().await {
        Ok(collections) => {
            let _ = tx.send(AppEvent::SceneCollectionsUpdated(collections));
        }
        Err(e) => tracing::warn!(%e, "scene collection list refresh failed"),
    }
}

async fn refresh_output_statuses(client: &impl OutputStatusReader, tx: &SyncSender<AppEvent>) {
    match client.read_stream_status().await {
        Ok(status) => {
            let _ = tx.send(AppEvent::StreamStatusUpdated(status));
        }
        Err(e) => tracing::warn!(%e, "stream status refresh failed"),
    }

    match client.read_record_status().await {
        Ok(status) => {
            let _ = tx.send(AppEvent::RecordStatusUpdated(status));
        }
        Err(e) => tracing::warn!(%e, "record status refresh failed"),
    }
}

/// Poll `GetStats` plus the stream byte counter and publish a combined
/// `StatsUpdated` event. Bitrate is derived from the delta in stream bytes
/// against the previous sample, so it is `None` until a second sample lands
/// and whenever the byte counter resets (e.g. the stream just (re)started).
async fn refresh_obs_stats(
    client: &ObsClient,
    tx: &SyncSender<AppEvent>,
    bitrate_sample: &BitrateSample,
) {
    let stats = match client.get_obs_stats().await {
        Ok(stats) => stats,
        Err(e) => {
            tracing::warn!(%e, "obs stats refresh failed");
            return;
        }
    };

    let bitrate_kbps = match client.get_stream_bytes().await {
        Ok(bytes) => bitrate_kbps_since_last_sample(bitrate_sample, bytes),
        Err(e) => {
            tracing::debug!(%e, "stream byte counter refresh failed");
            None
        }
    };

    let _ = tx.send(AppEvent::StatsUpdated {
        stats,
        bitrate_kbps,
    });
}

/// Compute kbps from the delta against the last (time, bytes) sample, then
/// store `bytes` as the new sample. Returns `None` on the first sample or
/// whenever the counter goes backwards (stream restarted).
fn bitrate_kbps_since_last_sample(bitrate_sample: &BitrateSample, bytes: u64) -> Option<f64> {
    let now = Instant::now();
    let mut sample = bitrate_sample.lock().ok()?;
    let previous = *sample;
    *sample = Some((now, bytes));

    let (prev_time, prev_bytes) = previous?;
    if bytes < prev_bytes {
        return None;
    }

    let elapsed_secs = now.duration_since(prev_time).as_secs_f64();
    if elapsed_secs <= 0.0 {
        return None;
    }

    Some((bytes - prev_bytes) as f64 * 8.0 / elapsed_secs / 1000.0)
}

async fn refresh_live_data(client: &ObsClient, tx: &SyncSender<AppEvent>, audio_filter: &[String]) {
    match client.get_scene_inventory().await {
        Ok(inv) => {
            let scene_names: Vec<_> = inv.scenes.iter().map(|s| s.id.clone()).collect();
            let current_scene = inv.current_id.clone();
            let _ = tx.send(AppEvent::SceneInventoryUpdated(inv));

            refresh_scene_audio(client, tx, current_scene.as_deref(), audio_filter).await;

            if !scene_names.is_empty() {
                match client.get_scene_graph(&scene_names).await {
                    Ok(graph) => {
                        let _ = tx.send(AppEvent::GraphUpdated(graph));
                    }
                    Err(e) => tracing::warn!(%e, "graph refresh failed"),
                }
            }
        }
        Err(e) => tracing::warn!(%e, "scene inventory refresh failed"),
    }
}

async fn refresh_current_scene_audio(
    client: &ObsClient,
    tx: &SyncSender<AppEvent>,
    audio_filter: &[String],
) {
    match client.get_scene_inventory().await {
        Ok(inv) => refresh_scene_audio(client, tx, inv.current_id.as_deref(), audio_filter).await,
        Err(e) => tracing::warn!(%e, "current scene lookup for audio refresh failed"),
    }
}

async fn refresh_scene_audio(
    client: &ObsClient,
    tx: &SyncSender<AppEvent>,
    scene_name: Option<&str>,
    audio_filter: &[String],
) {
    let Some(scene_name) = scene_name else {
        let _ = tx.send(AppEvent::AudioInputsUpdated(Vec::new()));
        return;
    };

    match client
        .get_scene_audio_inputs(scene_name, audio_filter)
        .await
    {
        Ok(inputs) => {
            let _ = tx.send(AppEvent::AudioInputsUpdated(inputs));
        }
        Err(e) => tracing::warn!(%e, scene_name, "scene audio refresh failed"),
    }
}

// ── Session event loop ────────────────────────────────────────────────────────

async fn run_event_loop(
    client: ObsClient,
    events: obws::events::EventStream,
    tx: SyncSender<AppEvent>,
    audio_filter: Vec<String>,
) {
    use obws::events::Event;

    let mut events = events;

    while let Some(event) = events.next().await {
        let app_event = match event {
            Event::StreamStateChanged { active, state } => Some(AppEvent::StreamStatusUpdated(
                output_status_from_event(active, state, None),
            )),
            Event::RecordStateChanged {
                active,
                state,
                path,
            } => Some(AppEvent::RecordStatusUpdated(output_status_from_event(
                active, state, path,
            ))),
            Event::RecordFileChanged { path } => {
                Some(AppEvent::RecordStatusUpdated(OutputStatus {
                    active: true,
                    state: OutputRunState::Active,
                    detail: Some(path),
                }))
            }
            Event::CurrentProgramSceneChanged { id } => {
                let _ = tx.send(AppEvent::CurrentSceneChanged(id.name.clone()));
                refresh_scene_audio(&client, &tx, Some(&id.name), &audio_filter).await;
                None
            }
            Event::InputMuteStateChanged { id, muted } => Some(AppEvent::InputMuteChanged {
                input: id.name.clone(),
                muted,
            }),
            Event::InputVolumeChanged { id, mul, db } => Some(AppEvent::InputVolumeChanged {
                input: id.name.clone(),
                volume_mul: mul,
                volume_db: db,
            }),
            Event::InputCreated { .. }
            | Event::InputRemoved { .. }
            | Event::InputNameChanged { .. } => {
                refresh_current_scene_audio(&client, &tx, &audio_filter).await;
                None
            }
            Event::SceneItemCreated { .. }
            | Event::SceneItemRemoved { .. }
            | Event::SceneItemListReindexed { .. }
            | Event::SceneItemEnableStateChanged { .. } => {
                refresh_current_scene_audio(&client, &tx, &audio_filter).await;
                None
            }
            Event::CurrentProfileChanged { name } => {
                match client.get_profiles().await {
                    Ok(mut profiles) => {
                        profiles.current = Some(name);
                        let _ = tx.send(AppEvent::ProfilesUpdated(profiles));
                    }
                    Err(e) => tracing::warn!(%e, "profile list refresh failed"),
                }
                None
            }
            Event::ProfileListChanged { profiles } => {
                match client.get_profiles().await {
                    Ok(mut list) => {
                        if list.items.is_empty() {
                            list.items = profiles;
                        }
                        let _ = tx.send(AppEvent::ProfilesUpdated(list));
                    }
                    Err(e) => tracing::warn!(%e, "profile list refresh failed"),
                }
                None
            }
            Event::CurrentSceneCollectionChanged { name } => {
                match client.get_scene_collections().await {
                    Ok(mut collections) => {
                        collections.current = Some(name);
                        let _ = tx.send(AppEvent::SceneCollectionsUpdated(collections));
                    }
                    Err(e) => tracing::warn!(%e, "scene collection list refresh failed"),
                }
                refresh_live_data(&client, &tx, &audio_filter).await;
                None
            }
            Event::SceneCollectionListChanged { collections } => {
                match client.get_scene_collections().await {
                    Ok(mut list) => {
                        if list.items.is_empty() {
                            list.items = collections;
                        }
                        let _ = tx.send(AppEvent::SceneCollectionsUpdated(list));
                    }
                    Err(e) => tracing::warn!(%e, "scene collection list refresh failed"),
                }
                None
            }
            // Re-fetch inventory + graph whenever the scene list changes
            Event::SceneListChanged { .. } => {
                refresh_live_data(&client, &tx, &audio_filter).await;
                None
            }
            _ => None,
        };

        if let Some(ev) = app_event {
            if tx.send(ev).is_err() {
                break; // UI gone — exit cleanly
            }
        }
    }

    let _ = tx.send(AppEvent::Disconnected);
}

fn output_status_from_event(
    active: bool,
    state: obws::events::OutputState,
    detail: Option<String>,
) -> OutputStatus {
    let state = match state {
        obws::events::OutputState::Starting => OutputRunState::Starting,
        obws::events::OutputState::Started => OutputRunState::Active,
        obws::events::OutputState::Stopping => OutputRunState::Stopping,
        obws::events::OutputState::Stopped => OutputRunState::Inactive,
        obws::events::OutputState::Reconnecting => OutputRunState::Reconnecting,
        obws::events::OutputState::Reconnected => OutputRunState::Active,
        obws::events::OutputState::Paused => OutputRunState::Paused,
        obws::events::OutputState::Resumed => OutputRunState::Active,
        obws::events::OutputState::Unknown => OutputRunState::Unknown,
        _ => OutputRunState::Unknown,
    };

    OutputStatus {
        active,
        state,
        detail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;
    use tokio::runtime::Runtime;

    fn controller_with_receiver() -> (Runtime, AppController, mpsc::Receiver<AppEvent>) {
        let runtime = Runtime::new().expect("test runtime");
        let handle = runtime.handle().clone();
        let (tx, rx) = mpsc::sync_channel(16);
        (runtime, AppController::new(handle, tx), rx)
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
            crate::domain::appearance::ThemeMode::default(),
            crate::domain::mixer::MixerSelection::default(),
            crate::storage::config::OutputConfig::default(),
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
