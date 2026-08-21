//! OBS refresh helpers and event-stream orchestration.

use std::sync::mpsc::{SyncSender, TrySendError};
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

pub(crate) async fn refresh_after_collection_change(
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

pub(crate) async fn refresh_profile_and_collection_lists(
    client: &ObsClient,
    tx: &SyncSender<AppEvent>,
) {
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

/// against the previous sample, so it is `None` until a second sample lands
/// and whenever the byte counter resets (e.g. the stream just (re)started).
pub(crate) async fn refresh_obs_stats(
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

    let stream = match client.get_stream_health().await {
        Ok(health) => Some(health),
        Err(e) => {
            tracing::debug!(%e, "stream status refresh failed");
            None
        }
    };

    let bitrate_kbps =
        stream.and_then(|health| bitrate_kbps_since_last_sample(bitrate_sample, health.bytes));

    let _ = tx.send(AppEvent::StatsUpdated {
        stats,
        bitrate_kbps,
        stream,
    });
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
    tx: SyncSender<AppEvent>,
    bitrate_sample: BitrateSample,
) {
    let mut ticker = tokio::time::interval(STATS_POLL_INTERVAL);
    // A slow OBS response should not queue up a burst of catch-up polls.
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        ticker.tick().await;
        refresh_obs_stats(&client, &tx, &bitrate_sample).await;
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
    tx: &SyncSender<AppEvent>,
    audio_filter: &[String],
) {
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

/// Translate OBS events into `AppEvent`s for as long as the session lives.
///
/// The stream yields `ObsEvent`, not raw `obws` variants: protocol details are
/// normalized in the OBS adapter layer so this function only has to decide what
/// the app should *do* about each one.
pub(crate) async fn run_event_loop(
    client: ObsClient,
    mut events: ObsEventStream,
    tx: SyncSender<AppEvent>,
    audio_filter: Vec<String>,
) {
    while let Some(event) = events.next().await {
        let app_event = match event {
            ObsEvent::StreamStatusUpdated(status) => Some(AppEvent::StreamStatusUpdated(status)),
            ObsEvent::RecordStatusUpdated(status) => Some(AppEvent::RecordStatusUpdated(status)),

            ObsEvent::CurrentProgramSceneChanged(scene) => {
                let _ = tx.send(AppEvent::CurrentSceneChanged(scene.clone()));
                refresh_scene_audio(&client, &tx, Some(&scene), &audio_filter).await;
                None
            }

            ObsEvent::InputMuteChanged { input, muted } => {
                Some(AppEvent::InputMuteChanged { input, muted })
            }
            ObsEvent::InputVolumeChanged {
                input,
                volume_mul,
                volume_db,
            } => Some(AppEvent::InputVolumeChanged {
                input,
                volume_mul,
                volume_db,
            }),

            // Dropping a levels frame when the UI is busy costs a 50 ms gap in
            // the meters; blocking here would stall every other OBS event
            // behind it. Only a closed channel ends the session.
            ObsEvent::InputLevelsUpdated(levels) => {
                if let Err(TrySendError::Disconnected(_)) =
                    tx.try_send(AppEvent::InputLevelsUpdated(levels))
                {
                    break;
                }
                None
            }

            ObsEvent::InputsChanged | ObsEvent::SceneItemsChanged => {
                refresh_current_scene_audio(&client, &tx, &audio_filter).await;
                None
            }

            ObsEvent::CurrentProfileChanged(name) => {
                refresh_named_list(&client, &tx, NamedList::Profiles, Some(name), Vec::new()).await;
                None
            }
            ObsEvent::ProfileListChanged(profiles) => {
                refresh_named_list(&client, &tx, NamedList::Profiles, None, profiles).await;
                None
            }
            ObsEvent::CurrentSceneCollectionChanged(name) => {
                refresh_named_list(
                    &client,
                    &tx,
                    NamedList::SceneCollections,
                    Some(name),
                    Vec::new(),
                )
                .await;
                // A different collection is a different set of scenes.
                refresh_live_data(&client, &tx, &audio_filter).await;
                None
            }
            ObsEvent::SceneCollectionListChanged(collections) => {
                refresh_named_list(&client, &tx, NamedList::SceneCollections, None, collections)
                    .await;
                None
            }

            ObsEvent::SceneListChanged => {
                refresh_live_data(&client, &tx, &audio_filter).await;
                None
            }

            ObsEvent::Ignored => None,
        };

        if let Some(ev) = app_event {
            if tx.send(ev).is_err() {
                break; // UI gone — exit cleanly
            }
        }
    }

    let _ = tx.send(AppEvent::Disconnected);
}

/// Which of the two OBS name lists a refresh concerns.
#[derive(Clone, Copy)]
enum NamedList {
    Profiles,
    SceneCollections,
}

impl NamedList {
    /// Name used when a refresh of this list fails, so the log still says
    /// which list it was.
    const fn log_name(self) -> &'static str {
        match self {
            Self::Profiles => "profile",
            Self::SceneCollections => "scene collection",
        }
    }
}

/// Re-fetch a profile or scene-collection list and publish it.
///
/// OBS reports the change but not always the whole list, so the list is read
/// back from OBS. `current` overrides the active entry when the event named
/// it, and `fallback_items` is used when OBS returns an empty list — which it
/// does for the list-changed events, whose payload is the list itself.
async fn refresh_named_list(
    client: &ObsClient,
    tx: &SyncSender<AppEvent>,
    list: NamedList,
    current: Option<String>,
    fallback_items: Vec<String>,
) {
    let fetched = match list {
        NamedList::Profiles => client.get_profiles().await,
        NamedList::SceneCollections => client.get_scene_collections().await,
    };

    match fetched {
        Ok(mut fetched) => {
            if let Some(current) = current {
                fetched.current = Some(current);
            }
            if fetched.items.is_empty() {
                fetched.items = fallback_items;
            }
            let event = match list {
                NamedList::Profiles => AppEvent::ProfilesUpdated(fetched),
                NamedList::SceneCollections => AppEvent::SceneCollectionsUpdated(fetched),
            };
            let _ = tx.send(event);
        }
        Err(e) => tracing::warn!(%e, list = list.log_name(), "OBS name list refresh failed"),
    }
}
