//! Explicit report exports. No connection configuration or credentials are read.

use crate::domain::{
    output_result::OutputResultStatus,
    session_report::{ReportDocument, ReportEvent},
    workflow::ActivityOrigin,
};
use std::{io, path::Path};

#[derive(Clone, Copy)]
pub(crate) enum Format {
    Json,
    Csv,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Csv => "csv",
        }
    }
}

pub(crate) fn write(path: &Path, report: &ReportDocument, format: Format) -> io::Result<()> {
    let bytes = match format {
        Format::Json => serde_json::to_vec_pretty(report).map_err(io::Error::other)?,
        Format::Csv => csv(report)?.into_bytes(),
    };
    super::atomic::write(path, &bytes)
}

/// Quote every cell and neutralize spreadsheet formulas, including prefixes
/// hidden behind whitespace. JSON export retains the exact original strings.
fn cell(value: &str) -> String {
    let suspicious = value
        .trim_start_matches(|c: char| c.is_whitespace() || c.is_control())
        .starts_with(['=', '+', '-', '@']);
    format!(
        "\"{}{}\"",
        if suspicious { "'" } else { "" },
        value.replace('"', "\"\"")
    )
}

fn row(output: &mut String, values: &[String]) {
    output.push_str(
        &values
            .iter()
            .map(|value| cell(value))
            .collect::<Vec<_>>()
            .join(","),
    );
    output.push_str("\r\n");
}

fn json(value: &impl serde::Serialize) -> io::Result<String> {
    serde_json::to_string(value).map_err(io::Error::other)
}
fn number(value: Option<f64>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn csv(document: &ReportDocument) -> io::Result<String> {
    let mut output = String::new();
    row(
        &mut output,
        &[
            "type",
            "timestamp_unix_ms",
            "evidence",
            "description",
            "obs_host_path",
            "cpu_percent",
            "fps",
            "render_ms",
            "free_disk_mib",
            "bitrate_kbps",
            "render_skipped_frames_cumulative",
            "output_skipped_frames_cumulative",
            "stream_skipped_frames_cumulative",
            "details_json",
        ]
        .map(str::to_owned),
    );
    let report = &document.report;
    let metadata = serde_json::json!({
        "version": report.version, "id": report.id, "session_id": report.session_id,
        "connection_id": report.connection_id, "endpoint": report.endpoint,
        "connection_name": report.connection_name, "interrupted": report.interrupted,
        "started_unix_ms": report.started_unix_ms, "ended_unix_ms": report.ended_unix_ms,
        "omitted_events": report.omitted_events, "activity_gaps": report.activity_gaps,
        "samples_seen": report.samples_seen, "omitted_samples": report.omitted_samples,
        "peak_cpu_percent": report.peak_cpu_percent, "minimum_fps": report.minimum_fps,
    });
    let mut values = vec![String::new(); 14];
    values[0] = "session".into();
    values[1] = report.started_unix_ms.to_string();
    values[13] = json(&metadata)?;
    row(&mut output, &values);
    for record in &report.events {
        let mut values = vec![String::new(); 14];
        values[0] = "event".into();
        values[1] = record.timestamp_unix_ms.to_string();
        values[13] = json(record)?;
        match &record.event {
            ReportEvent::Activity(activity) => {
                values[2] = match activity.origin {
                    ActivityOrigin::Requested => "requested",
                    ActivityOrigin::Observed => "observed",
                    ActivityOrigin::Failed => "failed",
                    ActivityOrigin::Connection => "connection",
                }
                .into();
                values[3] = activity.description.clone();
            }
            ReportEvent::Output(result) => {
                values[2] = json(&result.status)?.trim_matches('"').into();
                values[3] = json(&result.kind)?.trim_matches('"').into();
                if result.status == OutputResultStatus::Observed {
                    values[4] = result.path.clone().unwrap_or_default();
                }
            }
            ReportEvent::OutputAcknowledged { kind } => {
                values[2] = "acknowledged".into();
                values[3] = json(kind)?.trim_matches('"').into();
            }
            ReportEvent::Scene { scene, .. } => {
                values[2] = "observed".into();
                values[3] = scene.clone().unwrap_or_default();
            }
            ReportEvent::Health { kind, phase, .. } => {
                values[2] = phase.as_deref().unwrap_or("cleared").into();
                values[3] = kind.clone();
            }
            ReportEvent::Ended { reason } => {
                values[2] = "session_ended".into();
                values[3] = reason.clone();
            }
        }
        row(&mut output, &values);
    }
    for sample in &report.samples {
        row(
            &mut output,
            &[
                "sample".into(),
                sample.timestamp_unix_ms.to_string(),
                "observed".into(),
                String::new(),
                String::new(),
                number(sample.cpu_percent),
                number(sample.fps),
                number(sample.render_ms),
                number(sample.free_disk_mib),
                number(sample.bitrate_kbps),
                sample.render_skipped_frames.to_string(),
                sample.output_skipped_frames.to_string(),
                sample
                    .stream_skipped_frames
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                json(sample)?,
            ],
        );
    }
    for note in &document.bookmarks {
        let mut values = vec![String::new(); 14];
        values[0] = "bookmark".into();
        values[1] = note.created_unix_ms.to_string();
        values[2] = "operator_note".into();
        values[3] = note.name.clone();
        values[13] = json(note)?;
        row(&mut output, &values);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        output_result::{OutputResult, OutputResultKind},
        session_report::SessionReport,
        workflow::ScopeBinding,
    };

    #[test]
    fn csv_quotes_multiline_cells_and_neutralizes_formula_prefixes() {
        assert_eq!(cell("a,\"b\"\nc"), "\"a,\"\"b\"\"\nc\"");
        for prefix in ["=", "+", "-", "@", " \t=", "\r\n@"] {
            assert!(cell(&format!("{prefix}SUM(1,2)")).starts_with("\"'"));
        }
        assert_eq!(cell("scene = main"), "\"scene = main\"");
    }

    #[test]
    fn exports_keep_acknowledgement_separate_from_file_evidence_and_roundtrip() {
        let mut report = SessionReport::new(1, 10, "connection".into(), "localhost:4455".into());
        let mut result = OutputResult {
            id: 1,
            kind: OutputResultKind::ReplaySave,
            scope: ScopeBinding::default(),
            status: OutputResultStatus::Acknowledged,
            requested_unix_ms: 10,
            updated_unix_ms: 20,
            path: None,
            detail: None,
        };
        report.push(20, ReportEvent::Output(result.clone()));
        result.id = 2;
        result.status = OutputResultStatus::Observed;
        result.path = Some("/obs/clip,\"one\".mkv".into());
        report.push(30, ReportEvent::Output(result));
        let document = ReportDocument {
            report,
            bookmarks: Vec::new(),
        };
        let dir =
            std::env::temp_dir().join(format!("scenedeck-report-export-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("exact-user-target.txt");
        write(&target, &document, Format::Json).unwrap();
        let decoded: ReportDocument =
            serde_json::from_slice(&std::fs::read(&target).unwrap()).unwrap();
        assert_eq!(decoded.report.events, document.report.events);
        write(&target, &document, Format::Csv).unwrap();
        let text = std::fs::read_to_string(&target).unwrap();
        assert!(text.contains("\"acknowledged\",\"replay_save\",\"\","));
        assert!(text.contains("\"observed\",\"replay_save\",\"/obs/clip,\"\"one\"\".mkv\""));
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
