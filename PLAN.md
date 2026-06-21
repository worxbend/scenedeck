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
Mixer scene-specific target resolution, refresh-target naming cleanup,
fallback-aware Mixer summary copy, direct Mixer summary copy tests, output
confirmation dialog appearance metadata, tightened focused Mixer manual
evidence instructions, reproducible focused Mixer fixture documentation, the
opt-in Mixer debug inspection path, rendered-branch inspection status
alignment, output-command error state/UI labels, output command failure
separation from generic OBS connection errors, and localized output command
failure recovery when follow-up status refreshes fail. Focused review reran
`cargo test --workspace --all-features command_failure -- --nocapture`,
`stream_command`, and `record_command`; all passed before the full validation
rule also passed.

The Mixer debug path now distinguishes loading placeholders, error
placeholders, missing no-target state, loaded visible cards, loaded empty audio,
and filtered-empty audio. Structured `volume_label` values are derived with the
same `AudioService::format_db` helper used by rendered audio cards from the
same visible-card `volume_db` value emitted in JSON. Focused manual Mixer
interaction evidence is still not complete. The latest focused run was blocked
because OBS was not running, `127.0.0.1:4455` refused WebSocket connections,
no temporary fixture could be verified, and the non-interactive session had no
control path for GTK ComboRows or Retry. No pass/fail behavior is claimed yet
for ComboRow timing, Retry activation, OBS mute/volume echoes, stale visible
cards, or runtime rebuild churn.

Output command errors now have per-output state fields, Live-page labels, async
failure coverage through a test-only output-command client, and no longer emit
generic `AppEvent::Error` for localized stream/record command failures. Failed
stream/record commands carry an explicit event-boundary
`OutputCommandFailureRecovery` payload with private fields and accessor-only
reads, so production construction must pass through the normalizing
`with_failed_command_fallback_status` constructor. Failed starts fall back to
inactive and failed stops fall back to active even when one or both follow-up
output-status refresh calls fail. The fallback calculation is covered for every
`OutputRunState`, including `Unknown` and `Paused` passthrough behavior, and
event-level tests prove transition fallback inputs are normalized before
storage. Output status refresh logic is unified through a narrow
`OutputStatusReader` helper shared by `ObsClient` and the output-command
wrapper. Live output controls now render as two output cards with stable slots
for state/elapsed copy, pending command progress, concise errors, and the last
recording path. Backend error details remain in tooltips. The remaining output
layout gap is proof and truncation: the visible recording path is still the raw
path string, and no GTK render/manual check has proved that long paths cannot
stretch or destabilize the card.

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

### Mixer Fallback Copy, Output Dialog Semantics, And Evidence Gate

Working tree, reviewed 2026-06-21:

- Added `MixerSceneRefreshTargetReason` and `MixerSceneRefreshTarget` so
  `AppState::mixer_scene_refresh_target_details` reports both the effective
  scene-specific refresh target and the fallback rule that selected it.
- Updated Mixer summary copy to distinguish direct selected/pinned targets from
  selected-current, pinned-selected, and pinned-current fallback cases.
- Kept `mixer_scene_refresh_target` as the dispatch-facing string helper and
  `visible_mixer_render_source` as the display-source contract.
- Added output confirmation dialog metadata for action copy and response
  appearance.
- Changed start stream/start recording confirmations to suggested appearance
  while leaving stop stream/stop recording destructive.
- Recorded a second blocked focused Mixer run with better environment evidence:
  OBS WebSocket at `127.0.0.1:4455` was reachable without authentication, OBS
  `32.1.2` and obs-websocket `5.7.3` were observed, and two scenes/two global
  audio inputs were found.
- Preserved the evidence gate for Mixer input-event optimization; no in-place
  card update path was added because repeated echo/rebuild churn was not
  observed.

Review verdict:

- Static validation passed in review:
  `cargo fmt --all -- --check`, `cargo check --workspace --all-features`,
  `cargo test --workspace --all-features`, and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Output confirmation semantics are complete for the scoped task and have pure
  tests covering all stream/record start/stop metadata.
- Mixer fallback copy is functionally implemented, but there are no direct
  tests for `source_summary`/`scene_target_summary`; the target reason contract
  is tested in `AppState`, while the final user-facing strings are not.
- The manual run remains blocked for interaction evidence. It improved the
  environment record but still produced no pass/fail claims for ComboRow timing,
  Retry behavior, mute/volume echoes, stale cards, or rebuild churn.
- The conditional optimization task was correctly skipped because the manual
  evidence gate did not show runtime cost.

### Mixer Summary Copy Tests And Manual Evidence Instructions

Working tree, reviewed 2026-06-21:

- Added focused helper-level tests in `src/ui/pages/mixer.rs` for the final
  `source_summary` strings:
  - Active mode follows the active OBS scene.
  - Direct Selected and direct Pinned scenes use direct labels.
  - Selected current-scene fallback, Pinned selected-scene fallback, and Pinned
    current-scene fallback all use explicit fallback wording.
  - Scene-specific modes with no target report `No scene selected`.
