//! Inter-process exclusion for read-check-replace transactions on one store.
//!
//! The snap build runs without GApplication uniqueness, so two SceneDeck
//! processes can save the same file. Each store's write holds an advisory
//! lock on a sibling `<stem>.lock` for the whole transaction: the check that
//! the file on disk is still replaceable and the atomic replacement after it.
//! The data file itself cannot carry the lock: `atomic::write` renames a new
//! inode over it, so a lock on the old inode would guard nothing.

use std::fs::{DirBuilder, File, OpenOptions, TryLockError};
use std::io;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

/// Held for one transaction; dropping it releases the lock.
#[must_use = "the store is only locked while this guard lives"]
pub(crate) struct StoreLock {
    _file: File,
}

/// Another process holds the store's lock. The write did not happen, and
/// retrying once that process finishes is safe.
#[derive(Debug, thiserror::Error)]
#[error("another SceneDeck process is saving {}; try again", .lock.display())]
struct StoreBusy {
    lock: PathBuf,
}

/// Lock the store at `data_path` without waiting.
///
/// A held lock fails at once with [`io::ErrorKind::ResourceBusy`] (see
/// [`is_busy`]): a save must never block its lane on another app process.
/// The lock file is opened without following a symlink and must be a regular
/// file, so a planted FIFO or link cannot stall or redirect the open.
pub(crate) fn lock_store(data_path: &Path) -> io::Result<StoreLock> {
    if let Some(parent) = data_path.parent() {
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)?;
    }
    let lock = data_path.with_extension("lock");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(&lock)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("expected a regular lock file at {}", lock.display()),
        ));
    }
    match file.try_lock() {
        Ok(()) => Ok(StoreLock { _file: file }),
        Err(TryLockError::WouldBlock) => Err(io::Error::new(
            io::ErrorKind::ResourceBusy,
            StoreBusy { lock },
        )),
        Err(TryLockError::Error(error)) => Err(error),
    }
}

/// Whether `error` is a refused [`lock_store`], as opposed to any other
/// failure that happens to carry `ResourceBusy`.
pub(crate) fn is_busy(error: &io::Error) -> bool {
    error
        .get_ref()
        .is_some_and(|source| source.is::<StoreBusy>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_support::TempDir;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn a_held_store_lock_refuses_a_second_holder_until_it_is_dropped() {
        let dir = TempDir::new("lock-held");
        let data = dir.path().join("config.json");
        let held = lock_store(&data).unwrap();

        let refused = lock_store(&data).err().unwrap();

        assert_eq!(refused.kind(), io::ErrorKind::ResourceBusy);
        assert!(is_busy(&refused));
        drop(held);
        assert!(lock_store(&data).is_ok());
    }

    #[test]
    fn the_lock_is_a_sibling_file_so_replacing_the_data_keeps_it_held() {
        let dir = TempDir::new("lock-sibling");
        let data = dir.path().join("config.json");
        let _held = lock_store(&data).unwrap();

        crate::storage::atomic::write(&data, b"{}").unwrap();

        assert!(dir.path().join("config.lock").is_file());
        assert!(is_busy(&lock_store(&data).err().unwrap()));
    }

    #[test]
    fn locking_creates_a_missing_private_parent_directory() {
        let dir = TempDir::new("lock-parent");
        let parent = dir.path().join("scenedeck");

        let _held = lock_store(&parent.join("bookmarks.json")).unwrap();

        let mode = std::fs::metadata(&parent).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700);
    }

    #[test]
    fn a_symlinked_or_special_lock_file_is_refused_without_reporting_busy() {
        let dir = TempDir::new("lock-special");
        let linked = dir.path().join("linked.json");
        std::os::unix::fs::symlink(dir.path().join("elsewhere"), dir.path().join("linked.lock"))
            .unwrap();
        let directory = dir.path().join("directory.json");
        std::fs::create_dir(dir.path().join("directory.lock")).unwrap();

        for data in [linked, directory] {
            let refused = lock_store(&data).err().unwrap();
            assert!(!is_busy(&refused), "{refused}");
        }
        assert!(!dir.path().join("elsewhere").exists());
    }

    #[test]
    fn an_unrelated_resource_busy_error_is_not_a_held_lock() {
        let other = io::Error::from(io::ErrorKind::ResourceBusy);

        assert!(!is_busy(&other));
    }
}
