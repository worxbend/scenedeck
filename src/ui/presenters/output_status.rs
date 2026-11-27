//! Stream and record status text.
//!
//! The transport bar and the window status bar describe an output the same
//! way, so they share these formatters.

use std::time::Instant;

use i18n_embed_fl::fl;

use crate::domain::output::{OutputKind, OutputStatus};
use crate::infra::i18n::LANGUAGE_LOADER;

pub(crate) fn output_label(
    kind: OutputKind,
    status: &OutputStatus,
    elapsed: Option<&str>,
) -> String {
    let kind = kind.label();
    let kind = kind.as_str();
    match elapsed {
        Some(elapsed) if status.active => fl!(
            LANGUAGE_LOADER,
            "live-output-label-with-elapsed",
            kind = kind,
            state = status.state.label(),
            elapsed = elapsed
        ),
        _ => fl!(
            LANGUAGE_LOADER,
            "live-output-label",
            kind = kind,
            state = status.state.label()
        ),
    }
}

/// The status-bar text for one output.
///
/// Every writer of the slot goes through here, so the once-a-second clock and
/// the output events cannot render the same state two different ways.
pub(crate) fn output_status_line(
    kind: OutputKind,
    status: &OutputStatus,
    active_since: Option<Instant>,
) -> String {
    let elapsed = active_since.map(format_elapsed);
    output_label(kind, status, elapsed.as_deref())
}

fn format_elapsed(since: Instant) -> String {
    let elapsed = since.elapsed().as_secs();
    let hours = elapsed / 3600;
    let minutes = (elapsed % 3600) / 60;
    let seconds = elapsed % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::output::OutputRunState;

    fn output_status(active: bool, state: OutputRunState) -> OutputStatus {
        OutputStatus {
            duration_ms: None,
            active,
            state,
            detail: None,
        }
    }

    #[test]
    fn output_label_shows_elapsed_time_only_while_active() {
        assert_eq!(
            output_label(
                OutputKind::Stream,
                &output_status(true, OutputRunState::Active),
                Some("00:01:23")
            ),
            "Stream: Active · 00:01:23"
        );
        assert_eq!(
            output_label(
                OutputKind::Record,
                &output_status(false, OutputRunState::Inactive),
                Some("00:01:23")
            ),
            "Record: Inactive"
        );
        assert_eq!(
            output_label(
                OutputKind::Stream,
                &output_status(true, OutputRunState::Active),
                None
            ),
            "Stream: Active"
        );
    }

    #[test]
    fn status_line_counts_elapsed_time_while_active() {
        assert_eq!(
            output_status_line(
                OutputKind::Stream,
                &output_status(true, OutputRunState::Active),
                Some(Instant::now())
            ),
            "Stream: Active · 0:00"
        );
    }

    #[test]
    fn status_line_hides_elapsed_time_while_inactive_with_a_start_time() {
        assert_eq!(
            output_status_line(
                OutputKind::Record,
                &output_status(false, OutputRunState::Unknown),
                Some(Instant::now())
            ),
            "Record: Unknown"
        );
    }

    #[test]
    fn status_line_without_a_start_time_shows_the_state_only() {
        assert_eq!(
            output_status_line(
                OutputKind::Record,
                &output_status(false, OutputRunState::Inactive),
                None
            ),
            "Record: Inactive"
        );
    }
}
