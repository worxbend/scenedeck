# Manual Test Runs

Record manual validation against a real OBS instance here. Use blocked entries
when a prerequisite is unavailable; do not infer results from unit tests or
source inspection.

## Focused Mixer Refresh Contract Entry Template

Use this structure for both blocked and passed focused Mixer runs. Mark a case
as passed only when the listed interaction was actually exercised against OBS
and SceneDeck.

Status: Passed, Failed, or Blocked.

Scope: focused Mixer interaction run for ComboRow mode and scene changes, Retry
after selected/pinned refresh failures, OBS mute and volume echo reconciliation,
stale visible cards, and perceived rebuild churn.

Environment:

- SceneDeck build: version and git commit.
- Host: OS and desktop/session type.
- Run context: interactive desktop, UI automation, or non-interactive session.
- OBS version: exact version and how it was collected.
- obs-websocket version: exact version and how it was collected.
- OBS WebSocket: host, port, reachability, and authentication state without
  recording secrets.
- OBS inventory: scene names/count, global audio inputs, and the differing
  scene-specific audio inputs used for the run.

Prerequisite result:

- OBS WebSocket reachable: pass/fail.
- At least two scenes: pass/fail.
- Global audio inputs available: pass/fail.
- Differing scene-specific audio input available between two scenes:
  pass/fail.
- SceneDeck GTK ComboRows and visible Mixer cards inspectable: pass/fail.
- Non-destructive selected/pinned refresh failure setup available: pass/fail or
  skipped with reason.

Executed observations:

- ComboRow Active mode changes and scene following: pass/fail with notes.
- ComboRow Selected mode fallback and explicit scene changes: pass/fail with
  notes.
- ComboRow Pinned mode explicit target and fallback order: pass/fail with
  notes.
- Retry after failed selected/pinned refresh: pass/fail with notes.
- OBS mute echoes updating visible Mixer cards: pass/fail with notes.
- OBS volume echoes updating visible Mixer cards: pass/fail with notes.
- Stale visible cards after OBS echoes: pass/fail with notes.
- Perceived rebuild churn during repeated volume echoes: pass/fail with notes.

Skipped cases:

- List each skipped case and the exact prerequisite or safety reason.

Non-claims:

- State every behavior that was not exercised. For blocked runs, explicitly say
  no pass/fail behavior is claimed for unexecuted ComboRow changes, Retry,
  mute echoes, volume echoes, stale cards, or rebuild churn.

## Focused Mixer Inspection Run Template

Use this structure for focused Mixer runs that capture the opt-in structured
inspection output. Mark every case `pass`, `fail`, `blocked`, or `skipped`,
and include an explicit reason in the row. Do not convert inspection output
into claims about pointer interaction success, visual layout quality, or
perceived rebuild churn unless those were also observed in an interactive
session.

Status: Passed, Failed, Blocked, or Mixed.

Scope: focused Mixer render-state inspection using the debug inspection path,
optionally paired with interactive OBS and GTK observations.

Inspection contract:

- `SCENEDECK_MIXER_INSPECT=1` lines describe the rendered Mixer branch that
  appended placeholders or visible cards, not merely the raw pre-render reducer
  state.
- When a scene-specific missing state dispatches an automatic refresh and the
  page shows the loading/requested placeholder, record the inspection status as
  loading/requested. Do not treat that branch as a passing `missing` result.
- Loaded scenes with no audio sources and loaded scenes with audio sources
  hidden by search are separate rendered states. Record `No Audio Sources` and
  `No Matching Audio Sources` evidence separately when those cases are
  exercised.
- Structured `volume_label` evidence is valid visible-card evidence only
  because inspection output shares the same dB formatter used by rendered Mixer
  audio cards. Do not accept labels produced by a duplicated inspection-only
  formatter as proof of the visible card label.
- Structured inspection can support rendered state and card-data claims. It
  still does not prove pointer interaction success, visual layout quality, or
  perceived rebuild churn without an interactive observation or equivalent
  instrumentation.

