//! Scene clicks follow the authoritative Studio mode and fail closed when it
//! cannot be read.

use crate::obs::testing::stateful::{Event, Request, StatefulObs, When, SCENE_WRITES};

#[tokio::test]
async fn scene_activation_preserves_current_mode_and_rejects_an_unavailable_mode() {
    for (studio_enabled, unavailable) in [(false, false), (true, false), (false, true)] {
        let obs = StatefulObs::default();
        let obs = if unavailable {
            obs.refuse(Request::GetStudioModeEnabled, When::Always)
        } else {
            obs
        };
        let mock = obs.connect().await;
        mock.emit(Event::StudioModeChanged(studio_enabled)).await;
        let result = mock
            .client
            .activate_scene_if("B", studio_enabled, &|| true)
            .await;
        assert_eq!(result.is_err(), unavailable);
        let frames = mock.stop().await;
        let writes = frames.matching(&SCENE_WRITES);
        if unavailable {
            assert!(writes.is_empty(), "unknown mode must fail closed");
        } else {
            assert_eq!(
                writes,
                vec![if studio_enabled {
                    Request::SetCurrentPreviewScene
                } else {
                    Request::SetCurrentProgramScene
                }]
            );
        }
    }
}
