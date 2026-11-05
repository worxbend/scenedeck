# OBS WebSocket Integration

SceneDeck uses the `obws` crate to communicate with OBS Studio. The wrapper in
`src/obs/client.rs` is the only place that should call `obws` directly.

## Connection

SceneDeck connects with:

- Host from config, default `127.0.0.1`.
- Port from config, default `4455`.
- Optional password from the system keyring.

After connecting, SceneDeck reads OBS version information, including
`availableRequests` and supported image formats, and starts an OBS event stream.
The operating UI gates individual writes by advertised request availability;
OBS can still reject a request for the current source, output or recording format.
See the [operating workspace guide](obs-operating-workspace.md) for the user flow.

The event subscription is `EventSubscription::ALL | INPUT_VOLUME_METERS`. `ALL`
covers every non-high-volume category; `INPUT_VOLUME_METERS` has to be requested
explicitly because obs-websocket classes it as high volume, and it is the only
source of the mixer volume meters.

## Reads

SceneDeck currently reads:

- OBS version, WebSocket version and advertised request/image-format capabilities.
- Scene inventory and current program scene.
- Scene item lists for dependency graph construction.
- Scene item lists for active scene audio discovery.
- Group item lists when active scene audio discovery enters groups.
- Scene item enabled state.
- OBS profiles and current profile.
- OBS scene collections and current scene collection.
- Stream status, including congestion, dropped frames, and sent bytes.
- Record status.
- OBS process statistics: CPU, memory, active FPS, average frame render time,
  and render/output frame counters.
- Special/global audio input names.
- Input mute state.
- Input volume state; failed source properties remain unknown.
- Studio Mode, Preview/Program and current/listed transitions.
- Replay-buffer and virtual-camera status and last replay path.
- Existing source filter list and enable state.
- Selected source media status, cursor and duration where supported.
- Built-in text-source content/settings and file-backed text mode.
- Audio monitoring, sync, balance and track routing.
- Source usage across scene/group paths and input UUID identity.
- OBS recording-volume free space.
- Bounded scene-card screenshots when previews are enabled; these are still
  images, not video.

Independent graph, scene-item enabled-state, and audio-state reads run with a
bounded concurrency limit of eight. Results retain OBS input order. Optional
thumbnail transactions use one additional single-flight lane on the same socket;
its unanswered request does not consume an ordinary slot or inherit the ordinary
ten-second response deadline. Explicit workspace stills retain that deadline.

Disconnect closes ordinary admission without cancelling already admitted reads,
drains them, then uses a private shutdown handle to stop and reread outputs.
Drainage, stop and confirmation share a three-second best-effort budget. If they
cannot establish inactive outputs, the terminal status says shutdown is
unconfirmed; uncertain mutations are never automatically replayed.

## Writes

SceneDeck currently writes:

- Current program scene.
- Current OBS profile.
- Current scene collection.
- Input mute state.
- Input volume multiplier.
- Start or stop streaming.
- Start or stop recording, pause/resume, split and chapter requests.
- Studio Mode, current Preview, transition selection/duration and Take.
- Owned scene-item enable state.
- Replay-buffer start/stop/save and virtual-camera start/stop.
- Existing filter enable state.
- Selected media play/pause/stop/restart/previous/next and seek.
- Built-in text content using overlay settings, with a pre-save conflict read.
- Audio monitor mode, sync offset, balance and six-track routing.

Scene roles are local metadata and do not write to OBS. Inventory writes the
local registry directly.

Audio preset Apply uses `GetSceneCollectionList`, `GetInputList`, `GetInputMute`
and `GetInputVolume` to revalidate the reviewed scope, identity and values. Its
directional `SetInputMute` and `SetInputVolume` steps target UUIDs when available.
Presets preserve linear gain in the protocol range 0–20, including zero, instead
of clamping captured values to the fader's range. Each changed property is read
back before proceeding; gain must be confirmed before unmuting. The sequential
worker reports per-input outcomes, skips missing or changed targets, and fences
remaining mutations on cancellation, scope changes or a retired socket. A partial
failure does not imply rollback, and an acknowledgement is not confirmed readback.