- Tightened `docs/manual-test-plan.md` for the focused Mixer refresh contract:
  prerequisites now require reachable WebSocket details, global audio inputs,
  differing scene-specific audio inputs, inspectable GTK ComboRows/cards, and
  explicit recording of skipped cases.
- Added a reusable focused Mixer run template to `docs/manual-test-runs.md` and
  expanded blocked-run entries with skipped cases and non-claims.

Review verdict:

- Scoped validation passed: `git diff --check` and
  `cargo test --workspace --all-features summary -- --nocapture` ran
  successfully; the summary filter executed seven Mixer summary tests.
- The previously identified final-copy coverage gap is closed at the helper
  level. The tests exercise the same `AppState::mixer_scene_refresh_target_details`
  path used by the UI before calling `source_summary`.
- No production behavior changed in this iteration.
- Manual evidence remains blocked. The improved checklist/template reduces the
  chance of overstated runtime claims, but it still does not prove ComboRow
  timing, Retry behavior, OBS mute/volume echoes, stale-card behavior, or
  rebuild churn.
- Remaining code-quality gap: the summary helper is still private to the Mixer
  page and tested through an internal module, which is fine for now, but any
  future reuse should extract a small copy/label helper rather than duplicating
  these strings in GTK code.

### Focused Mixer Fixture Documentation And Evidence Triage

Working tree, reviewed 2026-06-21:

- Documented a small non-destructive OBS fixture for the focused Mixer manual
  run: a throwaway OBS profile or clearly temporary `SceneDeck Test ...` scenes,
  a global audio input visible in both scenes, and a scene-specific audio input
  present in only one test scene.
- Documented cleanup guidance and kept destructive mutations to the user's
  normal OBS setup out of the default run.
- Recorded the available UI automation status for the target session: no usable
  tool was available for selecting GTK ComboRows, clicking Retry, or inspecting
  visible Mixer cards, so an interactive desktop session remains required.
- Recorded `docs/manual-test-runs.md` entry
  `2026-06-21 - Focused Mixer Refresh Contract (iteration 12)`.

Manual evidence:

- Status: blocked.
- Environment record: SceneDeck `0.1.3`, git commit `95806c4`, Linux `ubuntu`
  7.0.0-22-generic x86_64, GNOME Wayland, OBS process detected, OBS WebSocket
  reachable at `127.0.0.1:4455` without authentication, OBS `32.1.2`,
  obs-websocket `5.7.3`, scenes `Scene 2` and `Scene`, and global audio inputs
  `Desktop Audio` and `Mic/Aux`.
- Blocking prerequisites: the OBS fixture did not include a scene-specific
  audio input present in only one test scene, the non-interactive session could
  not drive or inspect GTK controls/cards, and no non-destructive selected or
  pinned refresh failure setup was available.
- No pass/fail behavior is claimed for Active mode scene following, absence of
  Active-mode scene-specific refresh dispatches, Selected fallback, Pinned
  fallback, Retry after selected/pinned refresh failure, OBS mute echoes, OBS
  volume echoes, stale visible cards, or perceived rebuild churn.
- No stale-card issue, retry failure, ComboRow timing issue, or noticeable
  rebuild churn was observed because the relevant interaction cases were not
  executed.

Triage decision:

- Keep the focused Mixer runtime evidence gap open. The iteration 12 entry
  improves fixture and environment documentation but remains a blocked manual
  run, not a passing interaction run.
- Keep in-place Mixer card optimization deferred. The current full-page Mixer
  rebuild path remains accepted behavior until repeated volume echoes are
  observed against an inspectable Mixer UI and produce noticeable churn.

### Mixer Debug Inspection Path

Working tree, reviewed 2026-06-21:

- Added `MixerInspectionSnapshot`, `MixerInspectionInput`,
  `MixerInspectionStatus`, and `MixerInspectionRenderSourceKind` in
  `src/controller/state.rs`.
- Added `AppState::mixer_inspection_snapshot` so debug evidence can read the
  same render-source, refresh-target, fallback, loading/error/missing, mute,
  volume, and formatted dB state used by the Mixer page.
- Added opt-in `SCENEDECK_MIXER_INSPECT=1` stderr output from
  `src/ui/pages/mixer.rs`, using `scenedeck_mixer_inspect {json}` lines with
  mode, selected/pinned scenes, refresh target/reason, render source/status,
  visible cards, and Retry visible/enabled state.
- Added unit coverage for snapshot variants and inspection JSON formatting.
- Updated `docs/manual-test-plan.md` and `docs/manual-test-runs.md` with a
  debug inspection execution path, limits, and a focused inspection run
  template.

Review verdict:

- Static validation passed in review:
  `cargo fmt --all -- --check`, `cargo check --workspace --all-features`,
  `cargo test --workspace --all-features`, and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- The debug path is narrow, opt-in, and useful for avoiding brittle GTK screen
  scraping. It does not introduce a visible production surface.
