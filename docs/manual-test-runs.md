# Manual Test Runs

Record manual validation against a real OBS instance here. Use blocked entries
when a prerequisite is unavailable; do not infer results from unit tests or
source inspection.

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
