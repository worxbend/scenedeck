# SceneDeck Implementation Plan

This is the active implementation ledger for SceneDeck improvement work. It
tracks completed phases, current review findings, and groomed next steps.

## Current State

SceneDeck is a Rust GTK4/libadwaita OBS controller with these major surfaces:

- Live page: OBS connection, primary scene switching, compact active-scene
  audio, stream controls, record controls, elapsed output timers, recording
  path copy, and configurable output confirmations.
- Mixer page: Active/Selected/Pinned modes, scene selector, search, grouping,
  scoped audio cards, scene-specific refreshes, loading/error/empty states, and
  explicit retry after selected/pinned refresh failures.
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
audio update, output confirmation decision-helper work, mixer retry-intent fix,
reducer-owned mixer input mirror containment, selected/pinned and Active-mode
Mixer input-event reconciliation, Mixer render-source reconciliation, legacy
Mixer mirror state removal, hidden snapshot invariant restoration, shared
Mixer scene-specific target resolution, and refresh-target naming cleanup. The
focused manual Mixer interaction run was recorded as blocked because a verified
OBS WebSocket session with the required scenes and inputs was unavailable.

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

- Static validation passed.
- The mixer reducer covers loading, success, failure, stale success, stale
  failure, and same-scene repeated loading.
- The duplicate guard was useful but initially conflated automatic rebuilds
  with explicit user retry intent.
- Output confirmation logic is testable, but all confirmation dialogs still use
  destructive response styling, including start stream/start recording.
- The audio optimistic update path is functionally complete, but GTK-level
  mixer card update coverage remains thin.

### Mixer Retry Intent and Refresh Contract Documentation

Working tree, reviewed 2026-06-21:

- Added `MixerRefreshRequestIntent` and `should_request_mixer_scene_audio`.
- Automatic mixer rebuilds still dedupe loaded, loading, tracked, and failed
  scenes so persistent OBS failures do not loop.
- Explicit mode/scene callbacks and the new Retry button can dispatch after a
  matching mixer refresh failure.
- Updated the stale Mixer page module comment.
- Documented the scene-level freshness invariant for
  `MixerAudioRefreshState::requested_scene`.
- Added comments warning that legacy mixer audio fields mirror the reducer.
- Added tests for automatic failure dedupe, explicit retry after failure,
  loaded/in-flight/tracked dedupe, and repeated same-scene loading.

Review verdict:

- The intended P0 retry regression is fixed for the normal failed-state flow.
- The manual Retry button is useful and avoids relying only on combo-row
  reselection semantics.
- No request-token machinery was added; scene-level freshness remains the
  explicit contract.
- Remaining high-priority gap: `InputMuteChanged` and `InputVolumeChanged`
  still mutate `mixer_audio_inputs` directly in `src/ui/window.rs` without
  updating `mixer_audio_refresh.loaded.inputs`. That violates the new mirror
  contract and can let future reducer syncs resurrect stale input values.
- Legacy mixer mirror fields remain public because GTK pages read them
  directly; comments help, but do not mechanically prevent future bypasses.

### Mixer Reducer-Owned Input Mirrors

Working tree, reviewed 2026-06-21:

- Added `AppState::update_mixer_input_mute` and
  `AppState::update_mixer_input_volume` so OBS input events update
  `MixerAudioRefreshState::loaded.inputs` and then resync the legacy mirror
  fields from the reducer-owned snapshot.
- Routed `InputMuteChanged` and `InputVolumeChanged` in `src/ui/window.rs`
  through those APIs while preserving active-scene `audio_inputs` updates.
- Made the legacy mixer mirror fields private to `src/controller/state.rs` and
  added read-only accessors for Mixer page rendering.
- Updated `src/ui/pages/mixer.rs` to use the accessors.
- Added reducer tests proving mute and volume updates keep mirrors and loaded
  snapshots synchronized and survive same-scene loading/failure transitions
  without restoring stale values.

Review verdict:

- Static validation passed: `cargo fmt --all -- --check`,
  `cargo check --workspace --all-features`,
  `cargo test --workspace --all-features`, and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- The planned P0 mirror drift fix is complete for known OBS input event paths.
- The private fields are a meaningful mechanical fence; `rg` shows no direct
  mirror reads or writes outside `src/controller/state.rs`.
- Remaining gap: visible Mixer page cards are not directly reconciled on
  `InputMuteChanged` / `InputVolumeChanged`; state is correct, but a selected
  or pinned Mixer card can stay visually stale until the page rebuilds.
- The legacy mirror fields still exist for UI rendering compatibility. Longer
  term, the Mixer page should read a single derived snapshot/accessor instead
  of coordinating four mirror accessors itself.

### Mixer Visibility Contract and Selected/Pinned Reconciliation

