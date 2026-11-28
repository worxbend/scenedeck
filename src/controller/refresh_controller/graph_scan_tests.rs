//! Graph scans invalidated while they read, or while their completion waits
//! for GTK capacity, never publish a stale graph as current.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::controller::event::AppEvent;
use crate::controller::event_sender::EventSender;
use crate::controller::lifecycle::SessionScope;
use crate::controller::test_support::events::{
    graph_ready, peel, recv_event, recv_matching, TEST_DEADLINE,
};
use crate::controller::test_support::loopback::pump;
use crate::controller::test_support::state::app_state;
use crate::obs::testing::stateful::{Event, Request, StatefulObs, When};

#[tokio::test]
async fn manual_graph_scan_invalidated_midscan_cannot_publish_ready() {
    let mut mock = StatefulObs::default()
        .hold(Request::GetSceneItemList, When::Nth(5))
        .hold(Request::GetSceneList, When::Nth(3))
        .connect()
        .await;
    let scope = SessionScope::initial(Arc::new(Mutex::new(None))).next();
    scope.install(mock.client.clone());
    let (tx, rx) = mpsc::sync_channel(128);
    let sender = EventSender::new(tx).scoped(&scope);
    let task = tokio::spawn(crate::controller::refresh_controller::run_event_loop(
        mock.client.clone(),
        mock.take_events(),
        sender.clone(),
        vec![],
        Arc::new(Mutex::new(None)),
    ));
    graph_ready(&rx).await;
    let client = mock.client.clone();
    let manual = tokio::spawn(async move {
        crate::controller::refresh_controller::refresh_live_data(&client, &sender.fresh(), &[])
            .await;
    });
    mock.wait(Request::GetSceneItemList, 5).await;
    // Ignore earlier loading/audio publications; establish the actual structural
    // invalidation before releasing the old graph's held scene response.
    while rx.try_recv().is_ok() {}
    mock.emit(Event::SceneListChanged).await;
    recv_matching(&rx, |event| {
        matches!(peel(event.clone()), AppEvent::Observation { event, .. } if matches!(*event, AppEvent::GraphInvalidated))
    })
    .await;
    mock.release(Request::GetSceneItemList).await;
    tokio::time::timeout(TEST_DEADLINE, manual)
        .await
        .unwrap()
        .unwrap();
    mock.wait(Request::GetSceneList, 3).await;
    while let Ok(event) = rx.try_recv() {
        let event = peel(event);
        assert!(
            !matches!(event, AppEvent::Observation { event, .. } if matches!(*event, AppEvent::GraphUpdated(_) | AppEvent::GraphFailed(_))),
            "a manual scan invalidated mid-read must not publish a graph completion"
        );
    }
    mock.release_all().await;
    graph_ready(&rx).await;
    mock.client.retire();
    task.await.unwrap();
    scope.retire();
    mock.stop().await;
}

#[tokio::test]
async fn manual_graph_completion_waiting_for_gtk_capacity_rechecks_ingress_revision() {
    graph_completion_with_full_gtk_queue(GraphScan::Completes).await;
}

#[tokio::test]
async fn manual_graph_failure_waiting_for_gtk_capacity_rechecks_ingress_revision() {
    graph_completion_with_full_gtk_queue(GraphScan::Fails).await;
}

/// How the manual graph scan ends.
#[derive(Clone, Copy)]
enum GraphScan {
    Completes,
    /// Its scene item read is refused.
    Fails,
}

