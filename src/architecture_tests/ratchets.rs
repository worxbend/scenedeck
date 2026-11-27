//! Ratchets: counts that may fall but never rise.
//!
//! Each ceiling is the count measured when the ratchet landed. A change
//! that lowers a count lowers its ceiling in the same commit; a ceiling
//! never goes up. A file missing from a table may not gain a match.

use super::scanner::{line_budget, ratchet};

/// A new or unlisted file may hold at most this many production lines.
const PRODUCTION_LINE_CAP: usize = 800;

/// Files already over the cap. Each split lowers its entry or removes it.
const OVERSIZED_FILES: &[(&str, usize)] = &[
    ("controller/state.rs", 1224),
    ("obs/client.rs", 885),
    ("ui/pages/inventory.rs", 1505),
    ("ui/pages/live.rs", 952),
    ("ui/pages/mixer.rs", 1216),
    ("ui/pages/settings.rs", 822),
    ("ui/pages/stats.rs", 912),
    ("ui/widgets/audio_card.rs", 884),
    ("ui/widgets/operating_panel.rs", 1613),
    ("ui/window.rs", 1548),
];

/// Production files stay small enough to read in one sitting.
///
/// Only shipped lines count: a test-only item does not, wherever it sits,
/// so moving tests out never looks like progress and adding them never
/// looks like growth.
#[test]
fn production_files_stay_within_budget() {
    let offenders = line_budget("", OVERSIZED_FILES, PRODUCTION_LINE_CAP);

    assert!(
        offenders.is_empty(),
        "Split the file instead of raising its ceiling. Unlisted files are \
         capped at {PRODUCTION_LINE_CAP} production lines:\n{}",
        offenders.join("\n")
    );
}

/// The `AppState` borrow patterns counted under `src/ui/`, in the column
/// order of [`APP_STATE_BORROWS`].
const APP_STATE_BORROW_PATTERNS: [&str; 3] =
    [".state.borrow_mut()", ".state.borrow()", "nav.state"];

const APP_STATE_BORROWS: &[(&str, [usize; 3])] = &[
    ("ui/action_catalog.rs", [0, 1, 1]),
    ("ui/action_shortcuts.rs", [7, 17, 27]),
    ("ui/audio_presets.rs", [1, 11, 12]),
    ("ui/bookmarks.rs", [12, 10, 24]),
    ("ui/command_palette.rs", [3, 6, 10]),
    ("ui/global_shortcuts.rs", [0, 4, 4]),
    ("ui/health_alerts.rs", [7, 5, 13]),
    ("ui/health_alerts/audio_editor.rs", [0, 5, 5]),
    ("ui/hotkeys.rs", [0, 4, 4]),
    ("ui/navigation.rs", [1, 6, 0]),
    ("ui/operator_picker.rs", [13, 18, 31]),
    ("ui/pages/doctor.rs", [5, 6, 15]),
    ("ui/pages/graph.rs", [5, 3, 12]),
    ("ui/pages/inventory.rs", [16, 19, 45]),
    ("ui/pages/live.rs", [14, 16, 37]),
    ("ui/pages/mixer.rs", [13, 8, 22]),
    ("ui/pages/settings.rs", [3, 9, 15]),
    ("ui/pages/settings/connections.rs", [5, 16, 22]),
    ("ui/pages/stats.rs", [3, 10, 14]),
    ("ui/persist.rs", [3, 3, 7]),
    ("ui/persist/config.rs", [2, 0, 2]),
    ("ui/persist/registry_scope.rs", [2, 2, 5]),
    ("ui/preflight.rs", [0, 8, 8]),
    ("ui/preflight/editor.rs", [0, 7, 7]),
    ("ui/previews.rs", [0, 5, 5]),
    ("ui/quick_actions.rs", [13, 25, 38]),
    ("ui/report_history.rs", [1, 7, 11]),
    ("ui/session_reports.rs", [2, 1, 4]),
    ("ui/sidebar.rs", [0, 1, 1]),
    ("ui/widgets/audio_card.rs", [5, 4, 9]),
    ("ui/widgets/deck_controls.rs", [1, 11, 12]),
    ("ui/widgets/operating_panel.rs", [12, 13, 27]),
    ("ui/widgets/operating_panel/media_progress.rs", [5, 2, 8]),
    ("ui/widgets/scene_card.rs", [0, 3, 3]),
    ("ui/widgets/studio_controls.rs", [0, 5, 5]),
    ("ui/widgets/transport_bar.rs", [0, 2, 2]),
    ("ui/widgets/volume_meter.rs", [0, 1, 1]),
    ("ui/window.rs", [43, 59, 110]),
    ("ui/workspace.rs", [6, 0, 7]),
];

