# SceneDeck: features that could distinguish an OBS controller

Second research pass: 2026-10-19. SceneDeck: `003d9ba1aa72cdbddb16884c9eb0a8b97704418b` (0.4.0).

This supplements the [first research report](obs-only-feature-research.md), [review](obs-only-research-review.md), [UI/UX plan](../tasks/plan.md) and [task breakdown](../tasks/todo.md). Those documents are preserved. This pass changes the emphasis from closing UI/control gaps to choosing useful product advantages. All recommendations are proposals; no application implementation or release commitment is implied.

## Recommendation

**Make SceneDeck a desktop operating desk that explains a change's scope, checks the setup against the operator's intentions, and helps identify what changed since rehearsal.**

The strongest three opportunities are:

1. **Source inspector with search and “Used in…” context.** Find the correct item and understand which operations affect shared sources or containers.
2. **Factual preflight built on Doctor.** Check explicit expectations against fresh OBS observations; explain failures and unknowns beside the relevant controls.
3. **Rehearsal baseline and comparison.** Save selected audio settings locally, then show differences before the next show. Start read-only.

A stable, individually pinned audio watchlist is a smaller complementary opportunity. Recording/replay result feedback, a recent activity timeline and manual scene cues are useful follow-ups for particular workflows.

Source visibility, Studio Mode, recording pause, replay, media transport, filter controls and text editing still matter. They make SceneDeck more complete, but competitors already implement much of this surface. Deliver some of these controls alongside one distinctive workflow rather than building every possible control first.

The working audience remains one Linux operator controlling a local or remote OBS instance. “Better” means fewer trips into OBS, faster identification of the correct target, clearer results and fewer preventable operating mistakes. These are hypotheses to test, not measured claims of superiority.

## Scope, workflow and stronger evidence

The boundary is native OBS WebSocket requests/events plus local application data. Chat, platform accounts/APIs, vendor requests and third-party integrations remain excluded. Comparing Companion's OBS module does not propose integrating with Companion.

Parallel research examined SceneDeck's extension points, desktop workflows and competitor source. The synthesis also checked native OBS request implementations and independently reviewed the proposed scope. This separation was useful for challenging UI ideas against both existing code and actual protocol behavior.

Unlike the first pass, shell networking worked in the parent environment. Shallow checkouts outside the project made these observations reproducible:

| Repository | Inspected revision | Evidence boundary |
|---|---|---|
| [OBS Blade](https://github.com/Kounex/obs_blade/tree/4f01d3254935e5455c314a45390eadb94a321198) | `4f01d3254935e5455c314a45390eadb94a321198` | Current source widgets and store wiring; not an installed mobile release |
| [Niek/obs-web](https://github.com/Niek/obs-web/tree/1cc43ff00907b234e7220339444485031dddba11) | `1cc43ff00907b234e7220339444485031dddba11` | Browser controller source; not a browser/OBS usability benchmark |
| [Companion OBS module](https://github.com/bitfocus/companion-module-obs-studio/tree/65760a249b8190e398a25faad37bdc2da9a03379) | `65760a249b8190e398a25faad37bdc2da9a03379` | Native OBS actions/feedbacks only; not the rest of Companion |
| [OBS WebSocket](https://github.com/obsproject/obs-websocket/tree/e28f84975984340e84fc4879cfc05741cc0afde1) | `e28f84975984340e84fc4879cfc05741cc0afde1` | Protocol and C++ handlers; an installed OBS may expose an older subset |

Absence from the inspected paths is not proof that a competitor lacks a feature. Proposed differentiation means a promising integrated workflow, not an exclusivity claim. Current development branches can also differ from distributed releases.

## What the second pass changes

The first report deliberately left some Blade widget behavior unverified. Current source closes those gaps:

| Area | Verified current behavior | Consequence for SceneDeck |
|---|---|---|
| Source text | Blade reads input settings and submits a text overlay only after an explicit save; failure keeps the editor open. [Text sheet](https://github.com/Kounex/obs_blade/blob/4f01d3254935e5455c314a45390eadb94a321198/lib/views/dashboard/widgets/dashboard_content/scene_content/scene_items/text_source_sheet.dart#L47) | A small built-in text-source editor is useful parity, especially for lower thirds. |
| Filters | Blade toggles filters and builds dynamic settings controls. This does not establish support for every property type. [Filter list](https://github.com/Kounex/obs_blade/blob/4f01d3254935e5455c314a45390eadb94a321198/lib/views/dashboard/widgets/dashboard_content/scene_content/scene_items/filter_list/filter_list.dart#L121) | Start with enable/disable, not a speculative universal settings editor. |
| Advanced audio | Blade has balance, monitoring and sync-offset mutations. [Settings](https://github.com/Kounex/obs_blade/blob/4f01d3254935e5455c314a45390eadb94a321198/lib/views/dashboard/widgets/dashboard_content/scene_content/audio_inputs/audio_settings_sheet.dart#L94), [sync](https://github.com/Kounex/obs_blade/blob/4f01d3254935e5455c314a45390eadb94a321198/lib/views/dashboard/widgets/dashboard_content/scene_content/audio_inputs/audio_slider.dart#L167) | These deepen SceneDeck's Mixer; they do not establish a unique advantage. |
| Recording/replay | Blade exposes pause/resume, replay start/stop/save and request-gated split/chapter controls. [Recording](https://github.com/Kounex/obs_blade/blob/4f01d3254935e5455c314a45390eadb94a321198/lib/views/dashboard/widgets/dashboard_content/exposed_controls/recording_controls.dart#L64), [replay](https://github.com/Kounex/obs_blade/blob/4f01d3254935e5455c314a45390eadb94a321198/lib/views/dashboard/widgets/dashboard_content/exposed_controls/replay_buffer_controls.dart#L33) | Close the output gaps and invest in trustworthy result feedback. |
| Media | Blade has a collection-wide media hub, pad/list arrangement, source visibility context and transport/seek controls. [Transport](https://github.com/Kounex/obs_blade/blob/4f01d3254935e5455c314a45390eadb94a321198/lib/views/dashboard/widgets/dashboard_content/scene_content/media_hub/media_transport_sheet.dart#L78), [arrangement](https://github.com/Kounex/obs_blade/blob/4f01d3254935e5455c314a45390eadb94a321198/lib/views/dashboard/widgets/dashboard_content/scene_content/media_hub/media_hub_arrange_sheet.dart#L16) | A media watchlist or “active in Program” badge alone would not distinguish SceneDeck. |
| Personalization/history | Blade persists hotkey favorites, section order and stream/record statistics. [Favorites](https://github.com/Kounex/obs_blade/blob/4f01d3254935e5455c314a45390eadb94a321198/lib/views/dashboard/widgets/dashboard_content/exposed_controls/hotkeys_control/hotkey_entry.dart#L29), [layout](https://github.com/Kounex/obs_blade/blob/4f01d3254935e5455c314a45390eadb94a321198/lib/views/settings/dashboard_customisation/order/widgets/element_list.dart#L169), [history](https://github.com/Kounex/obs_blade/blob/4f01d3254935e5455c314a45390eadb94a321198/lib/stores/views/dashboard.dart#L1348) | Generic favorites, dashboard customization and archived charts are existing competitor strengths. |
| State reliability | Blade prevents stale mutations and reconciles failed writes. Current `StaleGuard` dims and ignores interaction; this refines the earlier documentation-only account. [Guard](https://github.com/Kounex/obs_blade/blob/4f01d3254935e5455c314a45390eadb94a321198/lib/shared/design/stale_guard.dart#L8), [mutation boundary](https://github.com/Kounex/obs_blade/blob/4f01d3254935e5455c314a45390eadb94a321198/lib/stores/views/dashboard.dart#L690) | Honest stale/pending/failure states are a baseline requirement, not a differentiator by themselves. |

Two additional controllers sharpen the comparison:

- **Niek/obs-web** already has separate Program/Preview source drawers, output controls and an audio mixer. The inspected source drawer explicitly lists only top-level scene items, giving SceneDeck a concrete opportunity to make nested ownership and navigation clearer. Its mixer includes metering, mute, dB entry, monitoring and persistent source hiding. [Scene panel](https://github.com/Niek/obs-web/blob/1cc43ff00907b234e7220339444485031dddba11/src/SceneItemsPanel.svelte#L24), [mixer](https://github.com/Niek/obs-web/blob/1cc43ff00907b234e7220339444485031dddba11/src/AudioMixer.svelte#L9), [channel](https://github.com/Niek/obs-web/blob/1cc43ff00907b234e7220339444485031dddba11/src/AudioMixerChannel.svelte#L111).
- **Companion's OBS module** establishes a high breadth benchmark: text/filter operations, advanced audio including tracks and fades, media targeting, recording chapters/splits, transitions and state feedback. Its smart scene action already stages a scene or transitions to it when staged. SceneDeck's opportunity is making a focused workflow understandable without requiring users to assemble an action surface. That is a design hypothesis, not a measured setup-time comparison. [Actions reference](https://github.com/bitfocus/companion-module-obs-studio/blob/65760a249b8190e398a25faad37bdc2da9a03379/companion/HELP.md), [scene action](https://github.com/bitfocus/companion-module-obs-studio/blob/65760a249b8190e398a25faad37bdc2da9a03379/src/actions/scenes.ts#L63), [media targeting](https://github.com/bitfocus/companion-module-obs-studio/blob/65760a249b8190e398a25faad37bdc2da9a03379/src/actions/media.ts#L52).

These findings strengthen the first pass's recommendation to preserve SceneDeck's roles, Mixer, Graph and Doctor. They weaken the case for presenting ordinary media controls, a dashboard builder or more statistics charts as the main reason to switch.

## Seven feature directions

The ordering balances broad operator value, fit with existing code and uncertainty. Cost means relative engineering/state complexity, not a calendar estimate. Even a moderate feature crosses OBS/domain/controller/UI/localization boundaries.

| Priority | Direction | Value | Cost | Differentiation potential | Main condition |
|---|---|---|---|---|---|
| 1 | Searchable source inspector with usage context | High | Medium–high | High as an integrated workflow | Nested/shared sources are common enough to create confusion |
| 2 | Fresh preflight against explicit expectations | High | Medium | High | The checks identify real preparation mistakes without noisy warnings |
| 3 | Read-only rehearsal baseline/diff | High for repeatable setups | Medium | High | Operators want to reproduce an intentional setup |
| 4 | Stable individual audio watchlist | High during scene changes | Medium | Medium | Scene-level Mixer pinning does not already solve the need |
| 5 | Recording/replay result tray and markers | High for recording-heavy use | Medium | Medium | Clear confirmation and retrieval context matter more than more buttons |
| 6 | Manual scene cues with notes | High for structured shows | Medium initially; high with staging | Medium–high | The show has a repeated running order |
| 7 | Recent operations alongside health charts | Medium | Medium | Medium | Operators need help reconstructing a problem |

### 1. Find the correct source and understand the change

**Journey:** Search “Guest,” choose `Interview → Overlay group → Guest name`, hide that item, then inspect the Guest microphone without changing Program. A “Used in…” list explains other scene paths affected by source-wide settings.

**Smallest useful delivery:** extend the planned Live source inspector with scene-local search, explicit owner breadcrumbs and separate item/source controls. Follow with a read-only collection usage index. Enter selects/inspects; a separate control performs the mutation. A usage result can open the existing Selected-scene Mixer without taking that scene live.

**Build on:** recursive discovery in `src/obs/client.rs:281`, input scope/path in `src/domain/audio.rs:40`, Selected/Pinned Mixer in `src/domain/mixer.rs:104`, graph reachability in `src/domain/graph.rs:94`. Current graph edges contain scenes, not all sources; current audio discovery intentionally deduplicates inputs.

**New work:** retain scene-item owner, ID, source identity, parent paths, enabled state and incomplete reads. Index source usages from those records. Cache containers once, but retain all relevant parent paths; a global visited set must not erase a second usage path.

**Hard boundary:** input mute/volume/filter/media settings affect a shared source. Item visibility affects the item in its owning container, which can itself be reused by several parent scenes. “This item” must not imply “only this screen occurrence.” Show structural usage separately from OBS-reported activity; neither proves that a viewer can see/hear it.

**Accept when:** duplicate input instances have distinct item controls; shared nested children show multiple usage paths but target one owned item; failed traversal is visible; collection changes invalidate results; selecting/inspecting never switches Program.

**Validate value:** compare finding and hiding a nested lower third in a realistic collection using SceneDeck, OBS and a comparator. Count wrong-target attempts and trips back to OBS. Start with local search if whole-collection usage proves too costly or rarely used.

### 2. Check the intended setup before the show

**Journey:** Click **Check setup**. SceneDeck reports “Required microphone is muted,” “Expected opening scene differs,” “Recording already running,” and the OBS recording disk headroom. Each row shows when it was checked and links to the relevant control.

**Smallest useful delivery:** one explicit required microphone expectation plus fresh output/profile/collection context, disk headroom and existing structural findings. Add required item visibility once the inspector's item inventory exists. A deliberate “speak now” interval can report observed input activity later.

**Build on:** `DoctorService::run` in `src/services/doctor_service.rs:20`, current output state, polling and meters. Keep structural diagnostics separate from observed expectations rather than changing the meaning of every existing Doctor result.

**New work:** locally scoped expectations; read results with Pass/Fail/Unknown, evidence and observation time; a bounded fresh-read operation. The current Doctor button only rebuilds from cached state (`src/ui/pages/doctor.rs:43`, `:82`). Scene-item changes refresh audio but not graph (`src/controller/refresh_controller.rs:267`). Neither behavior is sufficient for a live readiness claim.

**Low-cost enabling feature:** preserve `GetStats.availableDiskSpace`, which the installed `obws` response already contains and `src/obs/mapper.rs:100` drops. OBS computes it against the recording output path, so label it **OBS recording disk space**, not controller disk space. Missing/invalid readings stay unknown. [OBS implementation](https://github.com/obsproject/obs-websocket/blob/e28f84975984340e84fc4879cfc05741cc0afde1/src/utils/Obs_ObjectHelper.cpp#L26).

**Hard boundary:** a sequence of reads is not a frozen OBS snapshot. Invalidate or mark results stale when scope changes; do not promise the show will be correct after the check. Quiet audio is not automatically broken, source existence does not prove camera health, and meters do not prove audience delivery. Warnings should not automatically block Start.

**Accept when:** absent, muted, unreadable and stale states differ; no mutation is issued by the check; a collection change during collection cannot yield “all clear”; expected silence produces no unsolicited alarm.

**Validate value:** seed a few realistic setup mistakes and observe whether the report helps an operator correct them. Stop expanding the check catalog if the result becomes a checklist users ignore.

### 3. Explain what changed since rehearsal

**Journey:** After a good rehearsal, save a baseline for Microphone, Guest and Music. Before the next recording, compare: “Guest gain −6 → −18 dB,” “Music is now muted,” “Expected input missing.”

**Smallest useful delivery:** capture and compare mute/volume for explicitly selected inputs. Store a small versioned local document scoped to the OBS endpoint and scene collection. Comparison is read-only. Add selected visibility properties after item identity is reliable.

**Build on:** existing audio snapshots/setters, local background persistence and `src/storage/atomic.rs`. Registry YAML currently exports local scene metadata; it is not an OBS operational backup.

**New work:** typed baseline records, binding/rebinding rules and a readable property diff. Use appropriate gain tolerance rather than treating floating-point noise as a meaningful change. Exclude arbitrary settings blobs, credentials and unselected properties.

Capture and comparison require fresh, scoped reads, not an unqualified copy of cached Mixer state. Preserve unreadable results per property, show the observation time/range and invalidate the result if the endpoint or collection changes during reading. These observations do not constitute a frozen OBS snapshot.

**Hard boundary:** this is a baseline of selected settings, not a scene-collection backup or full restore. Another collection with the same names must not silently match. Prefer UUIDs where the connected server supplies them, and require explicit rebinding when identity cannot be established. This need not force a wholesale rewrite of the existing registry in the first slice.

**Accept when:** capture only writes locally; comparison never mutates OBS; fresh/unchanged/missing/unreadable states are clear; a scope change during reading invalidates the result; cross-collection matching is prevented; changed values are legible without reading JSON.

**Later, only if needed:** restore one selected property with an explicit before/after value and authoritative reread. No “restore everything,” automatic rollback or transactional claim. OBS batch execution stops on failure without undoing successful earlier requests. [Batch handler](https://github.com/obsproject/obs-websocket/blob/e28f84975984340e84fc4879cfc05741cc0afde1/src/requesthandler/RequestBatchHandler.cpp#L169).

**Validate value:** use one real rehearsal and later setup. If operators rarely need reproducibility, prioritize immediate controls instead.

### 4. Keep critical audio controls in fixed positions

**Journey:** Watch Microphone, Guest and Music once. Intro → Interview → Break changes the scene, while those controls keep their positions and report their current state.

**Smallest useful delivery:** individual source pinning on existing audio cards plus a compact fixed-order strip. Preserve the full Mixer and its scene-based Active/Selected/Pinned modes. Individually watched inputs are distinct from the existing pinned scene.

**Build on:** audio cards, meters, fine gain adjustment and fader locks already implemented in `src/ui/widgets/audio_card.rs`; meter data in `src/domain/meter.rs:66`.

**New work:** independent watched-input selection/lookup, scope persistence and fresh/missing/inactive labels. Do not synthesize a zero meter when no fresh sample arrives. A quiet, opt-in activity check can later support preflight; it should not auto-unmute or change gain.

**Accept when:** controls remain ordered across scene changes, reflect external mute/volume events, explain missing/renamed inputs and distinguish unavailable meters from silence. Keyboard and accessible labels identify the source.

**Useful parity next:** an audio details sheet for monitoring, sync offset and track assignment. Monitoring changes playback on the OBS machine; it does not deliver audio to SceneDeck. Advanced audio is already represented in competitor controls, so its value is convenience within this stable workspace.

**Validate value:** watch an operator through a real scene sequence. If pinning a whole scene already suffices, defer individual pinning.

### 5. Make recording and replay results easy to verify

**Journey:** Save a replay once and see a request acknowledgement followed by an observed saved path. Recording pause is unmistakable. A named chapter or local note can mark a useful moment without leaving the operating screen.

**Smallest useful delivery:** complete pause/replay controls from the first plan and add a bounded recent-results tray. Show requested, acknowledged, observed completion, failed or unconfirmed distinctly. Provide copy-path, identifying it as a path on the OBS host.

Responses and events may arrive in either order. An acknowledgement must not erase a newer observed result, and an event without request identity must not be treated as uniquely correlated to a local click.

**Build on:** output pending guards, command-scoped failure events and current recording paths (`src/controller/output_controller.rs`, `src/controller/event.rs:241`).

**New work:** replay events/status and result records; split/chapter controls gated by available requests and actual output support. OBS can reject a chapter or split even when the request exists. [Recording handler](https://github.com/obsproject/obs-websocket/blob/e28f84975984340e84fc4879cfc05741cc0afde1/src/requesthandler/RequestHandler_Record.cpp#L190).

**Hard boundary:** a shared OBS event does not identify which controller requested the save. Report what was requested and what was observed without inventing correlation guarantees. [Save request](https://github.com/obsproject/obs-websocket/blob/e28f84975984340e84fc4879cfc05741cc0afde1/src/requesthandler/RequestHandler_Outputs.cpp#L238), [saved event](https://github.com/obsproject/obs-websocket/blob/e28f84975984340e84fc4879cfc05741cc0afde1/src/eventhandler/EventHandler_Outputs.cpp#L169). A remote path does not grant file download, playback, editing or local-open support over this protocol. A local “Mark moment” note is not an embedded OBS recording chapter; distinguish their time bases and account for recording pauses if notes use recording time.

**Accept when:** repeated clicks do not create misleading duplicate success entries; rejected/unsupported operations recover; delayed saves remain unconfirmed until evidence arrives; a path on another host is never treated as a local file.

**Validate value:** test recording-heavy workflows. Do not turn the tray into an asset manager or replay editor.

### 6. Run a repeated show with a manual cue list

**Journey:** Opening → Interview → Main → Break → Main. Each cue has its own note and state even when two cues use the same scene. The operator deliberately selects, executes and advances.

**Smallest useful delivery:** ordered scene references and plain notes, with separate **Go** and **Next** actions in Direct mode. Show “OBS is on target” from confirmed state; do not auto-advance from a scene event. With Studio Mode later, use explicit **Stage next** and **Take**.

**Build on:** current curated scene order and switching. Do not overload `scene_order`: its duplicate removal in `src/storage/registry.rs:143` conflicts with repeated cue scenes. Cues need their own IDs and collection binding.

**Hard boundary:** a cue is not a transaction or a rehearsal sandbox. Selecting the already-current scene might emit no change event, and another controller can switch Program. Shared inputs can affect Program while a different scene is staged.

**Accept when:** repeated scene targets retain separate notes/progress; external changes update Program without advancing cues; rejected or stale targets preserve position. Shortcuts name the current action and do not steal text-entry keys.

**Defer:** timers, branching, arbitrary action bundles, scripts, automatic media starts and automatic output controls. If users mostly improvise, ordinary scene ordering plus Studio Mode is enough.

### 7. Put recent actions beside health changes

**Journey:** Frame drops rise after a scene switch. Stats shows the observation time alongside Program changes, output events and failed SceneDeck commands, making the sequence easier to investigate.

**Smallest useful delivery:** a bounded session-only event list beside existing charts, with timestamps and provenance. Selecting an event highlights its time. Persist/export only after operators demonstrate a need.

**Build on:** the 120-sample `StatsHistory` in `src/domain/stats.rs:10`, current event flow and Stats page. This extends existing telemetry; it is not a second statistics dashboard.

**New work:** timestamped records, explicit gaps on disconnect, local-command versus observed-event labels, and matching chart times. Do not log every meter frame or fader tick; continuous controls need coalescing.

**Accept when:** event memory remains bounded; disconnect intervals are visible; external changes are not attributed to SceneDeck; counter resets do not produce false spikes; metric/event timestamps share a clear basis.

**Hard boundary:** timing is evidence of correlation, not proof that a scene caused an encoder/network problem. OBS observations cannot reconstruct events missed while disconnected. Blade already has session statistics, so the proposed value is operational context, not chart retention alone.

## Desktop interaction plan

Use the current GTK4/libadwaita shell. Add context to Live and details to existing pages; avoid a sidebar entry for every feature.

```text
Live                                            OBS / collection / freshness
┌────────────────────────────┬────────────────────────────────────────────┐
│ Program / Preview + Take*  │ Inspecting: Interview                      │
│ Curated scene deck         │ Search sources                             │
│                            │ Items → selected item → shared-source info │
│ Watched audio              │ Used in… / supported media or text actions │
├────────────────────────────┴────────────────────────────────────────────┤
│ Check setup: 1 issue, 1 unknown         Recording / replay result state │
└─────────────────────────────────────────────────────────────────────────┘
* Preview/Take appear only with the separately implemented Studio workflow.
```

Live handles immediate operations; Mixer handles detailed audio; Doctor handles check details and baseline comparison; Stats handles health and recent events. Graph remains a useful structural view, with links into the same source/scene context. Compact windows use a context sheet/tab while preserving active outputs; exact breakpoints require rendered measurement.

Three interaction rules carry most of the value:

1. **Inspection never performs an output action.** Keep selection, staging, Take, source mutations and output controls visually and behaviorally distinct. An optional command palette should initially find/inspect known targets; it is not a prerequisite for source search.
2. **Describe scope where the action lives.** Show the inspected scene even when it differs from Program. Distinguish item ownership, shared source settings, structural usage and reported activity. Use text and icons as well as color.
3. **Show evidence beside status.** Pending and unconfirmed are not success; incomplete checks are not a pass. Keep feedback local and bounded instead of producing a toast for every update.

Reuse native GTK controls, existing focus behavior and audio cards. Require keyboard operation, accessible labels and readable alternatives for custom visualizations in each chosen slice. The first-pass adaptive-shell and accessibility tasks still apply; they need not delay every new OBS control.

## Delivery order and implementation boundaries

This is a feature sequence, not a list of session-sized coding tasks. The existing UI task breakdown remains intact. Before implementing a new feature, split its actual file surface and include the protocol, state, UI, locale and verification work; do not describe an entire cross-layer feature as a three-file change.

| Step | Concrete outcome | Why now | Validation gate |
|---|---|---|---|
| 0 | Retain U1 Disconnect clarity and U2 Live empty-state work | Prevent confusing existing operation before adding more controls | A disconnect cannot unexpectedly stop active outputs without the chosen explicit interaction contract |
| 1 | C1 source visibility with ownership and scene-local search; narrow pause and disk-space slices can proceed independently | Closes a common gap and establishes useful context without a new framework | Duplicate items, shared containers, external changes, failures and scope changes are handled |
| 2 | Read-only “Used in…” plus the first small preflight | First distinctive release outcome: find the affected setup and check an intentional expectation | Fresh/incomplete data is honest; an operator can correct a seeded mistake from the report |
| 3 | Audio baseline comparison; individually watched inputs if demonstrated useful | Reuses explicit targets and scoped observations to improve repeatability | No cross-collection binding; no OBS writes during capture/compare; stable watch controls |
| Parallel parity track | C2 replay with results; C3 Studio/Take; C5 media and C6 filter toggles; narrow built-in text editing | Keep daily controls complete while validating a distinctive direction | Each is a separate end-to-end feature; source-wide effects and unsupported actions are clear |
| Later by workflow | Manual cues, recent activity timeline, selected-property restore | Larger benefits depend on show structure and actual troubleshooting needs | Demonstrated use before adding sequence state, history retention or multi-write behavior |

Studio can be prioritized earlier if transitions dominate the operator's work. Read-only preflight/audio baseline does not technically depend on Studio, screenshots or full collection-wide discovery. Optional images follow a measured performance spike; no whole-collection continuously refreshed multiview is needed for the proposed source/check workflows.

Preserve these architectural boundaries:

- Keep `obws` inside `src/obs/`; map typed results into pure domain state, then use the existing command/event path and GTK ownership model.
- Add only the data needed for the selected vertical slice. Do not first create a universal action registry, new event bus, generic rule engine or second application store.
- Reuse existing scoped command failures, output pending guards, bounded discovery and stale selected-Mixer response handling. Add session/collection invalidation where the new feature requires it; existing scene names are insufficient persistent identity for cross-collection baselines.
- Keep screenshots and topology refreshes bounded and cancellation-aware. High-volume meters are already droppable to protect control events (`src/controller/refresh_controller.rs:255`); new image/history work must not crowd that channel.
- Local baselines/preferences use the established background I/O path. A full database or remote service is unnecessary for the first version.

## Protocol feasibility and traps

The installed `obws` 0.15.0 client already contains the native operations needed for these directions. API availability is only part of feasibility: most work is identity, state reconciliation and user interaction.

| Need | Native surface | What SceneDeck must supply |
|---|---|---|
| Source context/visibility | `GetSceneItemList`, `GetGroupSceneItemList`, `SetSceneItemEnabled`, source activity/events | Owner/item identity, complete usage paths, freshness and shared-container explanation |
| Setup checks/baselines | Existing audio/status reads and meters, `GetStats`, selected item reads | Explicit expectations, local scope/bindings, evidence and honest incomplete reads |
| Audio detail | Input monitoring, sync-offset, balance and track getters/setters | Conditional controls; explain OBS-host monitoring and shared effects |
| Text/media/filter tools | Input settings, media status/actions/cursor, filter list/enable | Small supported source-kind forms; seek validation; source-wide effect context |
| Staging/cues | Preview/Program scene requests, Studio mode, transition requests/events | Deliberate Stage/Take/Next semantics; no simulated isolation or automatic advancement |
| Capture results | Record/replay status and events; pause, save, split, chapter | Configuration-aware feedback, remote paths and request/observation distinction |

References: [scene-item handlers](https://github.com/obsproject/obs-websocket/blob/e28f84975984340e84fc4879cfc05741cc0afde1/src/requesthandler/RequestHandler_SceneItems.cpp), [input handlers](https://github.com/obsproject/obs-websocket/blob/e28f84975984340e84fc4879cfc05741cc0afde1/src/requesthandler/RequestHandler_Inputs.cpp), [recording handlers](https://github.com/obsproject/obs-websocket/blob/e28f84975984340e84fc4879cfc05741cc0afde1/src/requesthandler/RequestHandler_Record.cpp), [protocol](https://github.com/obsproject/obs-websocket/blob/e28f84975984340e84fc4879cfc05741cc0afde1/docs/generated/protocol.md).

Additional constraints:

- Preserve `GetVersion.availableRequests`; SceneDeck currently maps only version strings (`src/obs/mapper.rs:19`). A request being listed does not prove a source/output supports the operation.
- Native snapshots are still images, not a video/audio feed. Fetch only visible targets, bound in-flight work, mark image age and measure OBS render cost before choosing a rate. [Screenshot implementation](https://github.com/obsproject/obs-websocket/blob/e28f84975984340e84fc4879cfc05741cc0afde1/src/requesthandler/RequestHandler_Sources.cpp#L147).
- OBS source activity is not a pixel-visibility or audio-delivery test; UI activity can include projectors/properties, not just Preview. [Source activity implementation](https://github.com/obsproject/obs-websocket/blob/e28f84975984340e84fc4879cfc05741cc0afde1/src/requesthandler/RequestHandler_Sources.cpp#L112).
- Generic input settings are source-kind-specific. First support a known built-in text kind using explicit Save and overlay only changed fields; do not treat a settings JSON object as a complete editable schema.
- A fresh-read operation must retain failed subreads. Existing best-effort audio discovery skips a failed nested container and defaults an enabled-state read failure to true (`src/obs/client.rs:337`, `:360`); reusing that result as complete readiness evidence would be wrong.
- OBS remains authoritative. Reconnect does not replay queued mutations, baseline comparison does not repair anything, and cue selection does not imply completion.

## Focus and assumptions

The three strongest directions form one coherent product path: **understand the current setup → check intended conditions → compare against a known rehearsal**. They share narrowly scoped observations, not a requirement for a generic automation platform.

| Assumption | Low-cost validation | Result that changes the plan |
|---|---|---|
| Nested/shared sources are a recurring operating problem | Observe a representative collection and a lower-third/microphone operation | Small simple collections favor output/Studio parity first |
| Preflight expectations catch useful mistakes | Seed muted/missing/stale cases and watch correction without coaching | Noise or ignored checks means reduce the checks, not add automatic blocking |
| Rehearsal state matters across sessions | Compare one intentional setup after normal preparation changes | Mostly improvised sessions favor watchlist and ordinary controls |
| A desktop view is available during operation | Observe whether the operator can focus SceneDeck while presenting/gaming | Unfocused operation needs a separate shortcut study; existing shortcuts are window-local |
| Images improve decisions enough to justify polling | Benchmark one visible Program/Preview pair on actual OBS | Use on-demand images or omit them if latency/render cost harms the show |

Do not build initially: a full scene/composition editor; arbitrary macro language; timed cue automation; “panic restore everything”; universal filter/property forms; multi-OBS orchestration; an unrestricted dashboard designer; full-motion preview; cloud history; replay-file transfer/editing. Native extra canvases, virtual camera, saved OBS endpoints and hotkey favorites remain valid demand-driven additions, but are weaker reasons to choose SceneDeck than the core workflow above.

## Review and verification

The scope review rejected three misleading claims: a media watchlist would be unique, cached Doctor results establish readiness, and a settings preset can be restored transactionally. An independent review also identified a missing freshness contract for baseline capture/comparison; the report now requires fresh scoped reads, observation times and invalidation during scope changes. The plan starts with explicit context, fresh read-only checks and selected-property comparisons. Existing Mixer, meters, Graph, Doctor, scene roles and statistics were treated as implemented foundations.

For implementation, meaningful fixtures must cover duplicate source instances, a shared nested container, failed subreads, external mutations, collection changes during a read, renamed/missing inputs, delayed output events and reconnect gaps. Follow `CLAUDE.md` for Cargo/Clippy checks and isolated locale parity; exercise chosen flows against real OBS and rendered GTK, including keyboard/large-text cases. These are future acceptance gates, not test results from this research.

This pass inspected pinned source and local architecture, checked document links/formatting and verified preservation of the four first-pass report/plan files. It did not run SceneDeck/Blade/obs-web/Companion, launch OBS, measure comparative performance, validate accessibility at runtime or run the full Cargo suite. No application source changed. The largest remaining uncertainty is which repeated operator tasks matter most; the proposed order is an evidence-informed recommendation, not validated market demand.