Environment:

- SceneDeck commit: `TODO`.
- OBS version: `TODO`.
- obs-websocket version: `TODO`.
- WebSocket URL/auth mode: `TODO`; do not record secrets.
- Desktop/session type: `TODO`.
- Fixture scene names: `TODO`.
- Global input names: `TODO`.
- Scene-specific input names: `TODO`; identify which fixture scene owns each
  scene-specific input.
- Inspection method: `SCENEDECK_MIXER_INSPECT=1` structured output,
  interactive desktop observation, UI automation, or a combination.
- Captured structured inspection lines: paste the relevant lines or reference
  the attached log artifact.

Fixture notes:

- Use a throwaway OBS profile or clearly temporary `SceneDeck Test ...` scenes.
- Keep at least one global audio input visible in both fixture scenes.
- Keep at least one scene-specific audio input present in only one fixture
  scene.

Per-case results:

| Case | Result | Explicit reason and evidence |
| --- | --- | --- |
| Active mode following | TODO: pass/fail/blocked/skipped | TODO: cite the inspection line showing mode, visible source, and visible input names, or explain why unavailable. |
| No Active scene-specific refresh target | TODO: pass/fail/blocked/skipped | TODO: cite the inspection line showing no scene-specific refresh target in Active mode, or explain why unavailable. |
| Selected direct | TODO: pass/fail/blocked/skipped | TODO: cite selected scene, refresh target/reason, render source/status, and visible card input names. |
| Selected fallback | TODO: pass/fail/blocked/skipped | TODO: cite fallback reason, effective refresh target, render source/status, and visible card input names. |
| Pinned direct | TODO: pass/fail/blocked/skipped | TODO: cite pinned scene, refresh target/reason, render source/status, and visible card input names. |
| Pinned fallback | TODO: pass/fail/blocked/skipped | TODO: cite fallback reason, effective refresh target, render source/status, and visible card input names. |
| Missing to automatic loading/requested placeholder | TODO: pass/fail/blocked/skipped | TODO: cite the inspection line from the rendered branch showing loading/requested placeholder state after automatic refresh dispatch, or explain why unavailable. |
| Retry after failure | TODO: pass/fail/blocked/skipped | TODO: cite error/loading/Retry visible and enabled state before and after retry, or explain why failure setup was unavailable. |
| Loaded with no audio sources | TODO: pass/fail/blocked/skipped | TODO: cite the rendered status for `No Audio Sources` and the empty visible card list, or explain why no empty fixture scene was available. |
| Loaded with no matching audio sources | TODO: pass/fail/blocked/skipped | TODO: cite the rendered filtered-empty status for `No Matching Audio Sources`, the search query/filter used, and the empty visible card list. |
| OBS mute echo | TODO: pass/fail/blocked/skipped | TODO: cite before/after inspection lines showing the visible card mute state changed for the expected input. |
| OBS volume echo | TODO: pass/fail/blocked/skipped | TODO: cite before/after inspection lines showing the visible card volume value/label changed for the expected input. |
| Stale visible cards | TODO: pass/fail/blocked/skipped | TODO: cite inspection lines proving cards matched the current render source after scene or OBS input changes, or describe the stale mismatch. |
| Rebuild churn | TODO: pass/fail/blocked/skipped | TODO: cite interactive observation or instrumentation; inspection lines alone do not prove perceived churn. |

Inspection evidence captured:

- Mode, selected scene, and pinned scene: `TODO`.
- Scene-specific refresh target and fallback reason: `TODO`.
- Render source kind and rendered status: loading/requested placeholder, error
  placeholder, missing/no target, loaded cards, loaded with no audio sources,
  or loaded with no matching audio sources after filtering: `TODO`.
- Visible card input names: `TODO`.
- Mute states: `TODO`.
- Volume values and labels: `TODO`; for labels, note that the build shares the
  rendered audio-card dB formatter.
