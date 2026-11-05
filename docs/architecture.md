# Architecture Overview

SceneDeck separates GTK UI code, command orchestration, OBS WebSocket access,
domain models, services, and local storage.

## Runtime Model

`src/app.rs` creates a Tokio runtime before the GTK application starts. The
runtime handle is passed into the controller so OBS work can run asynchronously
while GTK remains responsive.

The runtime outlives the GTK main loop:

```text
main()
  -> logging::init()
  -> scenedeck::run()
     -> create Tokio runtime
     -> create adw::Application
     -> build window on activate
     -> app.run()
     -> drop runtime
```

## State Ownership

`AppState` lives on the GTK thread in `Rc<RefCell<AppState>>`. It is the UI
source of truth for:

- Current page.
- Theme mode.
- OBS connection status.
- Scene inventory and graph.
- OBS profiles and scene collections.
- Stream and record status.
- Active audio inputs.
- Scoped operating observations, selected source details and freshness.
- Session activity, bounded output evidence and session-scoped scene thumbnails.
- One-shot startup notice text.

GTK widgets are updated from events in `src/ui/window.rs`. Doctor diagnostics
are derived from the current inventory, registry snapshot, and graph whenever
the Doctor page is rebuilt. Doctor contains structural diagnostics; retired
setup-expectation checks are not part of the current page. See
[OBS operating workspace](obs-operating-workspace.md).

## Command Flow

User actions dispatch executable `AppCommand` values through
`NavigationContext`. Commands that are purely local UI decisions are not kept
as placeholder controller variants.

```text
GTK signal
  -> NavigationContext::dispatch(AppCommand)
  -> AppController::handle(command)
  -> optional Tokio task
  -> ObsClient request
  -> AppEvent sent through std::sync::mpsc
  -> GTK polling timer applies event
```

Live scene shortcuts join this same path rather than bypassing it. A
window-level `EventControllerKey` runs in the capture phase, so bare digits and
the leader key are seen before a focused widget consumes them. It translates the
GDK key event into a `domain::hotkey::KeyStroke`, asks
`services::hotkey_service::SceneHotkeyResolver` what the stroke means, and
dispatches the resulting `AppCommand::SwitchPrimaryScene`. The resolver holds
the leader-armed state and takes the current `Instant` as an argument, which
keeps every timing rule testable without GTK or a real clock. Configuration is
read from `AppState` per key press, so a change in Settings applies to the very
next stroke.

Volume meters run the other way round. OBS reports every active input's levels
about twenty times a second, which is far more traffic than the widget tree
should carry, and every frame of it is disposable. The session event loop
therefore *tries* to send `AppEvent::InputLevelsUpdated` and drops it when the
channel is full rather than blocking every other OBS event behind it; the window
stores the reading in `AppState` and rebuilds nothing. Each meter widget then
pulls its own input's row on the frames it draws, advancing the fall-off from
`services::meter_service` with the frame clock's own timestamp. A snapshot
sequence number tells a widget whether it has already consumed what it is
looking at, which is what lets an input that stopped reporting decay to silence
instead of freezing.

The application-owned GTK event pump polls the receiver every 50 ms, so state
reconciliation and widget mutation stay on the main thread. Closing the shell
removes its widget subscribers but leaves telemetry and disconnect observations
live for a surviving Stats pop-out. Last-window removal or shutdown releases the
pump. The widgets needed by event application are grouped in an `EventUiContext`,
keeping the event adapter's dependency boundary explicit as the window grows. Connection and output event families are applied
by focused handlers; the general handler owns scene, audio, graph, diagnostic,
and statistics updates. Transport ingress advances mutation/topology fences
before GTK publication, using a 64-item handoff plus one selected publication;
overflow retires the connection and reports incomplete evidence instead of
stalling safety invalidation or growing the queue. Workspace state reconciliation lives separately in
`ui::event_reconciliation`; shell effects follow accepted state publication.
Operator actions share the workspace's complete input identity catalog. Enabled
bindings and scoped action pickers contribute deduplicated media/filter read
requirements, separate from Sources selection and drafts. The OBS adapter retains
per-property failures; the catalog resolves actions only against current identity,
capability and freshness. Temporary picker leases retire on close or scope change,
and an inspection selected during an existing read leaves the workspace dirty for
a subsequent read. None of these refreshes queue a later mutation. Live pins use
the same catalog and property-read requirements. Their buttons and menus survive
ordinary observations, while changed pin lists or scope ownership rebuild the
strip. Mute echoes update pin state directly without invalidating unrelated
controls. The bounded audio-echo journal also merges newer mute/volume events over
workspace readbacks; lost journal coverage leaves audio unknown without discarding
unrelated media, scene or output observations.
Doctor similarly separates pure typed findings from localized presentation,
while preserving the existing `DoctorService::run` contract.