Working tree, reviewed 2026-06-21:

- Added `MixerVisibleAudioStatus` and `AppState::visible_mixer_audio_status`
  so the Mixer page reads one reducer-derived status for a target scene:
  loading, error, loaded inputs, or missing.
- Refactored selected/pinned Mixer rendering and retry dispatch to use the
  visible status helper instead of coordinating separate legacy mirror
  accessors.
- Added `prepare_mixer_scene_audio_request` tests covering tracker mutation,
  automatic failure dedupe, explicit retry after failure, loading dedupe, and
  tracked-request dedupe.
- Added `should_rebuild_visible_mixer_for_input_event` and routed OBS
  `InputMuteChanged` / `InputVolumeChanged` events to rebuild the Mixer page
  locally when a visible selected/pinned snapshot contains the changed input.
- Added predicate coverage for selected and pinned visible snapshots,
  unrelated inputs, non-Mixer pages, loading/error/missing snapshots, and
  stale other-scene snapshots.

Review verdict:

- Static validation passed: `cargo fmt --all -- --check`,
  `cargo check --workspace --all-features`,
  `cargo test --workspace --all-features`, and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- The reducer-derived read helper is a useful boundary and removes the Mixer
  page's previous manual combination of loading, error, loaded scene, and
  loaded inputs.
- The retry dispatch adapter closes most of the previously identified retry
  interaction coverage gap without requiring full GTK tests.
- Selected and pinned Mixer cards now refresh from local state after relevant
  OBS input events without dispatching a new OBS refresh.
- High-priority gap: Active Mixer mode still renders from `state.audio_inputs`,
  but `should_rebuild_visible_mixer_for_input_event` returns false for
  `MixerMode::ActiveScene`. Visible Active-mode Mixer cards can still remain
  stale after mute/volume events until another page rebuild occurs.
- The old legacy mirror accessors are now unused outside `AppState`; keeping
  them around weakens the new read-contract cleanup and invites future drift.

### Mixer Render-Source Contract and Active Reconciliation

Working tree, reviewed 2026-06-21:

- Added `MixerVisibleRenderSource` and
  `AppState::visible_mixer_render_source` so the current Mixer mode has an
  explicit visible data source: Active mode reads live active-scene
  `audio_inputs`, while Selected/Pinned modes read scene-specific refresh
  status.
- Refactored Mixer page rendering to consume `visible_mixer_render_source`
  instead of branching directly over active versus scene-specific state.
- Extended `should_rebuild_visible_mixer_for_input_event` so Active mode
  locally rebuilds the Mixer page when OBS mute/volume events affect a visible
  active-scene input.
- Removed the dead legacy mixer mirror read accessors.
- Added pure tests for Active/Selected/Pinned render-source selection,
  missing/stale scene-specific statuses, and Active-mode matched/unmatched input
  event reconciliation.

Review verdict:

- Static validation passed: `cargo fmt --all -- --check`,
  `cargo check --workspace --all-features`,
  `cargo test --workspace --all-features`, and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- The intended P0 Active-mode stale-card issue is fixed without dispatching new
  OBS refresh commands.
- The Mixer page now has a coherent render-source contract for all modes, and
  the old public accessor bypass has been removed.
- Remaining design gap: `src/ui/window.rs` still duplicates selected/pinned
  target-scene resolution instead of using `visible_mixer_render_source`; the
  behavior is currently correct, but duplicated visibility logic can drift.
- The private legacy mirror fields are now production-dead compatibility state;
  only `sync_mixer_audio_fields` and reducer tests still touch them.
- Rebuilding the whole Mixer page on every relevant OBS volume/mute event is
  simple and correct, but high-frequency OBS echo events could make this more
  expensive than direct visible-card updates.

### Mixer Contract Consolidation Cleanup

Working tree, reviewed 2026-06-21:

- Removed the private legacy Mixer mirror fields:
  `mixer_audio_scene`, `mixer_audio_inputs`, `mixer_audio_loading_scene`, and
  `mixer_audio_error`.
- Removed `sync_mixer_audio_fields`; Mixer reducer mutation methods now operate
  only on `MixerAudioRefreshState`.
- Rewrote mirror-focused reducer tests to assert through
  `visible_mixer_audio_status`.
- Refactored `should_rebuild_visible_mixer_for_input_event` to match on
  `state.visible_mixer_render_source()` instead of duplicating selected/pinned
  target-scene fallback logic.
- Added selected and pinned fallback regression coverage proving the rebuild
  predicate follows the render-source contract.

Review verdict:

