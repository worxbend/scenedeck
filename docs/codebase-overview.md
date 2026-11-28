# Codebase Overview

## Repository Layout

```text
assets/                 GTK CSS
data/                   desktop, metainfo, and GSettings metadata
docs/                   project documentation
packaging/flatpak/      Flatpak manifest and packaging notes
resources/              embedded GTK resources and icons
src/                    application source
```

## Source Map

`src/main.rs`

Entry point. Initializes logging and starts the app.

`src/app.rs`

Creates the Tokio runtime, GTK application, event channel, controller, initial
state, and main window. `app/tests.rs` (tests only) holds its tests, including
the native close and quit save-barrier tests.

`src/app_info.rs`

Shared application ID, name, and version constants.

`src/architecture_tests/`

Test-only fitness functions that run with `cargo test`.

- `scanner.rs`: `syn`-based scanner over production sources (test-only items
  excluded): forbidden paths, text ratchets, production line counts.
- `scanner_tests.rs`: self-tests for the scanner.
- `rules.rs`: module dependency rules (which `crate::` paths each layer may
  name).
- `ratchets.rs`: counts that may fall but never rise: production lines per
  file (800 for unlisted files), raw `AppState` borrows and `thread_local!`
  under `src/ui/`, and `fl!(`/`LANGUAGE_LOADER` under `src/domain/`.

`src/controller/`

Application orchestration.

- `command.rs`: user and UI commands.
- `event.rs`: events sent back to GTK.
- `state.rs`: UI-facing app state and its pure transitions (output events,
  command intent, connection and scene-inventory updates).
  `state/tests/` (tests only) holds its tests, one file per state slice
  (`outputs`, `health`, `connection`, `topology`, `workspace`, `telemetry`,
  `audio`). `fixtures.rs` holds the helpers the other `state` child modules'
  tests also use. `event_sequence.rs` replays one OBS session through the
  reducers `ui/window.rs` calls and compares each step to a golden snapshot.
- `state/mixer.rs`: Mixer audio view model — request fencing, visibility and
  inspection snapshots. `state/mixer/tests.rs` (tests only) holds its tests.
- `app_controller.rs`: top-level command routing and small per-command task
  dispatch. It delegates session, refresh, and output lifecycles to focused
  controller components. `app_controller/tests/` (tests only) holds the
  tests that drive `AppController::handle`, one file per command family
  (`outputs`, `config`, `refresh`, `scenes`, `session`).
- `output_controller.rs`: stream/record command guards, status refreshes,
  graceful disconnect shutdown, and the injectable output-client boundary.
- `session_controller.rs`: connection task ownership, reconnect replacement,
  graceful disconnect, and the injectable session-runner boundary.
- `workspace.rs`: session-scoped workspace reads and explicit native
  operations. Its loopback suites sit beside it (tests only):
  `workspace/lifecycle_tests.rs`, `workspace/cancellation_tests.rs` (a
  cancelled or collection-invalidated preflight never begins its write) and
  `workspace/scene_activation_tests.rs` (Studio mode changes fence an obsolete
  scene click).
- `runtime/single_flight.rs`: the single-worker queue behind graph refresh,
  Mixer audio reads and workspace reads. The queue runs only while its worker
  lives, and a refused spawn hands the job back so its failure is still
  published.
- `refresh_controller.rs`: OBS list/data refresh helpers, the session-owned
  one-second stats/bitrate poll loop, and OBS event-stream routing.
  `refresh_controller/event_loop_tests.rs` (tests only) pins one
  characterization test per `ObsEvent` arm of `run_event_loop`. Beside it
  (tests only), over a `StatefulObs`: `ingress_tests.rs` (what each event
  refreshes, and failing closed while GTK is blocked), `graph_scan_tests.rs`
  (an invalidated scan never publishes a stale graph) and `snapshot_tests.rs`
  (audio and output snapshots never undo a newer echo).
- `test_support/` (tests only): fixtures shared by controller tests and the
  UI tests that drive the controller. `events.rs` holds `TEST_DEADLINE`,
  the bounded waits `recv_event`, `recv_matching` (async, they yield instead
  of blocking the runtime), `graph_ready` and `wait_until` (synchronous tests
  only), and `peel`, which unwraps `SessionEvent` layers.
  `deps.rs` holds the provider fakes `StaticConfigProvider`, `NoServices`
  (panics on any config or keyring access) and `NoPassword` (a keyring with
  no saved password that panics on a write), and `dependencies()` built from
  them. `state.rs` holds the default `app_state()` and the unity-gain
  `input(id)`. `loopback.rs` runs the real event loop over a `StatefulObs`
  (`pump`) and pushes Program and mute/volume echoes through it.
  `workspace.rs` builds a `WorkspaceRequest` bound to a loopback client.

`src/domain/`

Application data types that do not depend on GTK or OBS crate types.