- The inspection model mostly follows the existing Mixer render-source and
  refresh-target contracts and is covered across Active, Selected, Pinned,
  loading, error, missing, and visible-card formatting cases.
- High-priority evidence gap: in `populate`, the Missing scene-specific branch
  dispatches an automatic refresh and renders the "Loading Mixer Audio"
  placeholder, but emits the inspection snapshot captured before that dispatch.
  The structured line can therefore report `status.kind = "missing"` while the
  visible UI shown to the tester is loading. This weakens the inspection output
  as executable evidence and should be fixed before a run is treated as
  authoritative.
- Design concern: the controller state module now imports `AudioService` only
  to format a debug/UI-facing dB label. This is not a functional regression, but
  future cleanup should keep presentation formatting close to UI/debug
  serialization if the inspection path grows.
- The debug inspection path cannot prove pointer interaction success, visual
  layout quality, or perceived rebuild churn; the docs correctly preserve those
  limits.

### Mixer Rendered Inspection Status Alignment

Working tree, reviewed 2026-06-21:

- Expanded `MixerInspectionStatus` to model rendered UI branches:
  loading placeholder, error placeholder, missing/no-target, loaded visible
  cards, loaded with no audio sources, and loaded with no matching audio sources
  after filtering.
- Changed Mixer inspection emission so `src/ui/pages/mixer.rs` passes the
  branch-specific rendered status into `format_mixer_inspection_line` instead
  of always serializing the pre-render reducer snapshot status.
- Fixed the specific Missing -> automatic refresh mismatch: that branch now
  emits `loading_placeholder_shown` while rendering the "Loading Mixer Audio"
  status page.
- Made `append_mixer_inputs` return the rendered loaded status so inspection
  output can distinguish visible cards, no audio sources, and filtered-empty
  status pages.
- Removed the direct `AudioService` import from `src/controller/state.rs` and
  replaced it with local debug dB formatting in the inspection snapshot.
- Updated the focused manual test plan and run template to record
  loading/requested, loaded-empty, and filtered-empty inspection evidence.

Review verdict:

- Scoped validation passed in review: `git diff --check` and
  `cargo test --workspace --all-features mixer_inspection -- --nocapture`.
- The original high-priority Missing -> Loading inspection mismatch is fixed:
  inspection output is now emitted from the same UI branch that appends the
  visible placeholder or cards.
- Empty loaded states are now represented clearly enough for future manual
  evidence to distinguish "loaded but no sources" from "loaded but search
  filtered all sources."
- New high-priority evidence bug: `src/controller/state.rs` now formats
  inspection snapshot dB labels with `format_mixer_inspection_db`, which only
  maps non-finite values to `-inf dB`. The rendered audio card uses
  `AudioService::format_db`, which also maps values `<= -100.0` to `-inf dB`
  and normalizes near-zero values to `0.0 dB`. For example, an input at
  `-120.0 dB` can be inspected as `-120.0 dB` while the visible card reads
  `-inf dB`, and `0.01 dB` can inspect as `0.0 dB` only by rounding rather than
  by the shared display rule. The debug path should not be treated as
  authoritative card-label evidence until this is fixed and covered by tests.
- Design concern remains: inspection status is partly controller-derived and
  partly UI-rendered. That split is acceptable for an opt-in evidence path, but
  display formatting must be shared or injected from the UI layer to avoid a
  second presentation model.

### Mixer Inspection Volume Label Alignment

Working tree, reviewed 2026-06-21:

- Removed `volume_label` from `MixerInspectionInput` so controller state no
  longer owns a duplicated presentation formatter.
- Changed Mixer inspection JSON serialization in `src/ui/pages/mixer.rs` to
  derive `volume_label` with `AudioService::format_db`, the same helper used by
  rendered audio cards.
- Added focused inspection coverage for `f64::NEG_INFINITY`, `-120.0`,
  near-zero positive and negative values, zero, and a normal value such as
  `-6.24`.
- Updated focused Mixer evidence instructions and run templates to require the
  shared rendered audio-card dB formatter before treating structured
  `volume_label` values as visible-card evidence.

Review verdict:

- Scoped validation passed in review:
  `cargo test --workspace --all-features mixer_inspection -- --nocapture`.
- The high-priority evidence-fidelity bug from the previous review is fixed:
  inspection labels now follow `AudioService::format_db`, including the
  `<= -100.0 dB` floor and near-zero normalization used by audio cards.
- The implementation correctly keeps the debug path opt-in and avoids adding a
  visible production control.
- No functional regression was found in the changed paths.
- Minor cleanup opportunity: `format_mixer_inspection_line` formats the label
  by looking up a matching snapshot input by name and falling back to the
  visible card value. In normal rendering those values come from the same
  cloned inputs, but serializing directly from the visible card would make the
  label/value relationship simpler and avoid a future mismatch if inspection
  callers ever pass a filtered or transformed visible-card list.

### Mixer Inspection Card Serialization And Blocked Evidence

Working tree, reviewed 2026-06-21:

- Simplified `format_mixer_inspection_line` so each visible card's
  `volume_label` is formatted directly from the same `input.volume_db` value
  emitted as that card's `volume_db`.
- Added a focused regression test that constructs a snapshot with one volume
  and passes a transformed visible card with a different volume, proving the
  JSON label follows the visible card rather than a same-name snapshot input.
- Recorded `docs/manual-test-runs.md` entry
  `2026-06-21 - Focused Mixer Inspection Run (iteration 16)`.

Manual evidence:

- Status: blocked.
- OBS was not running in the reviewed session, no `obs` binary was found in
  `PATH`, and a WebSocket probe to `ws://127.0.0.1:4455` failed with
  `ConnectionRefusedError [Errno 111]`.
- No OBS version, obs-websocket version, scene inventory, global input
  inventory, scene-specific fixture input, or `scenedeck_mixer_inspect` JSON
  lines were captured.
- The non-interactive session still had no documented control path for
  switching to Mixer, selecting ComboRows, clicking Retry, or driving search;
  `xdotool` and `ydotool` were unavailable.
- No pass/fail behavior is claimed for Active/Selected/Pinned rendering,
  fallback behavior, Retry, loaded-empty, filtered-empty, mute echo, volume
  echo, stale-card behavior, ComboRow timing, or perceived rebuild churn.

Review verdict:

- Scoped validation passed in review: `git diff --check` and
  `cargo test --workspace --all-features mixer_inspection -- --nocapture`.
- The inspection card serialization cleanup is correct and closes the prior
  minor evidence-contract issue: `volume_db` and `volume_label` can no longer
  be sourced from different per-card volume values.
- The manual evidence task did not produce runtime evidence. It only documents
  that this environment cannot currently execute the focused Mixer inspection
  run.
- Repeating the focused run in the same environment is unlikely to add value
  until OBS/WebSocket, a temporary fixture, and an interaction or control path
  are available.

### Output Command Error State And Live Rendering

Working tree, reviewed 2026-06-21:

- Added `last_stream_command_error` and `last_record_command_error` to
  `AppState`.
- Added stream/record command pending, succeeded, and failed events.
- Routed command pending/success/failure events through the UI so Live stream
  and record controls can show per-output error labels.
- Cleared output command errors on new pending command, command success,
  reconnect/connecting/disconnect resets, and Live output reset.
- Added state-level tests for stream/record error ownership and controller
  tests for no-client stream/record command failures.
- Tightened the focused Mixer evidence gate in the manual test docs so the
  blocked environment is not re-run as a full interaction checklist.

Review verdict:

- Full validation passed in review:
  `cargo fmt --all -- --check`, `cargo check --workspace --all-features`,
  `cargo test --workspace --all-features`, and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- The state and Live label slice is mostly implemented: stream and record
  errors are tracked independently and rendered in the output area.
- High-priority regression/unfinished contract: `set_streaming` and
  `set_recording` still send generic `AppEvent::Error` on OBS command failure
  before sending the output-specific failed event. The generic handler marks
  OBS status as error, clears output errors, resets controls, shows the
  disconnected/error Live view, and emits an OBS-error toast. This violates the
  stated goal of preserving OBS connection errors separately from output
  command errors and can still make stream/record failures look like connection
  failures.
- The no-client path correctly emits only the output-specific failure event,
  but async OBS command failures are not covered by tests because the
  controller still lacks an injected fake output command client.
- Output error label layout is functional, but long backend error text is
  inserted verbatim into the compact banner. A later output-card layout should
  give these errors a dedicated bounded area and possibly shorter display copy
  with the full text in a tooltip.

### Output Command Failure Separation

Working tree, reviewed 2026-06-21:

- Replaced direct stream/record command use of `ObsClient` with a narrow
  `OutputCommandClient` wrapper so controller tests can inject a failing
  output-command client while production still uses the live OBS client.
- Changed `set_streaming` and `set_recording` so async OBS command failures log
  a warning, emit only `StreamCommandFailed` or `RecordCommandFailed`, and then
  refresh stream/record output statuses.
- Preserved genuine no-client behavior as output-specific command failure
  events without changing OBS connection state.
- Added controller tests for async stream and record command failures proving
  no generic `AppEvent::Error` is emitted and both output statuses are
  refreshed afterward.
- Added state-level event-sequence tests proving localized command failures
  leave `ObsStatus::Connected`, keep the Live page selected, preserve the other
  output's last error, and clear only the retried/succeeded output error.
- Routed normal `StreamStatusUpdated` and `RecordStatusUpdated` UI handlers
  through `AppState` setters so status refreshes no longer need direct field
  writes.

Review verdict:

- Focused validation passed in review:
  `cargo test --workspace --all-features command_failure -- --nocapture`,
  `cargo test --workspace --all-features stream_command -- --nocapture`, and
  `cargo test --workspace --all-features record_command -- --nocapture`.
- The original high-priority bug is fixed: localized stream/record command
  failures no longer flow through the connection-level `AppEvent::Error`
  handler, so they no longer force the Live page into disconnected/error UI or
  erase output-specific error state.
