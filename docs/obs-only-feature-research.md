# SceneDeck: OBS-only feature research

Research date: 2026-10-18. SceneDeck snapshot: `003d9ba1aa72cdbddb16884c9eb0a8b97704418b`, version 0.4.0.

This is research and a proposed roadmap, not an approved implementation specification. No application code was changed. Three agents independently examined SceneDeck's backend, its UI/UX, and OBS Blade. The synthesis checks existing features before recommending additions. The user's follow-up explicitly prioritizes UI/UX comparison and an improvement plan; [the plan](../tasks/plan.md) therefore separates improvements to existing interactions from new OBS capability epics.

## Scope and evidence

The product question is: **How might SceneDeck let a Linux OBS operator run more of a show without returning to OBS, while keeping the operating interface fast and understandable?**

Confirmed constraints: OBS WebSocket communication only; no Twitch/YouTube/other chat, service accounts, platform APIs, or third-party integrations. Local preferences, saved connection records, and local telemetry history fit the boundary. Plugin/vendor APIs do not.

The working audience is a person operating their own OBS session from a Linux desktop, possibly with OBS on another computer. This is an assumption pending the user's answers. Proposed success: fewer trips to OBS for source visibility, recording/replay actions, and staging scene changes, with OBS remaining the authority for actual state. This is not yet a measured usability result.

Evidence levels used below:

- **Implemented:** traced through actual source, not inferred from a README or enum alone.
- **Partial:** some state/reads exist, but the user cannot perform the complete action.
- **Proposal:** our recommendation, not a claim about either application's current functionality.
- **Unverified at runtime:** source was inspected, but no live OBS session or rendered GTK interaction was exercised.

## What SceneDeck already does

| Area | Current implementation | Evidence in this snapshot |
|---|---|---|
| Connection | One host/port endpoint; password in Secret Service; connect/disconnect and version display | `src/storage/config.rs:77`, `src/storage/secret.rs:11`, `src/controller/session_controller.rs:62` |
| Scene operation | Discover scenes, show current and previous program scene, directly switch program; only locally classified Primary scenes appear as Live buttons | `src/obs/client.rs:102`, `:150`; `src/ui/pages/live.rs:476`; `src/ui/widgets/scene_card.rs:157` |
| Scene organization | Local roles, icons, accent colors, drag order and YAML registry import/export; does not require renaming OBS scenes | `src/ui/pages/inventory.rs:94`, `:248`, `:441` |
| OBS configuration selection | Searchable profile and scene-collection dropdowns; updates on OBS changes | `src/ui/sidebar.rs:109`, `src/controller/refresh_controller.rs:280` |
| Live audio | Global inputs first; enabled active-scene inputs, nested scenes and groups; deduplication, mute and volume | `src/obs/client.rs:281`, `:408`, `:429` |
| Audio interaction | Vertical dB faders, live per-channel meters, peak behavior, fine adjustment, local fader locks and custom icons | `src/ui/widgets/audio_card.rs:73`, `:203`, `:284`; `src/ui/widgets/volume_meter.rs:100` |
| Dedicated Mixer | Active, Selected and Pinned scene modes; inspect another scene without switching program; search/grouping and persisted selection preferences | `src/domain/mixer.rs:9`, `:104`; `src/ui/pages/mixer.rs:68`; `src/controller/app_controller.rs:174` |
| Stream/record | Persistent start/stop buttons; pending guards, state events, configurable confirmations, elapsed displays and recording paths | `src/ui/sidebar.rs:270`, `:350`; `src/controller/output_controller.rs:52`, `:96` |
| Telemetry | FPS, CPU/memory, render timing, missed/skipped frames, congestion, bitrate; gauges/charts; detachable Stats window | `src/domain/stats.rs:10`; `src/ui/pages/stats.rs:56`, `:311`; `src/ui/window.rs:944` |
| Telemetry history | Approximately two minutes of in-memory samples, not a durable session archive | `src/domain/stats.rs:131` |
| Graph | Nested-scene dependency visualization with movable graph nodes; read-only with respect to OBS | `src/obs/client.rs:111`; `src/ui/pages/graph.rs:42`, `:141` |
| Doctor | Structural checks for roles, stale scene metadata, protected live scenes, dependency rules and cycles; explanations rather than automatic repair | `src/services/doctor_service.rs:20`; `src/ui/pages/doctor.rs:59` |
| Shortcuts | Configurable scene digit/modifier/leader shortcuts while the SceneDeck Live window receives keyboard input | `src/services/hotkey_service.rs:61`; `src/ui/hotkeys.rs:34` |
| Appearance/onboarding | Native themes, system/light/dark, high contrast, motion preferences, light/dark custom CSS, Fluent localization, welcome and Help | `src/ui/theme.rs:54`, `:229`; `src/ui/pages/settings.rs:40`; `src/ui/window.rs:359` |

