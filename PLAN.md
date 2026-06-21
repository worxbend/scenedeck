# SceneDeck Implementation Plan

This is the active implementation ledger for SceneDeck improvement work. It
tracks completed phases, current review findings, and groomed next steps.

## Current State

SceneDeck is a Rust GTK4/libadwaita OBS controller with these major surfaces:

- Live page: OBS connection, primary scene switching, compact active-scene
  audio, stream controls, record controls, elapsed output timers, recording
  path copy, and configurable output confirmations.
- Mixer page: Active/Selected/Pinned modes, scene selector, search, grouping,
  scoped audio cards, scene-specific refreshes, and loading/error/empty states.
- Graph page: nested scene dependency display.
- Inventory page: local scene role assignment, stale registry cleanup, YAML
  import/export.
- Doctor page: structural diagnostics over scenes, registry roles, and graph.
- Settings page: appearance/theme controls, custom CSS paths, OBS connection,
  output confirmation preferences, and status.

Architecture boundaries remain intact:

- `src/obs/`: OBS WebSocket access and `obws` mapping only.
- `src/controller/`: async orchestration, command handling, events, OBS session
  lifecycle.
- `src/domain/`: pure app concepts.
- `src/services/`: pure or mostly pure logic.
- `src/storage/`: config, registry, XDG paths, and keyring persistence.
- `src/ui/`: GTK widgets, CSS loading, pages, navigation, and actions.

Latest validation run:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-features
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Status: all passed on 2026-06-21 after the output confirmation, audio
throttling, manual test plan, and mixer loading-state work.

## Completed Phases

### Baseline Audit

Commit: `bdb049e docs: add improvement roadmap audit`

- Added `docs/improvement-roadmap.md`.
- Documented current UI pages, state model, stream/record behavior, audio
  behavior, theme/config behavior, known gaps, and likely files to modify.

### Config v2 Appearance Migration

Commit: `6c4774d feat: add appearance config v2 migration`

- Added v2 `appearance` config shape.
- Preserved v1 top-level `theme_mode` compatibility.
- Added appearance domain types for theme id, custom CSS, and UI density.
- Added migration/default tests.

### Light/Dark Theme Infrastructure

Commit: `2e94d22 feat: add light dark theme infrastructure`

- Added light/dark-aware theme families.
- Added built-in theme registry and CSS provider manager.
- Added 10 built-in theme families with light and dark CSS files.
- Added Settings theme selector and custom light/dark CSS controls.
- Added custom theme docs, CSS reference, and example CSS files.

Important design choice:

- Theme selection is a theme family. `System`, `Light`, and `Dark` decide which
  side of the family is applied. Custom CSS also has separate light and dark
  paths.

### Stream/Record Operation Guards

Commit: `53c50d5 feat: guard output start stop operations`

- Added explicit stream/record commands.
- Added controller-side duplicate-operation guards.
- Live buttons show transition labels and disable during pending operations.

### Live Audio Control Foundation

Commit: `51240ac feat: improve live audio controls`

- Added audio source scope metadata: global, active scene, nested scene, group.
- Added parent scene path metadata.
- Added dB formatting/conversion helpers.
- Added scope badges, source path tooltips, +/- dB controls, reset-to-0 dB, and
  stronger lock styling.

### Mixer Page Foundation

Commit: `a849725 feat: add mixer page foundation`

- Added `Mixer` navigation page.
- Added `MixerMode`, `MixerGrouping`, and `MixerSelection`.
- Added Active/Selected/Pinned controls, scene selector, search, grouping, and
  grouped audio card display.

### Scene-Specific Mixer Refresh

Commit: `c5b211d feat: refresh mixer audio by scene`

- Added `RefreshMixerSceneAudio`.
- Added `MixerAudioInputsUpdated`.
- Added separate Mixer audio snapshot state.
- Selected and Pinned modes refresh audio for their target scene through the
  controller and OBS adapter.

### Mixer Preference Persistence

Commit: `03b6459 feat: persist mixer preferences`

- Added serializable Mixer preferences to config.
- Persisted Mixer mode, selected scene, pinned scene, and grouping.
- Kept search session-only.

### Output Elapsed Time and Recording Path Copy

Commit: `4d0db25 feat: show output elapsed time`

- Added UI-side active timers for stream and recording.
- Live output labels show elapsed duration while active.
- Last recording path is retained in UI state and can be copied.

### Output Confirmations, Audio Throttling, Manual Tests, Mixer Loading States

Working tree, reviewed 2026-06-21:

- Added `OutputConfig` defaults and config docs.
- Added Settings output safety toggles and Live confirmation dialogs.
- Added UI-side volume slider debouncing with pure helper tests.
- Added `docs/manual-test-plan.md`.
- Added mixer scene audio loading and failure events plus status pages.

Review verdict:

- Static validation passes.
- Output confirmations and documentation are largely complete.
- Audio slider throttling reduces OBS command volume, but direct +/- and reset
  controls should update the local scale/readout optimistically.
- Mixer loading states improve the empty-state flash, but refresh ordering still
  needs hardening against stale selected/pinned scene responses and duplicate
  initial refreshes.

## Groomed Next Steps

### P0: Harden Mixer Scene Refresh State

Problem:

