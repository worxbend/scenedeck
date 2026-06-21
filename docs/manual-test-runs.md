# Manual Test Runs

Record manual validation against a real OBS instance here. Use blocked entries
when a prerequisite is unavailable; do not infer results from unit tests or
source inspection.

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