The README underreports the separate Mixer. The completed historical roadmap explicitly documents it: `docs/improvement-roadmap.md:3`, `:27`. Mixer pinning, metering, grouping, themes, output confirmations and basic statistics must not reappear as missing-feature proposals.

Partial capabilities matter:

- Recording pause is recognized in state, but there is no Pause/Resume command (`src/obs/mapper.rs:113`, `src/controller/command.rs:5`).
- Scene-item visibility is queried during audio discovery, but SceneDeck cannot change it (`src/obs/client.rs:351`).
- Graph data keeps nested scene edges; it is not a source tree containing every item ID, visibility flag, filter or transform.
- SceneDeck shortcuts are not OBS's registered hotkeys. Those are distinct operations.

## How the UI/UX is built

The desktop UI is Rust GTK4/libadwaita, not a browser frontend. `site/` is a separate static marketing site. Widgets are built imperatively, with custom Cairo drawing for graphs, charts and audio meters.

The shell has a persistent sidebar, a page stack, a header and a bottom status bar (`src/ui/window.rs:38`, `:65`, `:144`; `src/ui/sidebar.rs:21`). Pages are Live, Mixer, Graph, Inventory, Doctor, Settings, Help and Stats. The default window is 1100×740. The header carries refresh and profile/collection selectors. Connection and stream/record controls remain available across pages.

Live is currently a button deck above an audio section, separated by a resizable divider. Wrapping cards and independently scrolling panes support different sizes. Its program display is a **scene name**, not an image. Scene buttons immediately change program. Selected/previous/ready text, icons, accents and shortcut badges supply recognition without thumbnails (`src/ui/pages/live.rs:187`, `:213`; `src/ui/widgets/scene_card.rs:64`, `:136`, `:157`).

The architecture already provides a suitable extension path:

```text
GTK signal → NavigationContext → AppCommand → controller → ObsClient
                                                        ↓
GTK updates ← AppEvent channel ← domain mapping ← OBS response/event
```

GTK owns widgets and `Rc<RefCell<AppState>>`; Tokio owns network work. A 50 ms GTK timer drains events. Only `src/obs/` imports `obws`. Local persistence uses background I/O and serialized write lanes separately from OBS commands. Preserve these boundaries instead of introducing a second state store or a new frontend framework.

Useful UI improvements to accompany features:

- Put source operations in a Live inspector, beside the scene context. Inventory remains the local organization view.
- Keep compact audio strips; advanced settings belong in a small details popover/dialog.
- Preserve immediate switching in Direct mode. Studio mode must visibly distinguish Preview from Program, with a separate Take action and unambiguous keyboard behavior.
- Start with fixed sections and a few show/hide preferences. A general dashboard editor would add substantial layout and persistence work before delivering missing OBS operations.
- Verify narrow windows before promising compact mode. Content reflows, but the shell has no explicit breakpoint policy, and the bottom status is a horizontal box (`src/ui/window.rs:144`, `src/ui/widgets/status_bar.rs:71`). Static inspection does not prove clipping.
- Add keyboard alternatives for pointer-only Inventory reordering and Graph node movement. Custom DrawingArea meters/graphs also need accessible value/structure review (`src/ui/pages/inventory.rs:248`, `src/ui/pages/graph.rs:98`, `src/ui/widgets/volume_meter.rs:100`). This is an existing usability concern, not a reason to rebuild GTK.