## OBS Session Lifecycle

`AppController` owns the active OBS session task. Reconnect aborts the previous
session, clears the shared client slot, and starts a new connection task.

The active OBS client is stored as:

```text
Arc<Mutex<Option<ObsClient>>>
```

The mutex is only held long enough to clone the cheap `ObsClient` handle. It
must not be held across `.await`.

Configuration and Secret Service reads needed for connection and OBS refreshes
run on Tokio's blocking pool rather than the GTK thread. After connection:

1. SceneDeck verifies the OBS version.
2. It publishes the client into the shared slot.
3. It refreshes profiles and scene collections.
4. It refreshes stream and record status.
5. It refreshes scene inventory, graph, and active scene audio.
6. It enters the OBS event loop.

Independent per-scene and per-input reads are issued with a bounded concurrency
limit while preserving OBS list order. This avoids serial request chains on
large collections without flooding OBS or making error ordering nondeterministic.

The session task also owns a one-second statistics poll, raced against the OBS
event loop with `tokio::select!`. obs-websocket has no push event for `GetStats`
or `GetStreamStatus`, so live telemetry must be polled; running it inside the
session — rather than from a GTK timer tied to a page — means the status bar and
the Stats page history stay current no matter which page is visible, and the
poll is torn down with the session on disconnect or reconnect.

On disconnect or event-stream end, the client slot is cleared and a
`Disconnected` event is sent.

## Boundaries

`src/obs/` is the only boundary that imports `obws`. OBS response types are
mapped to domain types before crossing into the controller or UI.

`src/controller/` owns orchestration. It translates commands into async work and
events into GTK-side updates. Runtime dependencies such as config loading and
OBS password retrieval are injected through `controller::dependencies`; the
default production adapters still use local storage and the system keyring.
Password retrieval takes the selected immutable connection ID from the same
configuration snapshot used for its endpoint. The live client carries that ID
into connection observations, so later Settings selections cannot change the
identity of an already connected session.
Stream and recording command guards, status refreshes, fake-client injection,
and graceful shutdown are isolated in `controller::output_controller` rather
than growing the main application controller.
OBS connect/reconnect/disconnect task ownership lives in
`controller::session_controller`, behind an injectable session-runner boundary.
Protocol refresh helpers and the OBS event stream live in
`controller::refresh_controller`. Fake session-runner tests cover connect,
reconnect, and disconnect lifecycle event sequences without a running OBS.

`src/domain/` contains stable app concepts such as scenes, roles, audio inputs,
output status, graph, diagnostics, registry snapshots, graph dependency
policies, appearance preferences, and OBS named lists.

`src/services/` contains pure or mostly pure higher-level logic, such as Doctor
checks, graph edge classification, live-switch validation, scene-hotkey
resolution, and volume-meter ballistics. Services consume
domain-facing snapshots and policies instead of storage adapter structs.

`src/storage/` owns local persistence: config JSON, collection-specific show
workspaces, legacy registry JSON,
XDG paths, and keyring integration. The scene registry storage type remains the
serde representation for JSON/YAML, and converts to `SceneRegistrySnapshot`
before application services use it.

