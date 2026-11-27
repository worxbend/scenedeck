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

pub(crate) fn format_elapsed(since: Instant) -> String {
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
}