async fn graph_completion_with_full_gtk_queue(scan: GraphScan) {
    let obs = StatefulObs::default();
    let obs = match scan {
        GraphScan::Completes => obs,
        GraphScan::Fails => obs.refuse(Request::GetSceneItemList, When::Nth(3)),
    };
    let mut mock = obs.connect().await;
    let scope = SessionScope::initial(Arc::new(Mutex::new(None))).next();
    scope.install(mock.client.clone());
    // Exactly the four invalidation/inventory/audio publications fit; the
    // completed graph must wait for final GTK admission.
    let (tx, rx) = mpsc::sync_channel(4);
    let sender = EventSender::new(tx).scoped(&scope);
    let client = mock.client.clone();
    let manual = crate::controller::refresh_controller::refresh_live_data(&client, &sender, &[]);
    tokio::pin!(manual);
    tokio::select! {
        () = &mut manual => panic!("graph completion must be blocked by the full queue"),
        () = mock.wait(Request::GetSceneItemList, 3) => {}
    }
    // One later wire reply proves the graph responses passed through obws's
    // receiver. Poll the actual scan through graph_is_current and into send.
    mock.client.get_version().await.unwrap();
    assert!(futures_util::poll!(manual.as_mut()).is_pending());
    let scan_revision = scope.graph_revision();
    let task = tokio::spawn(crate::controller::refresh_controller::run_event_loop(
        mock.client.clone(),
        mock.take_events(),
        sender.clone(),
        vec![],
        Arc::new(Mutex::new(None)),
    ));
    mock.emit(Event::SceneListChanged).await;
    tokio::time::timeout(TEST_DEADLINE, async {
        while scope.graph_revision() == scan_revision {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    // Release one slot and poll this independent worker FIRST. The obsolete
    // completion must not enter ahead of the queued session invalidation.
    // EventSender retries SyncSender capacity on a two-millisecond timer.
    // Let that timer mature while capacity is still unavailable, then poll
    // synchronously so a competing publisher cannot win the released slot.
    tokio::time::sleep(Duration::from_millis(5)).await;
    rx.try_recv().unwrap();
    assert!(futures_util::poll!(manual.as_mut()).is_ready());
    let admitted: Vec<_> = rx.try_iter().collect();
    assert!(!admitted.iter().cloned().map(peel).any(|event|
        matches!(event, AppEvent::Observation { event, .. } if matches!(*event, AppEvent::GraphUpdated(_) | AppEvent::GraphFailed(_)))
    ), "obsolete graph completion overtook ingress invalidation: {admitted:?}");
    mock.client.retire();
    task.await.unwrap();
    scope.retire();
    mock.stop().await;
}

#[tokio::test]
async fn graph_invalidated_midscan_never_publishes_ready_before_followup_completes() {
    let mut mock = StatefulObs::default()
        .hold(Request::GetSceneItemList, When::Nth(2))
        .hold(Request::GetSceneList, When::Nth(2))
        .connect()
        .await;
    let (tx, rx) = mpsc::sync_channel(128);
    let task = pump(&mut mock, tx);
    let mut state = app_state();
    mock.wait(Request::GetSceneItemList, 2).await;
    while let Ok(event) = rx.try_recv() {
        if matches!(peel(event), AppEvent::GraphInvalidated) {
            state.set_topology_loading();
        }
    }
    mock.emit(Event::SceneListChanged).await;
    loop {
        let event = peel(recv_event(&rx).await);
        let event = match event {
            AppEvent::Observation { revision, event } => {
                state.reconcile_observation(revision, *event)
            }
            event => Some(event),
        };
        if matches!(event, Some(AppEvent::GraphInvalidated)) {
            state.set_topology_loading();
            break;
        }
    }
    mock.release_all().await;
    mock.wait(Request::GetSceneList, 2).await;
    while let Ok(event) = rx.try_recv() {
        let event = peel(event);
        let event = match event {
            AppEvent::Observation { revision, event } => {
                state.reconcile_observation(revision, *event)
            }
            event => Some(event),
        };
        assert!(
            !matches!(event, Some(AppEvent::GraphUpdated(_))),
            "superseded graph must not mark topology Ready while follow-up is blocked"
        );
    }
    assert_ne!(
        state.topology_status,
        crate::controller::state::TopologyStatus::Ready
    );
    mock.release_all().await;
    graph_ready(&rx).await;
    mock.client.retire();
    task.await.unwrap();
    mock.stop().await;
}