Local persistence is deliberately not routed through the OBS `AppCommand` /
`AppEvent` stream. Config, registry, and password snapshots are preloaded on a
startup worker and cached in `AppState`. GTK page callbacks mutate those
snapshots and use `ui::background_io` for filesystem and Secret Service work;
completion callbacks return to the GTK thread for status updates. This keeps
the OBS controller contract focused while preventing blocking persistence calls
from running in GTK callbacks. Config and registry writes use separate FIFO
worker lanes, so a slow older snapshot cannot finish after and overwrite a
newer snapshot. Each atomic replacement also uses a unique, exclusively
created sibling temporary file, allowing concurrent writers without temp-file
collisions. Replaceable Config snapshots coalesce within each FIFO segment to
one executing write and one pending snapshot; explicit jobs and completion
callbacks remain ordering barriers. Admission is synchronous, so Close/Quit
includes accepted edits even before a worker starts. Registry writes retain
failed intent for explicit retry without rolling back newer edits. Config
completion retires the matching replaceable tail before its pending ticket, so
a worker-held handle cannot revive an unaccounted batch. Returned Config errors,
like worker panics, remain close-barrier failure evidence until an explicit
informed decision; Config snapshots are not automatically retried.

Configuration should prefer domain types over raw strings where the value has a
closed set of valid states. For example, `theme_mode` is stored as the same
lowercase string on disk but is represented as `ThemeMode` in Rust.

`src/ui/` owns GTK widget construction, CSS loading, page layout, navigation,
and action registration.

Theme loading is handled by `src/ui/theme.rs`. `ThemeManager` applies the base
SceneDeck CSS, the selected built-in theme family's effective light or dark
variant, and optional user CSS for the same effective side. `ThemeMode::System`
continues to use libadwaita's system preference; custom themes must provide
separate light and dark CSS paths when they need side-specific styling.
Custom CSS file contents are read through `ui::background_io`; only CSS parsing
and provider installation run on GTK.

The Stats page lives in `src/ui/pages/stats.rs` and draws from
`AppState::stats_history`, a bounded ring buffer filled by the session poll. Its
widgets are built once and refreshed in place; the cairo chart and gauge
renderers live in `src/ui/widgets/chart.rs` and read their values through a
closure at draw time, so a new sample only needs a `queue_draw`. Its gauge,
chart, and card sections are `FlowBox` containers with a per-section column
maximum, so the layout reflows continuously with the window width instead of
snapping at a single hand-picked breakpoint; the chart widget's minimum content
width is what tells those containers when to drop a column.

The Mixer page lives in `src/ui/pages/mixer.rs`. Its mode, scene selection,
search, and grouping controls are stored in `AppState` as a `MixerSelection`.
Active mode uses the active-scene audio snapshot that also feeds Live. Selected
and Pinned modes dispatch `RefreshMixerSceneAudio` through the controller and
receive a separate `MixerAudioInputsUpdated` snapshot. GTK never calls OBS
directly.

## Live Data Refresh

Scene inventory refresh also triggers:

- Active scene audio refresh.
- Scene dependency graph refresh.

Audio refresh for the current scene includes:

- OBS global audio sources first.
- Enabled input scene items from the current scene.
- Enabled inputs inside nested scenes and groups.

Input mute and volume events update the matching audio card without rebuilding
the whole mixer. A bounded history of the last 32 actual gain sends recognizes
delayed own-value echoes while a newer unsent edit is pending, including OBS f32
rounding; an unmatched observation clears that history and remains authoritative.
OBS events carry no command ID, so matching values are an arbitration policy,
not proof of who changed the gain.

## Error Handling

Connection failures can use the connection error/disconnected path. Scoped
workspace and output-command failures retain their own result/error state; a
failed source setting or unsupported request must not falsely mark the OBS
connection as disconnected. Domain-facing errors and warnings reach GTK through
the corresponding controller events.

Warnings that do not need user interruption use `tracing`.

Storage adapters should keep source errors typed until the UI boundary. For
example, registry loading distinguishes read failures from parse failures and
logs the concrete path before falling back to defaults for the desktop UX.

## Scene wall and Sources

