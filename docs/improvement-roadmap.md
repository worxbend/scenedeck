# SceneDeck Improvement Roadmap

This document began as the implementation plan for the theme, output-control,
audio, and Mixer work delivered in SceneDeck 0.2.0 through 0.4.0. That original
plan is complete. It is retained here as a concise historical record rather
than as a description of missing features.

For the current architecture and behavior, use:

- [Architecture](architecture.md)
- [Codebase overview](codebase-overview.md)
- [Configuration](configuration.md)
- [User guide](user-guide.md)
- [Custom themes](custom-themes.md)
- [Theme CSS reference](theme-css-reference.md)
- [Manual test plan](manual-test-plan.md)

## Completed Milestones

### Output Controls

SceneDeck has persistent stream and record controls, output status updates,
duplicate-operation guards, elapsed-time display, recording-path handling, and
configurable start/stop confirmations.

### Audio and Mixer

The Live page exposes active-scene audio controls and real-time meters. The
dedicated Mixer page supports Active, Selected, and Pinned scene modes, scene
selection, search, grouping, scene-specific refreshes, and persisted Mixer
preferences. OBS mute and volume events update matching audio cards without
rebuilding the entire panel.

### Appearance and Themes

The version 3 configuration schema contains the `appearance` section and
migrates older top-level `theme_mode` values. Settings exposes system, light,
and dark color modes; built-in light/dark-aware theme families; motion levels;
and separate custom CSS files for light and dark variants. Theme resources are
embedded in the application, and CSS load errors are reported in Settings.

### Navigation and Onboarding

The application shell now includes Live, Stats, Mixer, Graph, Inventory,
Doctor, Settings, and Help pages. The Help page and first-run welcome provide
in-app onboarding, while the persistent sidebar and status bar keep primary
operational state visible across pages.

## Current Follow-up Work

The completed milestones above should be treated as existing behavior, not as
future phases. Remaining improvement work is narrower:

- Expand the automated OBS WebSocket fixture beyond its handshake, version
  request, and event-delivery coverage into full refresh and mutation scenarios,
  and add rendered GTK interaction coverage. The existing manual Mixer records
  document environment gaps rather than a current successful full interaction
  run.
- Improve Mixer loading and empty states during selected or pinned scene
  refreshes.
- Continue decomposing large UI orchestration modules as behavior changes make
  focused boundaries practical.
- Keep translations synchronized with the English Fluent catalog.
- Profile scene and audio discovery against large OBS collections before
  changing request concurrency.

## Validation Baseline

Run these checks before and after substantial changes:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-features
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
