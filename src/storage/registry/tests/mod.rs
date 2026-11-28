//! `SceneRegistry` tests: in-memory curation, file persistence, and the
//! domain snapshot with the golden wire fixtures.

use crate::storage::test_support::TempDir;

mod curation;
mod persistence;
mod snapshot;

/// A path for a file that does not exist yet, inside a scratch directory
/// that is removed when the returned guard drops.
fn scratch_file(name: &str, extension: &str) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new(name);
    let path = dir.path().join(format!("registry.{extension}"));
    (dir, path)
}