- Retry visible/enabled state: `TODO`.

Non-claims:

- List any behavior not exercised. If this run used only structured inspection
  output, explicitly state that it does not prove pointer interaction success,
  visual layout quality, or perceived rebuild churn.

## 2026-06-21 - Focused Mixer Refresh Contract (iteration 10)

Status: Blocked.

Scope: focused Mixer interaction run for ComboRow mode changes, Retry behavior
after selected/pinned refresh failures, OBS mute and volume echo reconciliation,
stale Mixer cards, and visible rebuild churn.

Environment:

- SceneDeck build: `0.1.3`, git commit `a687f9f`.
- Host: Linux `ubuntu` 7.0.0-22-generic x86_64.
- Run context: non-interactive Codex session in
  `/home/worxbend/Workspace/AI/scenedeck`.
- SceneDeck launch check: `cargo run --bin scenedeck` started successfully and
  was stopped through the GTK `quit` action.
- OBS process: `pgrep -a obs` reported process `396269 obs`.
- OBS CLI version: unavailable; `obs --version` was not in `PATH`.
- OBS WebSocket: reachable at configured `127.0.0.1:4455`.
- OBS WebSocket credential state: authentication disabled for the local
  WebSocket endpoint, so no password was required.
- OBS version from WebSocket `GetVersion`: OBS `32.1.2`, obs-websocket
  `5.7.3`.
- OBS inventory from WebSocket: two scenes (`Scene 2`, `Scene`) and two audio
  inputs (`Desktop Audio`, `Mic/Aux`).

Blocking prerequisites:

- The focused plan requires at least one scene-specific audio input that differs
  between the two scenes. WebSocket inspection showed `Scene 2` had no scene
  items and `Scene` only nested `Scene 2`, so the required differing
  scene-specific audio setup was not verified.
- This non-interactive run could not safely drive or inspect the GTK ComboRow
  interactions. The app exposed only application-level GTK actions
  (`settings`, `quit`, `about`, `reconnect`), no `xdotool`/`ydotool` helper was
  available, and the GNOME Shell screenshot DBus call was denied.
- The selected/pinned refresh failure case was not forced because deleting or
  mutating OBS scenes would be destructive to the live OBS setup.

Results:

- Pass/fail: none recorded. The focused interaction cases were not executed.
- Skipped cases: Active mode scene following and no scene-specific refresh
  dispatch observation; Selected fallback observation; Pinned fallback order
  observation; failed selected/pinned refresh Retry; OBS mute echoes; OBS
  volume echoes; stale Mixer card checks; visible rebuild churn checks.

Observations:

- ComboRow mode changes: blocked, not executed.
- Retry behavior after failed selected/pinned refreshes: blocked, not executed.
- OBS mute echoes updating visible Mixer cards: blocked, not executed.
- OBS volume echoes updating visible Mixer cards: blocked, not executed.
- Stale Mixer cards after OBS echoes: blocked, not executed.
- Visible rebuild churn under repeated volume echoes: blocked, not executed.

Non-claims:

- This run does not claim pass/fail behavior for ComboRow mode changes,
  ComboRow scene changes, Retry after failed selected/pinned refresh, OBS mute
  echoes, OBS volume echoes, stale visible cards, or perceived rebuild churn.
- The reachable WebSocket/version/inventory checks only describe environment
  readiness; they are not evidence that the Mixer UI interactions passed.

Optimization gate:

- No Mixer input-event optimization was applied from this run. The required
  repeated mute and volume echo cases were not executed, so there is no manual
  evidence that the current full-page Mixer rebuild creates noticeable churn.
  The existing rebuild path remains the accepted behavior until a verified OBS
  interaction run shows runtime cost that justifies in-place visible-card
  bookkeeping.

## 2026-06-21 - Focused Mixer Refresh Contract

Status: Blocked.