- `audio.rs`: OBS audio input identity and volume state.
- `appearance.rs`: language, color mode, density, and theme preferences.
- `connection/credential.rs`: `ObsPassword`, the OBS password. Its `Debug`
  prints a placeholder and it has no `Display`; `expose()` is called only at
  the keyring, the OBS handshake and the Settings password field.
  `CredentialStore` is the one port for reading, writing and deleting a
  connection's password; the controller and the Settings editor share it.
- `connection/endpoint.rs`: `is_loopback`, which decides whether an OBS host
  keeps the plain `ws://` traffic on this machine. The Settings connection
  editor warns for any host it cannot prove local.
- `diagnostic.rs`: Doctor diagnostic model.
- `graph.rs`: scene dependency graph.
- `hotkey.rs`: Live scene-shortcut preferences, slot numbering, and the key
  symbols and modifier sets shortcuts are matched against.
- `icon.rs`: the curated catalogue of scene and audio-source icons, mapping
  persisted keys to Nerd Fonts icon names.
- `meter.rs`: audio level readings in decibels, the volume meter's zone
  thresholds, channel naming, and the per-input level snapshot.
- `mixer.rs`: Mixer modes and persisted selection/grouping preferences.
- `obs.rs`: named OBS lists such as profiles and scene collections.
- `output.rs`: stream and record status.
- `role.rs`: local scene roles.
- `scene.rs`: scene inventory.
- `stats.rs`: OBS performance statistics, stream health, and the bounded
  `StatsHistory` ring buffer backing the Stats page charts.

`src/infra/`

Infrastructure helpers.

- `error.rs`: application error type.
- `logging.rs`: tracing subscriber setup.
- `perf.rs`: opt-in `perf-trace` counters (events drained, pump batches,
  dropped meter frames, `collect_report` calls) and their 10 s rate log.
- `sync.rs`: `lock_or_recover`, the poison policy for ordinary `std` mutexes.

`src/obs/`

OBS WebSocket integration.

- `client.rs`: thin async wrapper around `obws::Client`. `client/tests.rs`
  (tests only) covers connection, bounded collection and stats;
  `client/control_tests.rs` (tests only) covers control execution, scene
  snapshots and thumbnails.
- `mapper.rs`: conversion from OBS response types into domain types.
- `audio.rs`, `sources.rs`, `operator_inputs.rs`, `requests.rs`: audio
  reads, source and graph traversal, operator property reads, and request
  admission. Each keeps its mock-wire tests in `<module>/tests.rs`
  (tests only).
- `scene_activation.rs`: scene clicks resolved against the authoritative
  Studio mode, failing closed when it is unknown. `scene_activation/tests.rs`
  (tests only) holds its tests.
- `testing/wire.rs` (tests only): loopback OBS WebSocket handshake, request,
  response and event frames, the `GetVersion` payload, and input, scene and
  scene-item rows.
- `testing/scripted.rs` (tests only): `ScriptedObs`, a loopback OBS that
  answers from a script (strict order or by `requestType`) and counts requests
  per type. `connect_with_events` also returns the client's event stream and
  an `EventFeed` that pushes OBS event frames into it.
- `testing/stateful.rs` (tests only): `StatefulObs`, a two-scene loopback OBS
  whose replies follow the events a test emits. Tests drive it with typed
  `Request`, `Event`, `Scene` and `When` values: hold replies until released,
  refuse requests, and count what arrived in a `RequestLog`. Controller tests
  use it so the wire JSON stays in `src/obs/`.

No other module should import `obws` directly.

`src/services/`

Higher-level logic over domain types.

- `doctor_service.rs`: architecture diagnostics.
- `graph_service.rs`: graph edge classification.
- `hotkey_service.rs`: scene-shortcut resolution, including the leader-key
  state machine.
- `meter_service.rs`: volume meter ballistics — peak fall-off, peak hold, and
  the decibel-to-position mapping the meter is drawn against.
- `audio_service.rs`: dB conversion, volume sanitization, and slider debouncing.

`src/storage/`

Persistence and local machine integration.

- `config.rs`: JSON config load, save, defaults, and migrations.
  `config/tests.rs` (tests only) holds its tests.
- `lock.rs`: the inter-process store lock. Every store write holds it on a
  sibling `<stem>.lock` and fails at once as Busy when another process holds
  it.
- `registry.rs`: local scene role registry. `registry/tests/` (tests only)
  holds its tests: `curation.rs` (in-memory edits), `persistence.rs` (JSON
  and YAML files) and `snapshot.rs` (the domain snapshot and the golden
  fixtures).
- `registry/import.rs`: limits on a YAML registry import: a tighter parser
  budget (aliases, anchors, nodes, merge keys) and per-collection count and
  length caps, checked before anything is written.
