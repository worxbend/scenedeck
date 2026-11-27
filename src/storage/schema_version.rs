//! Newer-file guard for the JSON files a downgraded build may meet.
//!
//! A file written by a newer build parses here with its unknown fields
//! dropped. Writing that parse back would erase the newer build's data and
//! keep its version number, so readers use what they understand and writers
//! refuse.

use std::io;
use std::path::Path;

use serde::Deserialize;

/// Only the `version` key, so a newer file is recognised whether or not the
/// full parse succeeds.
#[derive(Deserialize)]
struct VersionProbe {
    version: Option<u32>,
}

/// The `version` in `raw`, when it is above `supported`.
///
/// Text that is not a JSON object with a `u32` version also gives `None`:
/// the full parse that follows reports it, so the probe error is dropped.
pub(crate) fn newer_version(raw: &str, supported: u32) -> Option<u32> {
    serde_json::from_str::<VersionProbe>(raw)
        .ok()
        .and_then(|probe| probe.version)
        .filter(|version| *version > supported)
}

/// The write refusal for content at `version`, which this build cannot
/// represent without loss.
pub(crate) fn refuse_newer(path: &Path, version: u32, supported: u32) -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        format!(
            "refusing to save {}: schema version {version} is newer than the {supported} \
             this build writes",
            path.display()
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_version_above_the_supported_one_is_reported() {
        assert_eq!(newer_version(r#"{"version":6,"x":1}"#, 5), Some(6));
        assert_eq!(newer_version(r#"{"version":5}"#, 5), None);
        assert_eq!(newer_version(r#"{"version":1}"#, 5), None);
        assert_eq!(newer_version(r#"{"scenes":{}}"#, 5), None);
    }

    #[test]
    fn text_the_full_parse_must_report_is_not_reported_as_newer() {
        for raw in ["{oops", r#"{"version":"6"}"#, r#"{"version":-1}"#] {
            assert_eq!(newer_version(raw, 5), None, "{raw}");
        }
    }

    #[test]
    fn the_refusal_is_unsupported_and_names_the_file() {
        let error = refuse_newer(Path::new("/cfg/config.json"), 9, 5);

        assert_eq!(error.kind(), io::ErrorKind::Unsupported);
        assert!(error.to_string().contains("/cfg/config.json"), "{error}");
    }
}
