//! Scratch directories for storage tests.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// A fresh directory under the system temp dir, removed with its contents on
/// drop.
pub(crate) struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// `tag` names the test in the directory name, so a leftover is traceable.
    pub(crate) fn new(tag: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before 1970")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "scenedeck-{tag}-{}-{nanos}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path)
            .unwrap_or_else(|error| panic!("create {}: {error}", path.display()));
        Self { path }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let removed = std::fs::remove_dir_all(&self.path);
        // Panicking again while a failed test unwinds would abort the test
        // binary and hide the original failure.
        if !std::thread::panicking() {
            removed.unwrap_or_else(|error| panic!("remove {}: {error}", self.path.display()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_dirs_are_distinct_and_removed_with_their_contents() {
        let first = TempDir::new("temp-dir");
        let second = TempDir::new("temp-dir");
        assert_ne!(first.path(), second.path());
        std::fs::write(first.path().join("file.json"), b"{}").unwrap();
        let removed = first.path().to_path_buf();

        drop(first);

        assert!(!removed.exists());
        assert!(second.path().is_dir());
    }
}
