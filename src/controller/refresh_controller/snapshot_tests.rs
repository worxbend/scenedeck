//! Program audio and output status snapshots from the event loop, applied
//! through the reducers GTK uses: a snapshot never undoes a newer echo or
//! relabels a stale scene, and a failed one never reads as empty.

use std::sync::mpsc;

use crate::controller::event::AppEvent;
use crate::controller::state::{
    ActiveAudioStatus, MixerVisibleAudioStatus, MixerVisibleRenderSource,
};
use crate::controller::test_support::events::{graph_ready, recv_event};
use crate::controller::test_support::loopback::{consume_echo_pair, echo_pair, pump, switch_b};
use crate::controller::test_support::state::app_state;
use crate::domain::output::OutputKind;
use crate::obs::testing::stateful::{Event, Request, Scene, StatefulObs, When};

#[tokio::test]
async fn bootstrap_active_audio_snapshot_merges_newer_mute_and_volume_echoes() {
    let mut mock = StatefulObs::default()
        .hold(Request::GetInputVolume, When::Nth(1))
        .connect()
        .await;
    let (tx, rx) = mpsc::sync_channel(128);
    let task = pump(&mut mock, tx);
    let mut state = app_state();
    state.set_current_program("A".into());
    mock.wait(Request::GetInputVolume, 1).await;
    // Consume loading, then use the exact production reconciliation API used by GTK.
    loop {
        if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
            if matches!(*event, AppEvent::ActiveAudioLoading { .. }) {
                state.reconcile_observation(revision, *event).unwrap();
                break;
            }
        }
    }
    echo_pair(&mock).await;
    consume_echo_pair(&rx, &mut state).await;
    mock.release_all().await;
    loop {
        if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
            if let Some(AppEvent::ActiveAudioObserved { scene, result }) =
                state.reconcile_observation(revision, *event)
            {
                let inputs = result.unwrap();
                assert!(inputs[0].muted);
                assert_eq!(inputs[0].volume_mul, 0.25);
                state.apply_active_audio(scene, Ok(inputs));
                break;
            }
        }
    }
    assert!(state.audio_inputs[0].muted);
    assert_eq!(state.audio_inputs[0].volume_mul, 0.25);
    assert!(
        matches!(state.visible_mixer_render_source(), MixerVisibleRenderSource::ActiveScene(inputs) if inputs[0].muted)
    );
    mock.client.retire();
    task.await.unwrap();
    mock.stop().await;
}

#[tokio::test]
async fn failed_program_audio_clears_previous_controls_and_is_not_successful_emptiness() {
    for failure in [
        Some(Request::GetSceneItemList),
        Some(Request::GetSceneItemEnabled),
        Some(Request::GetInputMute),
        Some(Request::GetInputVolume),
        None,
    ] {
        let obs = StatefulObs::default();
        let obs = match failure {
            Some(request) => obs.refuse(request, When::WhileProgram(Scene::B)),
            None => obs.empty_scene(Scene::B),
        };
        let mut mock = obs.connect().await;
        let (tx, rx) = mpsc::sync_channel(128);
        let task = pump(&mut mock, tx);
        let mut state = app_state();
        state.set_current_program("A".into());
        // Complete the first real observation using the same state APIs as GTK.
        loop {
            if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
                if let Some(AppEvent::ActiveAudioObserved { scene, result }) =
                    state.reconcile_observation(revision, *event)
                {
                    state.apply_active_audio(scene, result);
                    break;
                }
            }
        }
        assert_eq!(state.audio_inputs[0].id, "MicA");
        graph_ready(&rx).await;
        switch_b(&mock).await;
        loop {
            if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
                if let Some(AppEvent::CurrentSceneChanged(scene)) =
                    state.reconcile_observation(revision, *event)
                {
                    state.set_current_program(scene);
                    break;
                }
            }
        }
        assert!(state.audio_inputs.is_empty());
        assert!(state.active_audio_notice().is_some());
        assert!(matches!(
            state.visible_mixer_render_source(),
            MixerVisibleRenderSource::Scene {
                status: MixerVisibleAudioStatus::Loading,
                ..
            }
        ));
        loop {
            if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
                if let Some(AppEvent::ActiveAudioObserved { scene, result }) =
                    state.reconcile_observation(revision, *event)
                {
                    state.apply_active_audio(scene, result);
                    break;
                }
            }
        }
        assert!(state.audio_inputs.is_empty());
        if failure.is_some() {
            assert!(matches!(
                state.active_audio_status,
                ActiveAudioStatus::Failed(_)
            ));
            assert!(matches!(
                state.visible_mixer_render_source(),
                MixerVisibleRenderSource::Scene {
                    status: MixerVisibleAudioStatus::Error(_),
                    ..
                }
            ));
            assert!(state.active_audio_notice().is_some());
        } else {
            assert!(matches!(
                state.active_audio_status,
                ActiveAudioStatus::Ready
            ));
            assert!(state.active_audio_notice().is_none());
            assert!(
                matches!(state.visible_mixer_render_source(), MixerVisibleRenderSource::ActiveScene(inputs) if inputs.is_empty())
            );
        }
        mock.client.retire();
        task.await.unwrap();
        mock.stop().await;
    }
}