- The fake client seam is narrow and `#[cfg(test)]`, which avoids broadening
  production controller abstractions just for tests.
- New high-priority hardening gap: if `set_streaming`/`set_recording` fails
  and the subsequent output status refresh also fails, the UI can remain on the
  synthetic pending status (`Starting`/`Stopping`) because `StreamCommandFailed`
  and `RecordCommandFailed` currently set only the error message. The buttons
  remain disabled for transitioning states until another status event arrives.
- Design cleanup opportunity: status refresh logic now exists in both
  `refresh_output_statuses` for `ObsClient` and
  `refresh_output_statuses_for_output_client` for the wrapper. This is small
  but should be unified if output-client abstraction grows.

### Output Command Failure Recovery

Working tree, reviewed 2026-06-21:

- Added `OutputCommandFailure` with a message and recovered `OutputStatus`.
- Changed `StreamCommandFailed` and `RecordCommandFailed` to carry the
  recovered failure payload instead of only a string.
- Computed localized fallback statuses from the synthetic pending status:
  failed starts recover to inactive and failed stops recover to active.
- Normalized carried recovery statuses so `Starting`, `Stopping`, and
  `Reconnecting` cannot remain as the immediate command-failure result.
- Routed failure events through `AppState::set_*_command_failure_with_recovery`
  in `src/ui/window.rs`, so Live controls update from the recovered status as
  soon as the failure event arrives.
- Extended the fake output-command client so status refreshes can fail.
- Added controller tests for stream and record command failures where the
  affected status refresh fails and where both stream/record status refreshes
  fail.
- Added reducer tests proving failed starts/stops leave non-transitioning
  output states and preserve the other output's command error.

Review verdict:

- Full validation passed in review:
  `cargo fmt --all -- --check`, `cargo check --workspace --all-features`,
  `cargo test --workspace --all-features`, and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- The high-priority stuck-control bug is fixed for the targeted path:
  localized command failure plus failed follow-up status refresh no longer
  leaves the affected Live button disabled in synthetic Starting/Stopping
  state.
- The implementation preserves the event-boundary rule from the previous
  iteration: ordinary stream/record command failures still do not emit generic
  `AppEvent::Error` or reset OBS connection state.
- No functional regression was found in the changed source paths.
- Design gap: `OutputCommandFailure.recovered_status` is a localized fallback,
  not necessarily an authoritative OBS status. Its name and placement in
  `controller::state` are serviceable for the current reducer path, but future
  code could treat it as a fresh OBS reading unless the type is clarified or
  moved closer to the output event contract.
- Design debt remains from the previous iteration: output status refresh logic
  is duplicated between the direct `ObsClient` helper and the
  `OutputCommandClient` wrapper helper.
- Output error presentation remains compact and technical. Long backend error
  strings are still shown directly in the Live banner, though tooltips preserve
  the full text.

### Output Failure Recovery Semantics And Refresh Helper Cleanup

Working tree, reviewed 2026-06-21:

- Renamed the output command failure payload to
  `OutputCommandFailureRecovery` and renamed the reducer-applied status field
  to `fallback_status`, making the local fallback semantics explicit at the
  event boundary.
- Replaced indirect fallback construction with
  `fallback_status_after_failed_output_command`, a pure helper that maps
  transition states to inactive or active based on the status' active flag and
  preserves non-transitioning states.
- Added focused reducer tests covering every `OutputRunState`, including
  transition normalization and `Unknown`/`Paused` passthrough.
- Wired controller stream/record command failures to compute the fallback
  status directly from the synthetic pending command state.
- Replaced duplicated `ObsClient` and output-command-wrapper status refresh
  functions with one `refresh_output_statuses` helper over a private
  `OutputStatusReader` trait.
- Kept status refresh failures localized to warnings; ordinary stream/record
  status refresh noise still does not emit generic `AppEvent::Error`.

Review verdict:

- Full validation passed in review:
  `cargo fmt --all -- --check`, `cargo check --workspace --all-features`,
  `cargo test --workspace --all-features`, and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Focused tests also passed:
  `cargo test --workspace --all-features command_failure -- --nocapture` and
  `cargo test --workspace --all-features failed_output_command -- --nocapture`.
- The planned output contract clarification is implemented and does not
  reintroduce connection-level error handling for localized stream/record
  command failures.
- The output status refresh duplication is removed without changing the
  observable stream-then-record refresh order or the current warning-only
  handling for refresh failures.
- No functional regression was found in the changed source paths.
- Minor design debt remains: `OutputCommandFailureRecovery` and
  `fallback_status_after_failed_output_command` are currently public from
  `controller::state`; they are really event/command-contract concepts and
  should be moved or narrowed if more output event types are added.
- Output error presentation remains compact and technical. The next
  high-value output slice is still concise visible error copy with full backend
  details available separately.

### Output Failure Contract Move And Concise Live Error Copy

Working tree, reviewed 2026-06-21:

