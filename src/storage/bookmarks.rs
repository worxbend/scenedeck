//! Explicit operator bookmarks, independent of optional automatic session history.

use crate::domain::bookmark::{valid_name, Bookmark, BOOKMARK_LIMIT};
use serde::{Deserialize, Serialize};
use std::{io, path::Path};

const FILE_LIMIT: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BookmarkArchive {
    version: u32,
    pub next_id: u64,
    pub next_session: u64,
    pub entries: Vec<Bookmark>,
}

impl Default for BookmarkArchive {
    fn default() -> Self {
        Self {
            version: 1,
            next_id: 1,
            next_session: 1,
            entries: Vec::new(),
        }
    }
}

impl BookmarkArchive {
    fn validate(&self) -> io::Result<()> {
        let mut ids = std::collections::HashSet::new();
        if self.version != 1
            || self.next_id == 0
            || self.next_session == 0
            || self.entries.len() > BOOKMARK_LIMIT
            || self.entries.iter().any(|entry| {
                !ids.insert(entry.id)
                    || entry.id == 0
                    || entry.id >= self.next_id
                    || entry.session_id == 0
                    || entry.session_id >= self.next_session
                    || !valid_name(&entry.name)
            })
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid or unsupported bookmark archive",
            ));
        }
        Ok(())
    }
}

pub(crate) fn read(path: &Path) -> io::Result<BookmarkArchive> {
    let raw = match super::small_file::read_to_string(path, FILE_LIMIT) {
        Ok(raw) => raw,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(BookmarkArchive::default())
        }
        Err(error) => return Err(error),
    };
    let archive: BookmarkArchive = serde_json::from_str(&raw).map_err(io::Error::other)?;
    archive.validate()?;
    Ok(archive)
}

pub(crate) fn write(path: &Path, archive: &BookmarkArchive) -> io::Result<()> {
    archive.validate()?;
    // Refuse a corrupt or newer-format file even if it changed after startup.
    read(path)?;
    let raw = serde_json::to_vec_pretty(archive).map_err(io::Error::other)?;
    if raw.len() > FILE_LIMIT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Bookmark archive exceeds file size limit",
        ));
    }
    super::atomic::write(path, &raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bookmark_archive_preserves_counters_and_refuses_corruption_and_unknown_versions() {
        let dir = std::env::temp_dir().join(format!(
            "scenedeck-bookmarks-{}-{}",
            std::process::id(),
            crate::controller::workspace::unix_ms()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bookmarks.json");
        assert!(read(&path).unwrap().entries.is_empty());
        let mut archive = BookmarkArchive {
            next_id: 4,
            next_session: 7,
            ..Default::default()
        };
        write(&path, &archive).unwrap();
        assert_eq!(read(&path).unwrap().next_id, 4);
        assert_eq!(read(&path).unwrap().next_session, 7);
        archive.version = 2;
        assert!(write(&path, &archive).is_err());
        assert_eq!(read(&path).unwrap().next_id, 4);
        std::fs::write(&path, "{broken").unwrap();
        assert!(read(&path).is_err());
        assert!(write(&path, &BookmarkArchive::default()).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{broken");
        std::fs::write(
            &path,
            r#"{"version":2,"next_id":1,"next_session":1,"entries":[]}"#,
        )
        .unwrap();
        assert!(read(&path).is_err());
        assert!(write(&path, &BookmarkArchive::default()).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
