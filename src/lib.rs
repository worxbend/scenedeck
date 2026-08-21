#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![allow(dead_code)]

//! SceneDeck application crate.
//!
//! The crate is organized by application boundary: domain types, storage
//! adapters, OBS adapters, controller orchestration, and GTK UI modules. The
//! binary entry point calls [`run`]; the internal modules stay crate-private so
//! the application does not accidentally expose its implementation layers as a
//! library API.

pub(crate) mod app;
pub(crate) mod app_info;
pub(crate) mod controller;
pub(crate) mod domain;
pub(crate) mod infra;
pub(crate) mod obs;
pub(crate) mod services;
pub(crate) mod storage;
pub(crate) mod ui;

/// Start the SceneDeck GTK application.
///
/// This initializes logging, creates the Tokio runtime used for OBS WebSocket
/// work, and then hands control to the GTK main loop.
pub fn run() {
    infra::logging::init();
    app::run();
}

#[cfg(test)]
mod architecture_tests {
    use std::path::{Path, PathBuf};

    /// Every `.rs` file under `src/`, with its path relative to `src/`.
    fn source_files() -> Vec<PathBuf> {
        fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
            let entries = std::fs::read_dir(dir).expect("src/ should be readable");
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    out.push(path);
                }
            }
        }

        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        walk(&root, &mut files);
        assert!(!files.is_empty(), "found no sources to check");
        files
            .into_iter()
            .map(|path| {
                path.strip_prefix(&root)
                    .expect("walked from src/")
                    .to_path_buf()
            })
            // This file is where the rules are written down, so its own
            // failure messages name the very crates the rules forbid.
            .filter(|path| path != Path::new("lib.rs"))
            .collect()
    }

    /// The OBS WebSocket library stays inside the OBS adapter.
    ///
    /// `CLAUDE.md` and `obs/mod.rs` both state this rule, and it had already
    /// been broken once: the controller's event loop matched raw `obws` event
    /// variants and carried its own copy of the mapping that `obs/mapper.rs`
    /// already owned. Nothing failed when that happened, because a documented
    /// rule that no check enforces is a suggestion.
    ///
    /// Keeping `obws` in one module is what makes the protocol replaceable and
    /// keeps protocol details out of code that decides what the app should do.
    #[test]
    fn only_the_obs_adapter_imports_obws() {
        let offenders: Vec<String> = source_files()
            .into_iter()
            .filter(|path| !path.starts_with("obs"))
            .filter_map(|path| {
                let full = Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("src")
                    .join(&path);
                let source = std::fs::read_to_string(&full).ok()?;
                let hits: Vec<String> = source
                    .lines()
                    .enumerate()
                    // Doc comments may name the crate while explaining the rule.
                    .filter(|(_, line)| {
                        let trimmed = line.trim_start();
                        !trimmed.starts_with("//") && trimmed.contains("obws")
                    })
                    .map(|(number, line)| {
                        format!("  {}:{}: {}", path.display(), number + 1, line.trim())
                    })
                    .collect();
                (!hits.is_empty()).then(|| hits.join("\n"))
            })
            .collect();

        assert!(
            offenders.is_empty(),
            "`obws` may only be used inside src/obs/. Wrap it in `ObsClient`, \
             `ObsEventStream`, or a `mapper` function and hand a domain type \
             across the boundary instead:\n{}",
            offenders.join("\n")
        );
    }

    /// Domain types stay free of GTK and of the OBS protocol.
    ///
    /// `src/domain/` holds what the app is about — scenes, roles, audio levels,
    /// output state. If a widget type or a protocol type reaches in there, the
    /// concepts stop being testable without a display or a running OBS, and
    /// the app's own vocabulary starts being shaped by its dependencies.
    #[test]
    fn domain_types_depend_on_neither_gtk_nor_obs() {
        let forbidden = ["gtk4", "adw::", "glib::", "gio::", "obws"];

        let offenders: Vec<String> = source_files()
            .into_iter()
            .filter(|path| path.starts_with("domain"))
            .filter_map(|path| {
                let full = Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("src")
                    .join(&path);
                let source = std::fs::read_to_string(&full).ok()?;
                let hits: Vec<String> = source
                    .lines()
                    .enumerate()
                    .filter(|(_, line)| {
                        let trimmed = line.trim_start();
                        !trimmed.starts_with("//")
                            && forbidden.iter().any(|name| trimmed.contains(name))
                    })
                    .map(|(number, line)| {
                        format!("  {}:{}: {}", path.display(), number + 1, line.trim())
                    })
                    .collect();
                (!hits.is_empty()).then(|| hits.join("\n"))
            })
            .collect();

        assert!(
            offenders.is_empty(),
            "src/domain/ must not depend on GTK or on `obws`; convert at the \
             adapter boundary instead:\n{}",
            offenders.join("\n")
        );
    }
}
