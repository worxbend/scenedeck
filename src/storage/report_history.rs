//! Bounded session history with optimistic conflict detection and atomic replacement.

use crate::domain::{
    bookmark::BOOKMARK_LIMIT,
    session_report::{SessionReport, EVENT_LIMIT, HISTORY_LIMIT, SAMPLE_LIMIT},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashSet, VecDeque},
    fs::{DirBuilder, OpenOptions},
    io,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::Path,
};

const FILE_LIMIT: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HistoryArchive {
    version: u32,
    pub entries: VecDeque<SessionReport>,
}

impl Default for HistoryArchive {
    fn default() -> Self {
        Self {
            version: 1,
            entries: VecDeque::new(),
        }
    }
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn valid_number(value: Option<f64>) -> bool {
    value.is_none_or(|value| value.is_finite() && value >= 0.0)
}

impl HistoryArchive {
    fn validate(&self) -> io::Result<()> {
        let mut ids = HashSet::new();
        if self.version != 1 || self.entries.len() > HISTORY_LIMIT {
            return Err(invalid("Invalid or unsupported session history"));
        }
        for report in &self.entries {
            if report.version != 1
                || report.id.is_empty()
                || report.id.len() > 256
                || !report
                    .id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
                || !ids.insert(&report.id)
                || report.events.len() > EVENT_LIMIT
                || report.samples.len() > SAMPLE_LIMIT
                || report.bookmark_ids.len() > BOOKMARK_LIMIT
                || report.bookmark_ids.contains(&0)
                || report.bookmark_ids.iter().collect::<HashSet<_>>().len()
                    != report.bookmark_ids.len()
                || report
                    .omitted_samples
                    .checked_add(report.samples.len() as u64)
                    != Some(report.samples_seen)
                || !valid_number(report.peak_cpu_percent)
                || !valid_number(report.minimum_fps)
                || report.samples.iter().any(|sample| {
                    [
                        sample.cpu_percent,
                        sample.fps,
                        sample.render_ms,
                        sample.free_disk_mib,
                        sample.bitrate_kbps,
                    ]
                    .into_iter()
                    .any(|value| !valid_number(value))
                })
            {
                return Err(invalid("Invalid session report in history"));
            }
        }
        Ok(())
    }

    pub fn upsert(&mut self, report: SessionReport) -> io::Result<()> {
        if let Some(existing) = self.entries.iter_mut().find(|entry| entry.id == report.id) {
            if existing.started_unix_ms != report.started_unix_ms
                || existing.connection_id != report.connection_id
                || existing.endpoint != report.endpoint
            {
                return Err(invalid(
                    "Session identity collision; history was not replaced",
                ));
            }
            *existing = report;
        } else {
            self.entries.push_back(report);
        }
        Ok(())
    }

    /// Receipt order is retained even if the wall clock moves backwards.
    pub fn retain(&mut self, count: usize) {
        while self.entries.len() > count.clamp(1, HISTORY_LIMIT) {
            self.entries.pop_front();
        }
    }
}

pub(crate) fn read(path: &Path) -> io::Result<HistoryArchive> {
    let text = match super::small_file::read_to_string(path, FILE_LIMIT) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(HistoryArchive::default())
        }
        Err(error) => return Err(error),
    };
    let archive: HistoryArchive = serde_json::from_str(&text).map_err(io::Error::other)?;
    archive.validate()?;
    Ok(archive)
}

pub(crate) fn write(
    path: &Path,
    expected: &HistoryArchive,
    replacement: &HistoryArchive,
) -> io::Result<()> {
    replacement.validate()?;
    if let Some(parent) = path.parent() {
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)?;
    }
    // Cooperative writers serialize the read/compare/replace transaction. Never
    // wait indefinitely for another app process, or follow a special lock file.
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path.with_extension("lock"))?;
    if !lock.metadata()?.is_file() {
        return Err(invalid("Expected a regular history lock file"));
    }
    lock.try_lock().map_err(io::Error::other)?;
    if &read(path)? != expected {
        return Err(io::Error::other(
            "Session history changed on disk; reload before retrying",
        ));
    }
    let bytes = serde_json::to_vec(replacement).map_err(io::Error::other)?;
    super::small_file::check_size(bytes.len() as u64, FILE_LIMIT)?;
    super::atomic::write(path, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(id: u64) -> SessionReport {
        SessionReport::new(id, 100 - id, "connection".into(), "fixture:4455".into())
    }
    fn path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir()
            .join(format!("scenedeck-history-{label}-{}", std::process::id()))
            .join("history.json")
    }

    #[test]
    fn retention_uses_session_order_and_refuses_identity_collision() {
        let mut archive = HistoryArchive::default();
        for id in 1..=5 {
            archive.upsert(report(id)).unwrap();
            archive.retain(3);
        }
        assert_eq!(
            archive
                .entries
                .iter()
                .map(|entry| entry.session_id)
                .collect::<Vec<_>>(),
            [3, 4, 5]
        );
        let mut changed = archive.entries[0].clone();
        changed.endpoint = "replacement:4455".into();
        assert!(archive.upsert(changed).is_err());
        assert_eq!(archive.entries[0].endpoint, "fixture:4455");
    }

    #[test]
    fn writes_preserve_corrupt_newer_and_concurrently_changed_files() {
        let path = path("conflicts");
        let empty = read(&path).unwrap();
        let mut desired = empty.clone();
        desired.upsert(report(1)).unwrap();
        write(&path, &empty, &desired).unwrap();
        assert_eq!(read(&path).unwrap(), desired);
        let mut second = empty.clone();
        second.upsert(report(2)).unwrap();
        assert!(write(&path, &empty, &second).is_err());
        assert_eq!(read(&path).unwrap(), desired);
        for contents in ["{broken", "{\"version\":2,\"entries\":[]}"] {
            std::fs::write(&path, contents).unwrap();
            assert!(read(&path).is_err());
            assert!(write(&path, &desired, &second).is_err());
            assert_eq!(std::fs::read_to_string(&path).unwrap(), contents);
        }
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn bounded_reads_and_lock_conflicts_preserve_the_archive() {
        let path = path("bounds");
        let empty = HistoryArchive::default();
        write(&path, &empty, &empty).unwrap();
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path.with_extension("lock"))
            .unwrap();
        lock.try_lock().unwrap();
        assert!(write(&path, &empty, &empty).is_err());
        drop(lock);
        let file = OpenOptions::new().write(true).open(&path).unwrap();
        file.set_len(FILE_LIMIT as u64 + 1).unwrap();
        assert!(read(&path).is_err());
        assert!(write(&path, &empty, &empty).is_err());
        assert_eq!(
            std::fs::metadata(&path).unwrap().len(),
            FILE_LIMIT as u64 + 1
        );
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn fractional_telemetry_roundtrips_without_false_disk_conflicts() {
        let path = path("fractional");
        let empty = HistoryArchive::default();
        let mut archive = empty.clone();
        let mut report = report(1);
        let mut stats = crate::domain::health::tests::stats();
        // Without serde_json's exact float parser this loses one bit on read,
        // so the next checkpoint incorrectly sees a concurrent file change.
        stats.cpu_usage_percent = f64::from_bits(0x405c0d86c7a996b8);
        report.sample(crate::domain::session_report::ReportSample::new(
            10,
            stats,
            Some(123.23456789123456),
            None,
        ));
        archive.upsert(report).unwrap();
        write(&path, &empty, &archive).unwrap();
        assert_eq!(read(&path).unwrap(), archive);
        write(&path, &archive, &archive).unwrap();
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
