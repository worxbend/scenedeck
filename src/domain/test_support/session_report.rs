//! A session report that quotes identifying data everywhere it can.

use crate::domain::{
    bookmark::{Bookmark, RecordingTime},
    output_result::{OutputResult, OutputResultKind, OutputResultStatus},
    session_report::{ReportDocument, ReportEvent, SessionReport},
    workflow::{ActivityOrigin, ActivityRecord, ScopeBinding},
};

pub(crate) const HOST: &str = "studio.lan";
pub(crate) const ENDPOINT: &str = "studio.lan:4455";
pub(crate) const CONNECTION_NAME: &str = "Studio PC";
pub(crate) const HOST_PATH: &str = "/home/alice/Videos/replay.mkv";

/// A report that quotes its endpoint and an OBS host path in every field
/// that can carry them, structured or free text.
pub(crate) fn identifying_document() -> ReportDocument {
    let mut report = SessionReport::new(1, 10, "profile-id".into(), ENDPOINT.into());
    report.connection_name = Some(CONNECTION_NAME.into());
    report.push(
        20,
        ReportEvent::Output(OutputResult {
            id: 1,
            kind: OutputResultKind::ReplaySave,
            scope: ScopeBinding {
                endpoint: ENDPOINT.into(),
                collection: "Show".into(),
            },
            status: OutputResultStatus::Observed,
            requested_unix_ms: 20,
            updated_unix_ms: 20,
            path: Some(HOST_PATH.into()),
            detail: Some(format!("moving {HOST_PATH} failed")),
        }),
    );
    report.push(
        21,
        ReportEvent::Activity(ActivityRecord {
            timestamp_unix_ms: 21,
            origin: ActivityOrigin::Connection,
            description: format!("#1 endpoint={ENDPOINT} saved {HOST_PATH}"),
        }),
    );
    report.push(
        22,
        ReportEvent::Ended {
            reason: format!("{ENDPOINT} closed"),
        },
    );
    let bookmark = Bookmark {
        id: 1,
        name: format!("clip at {HOST_PATH}"),
        created_unix_ms: 21,
        session_id: 1,
        session_started_unix_ms: 10,
        session_elapsed_ms: 11,
        connection_id: "profile-id".into(),
        endpoint: ENDPOINT.into(),
        collection: Some("Show".into()),
        scene: Some("Camera".into()),
        recording: RecordingTime::Unavailable,
    };
    ReportDocument {
        report,
        bookmarks: vec![bookmark],
    }
}
