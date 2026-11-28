//! Event ingress over a stateful OBS: each event refreshes only what it
//! changed, and a blocked GTK queue fails the connection closed instead of
//! stalling mode ingress.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::controller::event::AppEvent;
use crate::controller::event_sender::EventSender;
use crate::controller::lifecycle::SessionScope;
use crate::controller::test_support::events::{
    graph_ready, peel, recv_event, recv_matching, TEST_DEADLINE,
};
use crate::controller::test_support::loopback::{pump, switch_b};
use crate::obs::testing::stateful::{Event, Request, Scene, StatefulObs, When, SCENE_WRITES};

#[tokio::test]
async fn scene_rename_publishes_identity_and_refreshes_topology() {
    let mut mock = StatefulObs::default().connect().await;
    let (tx, rx) = mpsc::sync_channel(128);
    let task = pump(&mut mock, tx);
    graph_ready(&rx).await;
    mock.emit(Event::SceneRenamed {
        scene: Scene::A,
        to: "Guest",
    })
    .await;
    let mut renamed = false;
    let mut refreshed = false;
    while !renamed || !refreshed {
        let event = match peel(recv_event(&rx).await) {
            AppEvent::Observation { event, .. } => *event,
            event => event,
        };
        match event {
            AppEvent::SceneRenamed { uuid, old, new } => {
                assert!(uuid.is_some());
                assert_eq!((old.as_str(), new.as_str()), ("A", "Guest"));
                renamed = true;
            }
            AppEvent::GraphUpdated(_) => refreshed = true,
            _ => {}
        }
    }
    task.abort();
    let _ = task.await;
    mock.stop().await;
}

#[tokio::test]
async fn visibility_and_transition_events_do_not_reload_topology_or_scene_mode() {
    let mut mock = StatefulObs::default().connect().await;
    let (tx, rx) = mpsc::sync_channel(128);
    let task = pump(&mut mock, tx);
    graph_ready(&rx).await;
    mock.emit(Event::TransitionEnded).await;
    mock.emit(Event::PreviewChanged(Scene::B)).await;
    mock.emit(Event::ItemEnabledChanged {
        scene: Scene::A,
        enabled: false,
    })
    .await;
    let mut values_changed = false;
    loop {
        let event = match peel(recv_event(&rx).await) {
            AppEvent::Observation { event, .. } => *event,
            event => event,
        };
        assert!(
            !matches!(
                event,
                AppEvent::WorkspaceInvalidated
                    | AppEvent::GraphInvalidated
                    | AppEvent::GraphUpdated(_)
            ),
            "value-only event invalidated unrelated state: {event:?}"
        );
        values_changed |= matches!(event, AppEvent::WorkspaceValuesChanged);
        if matches!(event, AppEvent::ActiveAudioObserved { result: Ok(_), .. }) {
            break;
        }
    }
    assert!(values_changed);
    task.abort();
    let _ = task.await;
    let requests = mock.stop().await;
    for request in [Request::GetProfileList, Request::GetSceneCollectionList] {
        assert_eq!(requests.count(request), 1);
    }
}

#[tokio::test]
async fn program_switch_refreshes_audio_without_reloading_topology_or_setup() {
    let mut mock = StatefulObs::default().connect().await;
    let (tx, rx) = mpsc::sync_channel(128);
    let task = pump(&mut mock, tx);
    graph_ready(&rx).await;
    switch_b(&mock).await;
    loop {
        let event = match peel(recv_event(&rx).await) {
            AppEvent::Observation { event, .. } => *event,
            event => event,
        };
        assert!(
            !matches!(
                event,
                AppEvent::GraphInvalidated | AppEvent::GraphUpdated(_)
            ),
            "a Program change must not blank Graph or Doctor"
        );
        if let AppEvent::ActiveAudioObserved { scene, result } = event {
            if scene.as_deref() == Some("B") {
                assert!(result.is_ok());
                break;
            }
        }
    }
    task.abort();
    let _ = task.await;
    let requests = mock.stop().await;
    for request in [
        Request::GetSceneList,
        Request::GetProfileList,
        Request::GetSceneCollectionList,
    ] {
        assert_eq!(
            requests.count(request),
            1,
            "Program switching repeated {request:?}"
        );
    }
}

#[tokio::test]
async fn bootstrap_failure_fences_mutations_before_waiting_for_full_gtk_queue() {
    let mut mock = StatefulObs::default()
        .refuse(Request::GetVersion, When::Nth(2))
        .connect()
        .await;
    let scope = SessionScope::initial(Arc::new(Mutex::new(None))).next();
    scope.install(mock.client.clone());
    let (tx, rx) = mpsc::sync_channel(1);
    tx.send(AppEvent::Connecting).unwrap();
    let task = tokio::spawn(crate::controller::refresh_controller::run_event_loop(
        mock.client.clone(),
        mock.take_events(),
        EventSender::new(tx).scoped(&scope),
        vec![],
        Arc::new(Mutex::new(None)),
    ));
    mock.wait(Request::GetVersion, 2).await;
    let fenced_without_ui =
        tokio::time::timeout(Duration::from_millis(250), mock.client.wait_retired())
            .await
            .is_ok();
    rx.try_recv().unwrap();
    recv_matching(&rx, |event| {
        matches!(peel(event.clone()), AppEvent::Error(_))
    })
    .await;
    task.await.unwrap();
    scope.retire();
    mock.stop().await;
    assert!(
        fenced_without_ui,
        "failed ingress owner left mutations admitted while its Error publication waited for GTK"
    );
}

#[tokio::test]
async fn transport_publication_overflow_retires_instead_of_stalling_mode_ingress() {
    let mut mock = StatefulObs::default().connect().await;
    let scope = SessionScope::initial(Arc::new(Mutex::new(None))).next();
    scope.install(mock.client.clone());
    let (tx, _rx) = mpsc::sync_channel(1);
    tx.send(AppEvent::Connecting).unwrap();
    let task = tokio::spawn(crate::controller::refresh_controller::run_event_loop(
        mock.client.clone(),
        mock.take_events(),
        EventSender::new(tx).scoped(&scope),
        vec![],
        Arc::new(Mutex::new(None)),
    ));
    // One publication may be selected by the publisher and another 64 queued.
    // There is no GTK drain: exceeding that bound must fail the connection closed.
    for _ in 0..crate::controller::refresh_controller::TRANSPORT_PUBLICATION_CAPACITY + 2 {
        mock.emit(Event::StudioModeChanged(false)).await;
    }
    tokio::time::timeout(TEST_DEADLINE, task)
        .await
        .unwrap()
        .unwrap();
    assert!(mock.client.retirement_reason().is_some());
    assert!(
        scope.has_terminal_reason(),
        "notification loss must remain explicit terminal evidence"
    );
    assert!(mock
        .client
        .activate_scene_if("B", false, &|| true)
        .await
        .is_err());
    scope.retire();
    let frames = mock.stop().await;
    assert!(frames.matching(&SCENE_WRITES).is_empty());
}
