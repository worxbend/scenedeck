# OBS operating workspace

SceneDeck is a native OBS WebSocket controller. Remote actions depend on the
requests advertised by OBS and the source/output configuration on that host.

## Live

The scene wall is the main switching surface. With previews off, scenes use
compact tiles. Turn **Scene previews** on in Settings for larger cards containing
shaded, rounded still images. Scene names and Program/Preview labels remain over
the image. The preference defaults to off conservatively. Unavailable images use the card fallback; they do
not disable otherwise usable scene controls.

The topbar **Studio** switch controls OBS Studio mode and follows external
changes. In direct mode, cards and their shortcuts switch Program. In Studio
mode they stage Preview. Transition selection, adjustable duration and **Take**
appear above the grid only in Studio mode. Fixed-duration transitions omit the
duration editor. Unknown or stale mode state disables scene actions until fresh
OBS state arrives.

The grid has one scrolling region. A compact audio strip sits below it; expand
it for larger controls, or use Mixer for detailed Active, Selected and Pinned
scene audio. Short windows collapse the strip to its heading until expanded,
and show scenes as tiles when there is no room for a preview card. The bottom output bar contains streaming, recording, replay buffer
and virtual camera. Its menu holds pause/resume, save replay, split/chapter
requests and output results. Controls remain in the same surface as the window
narrows. Start/stop confirmation preferences apply to streaming and recording.

Profiles and scene collections are available through **OBS setup** in the
header. The sidebar connection control and status bar report connection/output
state. Text entry retains normal typing behavior; scene shortcuts act on Live.

## Sources

Open **Sources** from the topbar. Choose a scene to inspect without switching
Program, then search its source items by input, owning container or nested path.
Each visibility action targets the owning scene/group and scene-item ID. Two
items referencing one input remain separate items.

A wide window shows the item list beside contextual details. Compact windows
open details with a Back action. The selected source remains an inspection
target when Program changes. Partial reads retain warnings; a structural usage
list does not prove that every appearance is currently visible or audible.
**Used in — structural paths** starts collapsed; expand it to inspect shared
paths. Long path buttons wrap, and the expanded choice survives detail refreshes
within the current connection and scene collection.

Input details expose supported controls:

- Existing filter enable/disable.
- Media play, pause, stop, restart, previous/next and seek.
- Built-in FreeType/GDI+ text with explicit Save and Cancel. A conflicting
  external edit is preserved. File-backed text stays an OBS-host file operation.
- Audio monitor mode, signed sync offset, balance and six-track routing.

Input settings affect that shared input wherever OBS uses it. Item visibility
is distinct from input settings, ancestor visibility and actual rendered pixels.
Source controls require current scoped observations. Selected-media polling runs
only while the selected media detail is visible on Sources.

## Inventory

Inventory uses the available page width and native draggable Scene/Role column
dividers. Role values wrap instead of disappearing into an ellipsis. **Column
widths** provides keyboard-accessible numeric widths and a reset action. Widths
survive refresh/restart; compact fitting does not replace the saved values.

Use the scene icon picker beside its name. The row options menu holds accent
color and keyboard move-up/move-down actions; the drag handle changes the same
persisted scene order. YAML import/export retains registry roles, order, icons,
accents and other supported registry metadata. These are local preferences, not
OBS scene mutations.

## Output evidence and Stats

The contextual output-results list distinguishes requested, acknowledged,
observed, failed and unconfirmed results. It holds at most 24 entries; pending
acknowledgements age into unconfirmed after 30 seconds or scope loss.
Acknowledgement does not prove a file was saved. OBS file events remain separate
evidence because OBS does not identify the requesting operator. Paths belong to
the **OBS host**; Copy copies text without assuming a local filesystem mapping.
Unsupported or configuration-rejected commands report a command result and do
not imply a disconnect. Unconfirmed requests are not automatically replayed.

Stats retains bounded sample history and session activity, with optional chart
markers for retained events. Markers provide timing context, not proof of cause.
Recording elapsed time is wall-clock time since OBS reported the start,
including pauses; it is not encoded-file duration. Disk space is reported for
the OBS recording volume. Activity and output evidence are session-only.

## Scope, freshness and local data

Workspace requests carry endpoint/collection scope, session epoch and request
sequence. The controller checks scope before acting and collection after reads;
GTK rejects obsolete responses. Relevant OBS events dirty the observations.
Native actions require a non-dirty observation within the 30-second freshness
window; an old intent is not automatically executed after refresh.

Scene stills run on an independent bounded capture path. Only visible Live
cards are eligible. Disabling previews, leaving Live, hiding the window or
losing the connection stops scheduling and cancels active capture work. Cached
images are scoped to the OBS session/collection and cleared on scope loss.
Canvas aspect is retained inside a 384 × 216 bound and a card matte. The
independent capture transaction reports a timeout after two seconds; a failed
scene backs off for 15 seconds from its prior attempt. An unanswered native
request pauses further image captures until it drains, while OBS controls remain
independent. Sources does not have a standalone still monitor.
An image failure does not invalidate control freshness. See
[OBS integration](obs-websocket.md) for current bounds.

The retired rehearsal, setup-expectation and cue interfaces have no replacement
page. Existing `workflows.json` files are left on disk and are not read or
written by routine application operation. Scene metadata stays in
`registry.json`; preferences stay in `config.json`; passwords stay in Secret
Service. See [configuration](configuration.md).

## Implementation map

| Responsibility | Files |
| --- | --- |
| OBS requests, source traversal and native screenshots | `src/obs/client.rs`, `src/obs/mapper.rs`, `src/obs/event.rs` |
| Scoped reads/actions and observations | `src/controller/workspace.rs`, `src/controller/state.rs` |
| Independent card capture and texture cache | `src/controller/thumbnails.rs`, `src/ui/previews.rs` |
| Scene wall, source details and outputs | `src/ui/pages/live.rs`, `src/ui/widgets/operating_panel.rs`, `src/ui/widgets/transport_bar.rs` |
| Studio state and transition controls | `src/ui/widgets/studio_controls.rs` |
| Output evidence | `src/domain/output_result.rs`, `src/ui/widgets/output_results_tray.rs` |
| Scoped activity and chart context | `src/domain/workflow.rs`, `src/ui/pages/stats.rs`, `src/ui/widgets/chart.rs` |

The retained `workflow` module name refers to shared scope/activity concepts;
it does not reintroduce the retired workflow features. This guide describes
implemented behavior, not a claim that every integration/visual check passed.