Live uses one scrolling scene grid with a separate expandable audio strip and
one compact output toolbar. Topbar Studio controls synchronize with OBS. Scene
activation resolves against fresh Studio state before choosing Program or
Preview; the conditional transition row owns the only Take action. Sources is
a separate topbar destination with adaptive list/detail navigation.

`domain::control` holds protocol-free scene items, Studio state, source details
and native actions. `domain::workflow` retains shared scope and bounded session
activity; retired expectations, baselines and cues are gone. Existing
`workflows.json` files are preserved without routine reads/writes.
`domain::output_result` keeps command acknowledgements distinct from observed
file evidence. Local config/registry edits remain on ordered background lanes.

`controller::workspace` owns scoped aggregate reads, actions and selected-media
refresh. `WorkspaceState` holds epoch/sequence, dirty/loading/pending state and
observations on GTK. The controller checks endpoint/collection before executing
and collection again after reads; GTK rejects obsolete epoch/sequence results.
Scope loss cancels owned work, clears selections and marks pending output
evidence unconfirmed. Read age starts at the first request.

Common workspace reads obtain operating/output/audio observations; scene-item,
selected-source detail and usage scans are included for Sources rather than
routine Live visits. Actions use authoritative rereads. Selected-media polling
runs only while the chosen media detail is visible. UI source callbacks retain
scope/freshness protection and explicit text Save/Cancel/conflict state. Media
polling pauses while a native popover is open; an already-in-flight media-only
reply preserves that popup when its binding is unchanged. Scope, full-read,
pending, error and freshness changes still invalidate it immediately. Structural
path expansion and named path focus survive ordinary detail refreshes.
The media timing readout updates in place through the existing status refresher.
It derives progress and remaining time only from valid observed cursor/duration
pairs, rejects stale workspace evidence, and does not introduce another timer or
OBS request. Seek editors remain separate from observed playback values.

## Scene thumbnails

`controller::thumbnails` owns an independent capture worker and a two-second UI
response deadline; it never uses the workspace mutation pending flag. Timeout
reports failure without dropping an unanswered protocol future: the single
worker drains it, rejecting additional image work on that connection. This
bounds `obws`'s retained pending requests. A replacement connection can retire
the old worker without resetting the active controls socket. `ui::previews`
selects visible Live cards, rate-limits requests, bounds its texture cache and applies images in
place. Session epoch, endpoint/collection scope, generation and scene identity
prevent obsolete captures from being attached to current cards. Disabling the
preference, leaving visible Live or disconnecting suppresses thumbnail delivery
and follow-up requests. Cancellation is checked after delayed canvas settings
before sending a screenshot. An already sent request drains silently.
Control results take priority over stills in the bounded event channel. Each
capture checks collection before/after the screenshot and reads video settings
for aspect-fit dimensions. Failed scenes back off for 15 seconds independently;
an unanswered transaction temporarily blocks other images while controls remain
independent.

The current limits and implementation are described in
[OBS integration](obs-websocket.md). These are static bounds, not measured
performance.

## Inventory and themes

Inventory uses responsive native list rows with wrapped scene names and role
descriptions. Role controls reflow below scene details at compact widths;
secondary scene actions remain in a popover. Layout is allocation-driven, with
no column configuration or layout writes. Legacy width preferences are
accepted as unknown keys for config compatibility, ignored, and omitted on save.
Inventory refreshes compare presentation data, retaining unchanged rows, menus
and layout widgets; membership, order and metadata changes reconcile locally.

Refresh requests do not redraw old page contents before readbacks arrive.
Same-scope inventory observations retain Live preview paintables and header
selector models. Live audio observations update existing cards in place;
a same-scene refresh temporarily disables retained controls, while an owner
change discards them. Sources separates editor content from loading/result/
freshness metadata: status-only publications retain editor widgets and drafts,
while changed targets, observed content or scope take the rebind path. These
optimizations do not bypass scope fences or permit stale mutation controls.