/// Pages and widgets stop reaching into `AppState` directly.
///
/// Every raw borrow is a chance to hold the `RefCell` across a dispatch or
/// a refresher and panic at runtime. `NavigationContext::read` and
/// `update` own that discipline; these counts only fall as callers move
/// to them.
#[test]
fn app_state_borrows_ratchet() {
    let offenders: Vec<_> = APP_STATE_BORROW_PATTERNS
        .iter()
        .enumerate()
        .flat_map(|(column, pattern)| {
            let ceilings: Vec<_> = APP_STATE_BORROWS
                .iter()
                .map(|(file, counts)| (*file, counts[column]))
                .collect();
            ratchet("ui", pattern, &ceilings)
        })
        .collect();

    assert!(
        offenders.is_empty(),
        "Use `NavigationContext::read` or `update` instead of borrowing \
         `AppState` directly:\n{}",
        offenders.join("\n")
    );
}

const UI_THREAD_LOCALS: &[(&str, usize)] = &[
    ("ui/background_io.rs", 2),
    ("ui/bookmarks.rs", 1),
    ("ui/persist.rs", 2),
    ("ui/report_history.rs", 1),
    ("ui/session_reports.rs", 1),
    ("ui/theme.rs", 1),
    ("ui/widgets/audio_card.rs", 1),
    ("ui/widgets/scene_card.rs", 1),
];

/// UI state lives in owned objects, not in hidden thread-local globals.
///
/// A `thread_local!` is state no constructor shows and no test can reset
/// without knowing it exists.
#[test]
fn thread_local_ratchet() {
    let offenders = ratchet("ui", "thread_local!", UI_THREAD_LOCALS);

    assert!(
        offenders.is_empty(),
        "Pass the state in through its owner instead of adding a \
         `thread_local!`:\n{}",
        offenders.join("\n")
    );
}

/// `fl!(` calls per domain file.
const DOMAIN_FL_CALLS: &[(&str, usize)] = &[
    ("domain/appearance.rs", 1),
    ("domain/audio.rs", 4),
    ("domain/audio_preset/run.rs", 10),
    ("domain/control.rs", 16),
    ("domain/diagnostic.rs", 6),
    ("domain/graph.rs", 6),
    ("domain/hotkey.rs", 23),
    ("domain/icon.rs", 32),
    ("domain/meter.rs", 12),
    ("domain/mixer.rs", 9),
    ("domain/output.rs", 9),
    ("domain/role.rs", 13),
];

/// `LANGUAGE_LOADER` mentions per domain file.
const DOMAIN_LANGUAGE_LOADERS: &[(&str, usize)] = &[
    ("domain/appearance.rs", 2),
    ("domain/audio.rs", 5),
    ("domain/audio_preset/run.rs", 12),
    ("domain/control.rs", 17),
    ("domain/diagnostic.rs", 7),
    ("domain/graph.rs", 7),
    ("domain/hotkey.rs", 24),
    ("domain/icon.rs", 33),
    ("domain/meter.rs", 13),
    ("domain/mixer.rs", 10),
    ("domain/output.rs", 10),
    ("domain/role.rs", 14),
];

/// Translated text leaves the domain; it does not spread through it.
///
/// A domain type that formats its own label ties the app's rules to the
/// active locale and to the Fluent loader. Presentation code owns labels.
#[test]
fn domain_presentation_ratchet() {
    let mut offenders = ratchet("domain", "fl!(", DOMAIN_FL_CALLS);
    offenders.extend(ratchet(
        "domain",
        "LANGUAGE_LOADER",
        DOMAIN_LANGUAGE_LOADERS,
    ));

    assert!(
        offenders.is_empty(),
        "Format labels outside src/domain/; the domain returns the value \
         and presentation code translates it:\n{}",
        offenders.join("\n")
    );
}
