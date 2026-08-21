//! Crash-safe file replacement.
//!
//! Every local file SceneDeck owns is rewritten whole rather than edited in
//! place, so a write that is interrupted half way through is the difference
//! between "the last setting did not stick" and "the file no longer parses and
//! every setting is gone".

use std::fs::{create_dir_all, remove_file, rename, File};
use std::io::{self, Write};
use std::path::Path;

/// Replace `path` with `contents`, or leave the existing file untouched.
///
/// `std::fs::write` truncates the target and then fills it, so a crash, a full
/// disk, or two writers racing can leave a file that is empty or half-written.
/// The loaders treat an unparsable file as absent, so that costs the user their
/// whole configuration.
///
/// This writes a sibling temporary file, flushes it all the way to disk, and
/// then renames it over the target. `rename` within a directory is atomic on
/// Linux: a reader sees either the old file or the new one, never a partial
/// one. The temporary has to be a sibling for that to hold — renaming across
/// filesystems fails with `EXDEV`, so it must not go to the system temp
/// directory.
///
/// The `sync_all` matters as much as the rename: without it the rename can
/// reach the disk before the data does, and a crash in that window leaves an
/// empty file — precisely the failure this is here to prevent.
pub(crate) fn write(path: &Path, contents: &[u8]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        create_dir_all(dir)?;
    }

    let temp = temp_path(path);

    // Scoped so the handle is closed before the rename.
    let write_result = (|| {
        let mut file = File::create(&temp)?;
        file.write_all(contents)?;
        file.sync_all()
    })();

    if let Err(error) = write_result {
        let _ = remove_file(&temp);
        return Err(error);
    }

    if let Err(error) = rename(&temp, path) {
        let _ = remove_file(&temp);
        return Err(error);
    }

    Ok(())
}

/// Sibling path used while the new contents are being written.
///
/// The suffix keeps it out of the way of any real file: the loaders look for
/// exact names (`config.json`, `registry.json`), so a leftover `.new` from a
/// killed process is ignored rather than read.
fn temp_path(path: &Path) -> std::path::PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".new");
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unique directory under the system temp dir for one test to work in.
    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "scenedeck-atomic-{}-{}-{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn writes_the_contents_and_leaves_no_temporary_behind() {
        let dir = temp_dir("write");
        let path = dir.join("config.json");

        write(&path, b"{\"version\":3}").unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"{\"version\":3}");
        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name())
            .filter(|name| name != "config.json")
            .collect();
        assert!(leftovers.is_empty(), "left behind {leftovers:?}");

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn replaces_an_existing_file_whole() {
        let dir = temp_dir("replace");
        let path = dir.join("registry.json");
        std::fs::write(&path, b"an older and much longer set of contents").unwrap();

        write(&path, b"short").unwrap();

        // No tail of the previous contents survives, which is what makes the
        // result parseable rather than a blend of two versions.
        assert_eq!(std::fs::read(&path).unwrap(), b"short");

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn creates_missing_parent_directories() {
        let dir = temp_dir("parents");
        let path = dir.join("nested").join("deeper").join("config.json");

        write(&path, b"{}").unwrap();

        assert!(path.exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_temporary_is_a_sibling_so_the_rename_stays_on_one_filesystem() {
        // A temporary in the system temp directory would be a different mount
        // on many setups, and `rename` across filesystems fails with EXDEV.
        let path = Path::new("/some/where/config.json");
        assert_eq!(temp_path(path).parent(), path.parent());
    }
}