The common CSS owns geometry, typography, semantic card states and motion.
Twelve built-in families provide light/dark palettes; optional custom CSS loads
for the effective side. Existing `.scene-card`, `.scene-card-active` and
`.scene-card-hotkey` selectors remain supported alongside the documented image,
scrim, Preview, audio/output and Inventory classes. See
[theme CSS reference](theme-css-reference.md).

Stats samples and activity records use local Unix-millisecond times. Charts
remain sample-spaced; event interpolation rejects long gaps, backwards clocks
and out-of-window events. An activity marker is contextual evidence. Stats and
its popout keep independent local marker selections.

### Preflight readiness

`domain::preflight` evaluates scoped scene/input identities, mute, recent signal,
and recording-space evidence. Missing and stale observations remain unknown.
Saved expectations live in configuration by actual connection ID and endpoint,
independent of the active collection; volatile checklist and signal evidence lives
only in the review window and is retired on plan or workspace-epoch changes.
`ui::preflight` and its editor use read-only workspace/statistics commands and the
ordered configuration persistence lane. The optional output-start continuation is
explicit, single-use and bound to the original OBS session/collection, and then
uses the existing output confirmation flow.

### Live health incidents

`domain::health::HealthMonitor` belongs to AppState and consumes scoped statistics
with monotonic observation times. Per-condition state requires sustained bad
intervals and a longer recovery cooldown; invalid, stale, duplicate, out-of-order
or reset counters cannot manufacture an incident or prove recovery. Rendering,
encoding and network counters retain separate baselines. The monitor's observed
output activity, rather than command fallback status, gates recording-space and
encoder checks.

Connection/output incident tracking is in `domain::health::events`. Connection
intent distinguishes deliberate retirement from loss. Output stop expectations
start at controller admission (`OutputCommandPending`), expire, and are retired on
definite failure or session change. Only actual output observations establish an
unexpected stop; acknowledgements and synthetic fallback states do not. Latched
incidents carry monotonically assigned IDs so stale dialog responses cannot
acknowledge a replacement incident.

`ui::health_alerts` presents the main-window cross-page summary and read-only detail
window. Its shell refresh source belongs to WindowSources, and details release their
weak widget timer when closed. Performance incident data is volatile; durable
optional session history on disk retains alert transitions as described below.


Selected-input checks live in `domain::health::audio`. `AudioRule` is persisted in
the scoped registry, while `AudioMonitor` owns volatile per-rule observations.
Workspace epoch, registry generation or rule changes reset that evidence. Current
complete input identities and mute reads resolve the target before name-keyed OBS
meters are consumed. Malformed multipliers and duplicate meter names preserve an
invalid flag even though visual meters still decay/display silence. A repeated
receipt timestamp cannot advance failure or recovery; stale/missing/invalid data
retains already raised incidents as unobserved.

The application event-pump tick evaluates audio health independently of shell
visibility. When enabled rules need inventory/mute evidence, it requests existing
read-only workspace observations with a ten-second retry interval, respecting
pending/loading work. The main shell and detached Stats each own a health summary;
closing/removing the detached window destroys its summary source. The application
pump remains until the last application window closes. Audio-rule editors fence
scope and their saved baseline, retain missing inputs, and use the registry's
ordered persistence/retry lane. Portable imports disable all audio rules.


Output admission, transport-receipt and read-request times travel with their
events, before executor/queue/GTK delays. Workspace snapshots retain separate
request-start times for stream and record reads, rather than dating those values
at the start or end of a larger workspace scan. AppState fences current output
observations and failed reads by those per-output monotonic times; the fences
survive collection changes and reset on connection retirement. Normal resync
output-read failures also publish unavailable evidence, with repeated identical
failure notices coalesced. A missing workspace output read is Unknown, not stopped.

Admitted output intent is connection-owned, so a collection change cannot discard
its pending event while allowing the output operation to proceed. Session fences
still retire it on connection replacement. The thirty-second stop-correlation
window uses source times; GTK backlog cannot renew it. File facts remain distinct
from current run state: a late reported recording path is retained as evidence,
while an older state cannot undo a newer restart or replace a newer latest path.
A completed StopRecord response emits `RecordingStopObserved` file evidence only,
not an inactive status. Its command-admission time conservatively orders the path
behind any later recording observation; current state comes from status reads or
transport events.


