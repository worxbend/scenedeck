//! A cancelled or collection-invalidated workspace action never begins its
//! final write after a held preflight read.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use tokio::runtime::Handle;

use super::WorkspaceController;
use crate::controller::lifecycle::SessionScope;
use crate::controller::test_support::events::TEST_DEADLINE;
use crate::controller::test_support::workspace::{binding, request};
use crate::domain::control::ControlAction;
use crate::obs::testing::stateful::{Request, StatefulObs, When};

#[tokio::test]
async fn cancelled_or_collection_invalidated_text_preflight_never_begins_final_write() {
    for cancel_ui in [true, false] {
        let mut mock = StatefulObs::default()
            .hold(Request::GetInputDefaultSettings, When::Nth(2))
            .connect()
            .await;
        let slot = Arc::new(Mutex::new(None));
        let scope = SessionScope::initial(slot.clone()).next();
        assert!(scope.install(mock.client.clone()));
        let (tx, _rx) = mpsc::sync_channel(32);
        let mut workspace = WorkspaceController::new(Handle::current(), tx, slot);
        workspace.set_scope(scope.clone());
        workspace.request(
            request(&mock.client),
            Some(ControlAction::SetText {
                input: "Text".into(),
                text: "after".into(),
            }),
            Some("before".into()),
        );
        mock.wait(Request::GetInputDefaultSettings, 2).await;
        if cancel_ui {
            workspace.cancel();
        } else {
            scope.advance_collection();
        }
        mock.release_all().await;
        tokio::time::timeout(TEST_DEADLINE, scope.quiesce())
            .await
            .unwrap();
        mock.client.get_version().await.unwrap(); // Drained socket remains reusable.
        scope.retire();
        let frames = mock.stop().await;
        assert_eq!(frames.count(Request::SetInputSettings), 0);
    }
}

#[tokio::test]
async fn cancelled_or_collection_invalidated_media_preflight_never_begins_cursor_write() {
    for cancel_ui in [true, false] {
        let mut mock = StatefulObs::default()
            .hold(Request::GetMediaInputStatus, When::Nth(1))
            .connect()
            .await;
        let slot = Arc::new(Mutex::new(None));
        let scope = SessionScope::initial(slot.clone()).next();
        scope.install(mock.client.clone());
        let (tx, _rx) = mpsc::sync_channel(32);
        let mut workspace = WorkspaceController::new(Handle::current(), tx, slot);
        workspace.set_scope(scope.clone());
        workspace.audio(
            binding(&mock.client),
            ControlAction::SeekMedia {
                input: "Media".into(),
                position_ms: 500,
            },
        );
        mock.wait(Request::GetMediaInputStatus, 1).await;
        if cancel_ui {
            workspace.cancel();
        } else {
            scope.advance_collection();
        }
        mock.release_all().await;
        tokio::time::timeout(TEST_DEADLINE, scope.quiesce())
            .await
            .unwrap();
        mock.client.get_version().await.unwrap();
        scope.retire();
        assert_eq!(mock.stop().await.count(Request::SetMediaInputCursor), 0);
    }
}
