//! OBS refresh helpers and event-stream orchestration.

use crate::controller::event_sender::EventSender;
use crate::controller::output_controller::refresh_output_statuses;
use crate::controller::session_controller::REQUEST_BUDGET;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::controller::event::AppEvent;
use crate::infra::error::AppError;
use crate::obs::client::ObsClient;
use crate::obs::event::{ObsEvent, ObsEventStream};

pub(crate) type BitrateSample = Arc<Mutex<Option<(Instant, u64)>>>;

/// Cadence of the session-owned statistics poll. One second matches OBS's own
/// stats dock and keeps the Stats page charts smooth without flooding the
/// WebSocket connection.
pub(crate) const STATS_POLL_INTERVAL: Duration = Duration::from_secs(1);

// ── Shared OBS refresh helpers ────────────────────────────────────────────────

pub(crate) async fn publish_profiles_after(
    operation: Result<(), AppError>,
    client: &ObsClient,
    tx: &EventSender,
) {
    let event = match operation {
        Ok(()) => match client.get_profiles().await {
            Ok(profiles) => AppEvent::ProfilesUpdated(profiles),
            Err(error) => AppEvent::CommandFailed {
                operation: "profile-readback",
                error,
            },
        },
        Err(error) => AppEvent::CommandFailed {
            operation: "profile",
            error,
        },
    };
    let _ = tx.send(event).await;
}

pub(crate) async fn refresh_after_collection_change(
    operation: Result<(), AppError>,
    client: &ObsClient,
    tx: &EventSender,
    audio_filter: &[String],
) {
    match operation {
        Ok(()) => {
            refresh_profile_and_collection_lists(client, tx).await;
            refresh_live_data(client, tx, audio_filter).await;
        }
        Err(e) => {
            let _ = tx
                .send(AppEvent::CommandFailed {
                    operation: "collection",
                    error: e,
                })
                .await;
        }
    }
}

pub(crate) async fn refresh_profile_and_collection_lists(client: &ObsClient, tx: &EventSender) {
    match client.get_profiles().await {
        Ok(profiles) => {
            let _ = tx.send(AppEvent::ProfilesUpdated(profiles)).await;
        }
        Err(error) => {
            let _ = tx
                .send(AppEvent::CommandFailed {
                    operation: "profile-readback",
                    error,
                })
                .await;
        }
    }

    match client.get_scene_collections().await {
        Ok(collections) => {
            let _ = tx
                .send(AppEvent::SceneCollectionsUpdated(collections))
                .await;
        }
        Err(error) => {
            let _ = tx
                .send(AppEvent::CommandFailed {
                    operation: "collection-readback",
                    error,
                })
                .await;
        }
    }
}

/// against the previous sample, so it is `None` until a second sample lands
/// and whenever the byte counter resets (e.g. the stream just (re)started).
pub(crate) async fn refresh_obs_stats(
    client: &ObsClient,
    tx: &EventSender,
    bitrate_sample: &BitrateSample,
) {
    let stats = match client.get_obs_stats().await {
        Ok(stats) => stats,
        Err(e) => {
            tracing::warn!(%e, "obs stats refresh failed");
            return;
        }
    };

    let stream = match client.get_stream_health().await {
        Ok(health) => Some(health),
        Err(e) => {
            tracing::debug!(%e, "stream status refresh failed");
            None
        }
    };

    let bitrate_kbps =
        stream.and_then(|health| bitrate_kbps_since_last_sample(bitrate_sample, health.bytes));

    let _ = tx
        .send(AppEvent::StatsUpdated {
            stats,
            bitrate_kbps,
            stream,
        })
        .await;
}

/// Poll OBS statistics for as long as the session lives.
///
/// obs-websocket v5 has no push event for `GetStats` or `GetStreamStatus`, so
/// a live view of FPS and dropped frames has to be polled. This runs inside the
/// session task rather than being driven by the GTK page that displays it, so
/// the status bar and the Stats page history stay current no matter which page
/// is open — and so it stops the moment the session task is aborted.
///
/// Never returns; callers race it against the OBS event loop.
pub(crate) async fn run_stats_poll_loop(
    client: ObsClient,
    tx: EventSender,
    bitrate_sample: BitrateSample,
) {
    let mut ticker = tokio::time::interval(STATS_POLL_INTERVAL);
    // A slow OBS response should not queue up a burst of catch-up polls.
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        ticker.tick().await;
        refresh_obs_stats(&client, &tx.fresh(), &bitrate_sample).await;
    }
}

