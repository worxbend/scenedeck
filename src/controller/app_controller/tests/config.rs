//! Config loading: the blocking-pool read, a crashed storage worker, and
//! graph and Mixer refreshes that must fail rather than query OBS when
//! `config.json` cannot be used.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::ThreadId;

use tokio::runtime::{Handle, Runtime};

use crate::controller::app_controller::{load_config_blocking, AppController};
use crate::controller::command::AppCommand;
use crate::controller::dependencies::ControllerDependencies;
use crate::controller::event::AppEvent;
use crate::controller::test_support::deps::NoPassword;
use crate::controller::test_support::events::recv_event;
use crate::controller::test_support::state::app_state;
use crate::infra::error::AppError;

struct ThreadRecordingConfigProvider {
    caller: ThreadId,
    observed: Arc<Mutex<Option<ThreadId>>>,
}

impl crate::controller::dependencies::AppConfigProvider for ThreadRecordingConfigProvider {
    fn load_config(&self) -> Result<crate::storage::config::AppConfig, AppError> {
        let worker = std::thread::current().id();
        assert_ne!(worker, self.caller);
        *self.observed.lock().expect("observed worker thread") = Some(worker);
        Ok(Default::default())
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
        Arc::new(NoPassword),
    );

    runtime
        .block_on(load_config_blocking(dependencies))
        .expect("the provider serves a config");

    assert!(observed.lock().expect("observed worker thread").is_some());
}

struct PanickingConfigProvider;

impl crate::controller::dependencies::AppConfigProvider for PanickingConfigProvider {
    fn load_config(&self) -> Result<crate::storage::config::AppConfig, AppError> {
        panic!("storage worker crashed")
    }
}

#[test]
fn a_crashed_config_worker_is_a_storage_error_not_default_settings() {
    let runtime = Runtime::new().expect("test runtime");
    let dependencies =
        ControllerDependencies::new(Arc::new(PanickingConfigProvider), Arc::new(NoPassword));

    let loaded = runtime.block_on(load_config_blocking(dependencies));

    assert!(
        matches!(loaded, Err(AppError::Storage(_))),
        "got {loaded:?}"
    );
}

/// A controller whose installed client talks to an OBS that answers only
/// the handshake, while `config.json` cannot be used.
async fn connected_controller_with_unusable_config() -> (
    AppController,
    mpsc::Receiver<AppEvent>,
    crate::obs::testing::scripted::RequestCounts,
) {
    let obs = crate::obs::testing::scripted::ScriptedObs::by_type([]);
    let requests = obs.counts();
    let (client, _server) = obs.connect().await;
    let (tx, rx) = mpsc::sync_channel(16);
    let controller = AppController::with_dependencies(
        Handle::current(),
        tx,
        ControllerDependencies::new(
            Arc::new(crate::controller::test_support::deps::UnusableConfigProvider),
            Arc::new(NoPassword),
        ),
    );
    assert!(controller.session.scope().install(client));
    (controller, rx, requests)
}

#[tokio::test]
async fn graph_refresh_with_an_unusable_config_is_rejected_without_querying_obs() {
    let (mut controller, rx, requests) = connected_controller_with_unusable_config().await;

    controller.handle(AppCommand::RefreshData);

    let event = recv_event(&rx).await;
    assert!(
        matches!(&event, AppEvent::GraphRefreshRejected(AppError::Storage(_))),
        "expected a storage rejection, got {event:?}"
    );
    assert_eq!(
        requests.snapshot().keys().collect::<Vec<_>>(),
        ["GetVersion"]
    );
    controller.shutdown().await.unwrap();
}

#[tokio::test]
async fn mixer_refresh_with_an_unusable_config_fails_without_querying_obs() {
    let (mut controller, rx, requests) = connected_controller_with_unusable_config().await;
    let request = app_state().set_mixer_audio_loading("Scene A".to_string());

    controller.handle(AppCommand::RefreshMixerSceneAudio(request.clone()));

    let event = recv_event(&rx).await;
    let AppEvent::MixerAudioInputsFailed {
        request: failed,
        message,
    } = event
    else {
        panic!("expected a mixer failure, got {event:?}");
    };
    assert_eq!(failed, request);
    assert!(message.contains("config.json is corrupt"), "got {message}");
    assert_eq!(
        requests.snapshot().keys().collect::<Vec<_>>(),
        ["GetVersion"]
    );
    controller.shutdown().await.unwrap();
}