## Events

`SceneNameChanged` carries the old and new scene names through the scoped event
channel. It invalidates scene controls and derived topology, then migrates local
registry metadata and order without overwriting a conflicting destination.
Registry imports defer this migration until their publication completes; retired
session/collection callbacks cannot modify the replacement scope.

SceneDeck currently handles these OBS event categories:

- Stream state changes.
- Record state changes.
- Record file changes.
- Current program scene changes.
- Input mute changes.
- Input volume changes.
- Input volume meters (high volume, roughly every 50 ms).
- Input created, removed, or renamed.
- Scene item created, removed, reindexed, or enabled/disabled.
- Current profile changed.
- Profile list changed.
- Current scene collection changed.
- Scene collection list changed.
- Scene list changed.
- Scene name changed, including the stable scene UUID.
- Replay saves and replay/virtual-camera state changes.
- Studio Mode, Preview and transition changes.
- Source-filter creation/removal/reordering/name/settings/enable changes.
- Input settings and selected media changes that invalidate operating state.

Events either update UI state directly or trigger a refresh of derived data such
as active scene audio, profile lists, collection lists, scene inventory, or the
dependency graph.

`InputVolumeMeters` is the one exception to "every event is delivered". It
carries `inputLevelsMul`, three linear multipliers per channel — magnitude, peak,
and pre-fader input peak — for every active input, about twenty times a second.
`src/controller/refresh_controller.rs` converts it to `domain::meter::InputLevels`
and offers it to the UI channel with `try_send`, dropping the reading when the
channel is full rather than blocking the events behind it. Nothing but the
meters depends on it, and the next reading is 50 ms away.

## Statistics Polling

obs-websocket v5 has no push event for performance statistics. `GetStats` and
`GetStreamStatus` are request-only, so a live view has to poll them.

SceneDeck polls both once per second from `refresh_controller::run_stats_poll_loop`.
That loop is raced against the OBS event stream inside the session task, so it:

- starts as soon as the session is established,
- runs regardless of which page is open, or whether any page is open at all,
- stops when the session task is aborted on disconnect or reconnect.

Each poll emits one `AppEvent::StatsUpdated`. The GTK side appends it to
`AppState::stats_history` — a bounded ring buffer of the last two minutes — which
backs both the permanent status-bar counters and the Stats page charts. Deriving
the rolling bitrate needs two consecutive byte-counter reads, so bitrate stays
empty for the first sample after the stream (re)starts.

## Active Scene Audio Discovery

For the active scene, SceneDeck returns audio cards in this order:

1. Global OBS audio sources from the special input list.
2. Enabled input items from the active scene.
3. Enabled input items inside nested scenes and groups.

The scan avoids duplicates. Sources that do not expose OBS mute and volume state
are skipped.

If `live.audio_inputs` in config is non-empty, it limits the discovered scene
inputs to that configured name list. Global sources are still listed first.

## Scoped operating reads and actions

`controller::workspace` owns cancellable aggregate workspace work, separate
from the continuous OBS session/event task and normal output controller.
Requests carry endpoint/collection scope, an epoch and a sequence number. The
controller checks endpoint/collection before acting and collection again after
reading. GTK applies only the current epoch/sequence. Disconnect or collection
scope changes cancel work and invalidate old selections/results.

A workspace read combines the control snapshot, audio and output observations.
Sources requests additionally include scene items, selected-source details and
usage. Live does not routinely scan selected-source details. Read age starts
with the first request. Relevant events dirty the snapshot; mutations require
fresh state and perform an authoritative reread. This is not an atomic
multi-request snapshot against another OBS operator.

