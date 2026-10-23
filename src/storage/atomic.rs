//! Crash-safe file replacement.
//!
//! Every local file SceneDeck owns is rewritten whole rather than edited in
//! place, so a write that is interrupted half way through is the difference
//! between "the last setting did not stick" and "the file no longer parses and
//! every setting is gone".

#[cfg(test)]
use std::fs::create_dir_all;
use std::fs::{remove_file, rename, DirBuilder, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

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
        DirBuilder::new().recursive(true).mode(0o700).create(dir)?;
    }

    let (temp, mut file) = create_temp_file(path)?;

    // Scoped so the handle is closed before the rename.
    let write_result = (|| {
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

    // The rename's durability is only as good as the directory entry's: sync
    // the parent so a crash cannot roll back to the old file after this call
    // has reported success. Best-effort — the data itself is already safe by
    // this point, and a failed directory sync is no reason to tell the caller
    // the write did not happen.
    if let Some(dir) = path.parent() {
        if let Ok(dir_handle) = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NONBLOCK | libc::O_NOFOLLOW)
            .open(dir)
        {
            let _ = dir_handle.sync_all();
        }
    }

    Ok(())
}

/// Sibling path used while the new contents are being written.
///
/// The suffix keeps it out of the way of any real file: the loaders look for
/// exact names (`config.json`, `registry.json`), so a leftover `.new-*` from a
/// killed process is ignored rather than read.
fn temp_path(path: &Path) -> std::path::PathBuf {
    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(
        ".new-{}-{}",
        std::process::id(),
        NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ));
    path.with_file_name(name)
}

/// Create a new sibling without ever truncating another writer's temporary.
///
/// The process ID and monotonic counter make collisions unusual; `create_new`
/// makes them harmless when a stale temporary from a reused PID is present.
fn create_temp_file(path: &Path) -> io::Result<(std::path::PathBuf, File)> {
    loop {
        let temp = temp_path(path);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)
        {
            Ok(file) => return Ok((temp, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
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
    fn atomic_files_and_new_directories_are_always_private() {
        use std::os::unix::fs::PermissionsExt;
        const CHILD: &str = "SCENEDECK_ATOMIC_MODE_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let status = std::process::Command::new("sh")
                .args(["-c", "umask 000; exec \"$@\"", "scenedeck-mode-test"])
                .arg(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "storage::atomic::tests::atomic_files_and_new_directories_are_always_private",
                    "--test-threads=1",
                ])
                .env(CHILD, "1")
                .status()
                .unwrap();
            assert!(status.success());
            return;
        }
        let dir = temp_dir("private");
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o777)).unwrap();
        let path = dir.join("private.yaml");
        for mode in [0o600, 0o640] {
            std::fs::write(&path, b"old").unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
            write(&path, b"new").unwrap();
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        let new = dir.join("created/settings.json");
        write(&new, b"{}").unwrap();
        assert_eq!(
            std::fs::metadata(&new).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(new.parent().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
            0o777,
            "never chmod arbitrary export parents"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn failed_atomic_rename_keeps_original_and_cleans_temporary() {
        let dir = temp_dir("failure");
        let target = dir.join("target");
        std::fs::create_dir(&target).unwrap();
        assert!(write(&target, b"new").is_err());
        assert!(target.is_dir());
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(dir).unwrap();
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

    #[test]
    fn concurrent_writers_receive_distinct_temporary_paths() {
        let path = Path::new("/some/where/config.json");

        assert_ne!(temp_path(path), temp_path(path));
    }
}