- Moved `OutputCommandFailureRecovery` and
  `fallback_status_after_failed_output_command` from `controller::state` to
  `controller::event`, where the stream/record failure events are defined.
- Updated controller and reducer imports so `AppState` consumes the recovery
  event payload instead of owning the payload type.
- Changed Live stream/record command error labels to show concise visible copy:
  `Stream command failed` and `Recording command failed`.
- Preserved the full backend error text in each label tooltip.
- Added helper-level Live tests for stream copy, recording copy, and absent or
  empty errors.
- Added a small CSS width hint for compact output command error labels.

Review verdict:

- Full validation passed in review:
  `cargo fmt --all -- --check`, `cargo check --workspace --all-features`,
  `cargo test --workspace --all-features`, and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Focused output tests also passed:
  `cargo test --workspace --all-features command_failure -- --nocapture` and
  `cargo test --workspace --all-features failed_output_command -- --nocapture`.
- No functional regression was found in the changed source paths. Localized
  output failures still stay out of generic `AppEvent::Error`, preserve OBS
  connection state, and recover out of synthetic transition states.
- The event-boundary move is mostly complete, but `AppState` still exposes
  `recover_stream_command_failure_from_current` and
  `recover_record_command_failure_from_current`. Those methods are currently
  test-oriented convenience APIs and keep a small amount of command-recovery
  orchestration vocabulary in reducer state.
- The concise visible-copy slice is implemented, but the compact Live output
  row still does not provide a stable card/details layout for pending state,
  elapsed time, recording path, and last error. The CSS `max-width` hint should
  not be treated as proof of final layout quality without a GTK render check.

### Output Recovery Ownership Boundary Tightening

Working tree, reviewed 2026-06-21:

- Removed `AppState::recover_stream_command_failure_from_current` and
  `AppState::recover_record_command_failure_from_current`, so reducer state no
  longer constructs `OutputCommandFailureRecovery` payloads from its current
  output status.
- Updated state tests to apply explicit `OutputCommandFailureRecovery`
  payloads through `set_stream_command_failure_with_recovery` and
  `set_record_command_failure_with_recovery`.
- Added reducer tests proving the reducer applies the event payload it receives
  rather than recalculating recovery from its current status.
- Narrowed `fallback_status_after_failed_output_command` from public to
  `pub(crate)`.

Review verdict:

- Focused validation passed in review:
  `git diff --check`,
  `cargo test --workspace --all-features failed_output_command -- --nocapture`,
  and `cargo test --workspace --all-features command_failure -- --nocapture`.
- The planned reducer ownership cleanup is complete: `AppState` now only
  applies output command recovery events and no longer exposes convenience
  methods that derive recovery from reducer-owned status.
- No functional regression was found. Existing localized command failure paths
  still avoid generic `AppEvent::Error`, preserve OBS connection state, and
  leave synthetic pending states immediately through fallback payloads.
- Remaining API-boundary issue:
  `OutputCommandFailureRecovery::from_current_status` is still public from
  `controller::event` and state tests still use it. That keeps a broad
  constructor available for deriving a command-recovery payload from any
  current status, even though production controller code already computes the
  fallback from the synthetic pending command state.
- `fallback_status_after_failed_output_command` is `pub(crate)`, which is an
  improvement but still broad enough for any crate module to call. That is
  acceptable while controller and reducer tests cover the helper directly, but
  it should not become a general state utility.

### Output Recovery Event API Narrowing

Working tree, reviewed 2026-06-21:

- Removed the public `OutputCommandFailureRecovery::from_current_status`
  constructor from `controller::event`.
- Moved focused fallback-state-machine tests from `controller::state` to
  `controller::event`, keeping fallback calculation coverage with the event
  contract.
- Replaced state-test uses of command recovery constructors with explicit
  `OutputCommandFailureRecovery` payloads so reducer tests prove only payload
  application behavior.
- Preserved focused output validation coverage:
  `cargo test --workspace --all-features failed_output_command -- --nocapture`
  and `cargo test --workspace --all-features command_failure -- --nocapture`.

Review verdict:

- The planned public-constructor cleanup is complete: `rg` shows no remaining
  `from_current_status` references, and reducer tests no longer derive recovery
  payloads from reducer-owned state.
- Focused validation passed in review, and `git diff --check` was clean.
- No production behavior regression was found. Localized stream/record command
  failures still stay out of generic `AppEvent::Error`, and production
  controller paths still construct normalized fallback payloads from the
  synthetic pending command status.
- Remaining API-boundary issue: `OutputCommandFailureRecovery` exposes public
  `message` and `fallback_status` fields, while `AppState` intentionally
  applies the exact carried payload. Any caller can therefore bypass
  `with_fallback_status` and create a transition-state fallback that would
  strand the UI again. Current production code does not do that, but the type
  does not enforce its own event-boundary invariant.
- The raw fallback helper remains `pub(crate)` for controller orchestration and
  event tests. That is acceptable for now, but it should remain out of UI and
  reducer code.