Selected-media detail has a bounded refresh while that detail is visible on
Sources. The progress readout uses the existing GetMediaInputStatus cursor and
duration fields, without extrapolation or additional requests. Missing/invalid
cursor or non-positive/unknown duration never implies completed playback. Traversal retains nested paths, handles groups/cycles and limits a scan
to 4,096 items, reporting partial results on limits or failed reads. Native
workspace actions require a non-dirty observation within 30 seconds; stale
actions trigger refresh instead of replaying the old intent automatically.
Scene activation retains the mode observed when accepted, rereads Studio mode,
and rejects changed or unavailable mode instead of rerouting the old click.
A mode-change revision is checked again at final mutation admission, including
when OBS changes mode and back while the click is queued. Graph scans likewise
carry invalidation/scan revisions so superseded completions cannot publish Ready.
Transport ingress advances these fences independently of GTK queue capacity;
graph completion also rechecks freshness on each final publication attempt.
These are client-side freshness fences, not a server-side atomic compare-and-set.

## Scene-card thumbnails

Settings controls `live.scene_previews`, defaulting to `false` conservatively.
`GetSourceScreenshot` runs through a separate `ThumbnailController`; it does not acquire workspace mutation ownership.
The current scheduler selects visible Live cards, favors Program/Preview for
equal-age initial requests, and refreshes eligible scenes fairly.

Current implementation bounds:

- One underlying capture transaction per active connection. A two-second UI
  deadline reports a timeout, while an unanswered request drains silently;
  retries cannot accumulate protocol request IDs. Controls use the same socket
  independently and are never reset to recover a stalled image.
- At least 500 ms between capture starts, and at least three seconds before
  repeating one successful scene. Failed/timed-out scenes wait at least
  15 seconds from their previous attempt independently. Ordinary image errors
  do not delay healthy scenes. An unanswered transaction holds the single image
  worker until its response or connection retirement; controls remain usable.
  Dense grids therefore refresh each scene less often.
- PNG requests fit the OBS base canvas aspect within 384 × 216 without
  upscaling. The card contains the resulting image with a matte where needed.
  Encoded PNG data is bounded to 512 KiB and its header/dimensions are validated
  before texture decoding.
- At most 24 cached textures. Hidden/disabled/disconnected work is cancelled:
  delivery and subsequent requests are suppressed, including a screenshot after
  delayed canvas settings. An already sent request may drain, and obsolete
  scope/epoch/generation responses are rejected.
- Capture completion uses nonblocking delivery on the bounded event channel;
  an overloaded channel can drop a still without delaying command results.

A successful capture performs four native requests in order:
`GetSceneCollectionList`, `GetVideoSettings`, `GetSourceScreenshot`, then
`GetSceneCollectionList` again. Canvas settings determine aspect-preserving
thumbnail dimensions; collection checks reject scope changes. At two capture
starts per second this is up to eight native requests per second, excluding
normal session polling. Failed/aborted transactions can end earlier. The
separate 960 × 540 still adapter remains internal; Sources has no standalone
preview monitor or manual capture action.

Images update existing card widgets. Image failures leave honest fallbacks and
do not invalidate otherwise fresh OBS controls. None of these limits establish
real input latency or OBS render cost. Record six-/twenty-scene capture p50/p95,
bytes/second, pending/cache bounds, interaction stalls and OBS render-time delta
in [manual test runs](manual-test-runs.md).

## Output evidence

Replay save, recording stop/split and chapter requests use a bounded local
result model with requested/acknowledged/observed/failed/unconfirmed states.
Acknowledgement and observed file evidence remain separate. OBS events do not
carry a SceneDeck requester identity, so matching paths or timestamps are not
used to attribute a save to a particular request. Unconfirmed operations are
not automatically retried. Paths shown by the tray are OBS-host paths; the
available action copies text, without assuming a local filesystem mapping.

## Remaining native API surface

Input creation/removal/rename, generic source/settings or filter editing,
scene transforms/reordering, registered OBS hotkey browsing and full snapshot
restoration are not part of the operating UI. Native extra canvases also remain
outside the current single-workspace model. Plugin/vendor requests and chat
service integrations are excluded.