- Static validation passed: `cargo fmt --all -- --check`,
  `cargo check --workspace --all-features`,
  `cargo test --workspace --all-features`, and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- The planned contract cleanup is complete: `rg` shows no remaining production
  mirror fields or `sync_mixer_audio_fields`, and the event predicate now shares
  the same render-source helper as Mixer rendering.
- No functional regression was found in the changed source paths.
- Test coverage regressed slightly in precision: the old same-scene
  loading/failure tests proved input-event mutations survived in the hidden
  loaded snapshot, while the rewritten tests mainly prove visible loading/error
  precedence and fresh success replacement.
- Remaining design gap: `src/ui/pages/mixer.rs` still has its own
  `mixer_target_scene` fallback helper for controls, summary text, and refresh
  dispatch, while `AppState::visible_mixer_target_scene` carries the render
  contract internally.
- Rebuilding the whole Mixer page on every relevant OBS input event remains
  correct but may be too expensive for high-frequency OBS volume echo events.

### Mixer Target-Scene Contract Consolidation

Working tree, reviewed 2026-06-21:

- Restored reducer tests that directly inspect the hidden
  `mixer_audio_refresh.loaded` snapshot after mute and volume input updates
  followed by same-scene loading and failure transitions.
- Made `AppState::visible_mixer_target_scene` public as the shared
  scene-specific refresh target contract for Selected and Pinned Mixer modes.
- Changed Active Mixer mode to report no scene-specific target through that
  helper, preventing shared request paths from dispatching scene-refresh
  commands while Active mode is rendering live `audio_inputs`.
- Removed the Mixer page's local `mixer_target_scene` fallback helper.
- Refactored summary text, automatic missing-state refresh, mode changes,
  scene changes, and retry dispatch to resolve scene-specific targets through
  `AppState::visible_mixer_target_scene`.
- Added pure tests for Active, Selected, and Pinned target resolution,
  including Selected fallback to current scene and Pinned fallback to
  selected/current scene.

Review verdict:

- Static validation passed: `cargo fmt --all -- --check`,
  `cargo check --workspace --all-features`,
  `cargo test --workspace --all-features`, and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- The two planned Mixer contract gaps are closed: hidden reducer snapshots are
  again covered directly, and scene-specific request targets are no longer
  re-derived inside `src/ui/pages/mixer.rs`.
- No functional regression was found in the changed source paths.
- Residual risk: the helper name `visible_mixer_target_scene` is easy to read
  as "the visible Mixer scene" even though it intentionally returns `None` for
  Active mode because it means "scene-specific refresh target." Future work
  should either rename it or add a parallel display-source helper before more
  Mixer UI code consumes it.
- The Mixer page still rebuilds all controls and audio cards for every relevant
  OBS input event; correctness is good, but large scenes and high-frequency
  volume echoes may need in-place card reconciliation.
- Retry and target-resolution behavior is covered by pure tests, but there is
  still no GTK/manual interaction record proving the controls, retry button,
  and mode changes behave correctly against a real OBS instance.

### Mixer Refresh-Target Naming And Manual Evidence

Working tree, reviewed 2026-06-21:

- Renamed `AppState::visible_mixer_target_scene` to
  `AppState::mixer_scene_refresh_target` so the helper explicitly describes
  its scene-specific refresh dispatch role.
- Kept `visible_mixer_render_source` as the authoritative Mixer display/render
  contract.
- Preserved the Active-mode invariant: Active mode visibly renders live
  active-scene audio but has no scene-specific Mixer refresh target.
- Updated Mixer page summary text, automatic missing-state refresh, mode
  changes, scene changes, and Retry dispatch to use
  `mixer_scene_refresh_target`.
- Added reducer tests proving Active has no scene-specific refresh target,
  Selected targets selected/current-scene fallback, and Pinned targets
  pinned/selected/current-scene fallback in that order.
- Recorded `docs/manual-test-runs.md` entry `2026-06-21 - Focused Mixer
  Refresh Contract` for the planned real-OBS interaction run.

Manual evidence:

- Status: blocked, not executed.
- Environment record: SceneDeck `0.1.3`, git commit `73bb5bc`, Linux
  `ubuntu` 7.0.0-22-generic x86_64, OBS process detected, OBS version not
  recorded because `obs --version` produced no output.
- Blocking prerequisite: the non-interactive run did not have a verified OBS
  WebSocket session with known credentials, at least two configured scenes, and
  multiple audio inputs.
- No pass/fail behavior is claimed for Active no-refresh dispatch, Selected
  fallback, Pinned fallback, Retry after failed selected/pinned refresh, OBS
  mute echoes, OBS volume echoes, stale-card behavior, retry problems, or
  full-page rebuild churn.

Review verdict:

- Static validation passed in review:
  `cargo fmt --all -- --check`, `cargo check --workspace --all-features`,
  `cargo test --workspace --all-features`, and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- The naming ambiguity called out in the previous review is resolved:
  `mixer_scene_refresh_target` now describes refresh dispatch semantics, and
  Active mode cannot accidentally be treated as a scene-specific refresh
  target by shared Mixer request paths.
- The focused manual run did not produce runtime evidence of stale cards,
  retry failure, or rebuild churn because it was blocked by missing OBS
  prerequisites.
- With retry and target behavior covered by pure tests and no runtime evidence
  of a retry/target defect, the main groomed Mixer implementation risk remains
  potential cost from whole-page rebuilds on high-frequency OBS input events.
- Remaining UX risk: Mixer summary copy uses the effective scene-specific
  refresh target for Selected/Pinned modes, so fallback states can read like a
  direct selection instead of making the fallback source explicit.

## Groomed Next Steps

### P1: Complete Focused Mixer Contract Manual Run

Problem:

- The focused 2026-06-21 Mixer run was recorded, but blocked before any
  interaction cases executed.
- The unverified areas are exactly the ones unit tests approximate poorly:
  GTK ComboRow timing, Retry button behavior, OBS event echoes, and perceived
  rebuild churn.

Plan:

- Run SceneDeck against a verified OBS WebSocket setup with known credentials,
  at least two scenes, and multiple audio inputs.
- Execute `Focused Mixer Refresh Contract` from
  `docs/manual-test-plan.md`.
- Record OBS version, SceneDeck build/commit, pass/fail results, skipped cases,
  and any stale-card, retry, or rebuild-churn observations in
  `docs/manual-test-runs.md`.
- Keep blocked entries explicit if any prerequisite remains unavailable; do not
  convert pure-test confidence into manual pass claims.

Files:

- `docs/manual-test-plan.md`
- `docs/manual-test-runs.md`

### P1: Reduce Mixer Page Rebuild Cost For High-Frequency Input Events

Problem:

- OBS volume events can arrive frequently while a user drags a control.
- The current reconciliation path rebuilds the entire Mixer page when a visible
  Mixer input changes. This is simple and correct, but it recreates controls,
  groups, and scroll content rather than updating the affected card in place.
- The focused 2026-06-21 manual run was blocked before the volume-echo case, so
  no real OBS evidence currently proves whether this churn is noticeable.

Plan:

- Use the focused manual run to observe repeated visible volume echoes before
  optimizing.
- If rebuild churn is noticeable, track visible Mixer audio cards like Live
  tracks `live.audio_cards` and update mute/volume on the matching card in
  place.
- Keep the full rebuild fallback for grouping/search/scene mode changes.

Files:

- `src/ui/pages/mixer.rs`
- `src/ui/window.rs`

Tests:

- Pure predicate coverage remains.
- Add GTK-level or widget-level coverage only if a test harness can inspect card
  updates without excessive brittleness.

### P1: Clarify Mixer Fallback Summary Copy

Problem:

- `mixer_scene_refresh_target` correctly returns the effective refresh target,
  including Selected fallback to current scene and Pinned fallback to selected
  or current scene.
- `source_summary` currently labels that effective target as `Selected scene`
  or `Pinned scene`, which can hide whether the value is a direct user choice
  or a fallback.

Plan:

- Add a small AppState helper or UI helper that returns both the effective
  Mixer scene-specific refresh target and its reason: selected, pinned,
  selected fallback, or current-scene fallback.
- Update Mixer summary copy to distinguish direct targets from fallbacks without
  changing dispatch behavior.
- Add pure tests for summary/fallback metadata so copy remains aligned with
  `mixer_scene_refresh_target`.

Files:

- `src/controller/state.rs`
- `src/ui/pages/mixer.rs`

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

- `requires_output_confirmation` is tested, but the dialog presentation is still
  generic.
- Start stream/start recording confirmations use destructive styling even
  though they are not destructive in the same sense as stop actions.

Plan:

- Add a small helper that maps output kind/action to dialog copy and response
  appearance.
- Keep stop stream/stop recording as destructive.
- Use neutral or suggested styling for start stream/start recording.
- Add pure tests for action-to-dialog metadata.

Files:

- `src/ui/pages/live.rs`

### P1: Settings Persistence Feedback

Problem:

- Output safety toggles update in-memory state before disk persistence is
  known.
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

### P1: Full Manual Test Plan Execution Record

Problem:

- `docs/manual-test-runs.md` now exists for focused Mixer evidence, but the
  broader `docs/manual-test-plan.md` still has no complete pass/fail run
  against a real OBS instance.

Plan:

- Add a dated full-plan manual test result entry.
- Record OBS version, SceneDeck build, skipped streaming cases, and failures.

Files:

- `docs/manual-test-plan.md`
- `docs/manual-test-runs.md`

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