### Output Recovery Payload Invariant Sealing

Working tree, reviewed 2026-06-21:

- Made `OutputCommandFailureRecovery` fields private in
  `src/controller/event.rs`.
- Replaced direct field reads with `message()` and `fallback_status()`
  accessors in reducer and controller tests.
- Renamed the normalizing constructor to
  `with_failed_command_fallback_status`, making the command-failure fallback
  semantics explicit at call sites.
- Added an event-level test proving transition fallback inputs are normalized
  before they are stored in the recovery payload.
- Added a `#[cfg(test)]` unchecked constructor so reducer tests can still prove
  `AppState` applies the carried event payload exactly, without exposing invalid
  production construction.

Review verdict:

- Focused validation passed in review:
  `git diff --check`,
  `cargo test --workspace --all-features failed_output_command -- --nocapture`,
  and `cargo test --workspace --all-features command_failure -- --nocapture`.
- The planned invariant encapsulation is complete for production API usage:
  direct struct literals can no longer create transition-state fallback payloads
  outside `controller::event`, and production stream/record failure paths use
  the normalizing constructor.
- No functional regression was found. Localized stream/record command failures
  still avoid generic `AppEvent::Error`, preserve OBS connection state, and
  recover out of synthetic pending states when follow-up status refreshes fail.
- Reducer tests intentionally use a test-only unchecked constructor for exact
  payload-application coverage. That keeps the reducer boundary honest while
  avoiding a production escape hatch.
- Minor API cleanup opportunity: `fallback_status_after_failed_output_command`
  remains `pub(crate)` because controller orchestration and event tests use it.
  It is acceptable at current scope, but future output event work should keep
  this helper inside controller/event orchestration rather than making it a
  general UI or reducer utility.

### Stable Live Output Cards And Mixer Evidence Gate

Working tree, reviewed 2026-06-21:

- Replaced the compact stream/record output row with two card-like Live output
  controls in `src/ui/pages/live.rs`.
- Added stable card slots for title, button/state row, pending progress copy,
  command error copy, and recording-path detail.
- Kept concise visible command errors and raw backend details in tooltips.
- Added pending-state copy for stream and recording `Starting`, `Stopping`, and
  `Reconnecting` states.
- Added recording-path display helper and helper-level tests for pending copy,
  elapsed-time copy, and path display behavior.
- Added CSS for `.output-card`, progress/detail/error rows, and compact icon
  button sizing.
- Tightened focused Mixer evidence docs so future entries treat the Mixer
  runtime gap as an environment-readiness gate and preserve non-claims for
  visual layout and rebuild churn.

Review verdict:

- Focused validation passed in review:
  `git diff --check` and
  `cargo test --workspace --all-features output -- --nocapture`.
- The planned card helper and display-model tests are present. Pending state,
  concise error copy, elapsed active-state copy, and last-recording-path copy
  are covered at the pure-helper level.
- No output command behavior regression was found in the touched code paths;
  command failure handling, fallback recovery, and connection-error separation
  were not changed.
- The card implementation is only partially proven as layout work. CSS
  `max-width` hints and wrapped labels do not prove GTK allocation behavior,
  and the visible recording-path detail currently renders the full raw path.
  Long unbroken paths can still plausibly increase card height or width until
  a bounded/ellipsized display helper and render/manual check prove otherwise.
- The Stream card has one fewer detail row than Recording. The minimum height
  masks some difference, but there is no render evidence that both cards remain
  visually aligned across themes, window widths, long error tooltips, and long
  recording paths.
- The Mixer documentation change is appropriate: no new blocked runtime run was
  added, and the evidence gate remains explicit.

## Groomed Next Steps

### P1: Make Focused Mixer Evidence Executable

Problem:

- The focused Mixer interaction contract still has no passing or failing
  runtime evidence.
- The debug inspection path is now branch-aligned and internally consistent:
  visible card `volume_db` and `volume_label` are serialized from the same card
  value with the rendered audio-card formatter.
- The latest run was blocked earlier than prior runs: OBS was not running,
  the configured WebSocket refused connections, no fixture inventory could be
  queried, and no `scenedeck_mixer_inspect` lines were captured.
- The environment still lacks a reliable way to drive or inspect GTK Mixer
  controls non-interactively. Structured inspection helps with rendered state
  and card data, but not pointer interaction, visual layout quality, or
  perceived rebuild churn.

Plan:

- First establish prerequisites instead of rerunning the checklist blindly:
  verify an OBS process, WebSocket reachability, auth mode, OBS version, and
  obs-websocket version.
- Prepare or verify a temporary OBS fixture with at least two scenes, global
  audio inputs, and one scene-specific input present in only one fixture scene.
- Choose one executable control path:
  an interactive desktop run, a documented automation tool, or a narrow
  debug-only app/control hook that can switch Mixer modes, select scenes, click
  Retry, set search text, and trigger page renders without becoming production
  UI.
