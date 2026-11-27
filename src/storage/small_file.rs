//! Bounded, regular-file-only inputs. Validate the opened inode, not a path
//! checked before open, so swapping a pathname for a FIFO cannot block open.

use std::fs::OpenOptions;
use std::io::{self, Read};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

pub(crate) const CONFIG_LIMIT: usize = 1024 * 1024;
pub(crate) const REGISTRY_LIMIT: usize = 8 * 1024 * 1024;
/// Room for an adopted legacy registry at its own cap, plus as much again for
/// every other show workspace.
pub(crate) const WORKSPACE_STORE_LIMIT: usize = 2 * REGISTRY_LIMIT;
pub(crate) const YAML_LIMIT: usize = 8 * 1024 * 1024;
pub(crate) const CSS_LIMIT: usize = 1024 * 1024;

/// Symlinks and special files are deliberately unsupported at this boundary.
/// O_NONBLOCK prevents a FIFO open from waiting for a writer; fstat on that
/// same handle rejects it before any read. The byte cap also covers growth
/// after fstat, including sparse files and a concurrently modified inode.
pub(crate) fn read_to_string(path: &Path, limit: usize) -> io::Result<String> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected a regular file",
        ));
    }
    if metadata.len() > limit as u64 {
        return Err(too_large());
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(too_large());
    }
    String::from_utf8(bytes).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

/// Whether `read_to_string(path)` failed with `error` because `path` itself is
/// a symbolic link.
///
/// O_NOFOLLOW reports a final-component link as ELOOP, but so does a link loop
/// in a parent directory. The second lstat tells them apart. It only picks the
/// message for the operator; the refusal already happened on the open.
pub(crate) fn is_symlink_refusal(path: &Path, error: &io::Error) -> bool {
    error.raw_os_error() == Some(libc::ELOOP)
        && std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink())
}

pub(crate) fn check_size(length: u64, limit: usize) -> io::Result<()> {
    if length > limit as u64 {
        Err(too_large())
    } else {
        Ok(())
    }
}

fn too_large() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "file exceeds the application byte limit",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn fixture() -> std::path::PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "scenedeck-small-file-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn byte_limit_includes_exact_boundary_and_invalid_utf8() {
        let dir = fixture();
        let path = dir.join("input");
        for bytes in [b"".as_slice(), b"1234"] {
            std::fs::write(&path, bytes).unwrap();
            assert_eq!(read_to_string(&path, 4).unwrap().as_bytes(), bytes);
        }
        std::fs::write(&path, b"12345").unwrap();
        assert_eq!(
            read_to_string(&path, 4).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        std::fs::write(&path, [0xff]).unwrap();
        assert_eq!(
            read_to_string(&path, 4).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(1024 * 1024)
            .unwrap();
        assert_eq!(
            read_to_string(&path, 4).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn only_a_symlink_at_the_final_component_counts_as_a_symlink_refusal() {
        let dir = fixture();
        let regular = dir.join("regular");
        std::fs::write(&regular, b"safe").unwrap();
        let link = dir.join("link");
        std::os::unix::fs::symlink(&regular, &link).unwrap();
        std::os::unix::fs::symlink(dir.join("loop-b"), dir.join("loop-a")).unwrap();
        std::os::unix::fs::symlink(dir.join("loop-a"), dir.join("loop-b")).unwrap();
        let behind_loop = dir.join("loop-a").join("input");

        let link_error = read_to_string(&link, 4).unwrap_err();
        let loop_error = read_to_string(&behind_loop, 4).unwrap_err();

        assert!(is_symlink_refusal(&link, &link_error));
        assert_eq!(loop_error.raw_os_error(), Some(libc::ELOOP));
        assert!(!is_symlink_refusal(&behind_loop, &loop_error));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn special_files_and_symlinks_fail_without_waiting_for_a_fifo_writer() {
        const CHILD: &str = "SCENEDECK_SMALL_FILE_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let mut child = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "storage::small_file::tests::special_files_and_symlinks_fail_without_waiting_for_a_fifo_writer", "--test-threads=1"])
                .env(CHILD, "1").spawn().unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
            loop {
                if let Some(status) = child.try_wait().unwrap() {
                    assert!(status.success());
                    return;
                }
                if std::time::Instant::now() >= deadline {
                    child.kill().unwrap();
                    child.wait().unwrap();
                    panic!("regular-file validation blocked");
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        let dir = fixture();
        let fifo = dir.join("fifo.yaml");
        assert!(std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success());
        assert_eq!(
            read_to_string(&fifo, 4).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            read_to_string(&dir, 4).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        let link = dir.join("link");
        std::os::unix::fs::symlink(&fifo, &link).unwrap();
        assert!(read_to_string(&link, 4).is_err());
        let regular = dir.join("regular");
        std::fs::write(&regular, b"safe").unwrap();
        let regular_link = dir.join("regular-link");
        std::os::unix::fs::symlink(&regular, &regular_link).unwrap();
        assert!(read_to_string(&regular_link, 4).is_err());

        // Swap regular/FIFO inodes at the name while reads race with rename.
        // A path-check-then-blocking-open implementation can hang here.
        let racer = regular.clone();
        let spare = fifo.clone();
        let holding = dir.join("holding");
        let writer = std::thread::spawn(move || {
            for _ in 0..500 {
                std::fs::rename(&racer, &holding).unwrap();
                std::fs::rename(&spare, &racer).unwrap();
                std::fs::rename(&holding, &spare).unwrap();
            }
        });
        for _ in 0..1000 {
            if let Ok(text) = read_to_string(&regular, 4) {
                assert_eq!(text, "safe");
            }
        }
        writer.join().unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }
}
