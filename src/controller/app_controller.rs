//! Application controller — routes commands into connection-owned tasks.
//!
//! GTK owns this controller; Tokio owns protocol work. The shared client slot
//! is installed and cleared only by its session generation. SessionScope owns
//! command cancellation, and EventSender fences both session and collection
//! identity before GTK applies a publication. No mutex guard crosses an await.

use i18n_embed_fl::fl;
use std::future::Future;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use tokio::runtime::Handle;

use crate::controller::command::AppCommand;
use crate::controller::dependencies::ControllerDependencies;
use crate::controller::event::{AppEvent, FailedOperation};
use crate::controller::event_sender::EventSender;
use crate::controller::lifecycle::SessionScope;
use crate::controller::output_controller::{refresh_output_statuses, OutputController};
use crate::controller::refresh_controller::{
    publish_profiles_after, refresh_after_collection_change, refresh_live_data, refresh_obs_stats,
    refresh_profile_and_collection_lists, BitrateSample,
};
use crate::controller::runtime::single_flight::{Admission, SingleFlight};
use crate::controller::session_controller::SessionController;
use crate::controller::state::MixerAudioRequest;
use crate::infra::error::AppError;
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::obs::client::ObsClient;

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
    mixer_queue: SingleFlight<(MixerAudioRequest, EventSender)>,
    refresh_queue: SingleFlight<()>,
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
            mixer_queue: SingleFlight::new(),
            refresh_queue: SingleFlight::new(),
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
            AppCommand::ApplyAudioPreset(plan) => self.workspace.preset(plan),
            AppCommand::CancelAudioPreset => self.workspace.cancel_preset(),
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
                self.workspace.cancel_preset();
                self.workspace.cancel();
                let dependencies = self.dependencies.clone();
                self.spawn_with_client(|c, tx| async move {
                    let result = c.set_current_scene_collection(&name).await;
                    let audio_filter = load_config_blocking(dependencies)
                        .await
                        .map(|config| config.live.audio_inputs);
                    refresh_after_collection_change(result, &c, &tx, audio_filter).await;
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

            AppCommand::SetOutput { kind, active } => self.outputs.set_output(kind, active),
            AppCommand::RefreshStats => self.refresh_stats(),

            AppCommand::RefreshData => self.refresh_data(),
        }
    }

    // ── Private ───────────────────────────────────────────────────────────────

    fn refresh_data(&mut self) {
        let Admission::Started(mut jobs) = self.refresh_queue.offer(()) else {
            return;
        };
        let dependencies = self.dependencies.clone();
        let client = self.connected_client();
        let scope = self.session.scope();
        let tx = self.event_tx.scoped(&scope);
        // This is one controller-owned worker, not optional command admission.
        // Even a disconnected/retired scope must finish Doctor's loading state.
        let worker_scope = scope.clone();
        let worker_tx = tx.clone();
        let worker = async move {
            let (scope, tx) = (worker_scope, worker_tx);
            while jobs.next().is_some() {
                let Some(client) = client.as_ref().filter(|_| scope.is_active()) else {
                    // End the worker before answering, so a request that
                    // arrives during the send starts a new one.
                    drop(jobs);
                    reject_graph_refresh(&tx, not_connected()).await;
                    return;
                };
                let scan = async {
                    let config = match load_config_blocking(dependencies.clone()).await {
                        Ok(config) => config,
                        Err(error) => return reject_graph_refresh(&tx, error).await,
                    };
                    let tx = tx.fresh();
                    refresh_profile_and_collection_lists(client, &tx).await;
                    refresh_output_statuses(client, &tx)
                        .await
                        .publish(&tx)
                        .await;
                    refresh_live_data(client, &tx, &config.live.audio_inputs).await;
                };
                tokio::select! {
                    biased;
                    () = scope.cancelled() => {
                        drop(jobs);
                        reject_graph_refresh(&tx, not_connected()).await;
                        return;
                    },
                    () = scan => {}
                }
            }
        };
        // A reserved single worker is retained by the session until after
        // disconnect drainage. Scope retirement then cancels and joins it.
        // Spawning is the only activity check, so a scope that retires between
        // a check and the spawn cannot strand the queue.
        self.refresh_task = scope.spawn_reserved(&self.runtime, worker).or_else(|| {
            // No request can be admitted here; still finish Doctor's loading.
            Some(
                self.runtime
                    .spawn(async move { reject_graph_refresh(&tx, not_connected()).await }),
            )
        });
    }

    fn refresh_stats(&self) {
        let bitrate_sample = Arc::clone(&self.bitrate_sample);
        self.spawn_with_client(|c, tx| async move {
            refresh_obs_stats(&c, &tx, &bitrate_sample).await;
        });
    }

    fn refresh_mixer_scene_audio(&self, request: MixerAudioRequest) {
        let scope = self.session.scope();
        let job = (request, self.event_tx.scoped(&scope));
        let Admission::Started(mut jobs) = self.mixer_queue.offer(job) else {
            return;
        };
        let dependencies = self.dependencies.clone();
        let client = self.connected_client();
        let worker = async move {
            while let Some((request, tx)) = jobs.next() {
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
                let inputs = match config {
                    Ok(config) => {
                        client
                            .get_scene_audio_inputs(request.scene(), &config.live.audio_inputs)
                            .await
                    }
                    Err(error) => Err(error),
                };
                match inputs {
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
        };
        if scope.spawn_reserved(&self.runtime, worker).is_none() {
            self.fail_refused_mixer_request();
        }
    }

    /// The Mixer set its loading state before dispatch; a refused worker must
    /// still answer that request.
    fn fail_refused_mixer_request(&self) {
        let Some((request, _)) = self.mixer_queue.take_pending() else {
            return;
        };
        self.event_tx.publish_refusal(
            &self.runtime,
            AppEvent::MixerAudioInputsFailed {
                request,
                message: fl!(LANGUAGE_LOADER, "controller-not-connected"),
            },
        );
    }

    fn sync_session_scope(&mut self) {
        // Active refresh workers belong to their SessionScope. Disconnect must
        // retain admitted reads until drainage, not abort them while rebinding
        // controller queues. Connect already retires the old scope first.
        self.refresh_task.take();
        self.refresh_queue = SingleFlight::new();
        self.mixer_queue = SingleFlight::new();
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
        crate::infra::sync::lock_or_recover(self.client_slot.as_ref()).clone()
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

async fn reject_graph_refresh(tx: &EventSender, error: AppError) {
    // A refused send means GTK already moved to a newer session, whose own
    // lifecycle event ends Doctor's loading state.
    let _ = tx.send(AppEvent::GraphRefreshRejected(error)).await;
}

fn not_connected() -> AppError {
    AppError::Connection(fl!(LANGUAGE_LOADER, "controller-not-connected"))
}

/// The stored config, read on the blocking pool. A failed read or a crashed
/// storage worker is an error, never a silent default: defaults would apply
/// the wrong audio filter without telling anyone.
async fn load_config_blocking(
    dependencies: ControllerDependencies,
) -> Result<crate::storage::config::AppConfig, AppError> {
    tokio::task::spawn_blocking(move || dependencies.load_config())
        .await
        .map_err(AppError::storage)?
}

#[cfg(test)]
#[path = "final_regression_tests.rs"]
mod final_regression_tests;

#[cfg(test)]
mod tests;