- Only after those prerequisites pass, run SceneDeck with
  `SCENEDECK_MIXER_INSPECT=1` and record Active, Selected, Pinned, Retry,
  mute echo, volume echo, loaded-empty, filtered-empty, stale-card, and rebuild
  churn results in `docs/manual-test-runs.md`.
- If OBS or a control path remains unavailable, stop adding near-identical
  blocked Mixer entries and move to independent P1 work while preserving the
  open runtime-evidence gap.

Files:

- `docs/manual-test-plan.md`
- `docs/manual-test-runs.md`
- possible narrow debug/control code in `src/ui/` only if an interactive run is
  not available and the hook remains opt-in

Tests:

- Manual evidence run, with `SCENEDECK_MIXER_INSPECT=1` output attached or
  pasted for the exercised cases.
- Any debug/control hook should have pure or focused integration coverage for
  command parsing and no-op behavior when disabled.

### P1: Reduce Mixer Page Rebuild Cost For High-Frequency Input Events

Problem:

- OBS volume events can arrive frequently while a user drags a control.
- The current reconciliation path rebuilds the entire Mixer page when a visible
  Mixer input changes. This is simple and correct, but it recreates controls,
  groups, and scroll content rather than updating the affected card in place.
- The focused iteration 12 manual run was blocked before the volume-echo case,
  so no real OBS evidence currently proves whether this churn is noticeable.
- No stale-card issue, retry failure, ComboRow timing issue, or rebuild churn
  was observed in the latest manual entry because the relevant cases were not
  executed.

Plan:

- Use the focused manual run to observe repeated visible volume echoes before
  optimizing.
- If rebuild churn is noticeable, track visible Mixer audio cards by input name
  and update mute/volume on the matching card in place, similar to the Live
  page's audio-card handle map.
- Keep the full rebuild fallback for grouping/search/scene mode changes.
- Preserve the current render-source contract: Active mode updates from live
  active-scene `audio_inputs`, while Selected/Pinned update from the
  scene-specific Mixer refresh snapshot.

Files:

- `src/ui/pages/mixer.rs`
- `src/ui/window.rs`

Tests:

- Existing pure predicate coverage remains.
- Add GTK-level or widget-level coverage only if a test harness can inspect card
  updates without excessive brittleness.

### P1: Prove And Tighten Live Output Card Layout

Problem:

- Stream/record controls now render as cards with stable slots, but layout
  stability has not been proven by GTK rendering.
- The recording card shows the full raw recording path as visible text. Long
  unbroken paths can still plausibly wrap poorly, increase card height, or
  push allocation despite CSS `max-width` hints.
- The Stream and Recording cards have different row counts, with card
  alignment depending on current minimum-height CSS rather than an explicit
  shared display model.
- Current tests cover helper strings, not GTK widget allocation, truncation,
  tooltip behavior, or theme-specific rendering.

Plan:

- Add a bounded recording-path display helper that keeps the raw path in the
  tooltip/copy button but shows a concise visible label, basename, or
  middle-ellipsized path in the card.
- Add widget-level constraints where GTK actually honors them, such as
  `ellipsize`, `max_width_chars`, consistent `halign`, or explicit row
  placeholders, instead of relying only on CSS `max-width`.
- Make the stream and recording card row structure intentionally consistent or
  document/test why their different row counts remain visually stable.
- Preserve current command-failure behavior: controller duplicate-operation
  guards remain authoritative, failures remain localized, and fallback statuses
  keep buttons out of synthetic pending states.
- Verify with a GTK render/manual check that long backend error details and
  long recording paths do not stretch, overlap, or destabilize the Live output
  area in narrow and normal window widths.

Files:

- `src/ui/pages/live.rs`
- `assets/scenedeck.css`

Tests:

- Keep existing helper tests for concise error copy.
- Add pure helper tests for the bounded recording-path visible copy, including
  empty paths, short paths, long slash-separated paths, and long unbroken
  filenames.
- Add manual or screenshot evidence for long tooltip/error-detail/path strings
  when a GTK control path is available.

### P1: Keep Output Recovery Helper Scope Narrow

Problem:

- `OutputCommandFailureRecovery` construction is now sealed, but
  `fallback_status_after_failed_output_command` remains `pub(crate)` for
  controller orchestration and event-level tests.
- The helper is an event/command contract rule, not a general reducer or UI
  utility. Future output work could accidentally reuse it in places that should
  apply an explicit event payload or a fresh OBS status reading instead.

Plan:

- Leave the helper in `controller::event` unless a new command orchestration
  module emerges.
- If more output command event types are added, consider wrapping fallback
  construction in a narrower event/command builder so only controller
  orchestration can call the raw helper.
- Do not call the helper from UI code or reducer methods; reducers should keep
  applying explicit carried payloads.
- Add an `rg`/test audit if new output recovery events are introduced.

Files:

- `src/controller/event.rs`
- `src/controller/app_controller.rs`
- `src/controller/state.rs`

Tests:

- Existing `failed_output_command` and `command_failure` focused tests remain
  the guardrail.
- Add focused tests only when new output command event construction paths are
  added.

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
