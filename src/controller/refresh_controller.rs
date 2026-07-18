//! OBS refresh helpers and event-stream orchestration.

use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use futures_util::StreamExt;

use crate::controller::event::AppEvent;
use crate::domain::output::{OutputRunState, OutputStatus};
use crate::infra::error::AppError;
use crate::obs::client::ObsClient;

pub(crate) type BitrateSample = Arc<Mutex<Option<(Instant, u64)>>>;

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

pub(crate) async fn run_event_loop(
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
