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

use futures_util::StreamExt;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

use crate::controller::command::AppCommand;
use crate::controller::dependencies::ControllerDependencies;
use crate::controller::event::AppEvent;
use crate::domain::output::{OutputRunState, OutputStatus};
use crate::infra::error::AppError;
use crate::obs::client::ObsClient;

pub struct AppController {
    event_tx: SyncSender<AppEvent>,
    runtime: Handle,
    dependencies: ControllerDependencies,
    /// Session task handle — aborted on disconnect or reconnect.
    session: Option<JoinHandle<()>>,
    /// Shared slot written by the session task once connected, read by
    /// per-command tasks.  `None` while disconnected.
    client_slot: Arc<Mutex<Option<ObsClient>>>,
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

            AppCommand::SetStreaming(active) => {
                self.with_client(|c, tx, rt| {
                    rt.spawn(async move {
                        match c.set_streaming(active).await {
                            Ok(()) => refresh_output_statuses(&c, &tx).await,
                            Err(e) => {
                                let _ = tx.send(AppEvent::Error(e));
                            }
                        }
                    });
                });
            }

            AppCommand::SetRecording(active) => {
                self.with_client(|c, tx, rt| {
                    rt.spawn(async move {
                        match c.set_recording(active).await {
                            Ok(path) => {
                                refresh_output_statuses(&c, &tx).await;
                                if let Some(path) = path {
                                    let _ = tx.send(AppEvent::RecordStatusUpdated(OutputStatus {
                                        active: false,
                                        state: OutputRunState::Inactive,
                                        detail: Some(path),
                                    }));
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(AppEvent::Error(e));
                            }
                        }
                    });
                });
            }

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

    fn disconnect(&mut self) {
        if let Some(h) = self.session.take() {
            h.abort();
        }
        if let Ok(mut slot) = self.client_slot.lock() {
            *slot = None;
        }
        let _ = self.event_tx.send(AppEvent::Disconnected);
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

async fn refresh_output_statuses(client: &ObsClient, tx: &SyncSender<AppEvent>) {
    match client.get_stream_status().await {
        Ok(status) => {
            let _ = tx.send(AppEvent::StreamStatusUpdated(status));
        }
        Err(e) => tracing::warn!(%e, "stream status refresh failed"),
    }

    match client.get_record_status().await {
        Ok(status) => {
            let _ = tx.send(AppEvent::RecordStatusUpdated(status));
        }
        Err(e) => tracing::warn!(%e, "record status refresh failed"),
    }
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
