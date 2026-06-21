# SceneDeck Improvement Roadmap

This document records the baseline audit for the production-quality live
operation console work. It is intentionally implementation-facing: each section
names the current state, known gaps, and files that future phases should touch.

## Baseline Summary

SceneDeck is a Rust 2021 GTK4/libadwaita desktop app for controlling OBS Studio
over OBS WebSocket. The app currently has these pages:

- Live: OBS connection-dependent operation surface with stream/record buttons,
  current program scene, Primary scene cards, and active-scene audio cards.
- Graph: nested scene dependency display.
- Inventory: local scene role registry editing, stale registry cleanup, and
  YAML import/export.
- Doctor: structural diagnostics over inventory, registry, and scene graph.
- Settings: color scheme selection, OBS host/port, OBS password keyring storage,
  and connection status.

The current architecture is cleanly layered:

- `src/obs/` owns `obws` access and maps OBS types into domain types.
- `src/controller/` owns async command routing, OBS session lifecycle, and event
  publication back to GTK.
- `src/domain/` owns app concepts that are independent of GTK and OBS protocol
  structs.
- `src/services/` owns pure or mostly pure derived behavior.
- `src/storage/` owns config, registry, XDG paths, and keyring persistence.
- `src/ui/` owns GTK widgets, CSS loading, page layout, navigation, and actions.

## Current State Model

`src/controller/state.rs` defines `AppState`, which lives on the GTK thread in
`Rc<RefCell<AppState>>`. It stores:

- current page
- theme mode
- OBS connection status
- scene inventory and scene graph
- OBS profiles and scene collections
- stream and record status
- active audio input list
- diagnostics
- optional startup notice from config loading

The current `Page` enum includes `Live`, `Graph`, `Inventory`, `Doctor`, and
`Settings`. A dedicated Mixer page will require adding a `Mixer` variant,
sidebar metadata, page construction, and refresh behavior.

## Current Stream and Record Behavior

The Live page has a horizontal output banner with stream and record controls:

- Stream button dispatches explicit start/stop commands.
- Record button dispatches explicit start/stop commands.

`src/controller/app_controller.rs` routes these commands to:

- `ObsClient::set_streaming(active)`
- `ObsClient::set_recording(active)`
- `refresh_output_statuses(...)`

OBS stream/record events update `OutputStatus` through `AppEvent` values. The
recording stop path can preserve the returned recording path in
`OutputStatus.detail`, and the Live page exposes that path as a tooltip and copy
button.

Current improvements:

- Pending output operations are guarded in the controller to avoid duplicate
  start/stop requests.
- Elapsed time is tracked locally and displayed while stream/record outputs are
  active.
- The last OBS recording path is retained in UI state and can be copied from
  the Live page.
- Output confirmation preferences are implemented and exposed in Settings:
  start stream and start recording confirmations default off, while stop stream
  and stop recording confirmations default on.

Known gaps:

- Last error is surfaced through generic app errors/toasts rather than an output
  control card state.

Output confirmation phase status: completed in the working tree. Full
validation should pass before committing as
`feat: add output confirmation preferences`.

Primary files for stream/record improvements:

- `src/domain/output.rs`
- `src/controller/command.rs`
- `src/controller/event.rs`
- `src/controller/state.rs`
- `src/controller/app_controller.rs`
- `src/obs/client.rs`
- `src/ui/pages/live.rs`
- `src/ui/window.rs`
- `assets/scenedeck.css`
- `docs/user-guide.md`
- `docs/obs-websocket.md`

## Current Audio Behavior

The controller refreshes audio after scene inventory refreshes and current
program scene changes. `ObsClient::get_scene_audio_inputs(...)` returns the
current active scene audio list with global audio sources first, followed by
audio-capable active-scene inputs including nested scenes and groups.

`src/domain/audio.rs` currently stores:

- input id
- display name
- source scope: global, active scene, nested scene, or group-derived
- parent scene path when known
- muted
- OBS linear volume multiplier
- OBS volume in dB
- local lock flag placeholder

`src/ui/widgets/audio_card.rs` renders compact Live page cards with:

- input name
- source scope badge and source path tooltip
- mute/unmute toggle
- local lock button that disables the slider
- inverted vertical volume scale
- dB readout
- +/- fine adjustment buttons
- reset-to-0 dB button

OBS mute and volume events update only matching `AudioCardHandle`s, so the app
already avoids rebuilding the whole audio panel for every input event.

Known gaps:

- Slider changes dispatch on every value change without throttling.
- The local lock state is widget-local and not represented in app/domain state.
- There is no dedicated Mixer page, scene selector, search/filter, or grouping.

Primary files for audio and Mixer improvements:

- `src/domain/audio.rs`
- `src/domain/mod.rs`
- `src/services/audio_service.rs`
- `src/obs/client.rs`
- `src/controller/command.rs`
- `src/controller/event.rs`
- `src/controller/state.rs`
- `src/controller/app_controller.rs`
- `src/ui/pages/live.rs`
- `src/ui/pages/mod.rs`
- `src/ui/window.rs`
- `src/ui/widgets/audio_card.rs`
- new `src/ui/pages/mixer.rs`
- possible new `src/ui/widgets/audio_strip.rs`
- possible new `src/domain/mixer.rs`
- `assets/scenedeck.css`

## Current Theme and Config Behavior

`src/domain/appearance.rs` currently defines `ThemeMode` with `System`, `Light`,
and `Dark`. Unknown persisted values fall back to `System`.

`src/storage/config.rs` stores schema version `1` and persists `theme_mode` as a
top-level field. Existing config shape:

```json
{
  "version": 1,
  "obs": {
    "host": "127.0.0.1",
    "port": 4455
  },
  "live": {
    "show_roles": ["primary"],
    "audio_inputs": [],
    "allow_switching_only": ["primary"]
  },
  "theme_mode": "system"
}
```

`src/ui/pages/settings.rs` exposes only the color scheme selector. CSS loading is
currently startup-only through `src/ui/mod.rs::load_css()`, which embeds
`assets/scenedeck.css` with `include_str!`.

Known gaps:

- There is no v2 `appearance` config section.
- There is no built-in theme registry or theme preview metadata.
- There is no user CSS file loading or reload path.
- CSS parse/load errors are not surfaced in Settings.
- UI density is not modeled.
- Theme resource files are not embedded in `resources/scenedeck.gresource.xml`.

Primary files for theme improvements:

- `src/domain/appearance.rs`
- `src/storage/config.rs`
- `src/controller/state.rs`
- `src/ui/mod.rs`
- `src/ui/pages/settings.rs`
- `src/ui/window.rs`
- `resources/scenedeck.gresource.xml`
- `build.rs`
- `assets/scenedeck.css`
- new `resources/themes/*.css`
- new `docs/custom-themes.md`
- new `docs/theme-css-reference.md`
- new `examples/themes/*.css`
- `docs/configuration.md`
- `docs/architecture.md`
- `docs/developer-guide.md`
- `docs/user-guide.md`
- `README.md`

## Mixer Page Foundation

The app now has a `Mixer` navigation page. The first implementation provides:

- Active, Selected, and Pinned mode controls.
- Scene selector.
- Search field.
- Grouping by scope, scene path, or no grouping.
- Reuse of scoped audio cards with mute, lock, fader, dB readout, fine
  adjustment, and reset controls.
- Scene-specific audio refresh for Selected and Pinned modes through controller
  command/event routing.
- Persistence for Mixer mode, selected scene, pinned scene, and grouping.

Next Mixer phases should add stronger empty states while scene audio is loading
and broaden tests around mixer mode transitions.

## UI and Navigation Gaps

The current shell uses `adw::NavigationSplitView`, a sidebar, a content stack,
and header dropdowns for OBS profiles and scene collections. This is a good
base for adding a Mixer page.

Known gaps:

- Mixer loading states can be clearer while selected/pinned scene audio is
  being fetched.
- Live page output controls are compact utility controls, not operational cards.
- Live page lacks a top status strip with consolidated OBS/profile/collection/
  output state.
- Empty states are present but minimal.
- Critical controls need stronger focus, accessible labels, and non-color-only
  status communication.
- CSS class names should be expanded and documented before custom themes are
  considered stable.

Primary files for navigation and visual polish:

- `src/controller/state.rs`
- `src/ui/window.rs`
- `src/ui/pages/live.rs`
- `src/ui/widgets/scene_card.rs`
- `src/ui/widgets/audio_card.rs`
- `assets/scenedeck.css`
- `docs/theme-css-reference.md`

## Architectural Cleanup Notes

The command enum already includes `SetSceneRole` and `RunDoctor`, but
`AppController` currently logs them as not implemented. Inventory and Doctor
perform local storage/service work directly from UI pages.

This is not a build problem, but future work should choose one direction:

- route role and diagnostic workflows through the controller for a stricter
  command/event architecture; or
- remove unused command variants and document these pages as local GTK workflows
  that do not touch OBS directly.

Files involved:

- `src/controller/command.rs`
- `src/controller/app_controller.rs`
- `src/ui/pages/inventory.rs`
- `src/ui/pages/doctor.rs`
- `docs/architecture.md`
- `docs/developer-guide.md`

## Implementation Order

1. Keep the baseline green before large UI work.
2. Add config v2 appearance model and migration tests.
3. Add a theme registry/manager and built-in CSS files.
4. Extend Settings for theme selection, custom CSS, density, reset, and errors.
5. Document custom themes and stable CSS hooks.
6. Refactor output state into explicit operations and richer cards.
7. Refactor audio widgets and add pure formatting/throttling/grouping tests.
8. Add the Mixer page and navigation.
9. Polish Live page, empty states, icons, focus states, and accessibility.
10. Finish docs, manual QA checklist, and full validation.

## Baseline Validation

Run before and after each major implementation phase:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-features
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
