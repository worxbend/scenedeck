# SceneDeck UI/UX redesign plan

**Status:** all 31 implementation tasks complete; native fixture qualification,
independent review and exact remaining limits recorded in the
[acceptance audit](docs/verification/redesign-2026-10-19/acceptance-audit.md).
**Updated:** 2026-10-19, incorporating the user's latest layout, feature-removal,
preview, theme and Inventory feedback. **Outcome:** final gates pass (454 default tests and 9 isolated native/locale
tests); screenshots, performance measurements, limitations and cleanup are indexed.

The delivery goal is a simpler, modern, polished native desktop interface.
Feature count and passing Rust tests do not establish that the design succeeds.
Every phase must produce an inspectable running-app result and visual evidence.

Execution checklist: [tasks/todo.md](tasks/todo.md). Design decisions:
[scene-wall brief](docs/ideas/live-scene-wall.md). Earlier research and delivery
remain available in [history](#history); their completed boxes do not apply to
this redesign.

## 1. Decisions from the user

| Feedback | Required result |
| --- | --- |
| Live is overloaded and poorly styled | Scene switching becomes the dominant task; remove the two large tools/workflow panels and nested-pane layout |
| Scene previews belong on scene cards | Shaded images inside the cards, with fully opaque readable text over them |
| Preview behavior belongs in Settings | One scene-preview preference; cards become larger automatically when enabled |
| Studio is useful and belongs in the topbar | Topbar toggle controls actual OBS Studio Mode and follows external changes |
| Studio needs transition configuration on Live | A compact transition/duration/Take row appears above the grid while Studio is enabled |
| Audio should be compact and expandable | A short audio strip below the grid; detailed mixing remains on Mixer |
| Virtual camera belongs on Live | Native start/stop in the compact output bar with stream, record and replay |
| Rehearsal, setup and show are useless | Remove baselines, setup expectations and manual cues entirely; no Show destination or replacement forms in Settings |
| Inventory roles render as `...` | Wider list, readable role column, genuinely resizable columns and a useful compact layout |
| Themes must follow the redesign | Update all 12 built-in families, both variants, plus custom-theme examples and compatibility documentation |
| Cards need a modern treatment | Consistent tiles, intentional typography/spacing, soft depth, restrained borders and clear state; validate previews both on and off |

Research stays OBS-only. Chat, platform APIs, vendor/plugin integrations, macros,
a full composition editor and additional preparation/workflow systems remain
outside scope.

## 2. Visual references and hierarchy

- [User's card reference](docs/ideas/scene-cards-user-reference.png): use its
  aligned compact tiles and restrained state outline as reference for preview-off
  mode. Improve its typography, spacing, sizing and interaction states.
- [Scene-wall concept](docs/ideas/live-scene-wall-concept.png): image-backed cards,
  quiet chrome and compact audio. This is an exploratory generated concept.
  Correct its Studio switch placement, remove Show navigation, add virtual camera
  and make audio expandable before treating it as an implementation target.
- [Inventory defect](docs/ideas/inventory-role-truncation.png): role values lose
  space to secondary controls. Fix the layout rather than relying on tooltips.
- [Previous running-app captures](docs/verification/2026-10-19/README.md): evidence
  of the earlier implementation, not an approved visual target.

The eye should find scenes first, the staged/Program state second, then the
transition action and quick audio/output controls. Avoid persistent prose,
redundant headings, repeated status pills and multiple copies of the same action.

### Target Live composition

```text
Window topbar     Live                 Studio [switch]       Sources

                  Fade [selector]    300 ms [duration]       Take
                  (only when Studio is enabled)

                  [ scene card ] [ scene card ] [ scene card ]
                  [ scene card ] [ scene card ] [ scene card ]

                  Audio   microphone / desktop / music       Expand

Compact output bar    Stream    Record    Replay    Virtual camera
                      connection / important feedback kept discreet
```

The scene grid has one scrolling region. No right-hand operating panel, Setup &
show tabs, preparation forms, standalone preview monitor or split handle on Live.
The audio strip is separate from grid scrolling. Avoid the earlier combination
of nested scrollers, stacked output toolbars and sidebar action duplicates.

### Component contract

| Component | Visual / interaction target |
| --- | --- |
| Preview-off card | Compact tile, approximately 140–180 × 80–104 logical px; scene name is dominant |
| Preview-on card | Approximately 240–300 px wide with a 16:9 image area; final sizes respond to viewport/text scale |
| Preview image | Clipped to rounded corners; preserve source aspect ratio, using a matte for unusual canvases; no stretching |
| Shading | Independently shade the image, initially exploring 60–75% image visibility plus a stronger lower scrim; text stays fully opaque |
| Title | Bottom-left on image-backed cards; up to two lines; readable over bright and detailed imagery; full name accessible and available in tooltip |
| Normal card | Scene name and optional shortcut/icon; no Ready, Last, On or action-explanation paragraphs |
| Program / Preview | Small named badges and restrained state border; both may appear on one card; no reliance on color alone |
| Registry accent | A small secondary accent; cannot replace the image, erase the scrim or masquerade as Program/Preview state |
| Keyboard focus | Distinct visible focus treatment, separate from OBS state; reserved outline space prevents layout jumps |
| Motion | Short, subtle response to interaction; honor Motion and desktop settings; no continuous card pulsing or image zoom |
| Audio | Compact mute, level, gain controls; explicit expansion for full faders; no baseline/watch configuration panel |
| Output bar | One compact action group; neutral inactive controls, clear active/pending states; preserve stop confirmations |

Delivery decision: previews remain off by default conservatively. The local
six/twenty-scene budgets pass; other computers and remote hosts remain outside
that measured fixture coverage.
A single **Scene previews** switch in Settings controls them without restarting.
Turning previews off stops capture work and returns compact cards. Do not add
opacity, shadow and refresh sliders to the main preference surface.

### State behavior

- Direct mode: card activation and scene shortcuts switch Program.
- Studio mode: card activation and shortcuts stage Preview; Take performs the
  transition. Show duration only for adjustable transitions.
- Unknown, failed or stale mode: disable the affected action and refresh. Never
  guess Direct mode. Pending toggles cannot send duplicate mode changes.
- Toggling Studio does not change the selected/Program scene as a UI side effect.
- Image freshness is independent of OBS control freshness. An image failure
  retains usable scene controls and an honest fallback; it never fabricates live
  video or output state. Old-session images must never appear on new scenes.
- Preserve keyboard focus and draft edits during refresh. Image updates must not
  rebuild the entire scene grid, Sources page or audio controls.

## 3. Navigation and feature scope

| Destination | Keep / redesign |
| --- | --- |
| Live | Scene wall, conditional transitions, compact audio and outputs |
| Sources | Dedicated focused page for owned scene items, visibility and selected input details; opened by a discreet topbar action |
| Mixer | Detailed audio control; preserve Active/Selected/Pinned behavior |
| Inventory | Full-width, resizable scene list for roles/order/icons/accents and registry import/export |
| Doctor | Existing structural diagnostics; remove saved setup-expectation results |
| Stats | Existing telemetry and bounded operational context; no Live activity log |
| Settings | Preferences, preview toggle, themes, connection and output safety |

Sources uses a scene/item list with contextual details; wide windows can show
both, compact windows drill into details with a clear Back action. Selecting a
source does not switch Program. Rework its presentation instead of pasting the
old operating accordion stack into another page.

Remove rehearsal capture/comparison, setup expectation configuration/checks,
manual cue lists and Show navigation. Preserve existing `workflows.json` files
on disk without reading/writing retired data as routine work. Shared scope,
activity and output-evidence types still needed by kept features remain intact.
Do not introduce watchlist configuration as a replacement workflow surface.

Keep native media, filters, built-in text editing and advanced audio in Sources.
Keep replay, recording pause/resume and virtual camera in the output controls;
less frequent split/chapter operations and result/path details use contextual
menus or popovers rather than permanent Live sections.

## 4. Theme redesign contract

The current implementation has **12 families / 24 stylesheets**, a common base
at `assets/scenedeck.css`, and provider/motion handling in `src/ui/theme.rs`.
Keep this native GTK/libadwaita architecture and existing saved theme IDs.

### Shared styling

- Geometry, spacing, typography and component states belong in the base CSS;
  theme files supply palette and intentional family-specific treatment.
- Use the system UI font, a small consistent type scale, 8/16/24 px spacing,
  roughly 12–16 px card corners and restrained shadows. Avoid web-only CSS,
  backdrop blur, external fonts and ornamental gradients across the app shell.
- Extend the existing named colors with semantic colors for image title/scrim,
  fallback, border, Program, Preview, focus and shadow. Keep Program/Preview
  distinguishable in every palette; a purple/red theme accent must not recolor
  every semantic state identically.
- Retain `.scene-card`, `.scene-card-active` and `.scene-card-hotkey` contracts.
  Add documented preview-image, scrim, Preview, audio-strip, transition-toolbar,
  output-toolbar and Inventory-column classes as actual widgets are introduced.
- Remove assumptions such as `.scene-card > box` when cards become overlays.
  Audit high-priority per-scene accent providers so they do not erase imagery.
- Keep `.audio-card` behavior on Mixer; new compact strip styling is scoped.
  Shared shell changes also need Mixer/Inventory/Doctor/Stats spot checks.

### Initial visual targets

| Role | Dark starting point | Light starting point |
| --- | --- | --- |
| Window | `#171A20` | `#F5F6F8` |
| Surface | `#22262E` | `#FFFFFF` |
| Foreground | `#F1F3F5` | `#1D242E` |
| Secondary text | `#ABB4C2` | `#5A6370` |
| Border | `#383F4B` | `#D7DDE5` |
| Interaction accent | `#6AA6FF` | `#2F6FED` |

These are starting values for the default SceneDeck treatment, not forced
palettes for Nord, Solarized or other families. Preview-title white/scrim colors
are independent of ordinary light-mode text. Validate rendered contrast before
finalizing values: normal text 4.5:1, large text 3:1, important boundaries/focus
3:1. High contrast uses solid title backing and stronger edges; shadows alone
cannot convey its state. Audio meter zone semantics stay consistent with OBS.

### Family batches

Each batch covers both `*-light.css` and `*-dark.css` for two families: four
files maximum. Preserve the family's identity while applying the shared design.

| Task | Families | Specific attention |
| --- | --- | --- |
| T02A | `adwaita-default`, `scenedeck-dark` | Native integration and reference dark/light treatment |
| T02B | `scenedeck-light`, `obs` | True light surfaces, OBS-adjacent palette without crowded chrome |
| T02C | `obsidian`, `nord` | Tonal separation, readable subdued text, image contrast |
| T02D | `dracula-inspired`, `solarized-dark` | Preserve palette character; strong semantic/focus separation |
| T02E | `high-contrast`, `stream-red` | Opaque title backing, explicit borders, red accent distinct from Program/stop |
| T02F | `studio-purple`, `ubuntu-violet` | Accent restraint, correct light counterparts and legible active controls |

Test live System/Light/Dark changes and custom CSS reload. Theme changes must
reuse thumbnail data, preserve selection/focus, and send no OBS mutations.
Update examples and document the supported selector changes; never rewrite
users' custom CSS files automatically.

## 5. Inventory correction

Replace the constrained `PreferencesPage`/crowded `ComboRow` layout in
`src/ui/pages/inventory.rs` with a full-width scene list, preferably native
`GtkColumnView` with resizable Name and Role columns. Preserve registry and OBS
behavior; this is a presentation change, not a scene editor.

- Use available content width instead of the narrow preferences-page clamp.
  Scene names expand, role labels have a measured minimum based on localized
  names, and column boundaries can be dragged. Default list width grows with
  the window; do not add an empty side pane just to create a resize handle.
- At normal widths, every role including Module, Secondary and Unassigned is
  visible in full. At compact widths, move secondary row actions into an
  accessible menu before sacrificing the role value. Keep long-name tooltip
  and role-description help separate from the selected role label.
- Preserve drag ordering and keyboard Move up/down, scene icons, accent selection
  and clear, stale-entry cleanup, and YAML import/export. Put import/export in
  a compact header action group rather than a large repeated explanatory row.
- Use a small accent chip and a clear unset state. A missing icon gets a sensible
  native fallback. Column resizing, focus and row selection survive refresh.
- Remember preferred column widths; clamp to the current viewport/text scale,
  restore when space returns and offer Reset columns. Persist only local layout
  preferences with the existing ordered write path.

## 6. Implementation tasks

Each task is S/M: a small reviewable unit with roughly 1–5 implementation files.
Paths below are expected seams, not permission to refactor whole modules. If a
sixth unrelated file or a second subsystem is needed, split the task before
editing. Coupled enum/dispatch changes must include exhaustive consumers and
leave a compiling, working checkpoint; never add success stubs or dead-code
allowances to stage unfinished work.

All implementation boxes are initially open in `tasks/todo.md`. The existing
autonomous-workflow authorization applies; checkpoints require actual visual
inspection and correction, not a new human-permission loop.

### V01

**Establish the visual target and fixtures.** Dependencies: none. Scope: S.
Files: design brief, concept/reference assets, a new visual-review evidence index.

Acceptance: revised light/dark references show Studio in the topbar, no Show,
virtual camera and both card modes; fixtures include bright/dark/busy scenes,
long names and both tallies; record the accepted layout/state checklist.
Verify: compare every reference to sections 1–5; document any difference before
coding. The first native card render must be compared against these targets.

### T01

**Introduce shared component styling.** Dependencies: V01. Scope: M.
Files: `assets/scenedeck.css`, `src/ui/theme.rs`, theme CSS reference.

Acceptance: semantic color defaults and scoped component classes exist; old
scene/accent selectors cannot override image layers; geometry and motion rules
have one owner and existing theme IDs/settings remain valid.
Verify: theme-focused tests, GTK CSS parse smoke, representative light/dark card
states. Remove implementation-mirroring assertions when they merely freeze the
old 132 px design; test meaningful state/contrast behavior instead.

### V02

**Render the new scene cards natively.** Dependencies: T01. Scope: M.
Files: `scene_card.rs`, `pages/live.rs`, base CSS, small fixture assets if needed.

Acceptance: compact and image-backed cards follow the component contract;
Program/Preview/focus/accent states coexist; image changes preserve the Button,
focus and keyboard action while names remain readable.
Verify: native GTK captures with previews on/off, bright images, missing images,
long names and both tallies on one card; Direct/Studio action regressions.
Existing OBS action logic remains authoritative while preview delivery is added.

### I01

**Make Inventory wide and genuinely resizable.** Dependencies: T01. Scope: M.
Files: `pages/inventory.rs`, base CSS, a small Inventory row helper if needed.

Acceptance: full-width list and draggable Name/Role columns replace the squeeze;
all role values remain legible; existing role/order/icon/accent/import/export
operations still work with the new rows.
Verify: reproduce the user's screenshot, select every role, resize columns/window,
then exercise drag and keyboard ordering and inspect persisted registry values.

### I02

**Preserve Inventory layout and compact usability.** Dependencies: I01. Scope: M.
Files: Inventory page/helper, `storage/config.rs`, config documentation.

Acceptance: column widths survive refresh/restart and reset correctly; narrow
windows relocate secondary actions while retaining role labels; invalid saved
widths clamp safely and older config remains readable.
Verify: long German/Ukrainian labels at 125%/200% text, keyboard column resizing
or an equivalent accessible width/reset control, refresh and restart cases.

**Checkpoint A:** native card screenshots match the revised direction; the
Inventory `...` defect is resolved at normal widths and remains operable compact.
Fix visual issues here before expanding data plumbing or restyling every theme.

### L01

**Create the dedicated Sources destination.** Dependencies: V01. Scope: M.
Files: page/state navigation, `ui/window.rs`, `ui/pages/mod.rs`, new Sources page,
existing inspector integration (split route registration if another seam arises).

Acceptance: one named topbar action opens Sources and navigation back is clear;
existing source controls remain reachable through that page; no Show destination
is added. New and changed English messages are included with their consumer.
Verify: open/back/keyboard navigation with connection, empty, loading and error
states; inspection never changes Program. Locale batches follow once copy settles.

### L02

**Replace the Live pane composition.** Dependencies: V02, L01. Scope: M.
Files: `ui/window.rs`, `pages/live.rs`, existing panel construction/export wiring.

Acceptance: remove the operating/workflow notebook and outer split from Live;
one responsive scene grid owns scrolling; audio/output areas remain reachable
without nested-pane clipping or persistent instructional prose.
Verify: six and twenty scenes at 1280×800, 1024×768, 800×700 and 640×600 logical
px; confirm no old panels or dead refresh timers remain attached to Live.

### L03

**Put Studio in the topbar and transitions on Live.** Dependencies: L02. Scope: M.
Files: `ui/window.rs`, new Studio control widget, Live page, inspector extraction,
widget exports (keep tightly coupled wiring in one compiling unit).

Acceptance: toggle reflects actual OBS mode including unknown/pending/error;
Live transition row appears only in Studio with correct duration availability;
card/shortcut staging and one Take action use existing scope/freshness guards.
Verify: external OBS mode changes, fixed/adjustable transitions, failed toggle,
rapid clicks, stale data and text-field shortcut suppression. No duplicate Studio
control remains inside Sources and no second Take remains in the output bar.

### L04

**Build compact expandable Live audio.** Dependencies: L02. Scope: M.
Files: Live page, audio-card/strip widget, meter widget as required, base CSS.

Acceptance: default strip exposes useful mute/level/gain without tall faders;
expansion provides fuller controls while Mixer stays intact; current audio
selection/routing and debounce semantics are preserved without workflow forms.
Verify: mute/gain both directions against OBS, external updates while dragging,
keyboard use, unknown/disconnected values and expansion without scroll/focus loss.

### L05

**Consolidate output controls.** Dependencies: L03, L04. Scope: M.
Files: `sidebar.rs`, `transport_bar.rs`, window integration, output-result popover,
base CSS (existing confirmation functions remain reused).

Acceptance: one compact output surface includes virtual-camera start/stop;
record pause/resume and replay actions are reachable with honest pending/results;
remove sidebar/toolbar duplicates and permanent result accordions.
Verify: native output states and capability absence; stop/cancel/confirm,
replay saved-path evidence, narrow layout and explicit stop-and-disconnect.
Keep less frequent split/chapter actions contextual; do not claim configuration
support merely because OBS advertises a request.

### R01

**Remove rejected workflow surfaces.** Dependencies: L02. Scope: M.
Files: Doctor page, workflow-panel construction/exports, UI workspace helpers,
user-facing help as needed (split UI removal from documentation batch).

Acceptance: no rehearsal, saved expectations, cue controls or Show navigation
remain; Doctor retains structural diagnostics; no equivalent forms move into
Settings or Sources.
Verify: navigate every page and inspect accessible/UI text; compare retained
Doctor diagnostics and confirm that old workflow files are untouched.

**Checkpoint B:** real OBS can be operated from the simplified Live page;
Sources is reachable; Direct/Studio/audio/output semantics pass. Capture light
and dark screenshots. No rejected workflow surface remains visible.

### P01

**Bound and measure scene-thumbnail requests.** Dependencies: V02. Scope: S.
Files: OBS screenshot adapter, existing workspace caller, benchmark evidence.

Acceptance: adapter accepts bounded thumbnail dimensions without breaking manual
still use; measure realistic bright/busy/moving scenes and local/slow-response
fixtures; choose resolution/rate from evidence before enabling periodic capture.
Verify: request shape and size limits; record payload bytes, request latency,
render-frame time and control responsiveness before/after. The old 20-capture
simple-fixture benchmark is background evidence only.

### P02

**Route independent scoped thumbnail work.** Dependencies: P01, V02. Scope: M.
Files: controller command/event/workspace/app-controller paths, GTK result branch.

Acceptance: lightweight thumbnail reads never trigger a full operating/preflight
scan; requests/results bind endpoint, collection, session and scene; cancellation
and timeouts cannot block control commands or apply old images after scope loss.
Verify: local WebSocket fixtures for delayed replies, reconnect/collection change,
rename/removal and failed screenshot. GTK accepts only the current target.

### P03

**Refresh visible card images without rebuilding.** Dependencies: P02, L02. Scope: M.
Files: Live page/card handles, window lifecycle, bounded preview scheduling/cache.

Acceptance: update existing card image layers; visible cards refresh fairly with
Program/Preview priority; hidden/offscreen/disabled/disconnected work stops and
cache entries expire on scope changes.
Verify: scroll a twenty-scene grid, hold focus, drag audio, edit Sources text and
switch pages during captures; assert bounded requests/textures and no queue growth.
Initial budget to validate: one in-flight capture, about two requests/second
aggregate, roughly three-second refresh for six visible cards, up to 384×216,
24 cached images and a 12 MiB decoded-texture ceiling. Reduce cost if measurements
fail; do not launch an unrestricted per-card timer or sweep the whole collection.

### P04

**Expose one preview preference in Settings.** Dependencies: P03. Scope: M.
Files: `storage/config.rs`, Settings page, Live preference binding/card sizing.

Acceptance: Scene previews persists and applies without restart; off cancels work,
clears unnecessary image cache and gives compact cards; on produces larger image
cards with a useful placeholder while the first image arrives.
Verify: old/new config, off/on while a reply is pending, restart, missing capability
and theme changes. Ordinary page visits/theme changes must not start extra scans.

### S01

**Give Sources a focused list/detail layout.** Dependencies: L01, T01. Scope: M.
Files: Sources page, source-row/detail widget, existing inspector integration, CSS.

Acceptance: scene selection/search/item visibility are visually separate from
input-wide settings; wide list/detail adapts to compact drill-in navigation;
no full operating accordion forest or preparation controls reappears.
Verify: nested/shared sources, empty matches, long names, selected source retained
across Program changes, and visible scope/partial-read information at the action.

### S02

**Polish contextual source editing.** Dependencies: S01. Scope: M.
Files: source-detail components, UI workspace actions, CSS (split detail families
if five-file scope is exceeded).

Acceptance: text Save/Cancel and external-edit conflicts are clear; supported
media/filter/audio properties are grouped with concise labels and comfortable
controls; read errors preserve useful context without enabling stale mutations.
Verify: native readbacks, cancel/conflict, unsupported controls, switching source
with a draft, scope loss, keyboard traversal and selected-media polling lifetime.

### R02

**Retire exclusive workflow runtime and reads.** Dependencies: R01, L04, S02. Scope: M.
Files: workspace observation/controller, controller state, UI apply/helpers,
workflow domain/storage references (split shared-type cleanup if needed).

Acceptance: remove routine baseline/preflight/cue storage and observation work;
retain shared scope/activity/output evidence and reads required by current audio
or Sources; leave pre-existing workflow data on disk unchanged.
Verify: request-trace comparison while idle on Live, entering Sources and changing
outputs; controller regression tests; filesystem before/after hashes. Do not
rename all shared types merely to remove the word workflow.

**Checkpoint C:** previews behave and look correct on real cards, Settings works,
and retired workflows no longer cause routine network/storage work. Source
editing stays usable while preview updates are running.

### T02A

**Update Adwaita Default and SceneDeck Dark.** Dependencies: T01, V02, L05, P04, S02, I02. Scope: M.
Files: the four `adwaita-default-*` / `scenedeck-dark-*` stylesheets.
Acceptance: both variants meet the shared theme contract, preserve identity and
render every new component state. Verify: the theme-batch matrix below.

### T02B

**Update SceneDeck Light and OBS.** Dependencies: T02A. Scope: M.
Files: four `scenedeck-light-*` / `obs-*` stylesheets.
Acceptance: true light treatment and OBS palette remain readable across both card
modes and output states. Verify: the theme-batch matrix below.

### T02C

**Update Obsidian and Nord.** Dependencies: T02A. Scope: M.
Files: four `obsidian-*` / `nord-*` stylesheets.
Acceptance: restrained tonal depth and legible secondary text preserve family
identity in both variants. Verify: the theme-batch matrix below.

### T02D

**Update Dracula-inspired and Solarized Dark.** Dependencies: T02A. Scope: M.
Files: four `dracula-inspired-*` / `solarized-dark-*` stylesheets.
Acceptance: palette identity survives; focus, Program, Preview and pending states
remain distinct in both variants. Verify: the theme-batch matrix below.

### T02E

**Update High Contrast and Stream Red.** Dependencies: T02A. Scope: M.
Files: four `high-contrast-*` / `stream-red-*` stylesheets.
Acceptance: high contrast has solid title backing and explicit boundaries;
red accent does not obscure semantic state. Verify: theme-batch matrix plus
high-contrast/reduced-motion checks.

### T02F

**Update Studio Purple and Ubuntu Violet.** Dependencies: T02A. Scope: M.
Files: four `studio-purple-*` / `ubuntu-violet-*` stylesheets.
Acceptance: disciplined accent use, readable light counterparts and no opaque
legacy card fills over previews. Verify: the theme-batch matrix below.

### T03

**Preserve custom themes and motion preferences.** Dependencies: T02A–T02F. Scope: M.
Files: `ui/theme.rs`, three example theme files, theme CSS reference.

Acceptance: new classes/defaults and retained selectors are documented; example
overlays work across card modes; desktop/App Motion settings suppress the new
transitions as intended without stopping necessary state updates.
Verify: theme provider reload/failure fallback, System switching, all motion levels
and old custom overlay smoke. User CSS paths/content and saved IDs stay intact.

**Theme-batch matrix:** capture Live with previews on/off in both light/dark
variants; include Program, Preview, both tallies, focus, accent, pending and missing
image states. Inspect Sources/Inventory/Mixer for shared-CSS regressions. Check
GTK parser warnings and contrast. Compile once per changed batch; do not claim
all variants passed from a single dark screenshot. One report indexes all 24.

**Checkpoint D:** all theme batches and example overlays satisfy the matrix;
no undefined colors, lost scrims, unreadable badges or color-only focus/state.

### A01

**Finish responsive and keyboard interaction.** Dependencies: I02, L05, P04, S02, T03. Scope: M.
Files: focused affected UI components and CSS, manual-test plan.

Acceptance: focus survives image/state updates and resize; text entry never
switches scenes; all actions including compact Inventory controls are reachable
without a pointer and expose useful names/state.
Verify: 640–1600 px layouts, 125%/200% text, long names, both card modes, no horizontal
clipping, no hidden Stop/Take actions, and AT-SPI plus actual Orca where available.
Record speech coverage separately from accessible-tree inspection.

### A02A

**Synchronize English, German, Swiss German and Spanish copy.** Dependencies: A01. Scope: M.
Files: those four Fluent catalogs.
Acceptance: concise finalized labels describe real actions, new keys/variables
match and retired-only workflow copy is cleaned safely. Verify: key/parser checks,
German long-label screenshots and observed action names.

### A02B

**Synchronize Italian, Polish, Portuguese and Ukrainian copy.** Dependencies: A02A. Scope: M.
Files: remaining four Fluent catalogs.
Acceptance: equivalent copy and variable parity, no resurrected workflow surfaces;
translation uncertainty is recorded. Verify: isolated locale test plus enlarged
Ukrainian layout. Native-speaker review is reported honestly, not inferred.

### Q01

**Run the integrated OBS and visual qualification.** Dependencies: A02B, R02. Scope: S.
Files: manual-test evidence and screenshot/measurement index.

Acceptance: all retained native actions pass appropriate fixtures/readbacks;
visual/runtime matrices are complete or exact gaps listed; no GTK criticals,
unbounded preview work, stale targeting or unexplained focus loss remains.
Verify: the commands and scenarios in section 7. Screenshots must show the actual
final binary, not only the concept or an earlier intermediate implementation.

### Q02A

**Update the product guides.** Dependencies: Q01. Scope: M.
Files: README, operating guide, configuration guide and user guide.

Acceptance: docs reflect the actual navigation and removed features, describe
thumbnail behavior and theme compatibility, and link final screenshots; product
claims agree with implemented behavior. Verify: document/link checks and a check
of `site/index.html` for claims affected by the redesign. Log any needed site
copy/assets as a separate bounded documentation follow-up before completion.

### Q02B

**Finish implementation documentation and close the plan.** Dependencies: Q02A. Scope: M.
Files: architecture, OBS WebSocket integration, manual-test plan, manual-test runs
and docs index; routine plan/checklist status updates accompany completion.

Acceptance: internal docs match the retained state/preview boundaries; evidence
records actual visual and OBS qualification with exact remaining limits; the
current checklist reflects redesign outcomes and preserves prior results.
Verify: document/link checks, task/acceptance audit and final comparison to V01.

## 7. Verification and definition of done

### Running-app visual gate

A phase passes only after screenshots are inspected against these outcomes:

1. At 1280×800 with six scenes, scenes dominate the page; no extra panel competes
   with them. At narrower sizes, grid scrolling leaves essential controls usable.
2. Bright, dark, detailed and unusual-aspect previews retain recognizable content
   and readable titles. Preview-off is a deliberately designed mode.
3. Program, Preview and focus can be identified quickly from labels and treatment.
   Take appears once, and only the appropriate Studio controls are visible.
4. Default audio is compact. Stream/record/replay/virtual camera are reachable
   without duplicate action surfaces or large status paragraphs.
5. Inventory's role names are readable and its columns can be resized. Long
   names/locales/text scaling do not reduce a role value to meaningless `...`.
6. Sources presents one focused task at a time; no rejected workflow appears on
   any page or in Settings.
7. All 24 theme variants and example overlays retain spacing, contrast and states.
   A visually weak result is reworked before adding more features.

### Native behavior and preview performance

Use a disposable OBS profile/collection and safe local outputs. Exercise Direct
and Studio card/shortcut behavior, external mode/scene/transition changes,
pause/resume, replay saved evidence and virtual camera where configured. Test
source visibility, text Save/Cancel/conflict, filter/media/audio setters separately.
Keep configuration-rejected requests distinct from successful operation.

For previews, measure with 6 and 20 scenes, a moving image, a remote-delay fixture,
fast page switching and reconnect/collection races. Record capture p50/p95,
bytes/second, in-flight/queued work, cache size, OBS render time and app interaction
responsiveness. Initial fixture targets: no preview-caused input stalls over
100 ms and no sustained mean OBS render-time increase over 1 ms; investigate
outliers and lower resolution/rate if needed. These are qualification targets,
not performance claims based on the previous simple benchmark.

Controls/events take precedence over thumbnails. Faulty or slow image reads must
back off independently. No real broadcast is needed for this qualification.

### Repository gates

Run focused checks after each meaningful slice. Run the complete required set at
phase checkpoints/final integration after relevant changes:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-features --locked
cargo test --workspace --all-features --locked
cargo test --workspace --all-features --locked -- --ignored --test-threads=1
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo build --locked
cargo audit
cargo deny check advisories bans licenses sources
```

Run locale/display tests separately because the Fluent loader is global. Record
baseline failures and existing advisory policy; never relax gates to obtain a
passing screenshot. Tests should verify meaningful behavior; do not freeze CSS
source strings or duplicate implementation logic as proof of visual quality.

## 8. Sequencing, risks and constraints

- First establish V01/T01/V02 and fix Inventory I01/I02. Then simplify the real
  Live surface with Sources available, rather than leaving controls orphaned.
- Complete L03–L05/R01 and inspect checkpoint B before starting a larger preview
  implementation. Do P01 early enough to validate the screenshot approach.
- Finish P02–P04/S01–S02/R02, then theme batches, accessibility/locales and final
  qualification. T02B–F can run independently after T02A if file ownership is
  coordinated; shared layout/controller files remain sequential.
- Match GTK 4.10/libadwaita 1.4 APIs. Keep GTK mutation on its main thread, OBS I/O
  on Tokio, and persistence on existing ordered background paths.
- Main risks are overloading Live again, imagery obscuring labels, thumbnail
  polling harming control latency, role-column squeeze and palette overrides
  destroying state contrast. The early native renders and repeated visual gates
  specifically address them.
- No new feature framework, arbitrary dashboard, theme-engine replacement,
  external service, release, commit or push is required by this planning task.
- Do not delete user registry/config/workflow files as cleanup. Do not change
  disconnect semantics or OBS scenes as an incidental consequence of navigation.

## History

Earlier results remain preserved; use them for implementation context, not as
active requirements when they conflict with the decisions above:

- [Previous detailed OBS feature plan](docs/plans/obs-feature-plan-2026-10-19.md)
- [Previous delivery checklist and first-pass history](tasks/todo.obs-features-history.md)
- [Original UI plan](tasks/plan.first-pass-history.md)
- [First research](docs/obs-only-feature-research.md),
  [research review](docs/obs-only-research-review.md),
  [second research](docs/obs-controller-feature-opportunities.md)
- [Previous implementation evidence and its limits](docs/verification/2026-10-19/README.md)

The old implementation passing its code gates did not make its Live layout
successful. Completion of this revision requires the simpler agreed interaction
and a convincing actual-app visual result.