## OBS Blade comparison: useful capabilities and UX advantages

OBS Blade is a Flutter phone/tablet remote. Its current [README](https://github.com/Kounex/obs_blade) distinguishes current `master`, older `foss` 3.2.0 builds, and WebSocket-4-era `legacy`. This research concerns current published `master` material, accessed 2026-10-18, not an installed release test. A reproducible commit SHA could not be obtained; some cached raw files were older than current documentation, so source certainty is called out rather than implied.

The [official product site](https://obs-blade.kounex.com/) advertises version 4.1, adaptive phone/tablet dashboards, optional/reorderable sections, source/media controls, and saved session charts. These are product claims supported by public materials, not measured usability results. The source references below distinguish concrete wiring where inspected.

| Capability / operating experience | OBS Blade evidence | SceneDeck today | Implication for SceneDeck |
|---|---|---|---|
| Scene switching | Existing dashboard/store flow | Implemented, role-curated | Keep SceneDeck's useful curation |
| Program image | Screenshot-based preview in store | Scene name only | Optional image adds confidence |
| Studio Mode | Preview/program/studio observables and requests | Missing | Explicit stage → Take workflow |
| Transition controls | Current/available transition state and duration handling | Missing | Choice, duration and Take before advanced T-bar work |
| Scene source visibility | Scene-item/group state and enable events | Read during audio discovery only | Nested source inspector is a major functional gap |
| Audio meters/mute/faders | Implemented | Implemented, with a dedicated Mixer | Existing strength; no replacement needed |
| Stream/record start/stop | Implemented | Implemented with safeguards | Preserve SceneDeck's guards |
| Recording pause/resume | Paused state; verify action wiring in source notes | State recognized; no action | Separate recording action from Stop |
| Replay / virtual camera | Dedicated status and output state | Missing | Add by operator demand; replay save has distinct feedback |
| OBS profiles/collections | Dashboard selectors | Implemented in header | Improve placement for narrow windows |
| OBS registered hotkeys | Hotkey list is requested and stored | Local scene shortcuts only | Optional action launcher, not a macro engine |
| Media transport | Product feature and source media-item classification | Missing | Reuse source context rather than add another page |
| Audio sync / source filters | Source/store imports, getters, batches and events | Missing | Validate individual edit widgets before claiming full editor parity |
| Session archive | Saved stream/record data and history UI | Two-minute in-memory history | Useful later; distinct from current live Stats |
| Dashboard arrangement | Product documents show/hide/reorder | Fixed pages/panes | A few section preferences may suffice |
| Tablet simultaneous context | Product documents side-by-side layout | Desktop has room, but no source/preview context | Borrow grouping and persistent context; retain desktop input patterns |
| Reconnect honesty | Current architecture describes disabled mutations and “LAST KNOWN” badges | Disconnected view / disabled outputs | Make unavailable/stale states explicit wherever new controls appear |
| Extra canvases | Native canvas support described alongside vendor additions | No canvas selector in app | Native-only later candidate; Aitum controls excluded |

Source anchors: [DashboardStore](https://github.com/Kounex/obs_blade/blob/master/lib/stores/views/dashboard.dart) contains output/studio/transition/input/preview state, initial request wiring, media-item classification and history hooks. The newer [WebSocket architecture](https://github.com/Kounex/obs_blade/blob/master/docs/obs-websocket-architecture.md) describes mutation acknowledgement, failed-write reconciliation, read ordering and stale-state badges. Its newer design must not be inferred from an older cached DashboardStore file. The current [record/stream service](https://github.com/Kounex/obs_blade/blob/master/lib/views/dashboard/services/record_stream.dart) explicitly sends Start or Stop after the relevant confirmation, preserving the direction the operator confirmed if OBS changes state in the meantime.

The independent Blade agent verified README-level behavior; the parent separately inspected the site, architecture documentation and available raw source. A local clone and exhaustive widget audit were not possible because shell networking could not resolve GitHub. Consequently, exact pause/filter/media/hotkey widget behavior, a full filter editor, and editable source-text controls are not claimed as implementation-verified parity targets. No runtime-quality superiority is established.

OBS Blade's strongest design lesson is **keeping the thing being controlled beside its relevant actions**. That reduces the context an operator must remember between screens. Source rows can carry visibility and media actions; Preview has its own identity; output actions are distinguished from scene navigation. This is our design assessment, not a timed comparison or a claim that every Blade interaction is superior.

SceneDeck has its own advantages: desktop keyboard scene control, local scene roles/order/icons, a separate Mixer for inactive or pinned scenes, and Graph/Doctor tools. Its native GTK controls and Secret Service storage already fit Linux. Replacing those with a mobile-style dashboard would discard useful product decisions.

## Proposed desktop UX

Make **Live the operating workspace**, Mixer the detailed audio workspace, Stats the health workspace, and Inventory/Graph/Doctor the preparation tools. Place operational pages first in navigation. Do not add a new sidebar page for every OBS feature.

```text
Wide Live
┌ Navigation ┬ Program: Main        Collection / Profile   Connection ┐
│ Live       │ Scene deck                  │ Sources for: Main       │
│ Mixer      │                             │ [eye] Camera            │
│ Stats      │                             │ [eye] Overlay           │
│ …          │ Compact audio strips        │ Selected-source details │
│ Outputs    │                                                     │
├────────────┴─────────────────────────────────────────────────────┤
│ Output state / elapsed                    health summary          │
└──────────────────────────────────────────────────────────────────┘

Studio adds: [Preview: selected scene] → [transition / Take] → [Program]
Compact mode: navigation collapses; output actions stay in content chrome;
              Sources / Audio share contextual tabs below the scene deck.
```

This is a target wireframe, not a claim about existing controls or an exact widget specification. Source controls and Studio sections appear only as their corresponding OBS capabilities are implemented.

Interaction rules:

1. **Scene selection:** Direct mode retains current immediate program switching. Studio mode clearly labels Preview selection, Program tally and Take; keyboard hints describe the current action. Do not rely on color alone.
2. **Sources:** The inspector names the inspected scene independently of the on-air scene. Each visibility control affects a specific scene-item instance. A selected source exposes only supported details; global source effects are identified.
3. **Outputs:** Keep action and status together. Pause, Stop and Save replay have different labels and feedback. Compact layout must not hide active outputs inside collapsed navigation. Keep a single update path for expanded/compact presentations.
4. **Feedback:** Distinguish disconnected, connecting, refreshing, stale, unsupported, pending and failed states. New controls cannot silently operate on last-known scene/collection data. Avoid toast storms and replacing whole panels after every fader event.
5. **Discovery:** If OBS has scenes but none have the Primary role, explain the filter and offer an Inventory link. Do not silently assign roles. Existing roles remain useful.
6. **Accessibility:** Preserve native faders/buttons and visible focus. Provide up/down alternatives to dragging, source-specific accessible names, a readable Graph relationship list, and meter values without announcing every animation frame.
7. **Adaptation:** Choose breakpoints from rendered measurements. Test the current 1100×740 default, 800×600, and a proposed 640×600 compact target, with long labels and large text. These are validation targets, not a newly promised minimum size.

## Native OBS feasibility and engineering constraints

The installed `obws` 0.15.0 source already exposes typed client operations for screenshots, Studio Mode, transitions, scene-item enable state, recording pause/resume/split/chapter, replay buffer, virtual camera, filters, media, audio settings and OBS hotkeys. Most candidate features therefore need new application state, routing, event handling and UI, not another protocol dependency.

The [official protocol](https://github.com/obsproject/obs-websocket/blob/master/docs/generated/protocol.md) confirms screenshot requests, source-item operations, Studio Mode, output and media actions. Screenshots are encoded still images, not a video stream. `GetVersion.availableRequests` enables capability checks; file split/chapter APIs arrived later than baseline v5.0, and recording format can constrain chapters. Treat source-specific media/settings as conditional.

Implementation decisions that cannot be skipped:

1. **Item identity:** visibility belongs to a particular scene-item instance. Use its owning scene/group and item ID; the same source can appear twice. Do not deduplicate scene items using the mixer input-name algorithm. Scene/profile/collection changes must invalidate old selections and replies.
2. **Authoritative state:** apply OBS events and reconcile command responses. A failed operation must not look like a lost connection or leave an optimistic toggle permanently wrong.
3. **Bounded preview work:** capture only visible previews; use modest resolution and a measured refresh rate, at most one in-flight request per target, and cancel on disconnect/page or target change. Decode away from GTK. Mark stale/failed images. Measure screenshot CPU/bandwidth separately from existing one-second statistics polling.
4. **Shared-source side effects:** input volume, filters and media transport affect the source wherever it is used. Staging another scene does not isolate every source setting from Program. Explain that where the user acts.
5. **Session lifetime:** attach new polling work to the existing session and stop it on replacement/disconnect. The UI event channel is bounded to 128; current disposable meter frames use `try_send` (`src/app.rs:33`, `src/controller/refresh_controller.rs:255`). Do not create an unbounded screenshot/event backlog.
6. **Capabilities:** SceneDeck currently retains only version strings (`src/obs/mapper.rs:19`). Preserve available requests when required and disable unsupported actions with a reason. Request presence does not guarantee an output is configured, a format supports an action, or a source is seekable.
7. **Remote paths:** recording, replay and saved-screenshot paths refer to the OBS machine. Do not present them as locally openable files without proving that mapping.

## Brainstorm: six useful variations

These are alternatives to discuss, not six simultaneous commitments.

| Variation and lens | Operator story | Value and hardest assumption |
|---|---|---|
| 1. Complete the operating desk — simplification | Show/hide a camera or overlay, pause recording, save a replay, remain on Live | High immediate utility. Assumes these are the repeated reasons the operator returns to OBS. |
| 2. Rehearse, then Take — expert lens | Stage a scene, inspect Preview, choose transition, send it to Program explicitly | Strong safety/production value. Requires clear Direct/Studio semantics and preview performance testing. |
| 3. Source-focused control — combination | One selected source exposes visibility, media transport, filters and advanced audio | Reuses a source inspector. Risk: shared inputs and per-scene items have different identities and side effects. |
| 4. A small OBS action deck — inversion | Expose a curated set of registered OBS actions as named buttons instead of building every editor | Broad reach for less custom UI. Risk: ambiguous hotkeys and duplicate output actions can weaken safety. Start with explicit actions, not a macro language. |
| 5. Know how the show went — audience shift | Preserve a local session summary with health issues and replay/recording paths | Helps post-show diagnosis. Assumes retrospective data is more valuable than more live controls; disconnect creates incomplete observations. |
| 6. A personalized cockpit — constraint removal | Arrange every OBS control, preview and chart for each workflow | Flexible, but highest UI/persistence cost and weak evidence of need. Prefer fixed layout plus a few choices first. |

The first three cluster into two serious directions: **operational completeness** (1+3) and **staged production** (2). The action deck and history are smaller later extensions. A general cockpit builder is deferred. This is preliminary evaluation pending the user's reaction, not a claimed final convergence.

| Direction | User value | Feasibility / failure assumption | Differentiation |
|---|---|---|---|
| Operational completeness | Frequent source/output actions stay in SceneDeck | Moderate work; value falls if these actions rarely occur during a show | Combines new controls with SceneDeck's existing role-curated deck and Mixer |
| Staged production | Operator can prepare a scene before taking it live | Higher state and screenshot cost; fails if visual confidence is too stale or mode behavior is ambiguous | Brings production staging into the current compact desktop console |
| Configurable cockpit | Personal arrangement for many workflows | High UI/persistence effort; weak evidence that fixed sections are the bottleneck | Limited advantage before missing OBS operations exist |

## Provisional recommendation and release order

Pending the user's workflow answers, prioritize operational completeness, then staged production. For the UI/UX-focused first increment, the strongest combination is **explicit Disconnect behavior + useful empty Live state + source visibility inspector**. Recording pause/replay can follow as separate slices. Compact-shell work is useful but must not become a prerequisite for every new control. Preserve SceneDeck's curation, Mixer, Graph and Doctor rather than reproduce OBS's full configuration interface.

| Order | Candidate outcome | Why this order | Dependencies and release test |
|---|---|---|---|
| Before expansion | Decide Disconnect semantics; strengthen focused OBS fixture cases | Current detach action stops outputs; more output kinds would widen the ambiguity | Verify actual button/confirmation flow, reconnect ownership and output-failure recovery |
| First increment | Explicit Disconnect confirmation, useful Live empty state, scene-item visibility inspector | Immediate safety/discovery gains plus one missing operation, without a continuous image pipeline | Item identity/events; duplicate items/groups, external changes and failures tested |
| Separate output slices | Recording pause/resume; native replay start/stop/save | Concrete actions that can follow independently as operator priorities allow | Existing output guards; unsupported/unconfigured output and failed-save tests |
| Next release | Explicit Studio workflow, Preview/Program state, transition choice/duration/Take, bounded image previews | A coherent new interaction model; materially more state and performance risk | Direct mode unchanged; preview selection never changes program; out-of-order image replies ignored |
| Following slices | Filter toggles; source media transport; advanced audio details | Reuse established source context and selection handling | Distinguish input-level side effects; test unsupported sources and seek bounds |
| Demand-driven | Virtual camera, curated OBS hotkeys, recording split/chapter, saved endpoints, local session history | Useful but audience-dependent | Capability/output configuration checks; keyring migration for endpoints; honest history gaps |

This is feature sequencing, not a claim that each row fits one coding session. A complete feature touches protocol/domain/controller/UI/locales/docs. Once a direction is approved, split delivery into small support and user-facing tasks with explicit acceptance criteria; do not hide a ten-file change under a misleading “small task” estimate.

## Assumptions to validate before implementation

| Assumption | Validation | What would change the recommendation |
|---|---|---|
| The target operator often returns to OBS for sources/outputs | Observe or list actions during one representative session | If mistakes during scene changes dominate, put Studio Mode first |
| WebSocket snapshots provide enough confidence | Prototype one Program/Preview pair and measure OBS frame time, CPU, transport and latency | Lower refresh/resolution, on-demand snapshots, or drop continuous previews |
| Direct and Studio workflows can coexist clearly | Walk through scene buttons and shortcuts in both modes, including remote changes | Separate controls more strongly; avoid implicit mode-dependent shortcuts |
| Native replay is configured and useful | Confirm real OBS replay settings/output errors | Substitute recording pause or virtual-camera work based on actual usage |
| Local session data is sufficient | Compare a session summary to what OBS was doing through reconnects | Mark gaps explicitly; postpone history rather than imply complete capture |

## Not doing

- Chat, platform logins, follower/activity feeds or external service APIs: expressly excluded.
- Aitum or other plugin/vendor controls: third-party integration, even when transported over OBS WebSocket.
- A full OBS scene editor, source creation/deletion, transform designer or generic settings forms in the first release: much greater risk and UI surface than the requested live controls need.
- A macro/scripting engine or a general dashboard framework: speculative infrastructure before validating ordinary actions.
- Another Mixer, basic stats page or theme overhaul: these already exist.
- Full-motion video: out of scope; native screenshot requests provide still frames, not a video transport.

## Verification performed and limits

SceneDeck source and documentation were inspected at the recorded snapshot. The bundled product screenshots were viewed as historical visual context; current SceneDeck behavior conclusions come from source. OBS Blade evidence is dated moving-branch material with the limitations above. The existing storage atomic-write unit module was compiled independently with Rust 1.97.0, edition 2021; all five tests passed. This is a narrow check for the requested review, not a complete application validation.

No GTK app launch, live OBS calls, packet capture, screen-reader test, screenshot-performance benchmark, full Cargo suite or OBS Blade build was performed. Existing roadmap follow-ups already call for broader WebSocket mutation fixtures and rendered GTK interactions (`docs/improvement-roadmap.md:54`). Proposed priorities remain conditional on operator needs.