#[tokio::test]
async fn bootstrap_scene_snapshot_cannot_relabel_stale_program_audio_as_current() {
    let mut mock = StatefulObs::default()
        .hold(Request::GetSceneList, When::Nth(1))
        .connect()
        .await;
    let (tx, rx) = mpsc::sync_channel(128);
    let task = pump(&mut mock, tx);
    mock.wait(Request::GetSceneList, 1).await;
    let mut state = app_state();
    state.set_current_program("A".into());
    switch_b(&mock).await;
    loop {
        if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
            if let Some(AppEvent::CurrentSceneChanged(scene)) =
                state.reconcile_observation(revision, *event)
            {
                state.set_current_program(scene);
                break;
            }
        }
    }
    mock.release_all().await;
    let mut stale_audio_discarded = false;
    loop {
        if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
            let old_audio = matches!(&*event, AppEvent::ActiveAudioObserved { scene: Some(scene), .. } if scene == "A");
            let reconciled = state.reconcile_observation(revision, *event);
            if old_audio {
                assert!(reconciled.is_none());
                stale_audio_discarded = true;
            }
            match reconciled {
                Some(AppEvent::SceneInventoryUpdated(inventory)) => {
                    assert_eq!(inventory.current_id.as_deref(), Some("B"));
                    state.scene_inventory = inventory;
                }
                Some(AppEvent::ActiveAudioObserved { scene, result }) => {
                    state.apply_active_audio(scene, result);
                    break;
                }
                _ => {}
            }
        }
    }
    assert!(stale_audio_discarded);
    assert_eq!(state.audio_inputs[0].id, "MicB");
    mock.client.retire();
    task.await.unwrap();
    mock.stop().await;
}

#[tokio::test]
async fn program_aba_during_audio_scan_rejects_the_old_same_name_completion() {
    let mut mock = StatefulObs::default()
        .hold(Request::GetInputVolume, When::Nth(1))
        .connect()
        .await;
    let (tx, rx) = mpsc::sync_channel(128);
    let task = pump(&mut mock, tx);
    let mut state = app_state();
    state.set_current_program("A".into());
    mock.wait(Request::GetInputVolume, 1).await;
    // Establish the actual request identity before invalidating its Program scope.
    loop {
        if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
            if matches!(*event, AppEvent::ActiveAudioLoading { .. }) {
                state.reconcile_observation(revision, *event).unwrap();
                break;
            }
        }
    }
    switch_b(&mock).await;
    mock.emit(Event::ProgramChanged(Scene::A)).await;
    let mut changes = 0;
    while changes < 2 {
        if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
            if let Some(AppEvent::CurrentSceneChanged(scene)) =
                state.reconcile_observation(revision, *event)
            {
                state.set_current_program(scene);
                changes += 1;
            }
        }
    }
    mock.release_all().await;
    loop {
        if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
            if matches!(*event, AppEvent::ActiveAudioObserved { .. }) {
                assert!(state.reconcile_observation(revision, *event).is_none());
                break;
            }
        }
    }
    assert!(state.audio_inputs.is_empty());
    loop {
        if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
            if let Some(AppEvent::ActiveAudioObserved { scene, result }) =
                state.reconcile_observation(revision, *event)
            {
                state.apply_active_audio(scene, result);
                break;
            }
        }
    }
    assert_eq!(state.audio_inputs[0].id, "MicA");
    mock.client.retire();
    task.await.unwrap();
    assert_eq!(mock.stop().await.count(Request::GetInputMute), 2);
}

#[tokio::test]
async fn output_status_snapshot_cannot_undo_a_newer_transport_echo() {
    let mut mock = StatefulObs::default()
        .hold(Request::GetStreamStatus, When::Nth(1))
        .connect()
        .await;
    let (tx, rx) = mpsc::sync_channel(128);
    let task = pump(&mut mock, tx);
    mock.wait(Request::GetStreamStatus, 1).await;
    let request_seen = std::time::Instant::now();
    mock.emit(Event::StreamStarted).await;
    let mut state = app_state();
    state.health.connected();
    loop {
        if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
            if let Some(event @ AppEvent::OutputStatusUpdated(OutputKind::Stream, _, _)) =
                state.reconcile_observation(revision, *event)
            {
                let AppEvent::OutputStatusUpdated(_, status, observed_at) = &event else {
                    unreachable!()
                };
                assert!(status.active);
                assert!(*observed_at >= request_seen);
                state.apply_output_event(&event, 0);
                break;
            }
        }
    }
    mock.release_all().await;
    loop {
        if let AppEvent::Observation { revision, event } = recv_event(&rx).await {
            if matches!(
                *event,
                AppEvent::OutputStatusUpdated(OutputKind::Stream, _, _)
            ) {
                let AppEvent::OutputStatusUpdated(_, _, observed_at) = &*event else {
                    unreachable!()
                };
                assert!(
                    *observed_at <= request_seen,
                    "readback time must precede its wait and publication"
                );
                assert!(state.reconcile_observation(revision, *event).is_none());
                break;
            }
        }
    }
    assert!(state.stream_status.active);
    assert_eq!(state.health.output_activity(OutputKind::Stream), Some(true));
    assert!(state.health.alerts(std::time::Instant::now()).is_empty());
    mock.client.retire();
    task.await.unwrap();
    mock.stop().await;
}