- `schema_version.rs`: reads a JSON file's `version` and refuses to write
  over a file from a newer build.
- `secret.rs`: `KeyringCredentialStore`, the Secret Service implementation of
  `CredentialStore`.
- `xdg.rs`: XDG config path resolution.
- `test_support.rs` (tests only): `TempDir`, a scratch directory removed
  with its contents on drop.
- `golden.rs` and `fixtures/` (tests only): checked-in wire fixtures for
  every persisted file, the `json_round_trip` and `yaml_round_trip` helpers
  the per-format tests call, and `enum-spellings.json`, which pins the
  persisted spelling of every enum variant.

`src/ui/`

GTK and libadwaita UI.

- `window.rs`: main shell, page orchestration, and event application.
  `window/tests/` (tests only) holds its tests, one file per scenario
  family (`outputs`, `scene_catalog`, `workspace`, `feedback`,
  `page_redraw`, `assembled_window`, and `render` for mapped-window layout
  checks).
- `sidebar.rs`: sidebar controls and header selectors. `sidebar/tests.rs`
  (tests only) holds its tests.
- `hotkeys.rs`: window-level Live scene shortcut handling.
- `navigation.rs`: page switching and command dispatch helper.
- `markup.rs`: the Pango markup boundary: `escape` and `plain_toast`, the
  only toast constructor, so OBS and network text shows as typed.
- `actions.rs`: app-level actions and keyboard shortcuts.
- `background_io.rs`: runs blocking work off the GTK thread, with FIFO lanes
  for ordered config and registry writes, and returns completion callbacks to
  GTK. `background_io/tests.rs` (tests only) holds its tests.
- `persist.rs`: saves local state from GTK callbacks: the registry write
  journal, the import gate and the close-dialog retry. Children:
  `persist/config.rs` (config snapshot saves) and `persist/registry_scope.rs`
  (show workspace selection and the eviction notice). `persist/tests.rs`
  (tests only) holds its tests.
- `pages/`: Live, Stats, Mixer, Graph, Inventory, Doctor, Settings, and Help pages.
  The Live, Stats, Mixer, Inventory and Settings pages keep their tests in
  `<page>/tests.rs` (tests only); Mixer's layout suite sits beside it in
  `mixer/layout_tests.rs`.
- `presenters/`: text formatters shared by several surfaces;
  `output_status` renders the stream and record labels and elapsed time;
  `save_error` renders the detail of a failed local save, including a save
  refused by another SceneDeck process.
- `widgets/`: reusable scene and audio cards, the volume meter, the icon
  chooser, the bottom status bar, `status_page` (the only status page
  constructor; it escapes the description), and the
  cairo chart/gauge renderers used by Stats. `operating_panel/tests.rs` and
  `audio_card/tests.rs` (tests only) hold those widgets' tests.
- `test_support.rs` (tests only): the `native_test!` runner that gives each
  native GTK test its own process and a 60 s deadline, the shared
  `navigation()` fixture, main-loop `pump` helpers, and the widget lookups
  `find_where`, `find_named` and `expect_named`.

## Assets and Resources

`assets/scenedeck.css` is loaded by the UI at startup. App artwork also lives
under `assets/`, including the installable app icon and wider logo/wordmark.

`resources/scenedeck.gresource.xml` declares embedded GTK resources.
`build.rs` runs `glib-compile-resources` and rebuilds resources when declared
files change.

## Local Data Files

The main config file is:

```text
$XDG_CONFIG_HOME/scenedeck/config.json
```

or:

```text
$HOME/.config/scenedeck/config.json
```

Connected scene curation is stored in
`$XDG_CONFIG_HOME/scenedeck/show-workspaces.json`, keyed by saved connection,
endpoint and scene collection. It keeps the 64 most recently used workspaces
and names any it evicts in a notification. Legacy `registry.json` is retained
and adopted once into the original setup. The scoped registry includes scene metadata, deck
views, action bindings, quick actions, preflight rules, audio-health selections
and audio presets.

Bookmarks live in `$XDG_CONFIG_HOME/scenedeck/bookmarks.json`. Optional bounded
session history lives in `$XDG_STATE_HOME/scenedeck/session-history.json` (normally
`~/.local/state/scenedeck/session-history.json`). These files do not include OBS
passwords. Each connection's password lives in Secret Service. See
[configuration](configuration.md) for schema, migration and persistence rules.

The newer feature modules follow the same boundaries: `domain::{health,
preflight,audio_preset,session_report,action_shortcut,operator_action}` define
rules/evidence; `controller/state/` owns live state and reports;
`controller/workspace/audio_presets.rs` owns scoped sequential writes; storage
modules own files. Palette, shortcut, preflight, health, preset and report UI
modules use NavigationContext. `ui/global_shortcuts/portal.rs` is an asynchronous
GIO desktop-portal adapter and never talks to OBS directly.