- Selected/Pinned refresh can be requested both from combo callbacks and from
  page population.
- Loading state is currently set from UI population before the controller emits
  its loading event.
- A late successful response for an older scene can overwrite the single
  `mixer_audio_scene` snapshot even after the user has selected a different
  target.

Plan:

- Make the controller event stream the single source of truth for mixer loading
  and failure state.
- Track the currently requested mixer target separately from the last loaded
  snapshot.
- Ignore or preserve stale responses that do not match the current
  Selected/Pinned target.
- Avoid duplicate refresh dispatches during mode/scene selection and rebuild.

Files:

- `src/controller/event.rs`
- `src/controller/state.rs`
- `src/controller/app_controller.rs`
- `src/ui/pages/mixer.rs`
- `src/ui/window.rs`

Tests:

- Add pure state-transition tests if possible, or extract a small mixer refresh
  state reducer that can cover loading, success, failure, stale success, and
  stale failure.

### P0: Improve Audio Card Optimistic Volume Semantics

Problem:

- Slider changes update the dB label immediately and debounce OBS commands.
- +/- dB and reset commands dispatch immediately but do not move the local scale
  or label until an OBS volume event arrives.
- If OBS rejects the command or event delivery lags, the UI appears inert.

Plan:

- Add one helper that updates local scale/readout and dispatches either
  debounced or immediate volume commands.
- Use it for slider, +/- dB, and reset paths.
- Preserve OBS event reconciliation and cancellation of pending debounced
  sends.
- Consider clamping outgoing slider values to OBS-supported ranges in one place.

Files:

- `src/ui/widgets/audio_card.rs`
- `src/services/audio_service.rs`

Tests:

- Extend debouncer tests for immediate-send cancellation and sanitized values.

### P0: Add Regression Coverage for Output Confirmation Defaults

Problem:

- Config defaults are tested, but the Live confirmation behavior itself is UI
  callback logic and not currently covered by a narrow testable decision
  function.

Plan:

- Extract a pure helper that maps output kind, active state, and `OutputConfig`
  to whether confirmation is required.
- Test all four output action/default combinations.
- Keep the GTK dialog construction in `src/ui/pages/live.rs`.

Files:

- `src/ui/pages/live.rs` or a small UI-adjacent helper module.
- `src/storage/config.rs`.

### P1: Surface Output Command Errors In Output UI

Problem:

- Output command failures still surface through generic OBS errors/toasts and
  can make output-specific failures look like connection failures.

Plan:

- Extend output UI state with last stream/record error.
- Preserve OBS connection errors separately from stream/record command errors.
- Show output errors on the Live output area and later on output cards.

Files:

- `src/controller/event.rs`
- `src/controller/state.rs`
- `src/controller/app_controller.rs`
- `src/ui/pages/live.rs`
- `src/ui/window.rs`

### P1: Output Control Cards

Problem:

- Stream/record controls are still compact banner controls.

Plan:

- Extract reusable output card widgets.
- Show state, elapsed time, pending state, last error, and recording path action.
- Keep duplicate-operation guards in the controller and clear pending state on
  success, failure, and disconnect.

Files:

- `src/ui/pages/live.rs`
- possible new `src/ui/widgets/output_card.rs`
- `assets/scenedeck.css`

### P1: Settings Persistence Feedback

Problem:

- Output safety toggles update in-memory state before disk persistence is known.
- Write failures are logged but not shown to the user, matching some existing
  Settings patterns but weak for safety preferences.

Plan:

- Reuse or add a Settings status row for output preference persistence failures.
- Decide whether failed writes should roll back the switch or keep the in-memory
  session value with an explicit warning.

Files:

- `src/ui/pages/settings.rs`

### P1: Manual Test Plan Execution Record

Problem:

- `docs/manual-test-plan.md` exists, but no run log captures manual execution
  results against a real OBS instance.

Plan:

- Add a dated manual test result section or separate release checklist entry.
- Record OBS version, SceneDeck build, skipped streaming cases, and failures.

Files:

- `docs/manual-test-plan.md`
- possible `docs/manual-test-runs.md`

### P1: UI Density

Problem:

- `UiDensity` is modeled but not applied broadly.

Plan:

- Add density class on the root window.
- Add compact/comfortable CSS rules for cards, audio controls, and spacing.
- Expose density changes in Settings if not already visible enough.

### P1: Live Page Operational Layout

Plan:

- Add status strip for OBS/profile/collection/current scene/output state.
- Improve scene card status, focus, and warning markers.
- Move compact audio panel into a clearer Live zone now that Mixer exists.

### P1: Controller Cleanup for Local Workflows

Current mismatch:

- `SetSceneRole` and `RunDoctor` commands still exist but are not implemented.
- Inventory and Doctor call storage/services directly from UI pages.

Decision needed:

- Either route those workflows through the controller, or remove unused command
  variants and document them as local UI workflows.

### P2: OBS Feature Expansion

Candidate features:

- Source visibility controls.
- Transition selection and duration.
- Studio Mode support.
- Scene notes.
- Scene favorites.
- Command palette.
- Audio presets.
- Backup/restore full SceneDeck profile.

## Validation Rule

Before each commit:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-features
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Keep commits phase-scoped and preserve module boundaries.
