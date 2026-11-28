//! OBS stats, stream health, bitrate history and audio-level telemetry.

use std::time::Instant;

use super::fixtures::{app_state, obs_stats, stream_health};
use crate::domain::meter::InputLevels;
use crate::domain::output::OutputStatus;
use crate::domain::stats::StatsMetric;

#[test]
fn telemetry_keeps_observation_time_through_publication_and_clears_it_on_reset() {
    let mut state = app_state();
    let observed_at = Instant::now() - std::time::Duration::from_secs(8);
    state.set_obs_stats_at(obs_stats(60.0), None, None, observed_at);
    state.set_audio_levels_at(
        vec![InputLevels::from_mul("Mic".into(), &[[0.5; 3]])],
        observed_at,
    );
    assert_eq!(state.obs_stats_observed_at, Some(observed_at));
    assert_eq!(state.audio_levels.observed_at, Some(observed_at));
    state.clear_obs_stats();
    state.clear_audio_levels();
    assert!(state.obs_stats_observed_at.is_none());
    assert!(state.audio_levels.observed_at.is_none());
}

#[test]
fn stats_updates_accumulate_history_for_the_charts() {
    let mut state = app_state();

    state.set_obs_stats(obs_stats(60.0), None, Some(stream_health()));
    state.set_obs_stats(obs_stats(59.0), None, Some(stream_health()));

    assert_eq!(state.stats_history.len(), 2);
    assert_eq!(
        state.stats_history.series(StatsMetric::Fps),
        vec![60.0, 59.0]
    );
    assert_eq!(state.stream_health, Some(stream_health()));
}

#[test]
fn bitrate_is_dropped_from_state_and_history_while_not_streaming() {
    let mut state = app_state();

    state.set_obs_stats(obs_stats(60.0), Some(6_000.0), Some(stream_health()));

    assert_eq!(state.stream_bitrate_kbps, None);
    assert_eq!(
        state
            .stats_history
            .latest()
            .and_then(|sample| sample.bitrate_kbps),
        None
    );
}

#[test]
fn bitrate_is_recorded_once_the_stream_output_is_active() {
    let mut state = app_state();
    state.set_stream_status(OutputStatus::active());

    state.set_obs_stats(obs_stats(60.0), Some(6_000.0), Some(stream_health()));

    assert_eq!(state.stream_bitrate_kbps, Some(6_000.0));
    assert_eq!(
        state
            .stats_history
            .latest()
            .and_then(|sample| sample.bitrate_kbps),
        Some(6_000.0)
    );
}

#[test]
fn clearing_stats_empties_the_history_and_stream_health() {
    let mut state = app_state();
    state.set_obs_stats(obs_stats(60.0), None, Some(stream_health()));

    state.clear_obs_stats();

    assert!(state.stats_history.is_empty());
    assert_eq!(state.stream_health, None);
    assert_eq!(state.obs_stats, None);
}