Scope: focused Mixer interaction run for Active, Selected, and Pinned refresh
target behavior, Retry after selected/pinned refresh failure, and OBS mute and
volume echo reconciliation.

Environment:

- SceneDeck build: `0.1.3`, git commit `73bb5bc`.
- Host: Linux `ubuntu` 7.0.0-22-generic x86_64.
- Run context: non-interactive Codex session in
  `/home/worxbend/Workspace/AI/scenedeck`.
- OBS process: `pgrep -a obs` reported process `396269 obs`.
- OBS version: not recorded; `obs --version` produced no output in this
  session.
- obs-websocket version: not recorded because WebSocket access was not
  verified in this run.

Blocking prerequisite:

- A verified real OBS WebSocket session with known credentials, at least two
  configured scenes, and multiple audio inputs was not available to this
  non-interactive run. Because the scene inventory, audio inputs, and
  WebSocket access could not be verified, the interaction cases were not
  executed and no pass/fail behavior is claimed.

Observations:

- Active mode follows current scene without scene-specific refresh dispatches:
  blocked, not executed.
- Selected mode falls back to current scene when no selected scene is
  configured: blocked, not executed.
- Pinned mode falls back from pinned to selected to current scene: blocked, not
  executed.
- Failed selected or pinned scene refresh can be retried with Retry: blocked,
  not executed.
- Mute-change OBS echoes update the visible Mixer card: blocked, not executed.
- Volume-change OBS echoes update the visible Mixer card: blocked, not
  executed.
- Volume echo frequency does or does not create noticeable full-page rebuild
  churn: blocked, not executed.

Skipped cases:

- ComboRow mode changes and scene changes: skipped because GTK interaction was
  unavailable in the non-interactive session.
- Retry after failed selected/pinned refresh: skipped because a verified
  WebSocket session and non-destructive failure setup were unavailable.
- OBS mute echoes, OBS volume echoes, stale visible card checks, and rebuild
  churn observation: skipped because OBS scene/audio prerequisites were not
  verified and the Mixer UI could not be inspected.

Non-claims:

- This run does not claim pass/fail behavior for ComboRow mode changes,
  ComboRow scene changes, Selected or Pinned fallback behavior, Retry after
  selected/pinned refresh failure, OBS mute echoes, OBS volume echoes, stale
  visible cards, or perceived rebuild churn.

## 2026-06-21 - Focused Mixer Refresh Contract (iteration 12)

Status: Blocked.

Scope: focused Mixer interaction run for ComboRow mode and scene changes, Retry
after selected/pinned refresh failures, OBS mute and volume echo reconciliation,
stale visible cards, and perceived rebuild churn.

Environment:

- SceneDeck build: `0.1.3`, git commit `95806c4`.
- SceneDeck launch check: `timeout 12s cargo run --bin scenedeck` compiled and
  launched `target/debug/scenedeck`; the non-interactive run stopped it by
  timeout because the GTK window could not be driven.
- Host: Linux `ubuntu` 7.0.0-22-generic x86_64.
- Desktop/session: GNOME on Wayland (`XDG_SESSION_TYPE=wayland`,
  `WAYLAND_DISPLAY=wayland-0`, `DISPLAY=:0`,
  `XDG_CURRENT_DESKTOP=ubuntu:GNOME`).
- Run context: non-interactive Codex session in
  `/home/worxbend/Workspace/AI/scenedeck`.
- OBS process: `pgrep -a obs` reported process `396269 obs`.
- OBS CLI version: unavailable; `obs` was not in `PATH`.
- OBS version from WebSocket `GetVersion`: OBS `32.1.2`.
- obs-websocket version from WebSocket `GetVersion`: `5.7.3`.
- OBS WebSocket: reachable at `127.0.0.1:4455`; WebSocket `Hello` contained no
  authentication challenge, so local authentication was disabled and no secret
  was required or recorded.
- OBS inventory from read-only WebSocket requests: two scenes, `Scene 2` and
  `Scene`; current program scene `Scene`; global audio inputs `Desktop Audio`
  and `Mic/Aux`.