## Bookmark timing and persistence

`domain::bookmark` distinguishes local operator notes from recording/chapter
commands. `AppState` starts a local observation-session clock on Connected and
retires it on connection reset, preserving existing notes across collection and
connection changes. Capture requires a current connection and keeps original
context. OBS duration is retained from GetRecordStatus only; transport events and
fallback statuses carry no timing. Duration advancement is excluded from run-state
change logging and disconnect-confirmation comparisons.

Bookmark capture uses the accepted output observation timestamp. A sample older
than five seconds, a future timestamp, uncertain/transitioning output, missing
duration or pending workspace mutation yields unavailable recording time. No local
extrapolation occurs. The captured sample age remains with the note.

`storage::bookmarks` validates version, count, IDs, names and file size, and uses
bounded regular-file reads and atomic writes. `ui::bookmarks` queues full snapshots
on its own background-I/O lane and exposes saved/error/retry state. Full snapshots
cover earlier edits; only a successful covering snapshot resolves their failure
evidence. Reading failure disables edits, and retry loading preserves the original
connection clock. Dialog capture is scope-bound; editing a historical note uses
its stable ID independently of current OBS scope. Closed/detached callbacks cannot
modify notes.

### Session reports and explicit exports

`domain::session_report` defines bounded report records and sanitized telemetry
samples. `controller::state::session_reports` collects accepted evidence after each
app-owned event-pump publication and health tick, independently of open pages.
The activity log has a receipt cursor so ring-buffer gaps are measurable. Output
result snapshots retain Requested/Acknowledged/Observed/Failed/Unconfirmed as
separate immutable evidence. An old result does not become a new session's result.

Connection reset collects final evidence before clearing caches and ends the
report without claiming output termination. A connection-generation mapping routes
late terminal audits to their original retained report. The recent memory archive
holds eight sessions; each retains 4,096 events and 2,048 samples, with omission
counts and aggregates over all valid samples. Samples retain request-start time,
not a synthesized measurement at the UI tick. Nonfinite and negative measurements
become unavailable before JSON serialization. Bookmarks are references resolved
against the current separate archive, so renames and deletions remain effective.

`ui::session_reports` presents an explicitly refreshed review snapshot; Stats and
the command palette both open it. `storage::report_export` writes the selected
snapshot atomically to the exact FileDialog target on a dedicated FIFO lane.
CSV quotes cells and neutralizes formula prefixes; JSON retains exact structured
values. Returned errors participate in shutdown failure reporting, and a successful
explicit export retry resolves its own target/report failure. `controller::state::report_history` holds desired and observed disk archives
separately, plus eligible session IDs and bounded checkpoint admission state.
`ui::report_history` admits at most one history operation at a time; a forced
checkpoint during a pending write chains the latest snapshot before the existing
shutdown barrier can complete. Ordinary checkpoints are spaced by ten seconds,
with earlier writes for session boundaries. Per-report omissions remain explicit.

`storage::report_history` validates a versioned 64 MiB/32-session archive, uses a
regular-file advisory lock and compares expected contents before atomic writes.
Startup loads history off GTK. Reports without a stored end become interrupted
in the local view without an invented timestamp. Reload merges locally owned
report intent; clear intent survives retry. Settings and history saves retain
separate failure tickets. Connection generation routes late terminal audit to a
retained disk-history report even after it has left the eight-session memory ring.

The main shell closes through a checkpoint barrier. Closing the last document
window, including detached Stats, or choosing Quit finalizes the observation
session before draining writes. A temporary `ExitCheckpoint` owns application
state until final publication without retaining shell widgets; the application
event pump may release its state on last-window removal. Pending output requests
are copied into the report as unconfirmed without mutating live request tickets.
A failed or timed-out drain resumes the same observation session if its connection
still exists, records the interruption, and offers Cancel/Retry/informed abandon.
Retry reloads conflicting history before saving local intent; explicit abandon
suppresses additional automatic history writes during that exit attempt. A
second checkpoint check includes connection/terminal evidence delivered while the
first write ran. The shutdown hook remains a fallback for non-window shutdown.
None of these paths changes OBS or claims output termination. The
history settings window uses an explicit Apply action, so changing the retention
draft cannot delete reports. Clear requires its own confirmation and excludes the
current connection until another explicit opt-in or the next connection.