Add protocol calls through `src/obs/client.rs`, map responses into domain
values and route work through controller commands/events. Local scene metadata
and appearance remain config/registry data. Retired workflow files are preserved
on disk without routine loading or saving.

Scene inventories retain non-nil scene UUIDs for workspace curation reconciliation.
Names remain the display and control targets. Reconciliation is local and never
renames or reorders scenes in OBS. Unknown UUIDs do not establish identity.

Explicit palette/shortcut mutations carry a known scene or input UUID through the
workspace request to the protocol adapter. The adapter uses `sceneUuid`,
`inputUuid` or `sourceUuid` instead of a cached name. Studio-mode activation still
checks the authoritative mode immediately before dispatch. The complete audio
inventory now retains UUIDs from its existing `GetInputList` response and reads
identified input values by UUID without an additional inventory request.

The workspace retains the complete input identity catalog, including non-audio
sources. Enabled action bindings, Live pins and temporary action pickers request
only their required `GetMediaInputStatus` and `GetSourceFilterList` properties. Reads merge by
identity and run with bounded concurrency; property errors remain independent.
They share the existing workspace sequence, session, collection and freshness
guards. A missing UUID cannot fall back to its old name, and a failed input-list
read cannot authorize an action from cached property data. Picker inspections
release when their dialog closes or scope changes without changing Sources
selection. No action is deferred until a read succeeds.

Mute and filter toggle intentions resolve to an explicit `SetInputMute` or
`SetSourceFilterEnabled` value using current observed state. Live pins recheck
their displayed state before admission. They share the same identity, freshness,
pending-operation and controller-scope guards as explicit setters.

Preflight reuses the workspace's existing `GetSceneList` membership, complete
`GetInputList` identities, and input mute reads. `GetStats` supplies free recording
space in MiB; volume-meter events supply post-fader signal. Stats carry monotonic
request-start timestamps, and meter updates carry event-reader receipt timestamps,
so time spent waiting for GTK does not make old observations appear fresh. The
preflight refresh requests only workspace and statistics reads. No additional
scene-list call, scene switch or output mutation is introduced by a review.

Core performance/output health monitoring adds no OBS mutation or new polling request. It consumes
existing GetStats/GetStreamStatus observations and normalized output events after
the controller's scope/revision fences. Counter resets and gaps restart interval
baselines. A stopped-output warning is distinct from recording/replay saved-file
evidence and never supplies a file path or promotes an acknowledgement to a saved
result.


Opt-in audio health rules reuse `GetInputList`, existing input mute reads and
`InputVolumeMeters` subscriptions. Enabled rules can trigger the existing
read-only workspace refresh independently of the current page or main-window
focus, with ten seconds between monitor refresh attempts. The complete catalog
resolves saved input identities; the dependency exposes meter readings by name,
so no meter is used without fresh, unambiguous identity and mute evidence. Invalid
multipliers retain an invalid flag for checks rather than becoming digital silence.
Missing or frozen meter events never establish silence or recovery. Audio health
rules introduce no OBS mutation and never infer stream/recording completion.


Output observations preserve transport-receipt times or the start of the specific
`GetStreamStatus`/`GetRecordStatus` request. This includes workspace reads and failed
readbacks. These times fence old observations after queue/GTK delays and remain
connection-owned across collection switches. Pending output-command publication
also belongs to the connection, independently of collection-specific snapshots.
A reported recording path is separate evidence from whether recording is currently
running. The StopRecord response publishes a file result only, with the command
admission time used to order the path conservatively. It never synthesizes an
inactive state: a delayed response cannot undo a newer observed restart. Current
run state comes from GetRecordStatus or RecordStateChanged.


Bookmarks retain the existing GetRecordStatus `outputDuration` response as optional
milliseconds alongside the run state. No extra OBS request or mutation is issued
when adding, renaming or removing a local bookmark. A timing value is a sampled
output clock, not a saved-file claim or an encoded-file offset. RecordStateChanged
has no duration, so that event cannot carry an earlier recording's clock forward.
Missing/failed reads and command fallback states likewise supply no timing evidence.