- Scene item inventory: `Scene 2` had no scene items; `Scene` contained nested
  scene source `Scene 2`. No scene-specific audio input was present in only one
  test scene.

Prerequisite result:

- OBS WebSocket reachable: pass.
- At least two scenes: pass.
- Global audio inputs available: pass.
- Differing scene-specific audio input available between two scenes: fail.
  The required fixture was not present; only global audio inputs were observed.
- SceneDeck GTK ComboRows and visible Mixer cards inspectable: fail.
  `xdotool`, `ydotool`, `dogtail-run`, `sniff`, `grim`, and `gnome-screenshot`
  were unavailable in this session; `gdbus` was present, but the GNOME Shell
  screenshot DBus call was denied with `org.freedesktop.DBus.Error.AccessDenied`.
  No committed SceneDeck harness exposed selectors for GTK ComboRows, Retry, or
  Mixer card readback.
- Non-destructive selected/pinned refresh failure setup available: skipped.
  No temporary `SceneDeck Test ...` fixture existed, and mutating the user's
  existing OBS scenes would be destructive for the default run.

Executed observations:

- ComboRow Active mode changes and scene following: blocked, not executed
  because GTK ComboRows could not be selected or inspected and the fixture did
  not include differing scene-specific audio.
- Active mode no scene-specific refresh dispatch observation: blocked, not
  executed because UI mode selection and dispatch observation were unavailable.
- ComboRow Selected mode fallback and explicit scene changes: blocked, not
  executed because GTK ComboRows and visible Mixer cards could not be inspected.
- ComboRow Pinned mode explicit target and fallback order: blocked, not
  executed because GTK ComboRows and visible Mixer cards could not be inspected.
- Retry after failed selected/pinned refresh: blocked, not executed because a
  non-destructive temporary failure fixture was unavailable and the Retry button
  could not be clicked.
- OBS mute echoes updating visible Mixer cards: blocked, not executed because
  visible Mixer cards could not be inspected.
- OBS volume echoes updating visible Mixer cards: blocked, not executed because
  visible Mixer cards could not be inspected.
- Stale visible cards after OBS echoes: blocked, not executed because mute and
  volume echo cases were not exercised and the UI could not be inspected.
- Perceived rebuild churn during repeated volume echoes: blocked, not executed
  because repeated visible volume echoes could not be observed.

Skipped cases:

- Active mode following live active-scene audio without dispatching
  scene-specific refreshes: skipped because Active mode could not be selected
  or inspected through GTK automation in this non-interactive Wayland session.
- Selected mode documented fallback behavior: skipped because the mode and
  scene ComboRows could not be driven and visible Mixer cards could not be
  inspected.
- Pinned mode documented fallback behavior: skipped because the mode and scene
  ComboRows could not be driven and visible Mixer cards could not be inspected.
- Retry after selected/pinned refresh failure: skipped because the failure setup
  would require mutating non-temporary OBS scenes and the Retry button could not
  be clicked.
- OBS mute echoes, OBS volume echoes, stale-card checks, and rebuild-churn
  observation: skipped because the Mixer UI was not inspectable and the
  required differing scene-specific audio fixture was absent.

Non-claims:

- This run does not claim pass/fail behavior for Active mode scene following,
  absence of Active-mode scene-specific refresh dispatches, Selected fallback,
  Pinned fallback, Retry after failed selected/pinned refresh, OBS mute echoes,
  OBS volume echoes, stale visible cards, or perceived rebuild churn.
- The reachable WebSocket/version/inventory checks only prove the OBS endpoint
  and partial fixture readiness. They are not manual evidence that the focused
  Mixer interaction contract passed.

Optimization gate:

- No runtime rebuild-churn issue was observed because repeated volume echoes
  were not exercised against an inspectable Mixer UI. This blocked run provides
  no evidence for changing the current full-page Mixer rebuild behavior.