### Audio presets

`domain::audio_preset` contains bounded named input snapshots. `PresetGain` has a
private finite 0–20 multiplier with checked deserialization, allowing the registry
to retain its equality contract without NaN or lossy decimal quantization.
`SceneRegistry::audio_presets` is scoped and persisted with existing curation.
`ui::audio_presets` captures from the fresh complete workspace input catalog/audio
snapshot, not the active-scene audio cards. It resolves UUIDs before capture,
preserves missing selections until the operator removes them, previews current and
saved values, and rejects retired scopes or concurrent preset edits. Explicit
save/replace reuses ordered registry persistence and same-value retries. Window
callbacks hold weak editor references; the bounded refresh source releases state
when the window closes.

`domain::audio_preset::run` separates immutable reviewed intent from per-input
results. The UI admits one fresh review through NavigationContext/AppCommand;
`controller::workspace::audio_presets` owns one sequential session-scoped worker.
It resolves current identities and checks previewed values before writing. Mute
precedes gain when muting; gain is acknowledged and read back before unmuting.
The adapter writes the saved linear multiplier without routing it through the
fader's narrower dB range. Each property has a lifecycle operation lease carrying
run, epoch, input identity and property; late terminal evidence preserves known
acknowledgements without asserting readback or crossing replacement runs.

Cancellation, topology and collection fences are checked at mutation admission.
An explicit scene-collection switch also cancels remaining preset work before
dispatching the switch. Network operations retain the normal ten-second budget;
publication into the bounded GTK queue is outside that deadline. A timed-out
socket cannot send later input mutations. AppState owns progress after editor
close, logs it for session reports, and retires unfinished rows on scope changes.
Results distinguish missing/changed inputs, acknowledgements, confirmed readback,
failure and uncertainty. Presets are not atomic OBS transactions and never replay.


### Desktop portal shortcuts

`ui::global_shortcuts` owns an explicit registration session per main-window
NavigationContext. It uses the existing asynchronous GIO bus API; no new runtime
or dependency is introduced. `portal.rs` checks the GlobalShortcuts version,
subscribes before request calls, verifies predicted request/session paths and
pins session calls/signals to the portal's unique bus owner. A dedicated D-Bus
peer calls [host Registry.Register](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.host.portal.Registry.html)
with `io.scenedeck.app` before any other portal method, so earlier GTK portal use
cannot lock in an unidentified peer. Older registries and sandbox-provided
identities can reject registration; CreateSession still decides admission. Bus
address lookup runs on GIO's blocking pool, and peer cleanup flushes close messages
then closes the private connection. Portal restart retires the session; a new
explicit Enable uses a newly registered peer. Request and session
handles close on drop, including cancellation while permission is pending.
Method calls have ten-second deadlines; human interaction is bounded to two
minutes. Portal loss, Closed, Stop, window removal and scope retirement end the
session. A watchdog runs throughout permission requests and active registration.

Only enabled, non-conflicting application bindings are offered (maximum 64).
SHA-256 IDs encode connection/endpoint/collection and the operator intention,
using UUID identity independently of display name, ordering or local key choice.
Duplicate action intentions share one desktop registration. BindShortcuts and
ShortcutsChanged provide the accepted subset and actual trigger descriptions;
unknown IDs are ignored. Activation rechecks scoped navigation and the captured
binding list before using the existing action catalog. Held keys dispatch once,
visible modals and active-window text editing suppress execution, and confirmed desktop bindings suppress
local copies. Stopping restores local handling. No persistent opt-in, background
registration, reconnect replay or global-key support claim for every desktop is
made. Tests use a fake portal on the native runner's disposable D-Bus session.
