//! The detail text for a failed local save.
//!
//! Every surface that reports a failed save puts this text into its own
//! "failed to save" message, so a save refused by another SceneDeck process
//! reads the same everywhere instead of as a raw lock-file error.

use i18n_embed_fl::fl;

use crate::infra::i18n::LANGUAGE_LOADER;
use crate::storage::lock::is_busy;

pub(crate) fn save_error_text(error: &std::io::Error) -> String {
    if is_busy(error) {
        fl!(LANGUAGE_LOADER, "storage-busy")
    } else {
        error.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::lock::lock_store;
    use crate::storage::test_support::TempDir;

    #[test]
    fn a_save_refused_by_another_process_names_that_process() {
        let dir = TempDir::new("save-error-busy");
        let data = dir.path().join("config.json");
        let _other_process = lock_store(&data).unwrap();
        let refused = lock_store(&data).err().unwrap();

        assert_eq!(
            save_error_text(&refused),
            "another running SceneDeck is saving the same file; nothing was written, try again"
        );
    }

    #[test]
    fn any_other_save_failure_keeps_its_own_detail() {
        let error = std::io::Error::other("disk full");

        assert_eq!(save_error_text(&error), "disk full");
    }
}