/// Compute kbps from the delta against the last (time, bytes) sample, then
/// store `bytes` as the new sample. Returns `None` on the first sample or
/// whenever the counter goes backwards (stream restarted).
pub(crate) fn bitrate_kbps_since_last_sample(
    bitrate_sample: &BitrateSample,
    bytes: u64,
) -> Option<f64> {
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

pub(crate) async fn refresh_live_data(
    client: &ObsClient,
    tx: &EventSender,
    audio_filter: &[String],
) {
    refresh_live_data_if(client, tx, audio_filter, &|| true).await;
}

fn invalidate_graph(client: &ObsClient, tx: &EventSender) -> AppEvent {
    let revision = client.next_observation();
    if let Some(scope) = tx.session_scope() {
        scope.observe_graph_revision(revision);
    }
    AppEvent::Observation {
        revision,
        event: Box::new(AppEvent::GraphInvalidated),
    }
}

async fn refresh_live_data_if(
    client: &ObsClient,
    tx: &EventSender,
    audio_filter: &[String],
    valid: &(impl Fn() -> bool + Sync),
) {
    let revision = client.next_observation();
    if let Some(scope) = tx.session_scope() {
        scope.observe_graph_revision(revision);
    }
    let _ = tx
        .send(AppEvent::Observation {
            revision,
            event: Box::new(AppEvent::GraphInvalidated),
        })
        .await;
    let graph_is_current = || {
        valid()
            && tx.accepts_current_scope()
            && tx
                .session_scope()
                .is_none_or(|scope| scope.graph_revision() == revision)
    };
    let graph = match client.get_scene_inventory().await {
        Ok(inv) => {
            let scene_names: Vec<_> = inv.scenes.iter().map(|s| s.id.clone()).collect();
            let current_scene = inv.current_id.clone();
            let _ = tx
                .send(AppEvent::Observation {
                    revision,
                    event: Box::new(AppEvent::SceneInventoryUpdated(inv)),
                })
                .await;

            refresh_scene_audio(client, tx, current_scene.as_deref(), audio_filter).await;
            client.get_scene_graph(&scene_names).await
        }
        Err(error) => Err(error),
    };
    if graph_is_current() {
        let event = match graph {
            Ok(graph) => AppEvent::GraphUpdated(graph),
            Err(error) => AppEvent::GraphFailed(error),
        };
        let _ = tx
            .send_if(
                AppEvent::Observation {
                    revision,
                    event: Box::new(event),
                },
                &graph_is_current,
            )
            .await;
    }
}

async fn refresh_scene_audio(
    client: &ObsClient,
    tx: &EventSender,
    scene_name: Option<&str>,
    audio_filter: &[String],
) {
    let revision = client.next_observation();
    let scene = scene_name.map(str::to_owned);
    let _ = tx
        .send(AppEvent::Observation {
            revision,
            event: Box::new(AppEvent::ActiveAudioLoading {
                scene: scene.clone(),
            }),
        })
        .await;
    let result = match scene_name {
        Some(scene) => client
            .get_scene_audio_inputs(scene, audio_filter)
            .await
            .map_err(|error| error.user_message()),
        None => Ok(Vec::new()),
    };
    let _ = tx
        .send(AppEvent::Observation {
            revision,
            event: Box::new(AppEvent::ActiveAudioObserved { scene, result }),
        })
        .await;
}

// ── Session event loop ────────────────────────────────────────────────────────

#[derive(Clone)]
struct RefreshInvalidation {
    revision: tokio::sync::watch::Sender<u64>,
    announced: Arc<std::sync::atomic::AtomicBool>,
    audio_scene: tokio::sync::watch::Sender<Option<String>>,
}

impl Default for RefreshInvalidation {
    fn default() -> Self {
        Self {
            revision: tokio::sync::watch::channel(0).0,
            announced: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            audio_scene: tokio::sync::watch::channel(None).0,
        }
    }
}

impl RefreshInvalidation {
    fn invalidate(&self) -> bool {
        self.revision.send_modify(|revision| {
            *revision = revision.checked_add(1).expect("refresh revision exhausted");
        });
        !self
            .announced
            .swap(true, std::sync::atomic::Ordering::AcqRel)
    }
}

async fn refresh_worker(
    client: &ObsClient,
    tx: &EventSender,
    audio_filter: &[String],
    dirty: &RefreshInvalidation,
) -> Result<(), AppError> {
    let version = client.get_version().await?;
    tx.send(AppEvent::Connected(version))
        .await
        .map_err(|()| AppError::Connection("UI event receiver closed".into()))?;
    let mut revisions = dirty.revision.subscribe();
    let mut audio_scenes = dirty.audio_scene.subscribe();
    let mut full_refresh = true;
    loop {
        let scan_tx = tx.fresh();
        if full_refresh {
            let scan_revision = *revisions.borrow_and_update();
            audio_scenes.borrow_and_update();
            dirty
                .announced
                .store(false, std::sync::atomic::Ordering::Release);
            refresh_profile_and_collection_lists(client, &scan_tx).await;
            refresh_output_statuses(client, &scan_tx).await;
            refresh_live_data_if(client, &scan_tx, audio_filter, &|| {
                *dirty.revision.borrow() == scan_revision
            })
            .await;
        } else {
            let scene = audio_scenes.borrow_and_update().clone();
            let scene = match scene {
                Some(scene) => Some(scene),
                // Visibility can change in a nested scene. Read the current
                // Program once, without invalidating/rebuilding topology.
                None => match client.get_scene_inventory().await {
                    Ok(inventory) => inventory.current_id,
                    Err(error) => {
                        let _ = scan_tx
                            .send(AppEvent::WorkspaceAudioFailed(error.user_message()))
                            .await;
                        None
                    }
                },
            };
            // Program changes do not change topology, profiles or outputs.
            // Refresh only the new scene's audio; UI observation fences discard
            // replies for scenes superseded during this read.
            refresh_scene_audio(client, &scan_tx, scene.as_deref(), audio_filter).await;
        }
        // Every invalidation during the scan is folded into one follow-up scan.
        tokio::select! {
            biased;
            changed = revisions.changed() => {
                if changed.is_err() { return Ok(()); }
                full_refresh = true;
            }
            changed = audio_scenes.changed() => {
                if changed.is_err() { return Ok(()); }
                full_refresh = false;
            }
        }
    }
}

// A bounded handoff keeps safety invalidation at transport ingress independent
// of GTK backpressure. Overflow retires the connection rather than dropping
// evidence, growing an unbounded queue, or trusting a missed mode notification.
pub(crate) const TRANSPORT_PUBLICATION_CAPACITY: usize = 64;

fn enqueue_transport_publication(
    pending: &tokio::sync::mpsc::Sender<(EventSender, AppEvent)>,
    client: &ObsClient,
    tx: &EventSender,
    event: AppEvent,
) -> bool {
    if pending.try_send((tx.fresh(), event)).is_ok() {
        return true;
    }
    tx.terminal_reason("OBS notification publication backlog exceeded its bound; reconnect and refresh; output evidence may be incomplete".into());
    client.retire();
    false
}

/// Pump transport events independently of bootstrap, graph/audio scans and stats.
pub(crate) async fn run_event_loop(
    client: ObsClient,
    mut events: ObsEventStream,
    tx: EventSender,
    audio_filter: Vec<String>,
    bitrate_sample: BitrateSample,
) {
    let dirty = RefreshInvalidation::default();
    let worker = refresh_worker(&client, &tx, &audio_filter, &dirty);
    let poll = run_stats_poll_loop(client.clone(), tx.clone(), bitrate_sample);
    let (pending, mut publications) =
        tokio::sync::mpsc::channel::<(EventSender, AppEvent)>(TRANSPORT_PUBLICATION_CAPACITY);
    let publisher = async {
        while let Some((sender, event)) = publications.recv().await {
            // A collection change can supersede an already queued publication.
            // That is not a transport failure; a closed current UI receiver is.
            if sender.send(event).await.is_err() && sender.accepts_current_scope() {
                return;
            }
        }
    };
    tokio::pin!(worker, poll, publisher);
    loop {
        let event = tokio::select! {
            outcome = &mut worker => {
                // No ingress is consumed while a terminal error waits for GTK.
                // Fence the connection first, so no delayed mutation can use it.
                client.retire();
                if let Err(error) = outcome { let _ = tx.send(AppEvent::Error(error)).await; }
                break;
            }
            () = &mut poll => break,
            () = &mut publisher => break,
            () = client.wait_retired() => break,
            event = events.next() => event,
        };
        let Some(event) = event else {
            // obws 0.15 yields None for both Lagged and Closed. A successful
            // request proves liveness before replacing the damaged subscription.
            match tokio::time::timeout(REQUEST_BUDGET, events.recover(&client)).await {
                Ok(Ok(())) => {
                    tx.advance_collection();
                    let fresh = tx.fresh();
                    let invalidation = invalidate_graph(&client, &fresh);
                    let queued = [
                        AppEvent::WorkspaceScopeChanging,
                        invalidation,
                        AppEvent::CommandFailed { operation: "events", error: AppError::Request("OBS event notifications were missed; snapshots are being resynchronized and output evidence may be incomplete".into()) },
                    ].into_iter().all(|event| enqueue_transport_publication(&pending, &client, &fresh, event));
                    if !queued {
                        break;
                    }
                    dirty.invalidate();
                    continue;
                }
                _ => break,
            }
        };
        let mut graph_invalidation = None;
        let publication = match event {
            ObsEvent::StudioModeChanged(enabled) => {
                // Fence accepted clicks independently of bounded UI publication.
                // OBS has no atomic mode-and-scene operation: a mode change not
                // yet received here can still race an already admitted write.
                if let Some(scope) = tx.session_scope() {
                    scope.invalidate_scene_activation();
                }
                Some(AppEvent::StudioModeChanged(enabled))
            }
            ObsEvent::VirtualCameraChanged(active) => Some(AppEvent::VirtualCameraChanged(active)),
            ObsEvent::ControlsChanged => Some(AppEvent::WorkspaceValuesChanged),
            ObsEvent::SceneItemVisibilityChanged => {
                dirty.audio_scene.send_replace(None);
                Some(AppEvent::WorkspaceValuesChanged)
            }
            ObsEvent::CollectionChanging => {
                tx.advance_collection();
                graph_invalidation = Some(invalidate_graph(&client, &tx.fresh()));
                Some(AppEvent::WorkspaceScopeChanging)
            }
            ObsEvent::ReplaySaved(path) => Some(AppEvent::ReplaySaved(path)),
            ObsEvent::StreamStatusUpdated(status) => Some(AppEvent::Observation {
                revision: client.next_observation(),
                event: Box::new(AppEvent::StreamStatusUpdated(status)),
            }),
            ObsEvent::RecordStatusUpdated(status) => Some(AppEvent::Observation {
                revision: client.next_observation(),
                event: Box::new(AppEvent::RecordStatusUpdated(status)),
            }),
            ObsEvent::CurrentProgramSceneChanged(scene) => {
                dirty.audio_scene.send_replace(Some(scene.clone()));
                Some(AppEvent::Observation {
                    revision: client.next_observation(),
                    event: Box::new(AppEvent::CurrentSceneChanged(scene)),
                })
            }
            ObsEvent::InputMuteChanged { input, muted } => Some(AppEvent::Observation {
                revision: client.next_observation(),
                event: Box::new(AppEvent::InputMuteChanged { input, muted }),
            }),
            ObsEvent::InputVolumeChanged {
                input,
                volume_mul,
                volume_db,
            } => Some(AppEvent::Observation {
                revision: client.next_observation(),
                event: Box::new(AppEvent::InputVolumeChanged {
                    input,
                    volume_mul,
                    volume_db,
                }),
            }),
            ObsEvent::InputLevelsUpdated(levels) => {
                let _ = tx.fresh().try_send(AppEvent::InputLevelsUpdated(levels));
                None
            }
            ObsEvent::InputsChanged | ObsEvent::SceneItemsChanged | ObsEvent::SceneListChanged => {
                if let Some(scope) = tx.session_scope() {
                    scope.observe_graph_revision(client.next_observation());
                }
                if dirty.invalidate() {
                    graph_invalidation = Some(invalidate_graph(&client, &tx.fresh()));
                    Some(AppEvent::WorkspaceInvalidated)
                } else {
                    None
                }
            }
            ObsEvent::CurrentSceneCollectionChanged(_) => {
                tx.advance_collection();
                dirty.invalidate();
                Some(AppEvent::WorkspaceScopeChanging)
            }
            ObsEvent::CurrentProfileChanged(_)
            | ObsEvent::ProfileListChanged(_)
            | ObsEvent::SceneCollectionListChanged(_) => {
                dirty.invalidate();
                None
            }
            ObsEvent::Ignored => None,
        };
        for publication in graph_invalidation.into_iter().chain(publication) {
            if !enqueue_transport_publication(&pending, &client, &tx, publication) {
                break;
            }
        }
        if client.retirement_reason().is_some() {
            break;
        }
    }
    tx.terminal_reason(
        client
            .retirement_reason()
            .unwrap_or_else(|| "OBS event connection ended; refresh required".into()),
    );
    client.retire();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn invalidations_during_a_scan_coalesce_without_a_third_spurious_wake() {
        let dirty = RefreshInvalidation::default();
        let mut revisions = dirty.revision.subscribe();
        let _ = *revisions.borrow_and_update();
        for _ in 0..256 {
            dirty.invalidate();
        }
        revisions.changed().await.unwrap();
        assert_eq!(*revisions.borrow_and_update(), 256);
        assert!(
            !revisions.has_changed().unwrap(),
            "follow-up scan consumed the whole burst"
        );
        dirty.invalidate();
        revisions.changed().await.unwrap();
        assert_eq!(*revisions.borrow_and_update(), 257);
    }
}
