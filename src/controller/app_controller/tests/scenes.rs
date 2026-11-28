//! Scene switching routed through `AppController`: a queued switch never
//! crosses a scene collection boundary.

use std::sync::mpsc;

use tokio::runtime::Handle;

use crate::controller::app_controller::AppController;
use crate::controller::command::AppCommand;
use crate::controller::test_support::deps::dependencies;
use crate::controller::test_support::events::TEST_DEADLINE;
use crate::obs::testing::stateful::{Request, StatefulObs, When};

#[tokio::test]
async fn queued_primary_scene_switch_cannot_cross_a_collection_boundary() {
    let mut mock = StatefulObs::default()
        .hold(Request::GetVersion, When::AfterFirst)
        .connect()
        .await;
    let (tx, _rx) = mpsc::sync_channel(16);
    let mut controller = AppController::with_dependencies(Handle::current(), tx, dependencies());
    let scope = controller.session_scope();
    assert!(scope.install(mock.client.clone()));
    let mut reads = Vec::new();
    for _ in 0..8 {
        let client = mock.client.clone();
        reads.push(tokio::spawn(async move { client.get_version().await }));
    }
    mock.wait(Request::GetVersion, 9).await;
    controller.handle(AppCommand::SwitchPrimaryScene("A".into()));
    scope.advance_collection();
    mock.release_all().await;
    for read in reads {
        read.await.unwrap().unwrap();
    }
    tokio::time::timeout(TEST_DEADLINE, scope.quiesce())
        .await
        .unwrap();
    assert!(mock.client.retirement_reason().is_none());
    controller.shutdown().await.unwrap();
    let frames = mock.stop().await;
    assert_eq!(
        frames.count(Request::SetCurrentProgramScene),
        0,
        "a queued scene switch must not target a replacement collection"
    );
}
