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
    //! Fitness tests for the module boundaries described in `CLAUDE.md`.
    //!
    //! Each boundary here was written down before it was checked, and at least
    //! one of them had already been broken while the documentation still said
    //! otherwise. Nothing failed when that happened, because a documented rule
    //! that no check enforces is a suggestion. These tests turn the prose into
    //! something that can be violated only on purpose.

    use std::path::{Path, PathBuf};

    fn src_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
    }

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

        let root = src_root();
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

    /// Lines of `path` that ship in the binary, paired with their 1-based number.
    ///
    /// Everything from the first `#[cfg(test)]` onward is dropped. Tests in
    /// this crate are inline modules at the end of each file, and a test is
    /// allowed to reach across a boundary to build a fixture — constructing an
    /// `AppController` to check a layout, for instance. The boundaries apply to
    /// what the application actually does.
    ///
    /// Doc comments and ordinary comments are dropped too: several of them
    /// name a forbidden crate while explaining why it is forbidden.
    fn production_lines(path: &Path) -> Vec<(usize, String)> {
        let source =
            std::fs::read_to_string(src_root().join(path)).expect("source should be readable");
        source
            .lines()
            .enumerate()
            .take_while(|(_, line)| !line.trim_start().starts_with("#[cfg(test)]"))
            .filter(|(_, line)| !line.trim_start().starts_with("//"))
            .map(|(index, line)| (index + 1, line.to_string()))
            .collect()
    }

    /// Report every production line under `scope` that names a forbidden term.
    ///
    /// `scope` is a path prefix relative to `src/`; an empty prefix means the
    /// whole crate. `exempt` lists files allowed to break the rule, which is
    /// how a boundary names its own gateway.
    fn violations(scope: &str, forbidden: &[&str], exempt: &[&str]) -> Vec<String> {
        source_files()
            .into_iter()
            .filter(|path| path.starts_with(scope))
            .filter(|path| !exempt.iter().any(|allowed| path == Path::new(allowed)))
            .flat_map(|path| {
                production_lines(&path)
                    .into_iter()
                    .filter(|(_, line)| forbidden.iter().any(|name| line.contains(name)))
                    .map(|(number, line)| {
                        format!("  {}:{}: {}", path.display(), number, line.trim())
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// The OBS WebSocket library stays inside the OBS adapter.
    ///
    /// Keeping `obws` in one module is what makes the protocol replaceable and
    /// keeps protocol details out of code that decides what the app should do.
    /// This had already been broken once: the controller's event loop matched
    /// raw `obws` event variants and carried its own copy of the mapping that
    /// `obs/mapper.rs` already owned.
    #[test]
    fn only_the_obs_adapter_imports_obws() {
        let offenders = violations("", &["obws"], &[]);
        let offenders: Vec<String> = offenders
            .into_iter()
            .filter(|hit| !hit.starts_with("  obs/"))
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
        let offenders = violations("domain", &["gtk4", "adw::", "glib::", "gio::", "obws"], &[]);

        assert!(
            offenders.is_empty(),
            "src/domain/ must not depend on GTK or on `obws`; convert at the \
             adapter boundary instead:\n{}",
            offenders.join("\n")
        );
    }

    /// Services stay free of GTK.
    ///
    /// `src/services/` is pure logic over domain snapshots — dB conversion,
    /// graph edge classification, the Doctor's checks. Keeping GTK out is what
    /// lets those rules be tested by calling them, with no display, no main
    /// loop, and no widget tree to assemble first.
    #[test]
    fn services_hold_no_gtk() {
        let offenders = violations("services", &["gtk4", "adw::", "glib::", "gio::"], &[]);

        assert!(
            offenders.is_empty(),
            "src/services/ must not depend on GTK. A service decides what is \
             true; a widget decides how to show it:\n{}",
            offenders.join("\n")
        );
    }

    /// Neither the domain nor the services know how anything is stored.
    ///
    /// `src/storage/` owns the serde representations, the file layout, and the
    /// keyring. Its types convert to domain snapshots at the boundary. If the
    /// dependency ran the other way, changing the shape of `config.json` would
    /// mean editing the rules of the application.
    #[test]
    fn domain_and_services_do_not_reach_into_storage() {
        let mut offenders = violations("domain", &["crate::storage"], &[]);
        offenders.extend(violations("services", &["crate::storage"], &[]));

        assert!(
            offenders.is_empty(),
            "src/domain/ and src/services/ must not depend on src/storage/. \
             Convert a storage type into a domain snapshot and pass that \
             instead:\n{}",
            offenders.join("\n")
        );
    }

    /// Pages and widgets reach the controller only through `NavigationContext`.
    ///
    /// `NavigationContext` exposes exactly two things: switch to a page, and
    /// dispatch a command. That narrow surface is what keeps the pages from
    /// growing their own opinions about connection lifecycles or OBS calls.
    ///
    /// `window.rs` and `navigation.rs` are exempt because they are where the
    /// application is assembled: one builds the controller, the other wraps it.
    #[test]
    fn pages_reach_the_controller_only_through_navigation() {
        let offenders = violations(
            "ui",
            &["AppController"],
            &["ui/window.rs", "ui/navigation.rs"],
        );

        assert!(
            offenders.is_empty(),
            "UI pages and widgets must go through `NavigationContext` \
             (`switch_to_page` / `dispatch`) rather than naming \
             `AppController` directly:\n{}",
            offenders.join("\n")
        );
    }
}
