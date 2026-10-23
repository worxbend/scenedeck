# Research review

What this change does: This research proposes expanding SceneDeck's native OBS controls while keeping the current Rust/GTK architecture. The working tree was initially clean; the requested default change review therefore also checked the last commit, `003d9ba`, which adds a best-effort parent-directory sync after atomic file replacement.

The last-commit review found no concrete regression in the added sync block. It inspected the atomic writer, config and registry callers, serialized background writes and existing tests. All five atomic-writer tests passed when compiled with Rust 1.97.0 and edition 2021. The sync is intentionally best-effort, so this does not establish a guarantee against every filesystem or power-loss failure.

The finding below concerns **existing connected behavior discovered during research**, not a regression introduced by that commit. Expected load is one desktop operator connected to one OBS instance.

## Must fix before expanding output controls

1. **Disconnect bypasses output-stop confirmations** (`src/ui/sidebar.rs:257`, `src/controller/session_controller.rs:76`, `src/controller/output_controller.rs:295`).

   **What this is:** The button labeled Disconnect dispatches directly to the session controller. Before releasing the session, that controller reads stream and recording status and attempts to stop each active output.

   **Problem:** With streaming and recording active and both Stop confirmations enabled, clicking Disconnect can end both without a dialog. Normal stop confirmations are wired through the output-button path (`src/ui/pages/live.rs:271`, `:385`), which Disconnect bypasses.

   **Fix:** First decide the product contract. The smallest change preserving today's contract is an explicit, cancel-default “stop outputs and disconnect” confirmation that honors the existing stop preferences. If Disconnect should only detach, remove output stopping from that action and retain a separately named stop action; that is a behavior change requiring an agreed product decision.

   **If we skip it:** The operator can terminate a broadcast or recording through a connection-management action whose wording does not explain the output effects. Adding replay/virtual-camera controls before defining this contract makes the inconsistency broader.

The shutdown behavior is intentional and tested (`src/controller/app_controller.rs:608`, test `disconnect_stops_active_outputs_before_disconnected_event`). Developer documentation describes graceful output shutdown. No equivalent warning was found in the README, user guide or English Help; Help describes stop confirmations as the default (`i18n/en/scenedeck.ftl:717`). This is a user-facing safety gap, not an accidental discovery that the function calls stop.

## Lean review of the proposal

The proposed sequence is reasonable: first clarify Disconnect, improve Live empty states and add source visibility; follow with separate recording/replay slices, explicit Studio behavior, bounded snapshots and media/filter/audio details. Compact-shell and accessibility improvements are independently scoped. These constraints keep it small:

- Reuse one Live source inspector. Visibility uses scene-item identity; audio, filters and media use input/source identity. Do not merge those concepts just to share a widget.
- Deliver recording pause and replay as separate features. Reuse existing pending/error patterns without first building an all-purpose output framework.
- Implement Preview/Program/Take state before adding image refresh. Test that scene shortcuts cannot accidentally send a staged scene to Program.
- Start with filter enable/disable. Generic filter editors, presets, macros and a dashboard designer have no validated first-release need.
- Grow the current OBS fixture alongside each feature. Do not create a separate large testing platform or postpone all verification until the end.

Verdict: fix 1 before broadening output controls. The last storage commit has no actionable finding from this scoped review.

Not checked: full Cargo validation, power-loss simulation, rendered GTK/dialog behavior, live OBS or OBS Blade runtime behavior.
