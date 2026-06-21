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

Status: all passed on 2026-06-21 after the mixer refresh contract, optimistic
audio update, and output confirmation decision-helper work.

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

- Static validation passed.
- Output confirmations and documentation were largely complete.
- Audio slider throttling reduced OBS command volume.
- Mixer loading states improved the empty-state flash, but refresh ordering
  still needed hardening against stale selected/pinned scene responses and
  duplicate initial refreshes.

### Mixer Refresh Contract, Optimistic Audio, Output Confirmation Helper

Working tree, reviewed 2026-06-21:

- Added `MixerAudioRefreshState` and `MixerAudioRefreshTransition` to separate
  requested scene, loaded snapshot, and visible error state.
- Routed mixer loading/success/failure events through reducer methods in
  `AppState`; stale success/failure responses no longer overwrite current
  selected/pinned mixer state.
- Moved mixer loading ownership out of UI pre-marking and into controller
  `MixerAudioInputsLoading` events.
- Added a UI-side duplicate request tracker to avoid repeated selected/pinned
  refresh dispatches during rebuilds and combo callbacks.
- Unified audio volume application so slider, +/- dB, and reset all update the
  local scale/readout immediately and cancel pending debounced sends on
  immediate commands.
- Centralized OBS volume multiplier sanitization and expanded debouncer tests.
- Extracted and tested a pure output confirmation decision helper for stream
  and recording start/stop actions.

Review verdict:

- Static validation passed:
  `cargo fmt --all -- --check`, `cargo check --workspace --all-features`,
  `cargo test --workspace --all-features`, and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- The mixer reducer covers the key stale-response cases and is a good fit for
  controller-owned async state.
- The mixer duplicate guard prevents dispatch storms, but it also suppresses
  explicit retries after a scene-specific refresh failure because the helper
  returns early while `mixer_audio_error` matches the scene.
- The mixer page module header is stale: it still says scene-specific OBS
  refresh is left for a future phase.
- Output confirmation logic is now testable, but all confirmation dialogs still
  use destructive response styling, including start stream/start recording
  actions.
- The audio optimistic update path is functionally complete, but helper
  coverage remains mostly service-level; the GTK-level dispatch/update helper is
  not directly tested.

## Groomed Next Steps

### P0: Restore Mixer Failure Retry Semantics

Problem:

- `request_mixer_scene_audio` suppresses dispatch when the current scene has a
  matching `mixer_audio_error`.
- The same helper is used by explicit mode/scene callbacks, so a user cannot
  retry a failed selected/pinned refresh by reselecting the target or toggling
  back to the mode.
- There is no manual retry button on the mixer error state.

Plan:

- Separate automatic rebuild dedupe from explicit user retry intent.
- Add a retry action to the mixer error status, or allow combo/mode callbacks
  to force a refresh that clears the matching error through the controller
  loading event.
- Keep automatic populate/rebuild from looping endlessly on persistent OBS
  errors.
- Add a narrow testable request-decision helper for loaded/loading/error/tracked
  combinations and explicit-versus-automatic requests.
- Update the stale module comment in `src/ui/pages/mixer.rs`.

Files:

- `src/ui/pages/mixer.rs`
- possible `src/controller/state.rs` if retry intent should be modeled in the
  reducer

Tests:

- Cover automatic no-loop after failure.
- Cover explicit retry dispatch after failure.
- Cover loaded and in-flight requests still dedupe correctly.

### P0: Make Mixer Refresh Target Semantics Explicit

Problem:

- The reducer tracks a single requested scene and treats any success for the
  requested scene as current. That is acceptable for target-level freshness, but
  it cannot distinguish two sequential refreshes for the same scene.
- UI state still carries both legacy fields (`mixer_audio_scene`,
  `mixer_audio_inputs`, `mixer_audio_loading_scene`, `mixer_audio_error`) and
  the new reducer, which risks future direct writes bypassing the contract.

Plan:

- Decide whether scene-level freshness is sufficient or whether requests need a
  monotonically increasing token.
- If scene-level freshness is sufficient, document the invariant and make
  direct legacy-field writes harder by narrowing visibility or adding comments
  at the field declarations.
- Audit all event paths for direct mixer audio field mutation.
- Consider clearing stale loaded snapshots on disconnect if selected/pinned
  mixer controls should not show old OBS data while disconnected.

Files:

- `src/controller/state.rs`
- `src/ui/window.rs`
- `src/ui/pages/mixer.rs`

Tests:

- Add reducer coverage for repeated same-scene loading/success if the desired
  behavior is documented.
- Add a disconnect-state test if clearing snapshots becomes required.

### P1: Surface Output Command Errors In Output UI

Problem:

- Output command failures still surface through generic OBS errors/toasts and
  can make stream/record command failures look like connection failures.

Plan:

- Extend output UI state with last stream/record error.
- Preserve OBS connection errors separately from stream/record command errors.
- Show output errors on the Live output area and later on output cards.
- Clear output-specific errors on new pending command, success, and disconnect.

Files:

- `src/controller/event.rs`
- `src/controller/state.rs`
- `src/controller/app_controller.rs`
- `src/ui/pages/live.rs`
- `src/ui/window.rs`

### P1: Refine Output Confirmation Dialog Semantics

Problem:

- `requires_output_confirmation` is now tested, but the dialog presentation is
  still generic.
- Start stream/start recording confirmations use destructive styling even
  though they are not destructive in the same sense as stop actions.

Plan:

- Pass response appearance based on output kind/action, or add a small helper
  that maps action to dialog copy and appearance.
- Keep stop stream/stop recording as destructive.
- Use neutral or suggested styling for start stream/start recording.
- Add pure tests for action-to-dialog metadata if extracted.

Files:

- `src/ui/pages/live.rs`

### P1: Settings Persistence Feedback

Problem:

- Output safety toggles update in-memory state before disk persistence is known.
- Write failures are logged but not shown to the user, matching some existing
  Settings patterns but weak for safety preferences.

Plan:

- Reuse or add a Settings status row for output preference persistence failures.
- Decide whether failed writes should roll back the switch or keep the
  in-memory session value with an explicit warning.

Files:

- `src/ui/pages/settings.rs`

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
