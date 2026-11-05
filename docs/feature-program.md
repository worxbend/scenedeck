# SceneDeck feature implementation program

Status: complete. Requested and completed 2026-11-05. Baseline: `11d1ff8` (0.5.3).

This is the durable implementation and continuation record for the approved
feature program. All delivery milestones are complete. The checkpoint sections
below retain historical implementation steps and intermediate failures; the final
acceptance record at the end supersedes their continuation instructions.

## Scope and decisions

- Implement all features in the user's combined list, merging duplicate ideas.
- Preflight is explicitly requested now, despite the earlier retirement of setup
  expectations. Introduce a deliberate new model; do not silently revive or
  overwrite legacy `workflows.json` files.
- Pause/resume recording, replay saving, media transport/seeking, source text
  editing, Studio mode and short Stats history already exist. Extend and test
  those paths rather than introduce competing controls.
- A show workspace belongs to a saved connection and scene collection. Named
  decks are curated views within that workspace. OBS profiles remain OBS's own
  concept. Secrets belong to the keyring, not exports or workspace files.
- Actions use the existing controller and scope/freshness checks. Connection
  recovery never replays old mutations. Audio presets report partial failures;
  they are not atomic OBS transactions.
- Shortcuts explicitly attached to an action do not move when cards are sorted
  or filtered. Preserve the existing slot shortcuts as a compatible option.
- Health monitoring uses recent samples, sustained conditions and recovery;
  unknown or stale observations never count as a healthy signal.
- Session records distinguish requested, acknowledged and observed outcomes.
  Bookmarks distinguish session/recording elapsed time from encoded-file time,
  particularly across pauses and reconnects.
- No releases, deployments, pushes, or real broadcasts are part of this work.
  Preserve unrelated untracked `QUALITY_REVIEW.md` and `TEMP.md`.

## Delivery checklist

Each milestone requires working UI, persistence where applicable, regression
coverage, all eight Fluent catalogs, and corresponding documentation. A domain
type or storage helper alone does not complete a milestone.

| ID | Milestone | Acceptance criteria | Status |
| --- | --- | --- | --- |
| F01 | Workspace identity and saved connections | Named connections with separate keyring entries; metadata isolated by connection/collection; legacy setup preserved; scene rename retains curation; explicit safe switching | Complete |
| F02 | Search, tags, favorites and named decks | Edit tags, search scenes, select/save curated views, retain order and bindings across filtering/restart | Complete |
| F03 | Action catalog, command palette and shortcuts | Search scenes and supported actions, execute through existing guards, configure stable bindings and detect conflicts; retain text-entry behavior; global shortcuts capability-gated through the desktop portal | Complete |
| F04 | Live quick actions | Pin/reorder/remove individual actions on Live; show observed state; invalidate stale targets and scopes; persist per workspace | Complete |
| F05 | Preflight | Expected collection, required scenes, chosen microphone mute/signal, recording disk space, manual checklist; explicit unknown/failed/passed states; inspect before start without firing an output | Complete |
| F06 | Actionable health alerts | Sustained network drops, render/encoder overload, low disk, lost connection/unexpected output stop; persistent cross-page presentation, recovery, cooldown; optional selected-input silence/clipping | Complete |
| F07 | Recording/media workflow | Timestamp bookmarks, media progress/remaining time, existing pause/resume/seek integrated and verified; explain pause/session timing semantics | Complete |
| F08 | Audio presets | Save selected-input gain/mute presets; preview changes, missing inputs and shared effects; apply with fresh scope and report per-input outcome; persist per workspace | Complete |
| F09 | Session reports | Opt-in bounded disk history; retain events, stats, bookmarks and output evidence; post-session report; CSV/JSON export without secrets; retention controls | Complete |
| F10 | Integration and release readiness | Cross-feature regression/native checks, migration/failure tests, documentation and product claims updated, no unverified runtime claims | Complete |

## Increment order

1. F01a (complete): preserve scene curation on observed scene rename, with
   collision and stale-session protection and retryable persistence.
2. F01b (complete): introduce workspace persistence and migration; isolate registry edits,
   imports and exports by connection/collection.
3. F01c (complete): named connections and per-connection keyring credentials, selection UI.
4. F01d (complete): UUID-backed scene reconciliation, including disconnected renames and name reuse.
5. F02 (complete): tags, favorites, search and named deck views.
6. F03/F04: one action catalog used by palette, bindings and quick actions.
7. F05/F06: shared pure readiness/health rules with separate preflight and live
   presentations; use existing telemetry and scoped source observations.
8. F07/F09: bookmarks and media details, then optional persistent session reports.
9. F08: previewable audio presets using scoped existing audio commands.
10. F03 global shortcuts and F10 final integration, compatibility and docs.

## Verification

Run targeted tests after each logical increment. At feature checkpoints run:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-features --locked
cargo test --workspace --all-features --locked
cargo test --workspace --all-features --locked --lib infra::i18n::tests::every_shipped_locale_parses_and_loads -- --exact --ignored --test-threads=1
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo build --locked
python3 tools/test-native.py
```

Use isolated protocol/native fixtures. Do not control the user's OBS production
session as a test. Report unavailable display/desktop/OBS integration evidence
separately from passing unit tests. Run the repository's dependency, Python and
site gates when their inputs change and at the final integration checkpoint.

## F01b implementation notes

- Keep the existing operating `WorkspaceState` distinct from persisted show
  workspaces. Use a stable saved-connection ID plus collection identity for the
  storage key; display names and a currently edited host are not identities.
- Introduce a versioned show-workspace store, with bounded reads and atomic
  writes. Leave retired `workflows.json` untouched. Bind the legacy registry to
  the original connection's first confirmed collection once, retaining the
  original file; do not copy it into every newly discovered collection.
- Capture the registry destination when accepting an edit. Ordinary snapshots,
  field writes, retries, YAML imports and exports must all use that destination.
  The two field-write callers (Inventory scene icons and audio-card icons) now
  mutate a registry loaded from the captured destination.
- Publish an asynchronously loaded/imported registry only if its connection,
  collection and selection generation still match for loads. An accepted import
  also updates that destination if the user returned to its cached workspace
  while the exclusive import gate was held. Show loading/error state and
  prevent edits from targeting the previous registry while selection is pending.
- Retain ordered persistence and failure evidence across workspace switches.
  A successful save for workspace B must not discard a failed save for A.
- New connection credentials must be keyed by immutable connection ID. Preserve
  the legacy keyring entry for the migrated default connection only. Never copy
  a password from the last selected connection into a new one implicitly.
- Add regressions for identical scene names in two connections/collections,
  delayed writes/imports across selection changes, corrupt files, interrupted
  migration and restarting after a successful migration. Full UUID reconciliation
  must be scoped before it can infer offline renames.

## Progress and continuation

### 2026-11-05 — workflow started

- Created the active persistent goal and this checklist.
- Inspected architecture, persistence lanes, session scope guards and existing
  capabilities. No sub-agents or external scheduled jobs were created.
- Baseline Rust tests passed: 634 passed, 56 ignored (native/locale suites).
- Completed slice: F01a, preserving curation on observed scene rename. Registry
  still keys metadata by name; offline UUID reconciliation remains in F01.
- Implemented rename identity propagation, collision-preserving registry
  migration and bounded deferral through imports. Five focused storage/adapter/
  controller regressions passed. Expanded Rust suite: 640 passed, 57 ignored.
  Format, check, all eight locale catalogs, Clippy with denied warnings and
  production build passed.
- Native runner prerequisites: Xvfb is absent from PATH. Downloaded the Ubuntu
  Xvfb package and extracted it privately under `/tmp/scenedeck-native-tools`;
  use its `usr/bin` on PATH for isolated native tests. No host package install.
- Next slice: F01b, scoped workspace persistence and legacy migration. F01 and
  the overall goal remain active; the other feature milestones are not yet
  implemented. Keep this section current before yielding or context compaction.

Native evidence: the first full run stopped on
`app::tests::native_close_waits_for_accepted_save_and_gtk_publication` with
`GTK completion deadline exceeded` (existing two-second fixture deadline).
Both that test and the new
`ui::window::tests::native_scene_rename_preserves_imported_curation_and_rejects_retired_scope`
passed on a focused run without code changes. The full rerun repeated the close
fixture timeout. The test-only `app::tests::fixture` now waits for the first
`after-paint` signal before beginning the existing two-second persistence
assertions; production deadlines and correctness assertions are unchanged.
The subsequent complete native suite passed all **56** isolated GTK tests,
including the new rename/import/stale-scope regression. Final formatting and
Clippy checks after the test fixture change also passed. No live OBS instance
was controlled. All changes are uncommitted; no pushes or releases were made.

Logs are in
`/tmp/scenedeck-feature-{tests,check,locale,clippy,clippy-final,build,native,native-focused,native-rerun,native-ready}.log`;
these are temporary evidence, not a permanent release artifact.


### 2026-11-05 — F01b implementation checkpoint

- Added versioned, size-bounded, atomic `show-workspaces.json`, keyed by the
  connected saved-connection ID, actual endpoint and collection. Config v4
  remembers the original endpoint for one-time legacy adoption; legacy files
  remain intact. Production sessions carry their own connection ID rather than
  consulting a later edited selection.
- Added selection generations, loading/error/ready state, and an in-session
  cache retaining accepted unsaved curation across switches. Late loads cannot
  publish into another selection. Live and Inventory show availability, and
  scene hotkeys cannot use stale curation while a selection loads.
- Inventory callbacks and audio icon editors capture the displayed registry
  generation. YAML imports retain their admitted destination, and late imports
  update only that workspace’s cache. Imports, migration and observed renames
  share an ordered gate so deferred renames wait through a newly started load.
- Registry save retirement is now destination-specific. A successful snapshot
  or import for B cannot erase failed intent for A. Retry still preserves
  operation order and explicit close-barrier failure evidence.
- Added storage/migration, cached-dirty-intent and destination-aware retry tests,
  plus a native synthetic-event scenario covering imports during switches,
  stale editors, identical names across connections/endpoints, corrupt-store
  refusal and retry after repair. All eight locales have loading/error text.
- Interim Rust suite: 644 passed, 58 ignored. A test fixture needed to declare
  its actual custom destination after the writer stopped treating every save as
  global. Full native, locale, check, Clippy and build gates are running; logs
  are `/tmp/scenedeck-workspace-*.log`. Record final outcomes below.

### Next F01 work

- F01c: named connection management in Settings, immutable IDs, per-connection
  credentials and deliberate selection. Existing connection inputs and
  `PasswordWrites` are in `src/ui/pages/settings.rs`; both startup and
  `ObsSessionRunner` currently fetch the one legacy secret. Update
  `ObsPasswordProvider` to take the selected ID, keeping `default` mapped to the
  existing secret username. New profiles must start without inherited secrets.
- Selection must wait for config persistence before reconnecting, retain current
  connection identity until OBS actually reconnects, and reject late password
  read/write publications for a different selected connection. Native fixtures
  must use fake secret adapters, never the user’s keyring.
- Offline scene UUID reconciliation is still outstanding within F01. Observed
  rename handling is implemented, but do not claim disconnected renames follow
  identity until UUID snapshots, migration and collision tests are complete.


### F01b verification completed after resume

- Final Rust suite: 645 passed, 58 ignored (57 native plus locale).
- Full native suite: all 57 passed. After the final Doctor/Graph availability
  and escaped error-text corrections, all 16 affected native cases passed
  across the focused run and its resumed final four cases. The interruption
  stopped the build/runner; process inventory confirmed no jobs survived before
  resuming. Logs: `scenedeck-workspace-native{,-final,-resume}.log` in `/tmp`.
- All eight locale catalogs, formatting, check, Clippy with warnings denied and
  build passed. Final build log: `/tmp/scenedeck-workspace-build-resume.log`;
  Rust/Clippy logs: `/tmp/scenedeck-workspace-{tests,clippy}-complete.log`.
- The previous goal turn made concrete progress: F01b is implemented and
  verified. F01 remains in progress because named connection management and
  offline UUID reconciliation are outstanding. Continue with F01c.


### F01c implementation in progress

- Credential reads/writes now require the immutable connection ID, including
  startup, injected controller providers, connection bootstrap and Settings.
  The default connection retains its old key; every new ID has a separate key
  and no default fallback. Focused key mapping, provider routing and password
  FIFO tests passed; no real keyring entries were accessed.
- Config v5 adds named saved endpoints, preserves the default connection and
  original registry endpoint, rejects duplicate/blank stored IDs, and mirrors
  the selected profile into the existing endpoint snapshot. Writes materialize
  profiles so reconnect does not trigger another migration write outside the
  ordered configuration lane. Future configuration versions are not rewritten
  merely to add profiles.
- Added a Settings connection picker, editable names, Add connection and
  explicit Connect saved setup. Selection clears the password immediately and
  loads only that connection’s credential; generation checks retire delayed
  publications. Connect waits for a successful config save. The status shows
  the actually connected ID’s label and endpoint.
- Native testing caught a GTK use-after-free when replacing the picker’s model
  from its selection notification. GDB identified `ComboRow::set_selected`;
  the picker now retains its model unless IDs/labels change. All three Settings
  native cases passed afterward, including delayed secret writes, new-profile
  isolation, rename/endpoint persistence and failed secret reads. Logs:
  `/tmp/scenedeck-connections-native-fixed.log`; failed evidence remains in
  `/tmp/scenedeck-connections-{native,debug}.log`.
- A further native test now exercises real controller dispatch with injected
  providers that stop before network/keyring access: a confirmed save must
  precede the selected-ID lookup, and a failed save must not reconnect. Full
  Rust checks are currently running in `/tmp/scenedeck-connections-tests.log`;
  run this native case and the final gates before marking F01c verified.
- Next required F01 slice remains offline scene UUID reconciliation. F02–F10
  remain in the delivery checklist and the durable goal is still active.


### F01c verified checkpoint

- Named connections, editable names/endpoints, credential isolation and explicit
  connection switching are implemented. This is concrete progress toward F01;
  offline UUID reconciliation still prevents declaring F01 complete.
- Final Rust suite: **651 passed, 60 ignored**. Full native suite: **59 passed**,
  including both new fake-service connection tests. Formatting, all eight
  locales, check, Clippy with warnings denied and production build passed.
  Logs: `/tmp/scenedeck-connections-tests-final.log`,
  `/tmp/scenedeck-connections-native-all.log`,
  `/tmp/scenedeck-connections-{check-final,locale,clippy-final,build-final}.log`.
- A final storage-only regression covers schema-less legacy configurations:
  materialization captures the original default endpoint before edits can
  redirect registry adoption. Final Rust/Clippy/build gates passed after this
  change; the full native run preceded this additional storage regression.
- No active tools remain at this checkpoint. All work is uncommitted. No real
  keyring entries, OBS outputs, releases or remote repositories were changed.

### Next: F01d identity reconciliation

- `src/obs/mapper.rs::map_scenes` currently discards `s.id.uuid`; the normalized
  `domain::scene::Scene` contains only its name. Native `SceneNameChanged` has
  a UUID too, currently discarded by the mapper. The installed obws 0.15.0
  definitions are in `responses/ids.rs` and `events.rs` under Cargo’s registry.
  The official [OBS protocol](https://github.com/obsproject/obs-websocket/blob/master/docs/generated/protocol.md#scenenamechanged)
  confirms UUID plus old/new names in the rename event.
- Keep named display/control targets while recording stable scene UUIDs within
  the existing connection/endpoint/collection scope. Do not infer identity from
  names when two distinct UUIDs reuse a name. Treat absent/invalid identities as
  unknown; existing synthetic fixtures often use nil UUIDs.
- Required regressions: disconnected rename, simultaneous name swaps, deletion
  followed by name reuse, destination metadata collision, stale collection
  publications, and importing portable metadata into another collection.
  Preserve conflicting/stale curation without silently attaching it to a
  different scene. Ordering must follow identity as well as metadata.
- Coordinate identity publication with the registry import/load gate and
  destination-aware writer already implemented. Preserve YAML portability;
  imported identity hints must not bind metadata to another collection’s UUIDs.
- Then continue F02 (tags/favorites/search/named decks) and the remaining full
  checklist. Do not redefine completion around the connection milestone.

### F01d implementation checkpoint

- OBS inventories and rename events now preserve non-nil scene UUIDs. Scoped
  registry reconciliation stages simultaneous moves, preserves all metadata and
  order, withholds curation on name reuse and retains destination conflicts.
- Inventory exposes retained entries for review/removal; YAML exports preserve
  their contents, while imports clear UUID authority and rebind named intent to
  the destination collection. Retained imported conflicts stay inert.
- Identity publication runs after workspace loading and import publication.
  Pending imports prevent Live scene activation; replaced curation generations
  retire old editors and scene cards. Deferred inventory updates reconcile even
  when an import fails. Post-import identity saves retain their own failure and
  retry evidence instead of being retired with the import's older writes.
- Initial adapter suite: 53 passed. Focused registry suite: 38 passed, including
  five new identity/migration regressions. Full Rust and native integration gates
  are in progress; do not mark F01d verified yet. Logs are
  `/tmp/scenedeck-identity-*.log`.

### F02 continuation notes

- Start with editable tags and favorites as a small Inventory slice, then add
  search and named saved deck filters. `SceneEntry.tags` already exists;
  `SceneEntry::is_empty` must account for any new favorite flag. UUID movement
  copies the entire entry, so new curation fields follow identity automatically.
- Inventory's row options are in `build_scene_options`; cached row presentation
  and `update_rows` must reflect new metadata without replacing an open editor
  during unrelated OBS readbacks. Normalize blank/duplicate tags and preserve
  role, protection, icon and accent on edits. Localize all eight catalogs.
- Put saved decks/current view in the scoped registry, never global config.
  Reuse the existing target-aware writer and import/export path. Search should
  match scene names and tags, with clear no-results/reset behavior. Keep the
  search control outside the list being filtered so typing retains focus.
- Keep `live_scene_order`/`scene_for_slot` as the unfiltered slot mapping.
  Filtering/favorite display order must look up each visible card's original
  slot instead of enumerating filtered cards. Explicit action bindings arrive
  in F03 and must likewise be independent of views.
- Curation generation is an identity/editor lifetime boundary, not a keystroke
  revision. Do not increment it on every search edit and invalidate the filter
  widget itself. The window already refreshes visible registry views when that
  generation changes; ordinary filter edits need their own targeted refresh.
- The F03–F10 checklist remains pending. No feature beyond F01 is complete yet.

### F01 verified checkpoint

- F01a–d are implemented. The final Rust suite passed **657** tests, with
  **62** ignored tests assigned to native/locale gates. The full isolated GTK
  runner passed all **61** native tests, including load/import identity races,
  stale callback rejection and retained-entry removal without touching the
  replacement scene. All eight locale catalogs passed.
- Formatting, Clippy with warnings denied and production build passed. Final
  check is recorded in `/tmp/scenedeck-identity-check-final.log`; other logs are
  `scenedeck-identity-{tests-final,native-all,locale,clippy-final,build}.log` in
  `/tmp`. No live OBS or real keyring services were used.
- All changes remain uncommitted. F02 is the next implementation milestone;
  the overall durable goal remains active until the complete checklist is done.

### F02a in progress — tags and favorites

- Added favorite metadata and tag normalization that preserves other fields.
  Both travel through the workspace writer, YAML and UUID reconciliation.
- Inventory's scene options now edit comma-separated tags and favorite state;
  a literal summary appears on each row. Incremental refresh keeps tag drafts
  and open popovers intact. Retired callbacks remain registry-generation bound.
- All eight locales are updated. Compilation passed; full Rust and native
  editor tests are in progress in `/tmp/scenedeck-tags-*.log`. Do not claim F02
  complete: filtering, favorite ordering and saved deck views are still next.

### F02a verified checkpoint

- Editable Inventory tags/favorites are implemented and verified. Tag edits trim
  whitespace, remove blank labels and combine case-insensitive duplicates while
  preserving the first spelling. Favorite-only entries survive clearing other
  fields. Both fields persist, export and follow UUID renames.
- Final Rust suite: **658 passed, 63 ignored**. All **8 affected native tests**
  passed: Inventory's layout/lifetime/editing cases and the workspace identity
  integration. The new native case confirms open popovers and unfinished drafts
  survive updates, and retired callbacks cannot change a replacement editor's
  registry. The full 61-test native run was at the preceding F01 checkpoint;
  one new native test was added in this slice.
- All eight locales, formatting, check, Clippy with denied warnings and build
  passed. Logs: `/tmp/scenedeck-tags-{check,tests,native,locale,clippy,build}.log`.
- No tools or jobs remain running. All changes are uncommitted; unrelated
  `QUALITY_REVIEW.md` and `TEMP.md` are untouched. The durable goal is active.

### Continue with F02b — filtering and named decks

- F02 is still in progress. No search, favorites-first ordering or saved deck
  controls have been implemented yet. Begin with the existing registry and
  Inventory/Live paths, preserving the verified F01 and F02a work.
- Prefer a small persisted filter (query matching names/tags and favorites-only)
  plus named saved filters within each workspace. Preserve saved order when
  filtering and retain the unfiltered slot mapping for badges/keyboard actions.
  Live may show favorites first; that presentation must not renumber shortcuts.
- Inventory's incremental refresh currently requires nonempty visible rows;
  filtering to zero matches must keep the search widget and focus alive. Track
  actual inventory availability separately and render an inline no-results/reset
  state. Reordering under an active filter must not silently reorder hidden rows.
- A separate in-memory curation revision may be needed for ordinary metadata/
  filter updates across pages. Do not reuse registry generation for keystrokes;
  it retires callbacks after identity/scope changes. The existing 200 ms window
  presentation loop can observe this revision without another polling source.
- Keep filter controls separate from the results list. Saved-view dialogs and
  callbacks must capture the registry scope and reject stale selection. Use
  weak widget references in callbacks, following existing lifetime tests.

### F02b implementation in progress

- Added scoped current filters (all query words across names/tags, case
  insensitive, optional favorites-only) and named saved filters. Shared controls
  support debounced search, reset including a pending draft, named save/apply/
  removal and scope-bound dialog/menu callbacks. Saved names cannot overwrite
  another view merely by changing capitalization.
- Inventory keeps its search control/focus when results become empty, shows a
  no-results message, and disables reordering while filtering. Live favorites
  and filtering preserve original Inventory slot assignments on cards/actions.
- Added an in-memory curation revision separate from registry generation, so
  ordinary local edits refresh views without retiring editors. Existing window
  polling observes it; no extra timer was introduced.
- Initial search slice: 659 Rust tests passed; the new Inventory search native
  test passed. Saved-deck/Live integration tests are running in
  `/tmp/scenedeck-decks-tests-saved.log`; final native/layout/localization gates
  remain required. F02 is not yet complete.

### F03/F04 continuation notes

- Use one catalog for the command palette, configurable bindings and Live quick
  actions. Derive availability/observed state from current snapshots at execution;
  do not store a previously constructed `ControlAction` as an executable macro.
- Existing action execution is in `src/ui/workspace.rs::act` (scope/request
  guards) and `NavigationContext::dispatch`. `ControlAction` already supports
  scene activation, input mute/gain, media restart, filter enable/disable and
  replay saving. Application navigation can remain usable offline.
- `src/ui/actions.rs` registers application actions/accelerators. A searchable
  palette can start as an application action (for example Ctrl+K), with a header
  affordance and literal searchable rows. Follow current weak-widget ownership
  and isolated native dialog tests. Source inspection uses the existing
  workspace selection functions, never direct OBS access from widgets.
- Explicit bindings must identify actions/scenes independently of filtered
  indices. Scene UUIDs are now available on `Scene` and in the scoped registry.
  Input/source snapshots carry `SourceItem.input_uuid`; inspect coverage before
  persisting input targets so name reuse cannot silently redirect a saved action.
- Preserve the compatible positional scene shortcuts, reject conflicts with
  reserved application shortcuts and other bindings, and retain text-entry
  suppression. Do not make global shortcuts a prerequisite for application
  shortcuts; the portal integration remains its later capability-gated slice.
- F02 introduced ordinary curation `revision` separate from editor-retiring
  `generation`. Local edits use the target-aware persistence helper and window
  refresh loop. Reuse this for action bindings/pins instead of new save lanes.

### F02 verified checkpoint

- Search and favorites-only controls are available on Inventory and Live.
  Search matches all words across names/tags, preserves focus through empty
  results, and retains pending drafts during unrelated refreshes. Reset also
  clears drafts before debounce. Inventory disables reordering under filters.
- Named decks save the current filter, including a just-typed draft, and can be
  applied or removed. Names are trimmed; blank/ambiguous imported names are
  rejected. Deletion leaves the current view and scene curation intact. Data is
  scoped, survives reloading, and is included in YAML exports/imports.
- Live displays favorites first. Card badges and shortcut activation retain the
  unfiltered Inventory slot mapping, including filtered-out scenes. All eight
  Help catalogs and user/configuration docs now describe this behavior.
- Final Rust suite: **663 passed, 66 ignored** (65 native plus locale). The full
  isolated native suite passed all **65** cases, including new search focus,
  saved-deck dialog/menu fencing, original Live shortcut badges, compact large
  text, curation failure/retry and widget lifetime checks. All eight locales,
  formatting, check, Clippy with warnings denied and production build passed.
- Logs: `/tmp/scenedeck-decks-{tests-verified,native-all,locale,check-final,clippy,build}.log`.
  Earlier focused logs are also retained under `/tmp/scenedeck-decks-*`.
- No processes or tools remain running. No real OBS session, keyring, remote
  repository, release or deployment was changed. All work is uncommitted.
- The preceding goal turn made concrete progress; F01 and F02 are complete.
  Continue with **F03a**, the shared action catalog and application command
  palette, then explicit customizable bindings and F04 quick actions. F03–F10
  and the durable goal remain active/pending; do not mark the goal complete.

### F03a implementation in progress

- Added a shared catalog of stable OBS intentions plus application navigation,
  with execution-time identity/capability/freshness checks. Scene UUIDs survive
  renames; missing/reused/ambiguous identities refuse execution. The catalog
  is independent of deck filters and positional slots.
- Added Ctrl+K and header access to a searchable palette. Source actions are
  intentionally limited to the observed inspected-scene inventory/detail.
  Navigation remains available offline. Stale palette scopes require reopening;
  changes in a displayed action's label require a refresh before activation.
- Eight locales and user documentation updated. Pure tests are running; native
  keyboard, layout, scope and lifetime tests plus final validation remain.
- F03 is not complete: persisted custom bindings/conflicts and the later global
  portal capability slice remain, followed by F04–F10.

### F03 continuation details

- `domain/operator_action.rs` holds serializable OBS intentions and UUID/name
  targets. `ui/action_catalog.rs` builds localized entries and resolves them
  immediately before dispatch through `ui::workspace::act` / inspection.
  `CatalogAction::Page` is application-only. Do not persist UI labels or an
  already-resolved `ControlAction`.
- Palette source controls currently depend on `workspace.snapshot.items` plus
  `detail`/`audio`. Those source items are populated only on Sources and are
  cleared by workspace reads elsewhere. Before promising cross-page input
  shortcuts or quick actions, extend target observations explicitly.
- `ObsClient::get_all_audio_inputs` already requests `GetInputList`, but drops
  each UUID while building `AudioInputSource`. Carrying that identity through
  the normalized `AudioInput` is the next small prerequisite: `workspace.audio`
  is already read on every workspace refresh. Do not add a redundant inventory
  request, use active-mixer names as permanent identity, or retain stale source
  detail indefinitely. Media/filter pins also need targeted fresh inspection.
- F03b should persist per-workspace bindings to those intentions, with a key
  recorder, conflict detection (including Ctrl+K, Ctrl+Q/R/comma, F1 and the
  current scene-slot configuration), explicit remove/change and missing-target
  handling. Keep text-entry suppression and existing compatible slot controls.
  Use curation revision for edits and registry generation to retire dialogs.
- Imported bindings/pins will need an explicit portability policy for UUIDs;
  do not blindly execute another workspace's UUIDs or silently redirect a known
  identity to a reused name. Add meaningful persistence/isolation/rename tests.

### F03a verified checkpoint

- Ctrl+K and the header search button open the application command palette.
  It searches all Primary scenes and their tags independently of filtered decks
  or the ten scene slots; includes offline page navigation, replay saving, and
  actions from current source observations. This is the palette slice, not a
  claim that configurable bindings or cross-page input targets are complete.
- Keyboard arrows/Enter/Escape work, selection scrolls into view, zero results
  preserve search focus, names render literally, and unchanged refreshes retain
  rows. A 360px window with large text fits. Repeated opening uses one palette;
  dismissed windows and search widgets are released.
- Execution checks current scope, registry generation/import gate, identity,
  capability and freshness. Explicit mute/filter intents never invert after an
  observed change. A displayed Stage label cannot become a Program switch before
  activation; the palette refreshes and requires a new activation.
- Final Rust suite: **667 passed, 68 ignored** (67 native plus locale). Both new
  native cases and the full isolated **67-case** GTK suite passed. All eight
  locales, formatting, check, Clippy with denied warnings, production build and
  `git diff --check` passed. Logs: `/tmp/scenedeck-palette-{tests-final,native-focused,native-all,locale,check-final,clippy,build}.log`.
- No processes or tool sessions remain running. No real OBS connection, keyring,
  remote repository, release or deployment was changed. All changes remain
  uncommitted; unrelated `QUALITY_REVIEW.md` and `TEMP.md` are untouched.
- Continue with input-identity propagation described above, then F03b persisted
  bindings and conflict detection. For saved action labels, resolve UUIDs to
  current names; do not require full structural equality with newly generated
  catalog entries, whose cached names may have changed. Preserve the palette's
  stricter displayed-label check for its one-shot actions.
- This goal turn made concrete verified progress. F01/F02 remain complete;
  F03 is in progress, F04–F10 remain pending, and the durable goal stays active.

### F03b implementation in progress

- Complete audio snapshots retain non-nil input UUIDs; palette audio actions now
  work across pages. Explicit actions carry UUIDs through WorkspaceRequest into
  OBS scene/input/source UUID fields, avoiding redirection through cached names.
- Added per-workspace explicit bindings, key recording, enable/edit/remove,
  conflict checking at edit and keypress, text-entry suppression, repeat
  suppression, and unavailable-action feedback with refresh but no deferred write.
  YAML imports retain identities and disable bindings for review.
- Media/filter bindings still require current source inspection. Cross-page
  targeted observation must be completed before F03/F04 can be called complete.
  Global portal shortcuts also remain a later required capability-gated slice.
- Audio-targeted tests passed (55 Rust cases); full Rust and native binding
  checks are in progress under `/tmp/scenedeck-bindings-*`. Final gates pending.

### Continue with F03c / F04 target observations

- F03b now has working persisted shortcuts, but this does not finish F03. Media
  and filter actions still derive availability from `workspace.detail`, which
  normal non-Sources reads clear. Complete fresh targeted observations for
  saved/pinned input actions before treating cross-page action controls as done.
- `controller/workspace.rs::read_observation` reads the complete audio inventory
  every time and validates the scene collection again at the end. Reuse the
  existing `GetInputList` response for a complete input identity catalog instead
  of only retaining audio identities. Preserve explicit partial/failure states.
- Carry a deduplicated list of the input observations needed by enabled bindings
  and quick actions in WorkspaceRequest. Read only the required media/filter
  properties with bounded concurrency; avoid fetching settings/text/advanced
  audio for every bound source. Keep this separate from the Sources editor's
  selected scene/input and drafts. Never change those selections to refresh a
  shortcut, and never replay a requested write after a read or reconnect.
- Publish targeted observations under the existing epoch/sequence/collection
  fences and expire them with workspace freshness. Resolve UUIDs through current
  inventory, not cached names; known missing UUIDs must remain unavailable.
  The new `target_uuid` field already carries a mutation's identity to OBS.
- `ui/action_catalog.rs::binding_entry` formats current names from saved UUID
  intentions. `binding_entries` offers both explicit mute/unmute and filter
  states, even when the source already has that state. Explicit setters send
  the requested state rather than inferring a toggle or claiming a no-op.
- `ui/action_shortcuts.rs` owns recording/edit UI, conflict checking and logical
  key normalization; `ui/hotkeys.rs` handles dispatch/release/focus suppression.
  Configured actions use the same catalog as the palette. Add quick-action
  persistence/rendering against this catalog, including observed-state feedback,
  reordering and stale target/scope handling.
- GlobalShortcuts portal support remains required as its later capability-gated
  opt-in slice; do not activate desktop shortcuts during tests. Update all eight
  in-app Help shortcut references with the final combined behavior.

- Configuration follow-up: media/filter choices in the shortcut editor currently
  come from the inspected detail. Opening Settings while existing bindings keep
  the workspace refreshed can clear that detail. The F03c action picker must
  support selecting and observing a source without depending on a timing window
  after leaving Sources. Keep a selected editor target under its own scope.
- Native verification note: the first complete run hit the existing save-on-quit
  fixture's 15-second decision deadline under heavy host load (load average ~35;
  no production failure was observed). The identical fixture then passed in
  5.39 seconds without changes. A fresh full native run has completed; final
  lint/build gates are still being checked. Preserve both logs for review.

### F03b verified checkpoint

- Settings now has a scoped action-shortcut manager and searchable action picker.
  Users record, edit, enable/disable and remove bindings. Exact modifier conflicts
  cover application shortcuts, current scene-slot settings and other enabled
  bindings. Later config changes disable conflicting custom bindings without
  rewriting them. Text input suppresses actions; holding a key fires once.
- Scene/audio/replay keys work across pages, independent of deck filtering/order.
  The existing workspace timer keeps observations fresh while enabled bindings
  are in use. Unavailable actions show feedback and refresh without queuing a
  mutation. Existing Sources-only media/filter availability remains an explicit
  unfinished F03c requirement, not a completed cross-page feature.
- Audio UUIDs are normalized from the existing full input read; nil UUIDs stay
  unknown. Palette and binding actions retain known identity through the
  controller request and send UUID fields on the wire. Audio observations also
  read by UUID when known. Studio activation keeps its authoritative mode check.
- Bindings persist independently per workspace. JSON round-trips preserve state;
  YAML imports disable bindings while preserving identity for review/rebinding.
  Existing disabled bindings remain disabled when edited.
- Verified **673 Rust tests**, with **71 ignored** (70 native plus locale). All
  **70 native GTK tests** passed on the fresh full run, including recording,
  conflict revalidation, stale scopes, persistence, edit/removal, 360px large-text
  layout, cross-page activation, typing suppression and held-key suppression.
  After the test-only Clippy cleanup, the affected native editor case passed
  again. All eight locales, formatting, check, Clippy with denied warnings,
  production build and `git diff --check` passed.
- Logs: `/tmp/scenedeck-bindings-{tests-final,native-all-recheck,native-timeout-recheck,locale,check-final,clippy-final,build,native-editor-final}.log`.
  The initial timeout run and focused runs remain alongside them.
- No tools or jobs remain running. No real OBS session, keyring, desktop shortcut
  registration, remote repository, release or deployment was changed. All work
  remains uncommitted; unrelated `QUALITY_REVIEW.md` and `TEMP.md` are untouched.
- This goal turn made concrete verified progress. Continue with F03c targeted
  media/filter observations and independent action selection, then F04 quick
  actions. F03 remains in progress, F04–F10 pending, and the durable goal active.

### F03c implementation and continuation

- The workspace now retains a complete input identity catalog, including
  non-audio inputs, from the existing `GetInputList`. Audio reads share that
  catalog. Enabled bindings contribute deduplicated media/filter requirements
  to `WorkspaceRequest.operator_reads`; the OBS adapter queries only those
  properties, with bounded concurrency and UUID addressing.
- `InputObservation` preserves independent property failures. The action catalog
  uses these observations rather than the Sources detail view. Missing,
  ambiguous or replaced identities, failed inventory/property reads, stale
  observations and retired scopes keep the affected actions unavailable.
- The reusable `ui/operator_picker.rs::ActionPicker` offers independent source
  inspection in the shortcut editor. It preserves the chosen action while new
  results arrive, keeps models stable while popups are open, renders names as
  literal text, and exposes an explicit observation refresh. A failed initial
  inventory can be retried without having a selected input.
- Temporary inspection tokens live in WorkspaceState and release on unmap,
  destruction or scope retirement. The token counter survives invalidation so
  an old dialog cannot release a new lease. A picker change during a pending
  read leaves the workspace dirty for the normal timer's next read. Sources
  scene/input selection and its local draft maps are independent.
- A failed inventory preserves the picker's selected source for recovery while
  the action catalog rejects cached input actions. Refreshes never defer writes.
  Saved media/filter bindings now operate across pages; global portal shortcuts
  remain the later required F03 slice.
- New domain, wire, reconciliation and native tests cover read deduplication,
  UUID/name reuse, independent errors, inventory failure, scope fencing,
  cross-page media dispatch, stable selection, retry, retirement and lifetime.
  All eight locale catalogs and the user/architecture/protocol docs are updated.
- Final verification is running under `/tmp/scenedeck-operator-*`. The first
  complete native run passed all 71 cases; after adding explicit picker retry,
  the full Rust run passed 678 tests with 73 ignored (72 native plus locale).
  Final native and build results will be recorded below before this checkpoint
  is considered verified.

### Continue with F04 after F03c verification

- Reuse `ActionPicker`, `OperatorAction`, `binding_entry` and the shared execution
  guards for pinning actions on Live. Implement pin/remove/reorder and observed
  status, persisted in the current workspace. Keep explicit desired state
  distinct from observed state; acknowledgements are not completion evidence.
- Extend `WorkspaceRequest.operator_reads` to include enabled pinned media and
  filter actions. Keep reads separate from Sources selection and drafts.
- Use the existing Live refresh/revision flow, weak GTK ownership, scoped
  callbacks and FIFO registry persistence. Preserve keyboard focus and compact
  layouts; test stale callbacks, failure/unknown observations and import behavior.
- Include schema defaults and workspace isolation tests. Imported pins should
  require review before activation, consistent with imported bindings. Keep
  global shortcut support, F05–F10 and the final Help/site claims audit pending.

### F03c verified checkpoint

- Completed cross-page media/filter observations and the independent source
  picker, including explicit retry after an initial inventory failure. Enabled
  shortcuts no longer require keeping their target selected in Sources.
- Verified **678 Rust tests**, with **73 ignored** (72 native plus locale), and
  **all 72 native GTK tests** on the final full run. All eight locales, formatting,
  type checking, Clippy with denied warnings, production build and
  `git diff --check` passed. No source or Fluent edits followed these checks.
- Final logs: `/tmp/scenedeck-operator-{tests-final,native-final,locale,fmt,check-final,clippy-final,build}.log`.
  Earlier focused and full native logs remain for the pre-retry-button checkpoint.
- No tool jobs remain running. All changes remain uncommitted; unrelated
  `QUALITY_REVIEW.md` and `TEMP.md` are untouched. No real OBS, keyring, desktop
  shortcut registration, remote repository, release or deployment was changed.
- Continue with **F04 Live quick actions** using the interfaces above. F03 remains
  in progress solely for its later global shortcut slice; F04–F10 remain pending.
  The durable goal remains active and is not complete.

### F04 implementation and verification in progress

- Added ordered workspace `quick_actions` containing an action and explicit
  enabled flag. Older registries default to empty; local saves retain order and
  identity; YAML imports disable pins for review. Workspace isolation is covered
  by actual file round trips.
- Live now has an add button and a wrapping pin strip at the top of its scene
  scroller. Every pin has enable/disable, change action, move up/down and remove
  controls. The existing ActionPicker configures pins without altering Sources
  selection. Edits preserve disabled state and use the registry's scoped FIFO
  persistence. Old callbacks validate their displayed pin list and scope.
- Pins show action intent separately from observed state. Mute, filter, media,
  replay and scene observations use current identities and freshness; unknown,
  failed or pending reads never imply an off state or a completed action.
- Added selectable mute/filter toggles alongside explicit setters. They resolve
  the latest valid observation to an explicit setter. Live checks both displayed
  label and observed value before admission and never queues a later write.
  These intentions are also available in the shared shortcut picker.
- Enabled pins share deduplicated media/filter reads with enabled bindings and
  temporary inspectors. Normal observations keep buttons and open menus intact;
  keyboard focus follows a reordered pin. Mute events update pins directly while
  retaining the existing no-rebuild/no-dirty behavior for unrelated pages.
- Fixed an existing workspace audio readback race exposed by toggles. Full
  workspace reads carry the transport observation clock; the bounded echo journal
  merges newer mute/volume events before publication. Journal overflow clears
  only audio evidence and adds a warning. Session/collection and workspace
  sequence fences still run first. Sources' corresponding observed values merge
  too; unavailable properties are not synthesized from an echo.
- Rust checks currently pass **684 tests**, with **76 ignored** (75 native plus
  locale). Focused pin/toggle GTK cases pass. All eight locales, formatting,
  check, Clippy with denied warnings and production build pass; the final full
  native suite is running under `/tmp/scenedeck-quick-native-all.log`.
- User/configuration/architecture/protocol docs are updated. The product site
  has no contradictory quick-action claim; the comprehensive Help/site audit
  remains F10. No real OBS, desktop shortcut registration, keyring or deployment
  was touched. No commits or pushes were made.

### F04 verified checkpoint

- F04 is complete: Live add/edit/enable/disable/reorder/remove controls, explicit
  and toggle actions, observed-state feedback, identity/freshness/scope checks,
  per-workspace persistence and import review are implemented end to end.
- Final validation: **684 Rust tests passed**, with **76 ignored** (75 native
  plus locale); **all 75 native GTK tests passed**. All eight locales, formatting,
  type checking, Clippy with denied warnings, production build and
  `git diff --check` passed. No source or Fluent changes followed these gates.
- Evidence includes actual workspace file isolation and YAML round trips,
  native pin persistence/lifetime, compact large-text layout, stable open menus,
  focus after reorder, stale callbacks, observed-vs-requested state, toggle
  rejection after a displayed-state change, shared read requirements, immediate
  mute-event UI updates, and protection from late audio readback/echo overflow.
- Final logs: `/tmp/scenedeck-quick-{tests-final,native-all,locale,fmt,check-final,clippy-final,build}.log`.
  Focused native/storage logs remain under the same prefix. No jobs remain live.
- All work remains uncommitted; unrelated `QUALITY_REVIEW.md` and `TEMP.md` are
  untouched. The durable goal is active. F01, F02 and F04 are complete; F03's
  global portal shortcut slice and F05–F10 remain unfinished.

### Continue with F05 preflight

- Implement configurable expected collection, required scenes, selected
  microphone mute/signal, minimum recording-disk space and a manual checklist.
  Show explicit passed/failed/unknown results and freshness. Running preflight
  must inspect only; it must never start an output or apply audio changes.
- Preserve the operator's expected collection independently of OBS's currently
  followed collection. Automatically adopting the current collection would hide
  exactly the mismatch the check is meant to catch. Choose a storage/UI boundary
  that keeps this expectation explicit when OBS switches collections.
- Reuse the complete input catalog and UUID intentions for selected microphones,
  scene UUIDs for required scenes, the existing audio-level freshness data and
  `ObsStats.disk_space_mb` (optional MiB on the OBS recording volume). Unknown or
  stale telemetry must remain unknown, never pass because it was coerced to zero.
- Manual items are session checks: persist their definitions but retire completed
  checkmarks when connection/collection/session changes. Integrate a clear entry
  point on Live with compact layouts and the existing output-start flow without
  bypassing the user's current output-confirmation preferences.
- Keep F06 sustained alerts, F07 recording/media/bookmarks, F08 audio presets,
  F09 reports/export, later F03 global shortcuts and F10 integration in scope.

### F05 implementation and focused verification

- Added per-connection-and-endpoint preflight plans in configuration. The explicit
  expected collection never follows OBS automatically. Scene and microphone UUIDs
  survive renames and cannot fall back to reused names. Duplicate plan keys remain
  unresolved until an explicit editor save replaces that key.
- Live has a read-only review and configuration editor. Requirements cover scene
  membership, microphone presence/mute/recent post-fader signal, minimum recording
  space and manual checklist definitions. Unknown, stale, failed and skipped states
  are distinct. The review owns volatile checkmarks/signal evidence and resets them
  on plan or workspace-epoch changes, reset and close.
- Workspace snapshots retain membership from their existing GetSceneList call.
  Meter events carry reader-receipt time; stats carry request-start time through GTK
  publication. Checks reject delayed/stale readings, invalid space values, missing
  input-property reads, ambiguous identities and mismatched collections.
- The editor preserves missing saved targets, clears target choices when changing
  their expected namespace, saves through the ordered configuration lane, reports
  failures with retry, and fences callbacks after scope or saved-plan changes.
- Optional start review defaults off. Explicit continuation is single-use, retired
  on cancellation or session/collection changes, rechecks inactive output state,
  and then consults the current existing output-confirmation preference. Ordinary
  reviews, refreshes, checklist edits and successful checks never start an output.
- Seven pure evaluator tests and config round-trip/isolation coverage pass. All
  four focused native GTK cases pass, including actual-connection persistence,
  failed-save retry, missing/replaced targets, volatile checklist lifetime, canceled
  or stale starts, duplicate responses and current output-confirmation settings.
  Broader tests and final layout/localization/check/build gates are in progress.
- New UI strings are present in all eight locales. User/configuration/architecture
  and protocol docs are updated. No real OBS, keyring, portal registrations,
  broadcasts, commits, pushes or deployments were performed.

### Next after F05: F06 sustained health alerts

- Begin with a pure recent-observation evaluator. `ObsStats` exposes cumulative
  render/output skipped and total frame counters; `StreamHealth` exposes network
  skipped/total counters, congestion and reconnecting state. Use interval deltas,
  elapsed monotonic time, sustained thresholds and recovery/cooldown rather than
  cumulative totals. Reset baselines on backwards counters, session changes or
  stale/missing samples. `obs_stats_observed_at` now carries request-start time.
- Keep network drops, encoder skips and render skips separate. The existing status
  bar's `set_stats` still highlights the cumulative encoder count whenever nonzero;
  remove that ongoing-incident implication when adding current alert state.
- Add persistent cross-page presentation (Live, Mixer, Sources and Stats), with
  actionable explanations and freshness/recovery. Preserve the existing compact
  window/output-control layouts and scoped widget lifetime behavior.
- Low recording space uses the actual OBS recording volume. Connection loss should
  remain visible while disconnected, and recover on an observed reconnect. Detect
  unexpected output termination from observed transitions and admitted stop intent;
  do not interpret acknowledgements, failed reads or session retirement as proof
  of a completed output action.
- Silence/clipping alerts are optional and apply only to explicitly selected
  identified inputs. Reuse meter timestamps; missing/frozen meters are not silence,
  and input UUIDs cannot fall back to reused names. Separate alert preferences from
  preflight requirements unless the UI explicitly asks the operator to reuse them.
- F07 bookmarks/media, F08 presets, F09 optional session reports, later F03 global
  shortcuts and F10 integration/Help/site verification remain required. The durable
  goal stays active until the full agreed program is implemented and verified.

### F05 verified checkpoint

- F05 is complete end to end: configurable expected collection, UUID-based
  required scenes, microphone presence/mute/recent-signal checks, recording-space
  minimum, volatile manual checklist, Live entry, read-only refresh and optional
  explicit review before output start. Existing output confirmation preferences
  remain in effect. The report shows current scene names after renames and observed
  free recording space; expected collection names retain meaningful whitespace.
- Persistence uses the actual connected saved-profile ID and endpoint. Missing
  targets survive edits; another collection cannot inherit their UUIDs. Save errors
  remain retryable, while closed or stale editor callbacks cannot save. Cancelled,
  duplicate or stale start responses cannot trigger an output later.
- Validation passed: **693 Rust tests**, **81 ignored** (80 native plus locale);
  **all 80 isolated native GTK tests**; all eight locales; formatting, type check,
  Clippy with denied warnings, production build, and `git diff --check`.
- Native coverage includes actual disk persistence and failed-save retry, actual
  versus selected connection isolation, missing targets and namespace changes,
  exact collection names, manual/signal reset and widget lifetime, cancelled/stale
  continuations, current output-confirmation preferences, and 360-pixel layouts at
  font size 22 with stable manual-checkbox focus during observation updates.
- Final logs: `/tmp/scenedeck-preflight-{tests-final,native-all,clippy-final,check,build,locale,fmt}.log`.
  Production/Fluent files were unchanged after their final build/check/locale
  gates; subsequent changes only strengthened native tests, which passed the full
  suite and final all-target Clippy gate. No jobs remain live.
- Updated user/configuration/architecture/protocol docs; comprehensive Help/site
  updates remain F10. No real OBS, keyring, portal registration, broadcasts, commits,
  pushes or deployments occurred. Unrelated `QUALITY_REVIEW.md` and `TEMP.md` remain
  untouched. All implementation changes are uncommitted.
- The durable goal is **active, not complete**. F01, F02, F04 and F05 are complete.
  Resume with F06 using the notes above; F07–F10 and F03 global shortcuts remain.

### F06a core alerts implemented

- Added a pure AppState-owned HealthMonitor. Network, render and encoder frame
  incidents use recent counter deltas (at least 1% for five seconds), with
  independent baselines. Low recording-volume space (under 1 GiB while observed
  recording) also requires five seconds. Ten seconds of consecutive valid healthy
  observations clear a warning; a brief recurrence retains it. Missing, invalid,
  stale, duplicate, out-of-order and reset data cannot count as recovery.
- Recording and encoding gates use independently retained observed output activity.
  Command fallback state cannot masquerade as a stopped output and clear a warning.
  Pending commands and failed readbacks remain distinct from output observations.
- Added connection-loss and stopped-output incidents. Deliberate Connect/Disconnect
  intent suppresses an expected connection retirement; an already visible incident
  survives retry until actual reconnect. A stopped-output alert requires previously
  observed active output, then observed inactive output without a recent admitted
  stop. Admission is established by OutputCommandPending, never a click alone.
  Definite failures retire stop expectations, uncertain outcomes do not invent
  stopped state, and expectations are bounded to thirty seconds.
- Already observed stop incidents survive a subsequent connection loss. Recovered
  outputs/new connection or explicit acknowledgement clear event incidents. Event
  IDs and single-use dialog responses prevent stale/cancelled acknowledgement from
  clearing a new incident. No alert restarts outputs or claims a file was saved.
- Main-window cross-page warning area shows the first incident plus remaining count;
  its details explain causes/actions, expose all incidents and open Stats. Text wraps
  on narrow screens and the scrollable details fit 360x600 at font size 22. Widget
  and timer ownership has native lifetime coverage. The cumulative status-bar
  encoder count is now neutral; current incidents are presented separately.
- Validation passed: **706 regular Rust tests**, **83 ignored** (82 native plus
  locale), **all 82 isolated native GTK tests**, all eight locales, formatting,
  type check, all-target Clippy with denied warnings, production build and
  `git diff --check`. Logs: `/tmp/scenedeck-health-{tests-final,native-all,clippy-final,check,build,locale,fmt}.log`.
- A subsequent test-only strengthening runs the compact Ukrainian, large-text
  full-window regression both with and without an active warning. Both passed,
  retaining output-control layout, keyboard focus, reachable scene cards and
  page navigation. Final formatting and all-target Clippy passed after that edit.
  Logs: `/tmp/scenedeck-health-{native-compact,clippy-compact}.log`; no jobs remain live.
- User/architecture/protocol docs describe the implemented core only. Optional
  selected-input silence/clipping monitoring is still required for F06. F06 is not
  complete and the overall durable goal remains active.

### Continue F06 after this core checkpoint

- Implement opt-in silence/clipping rules for explicitly selected identified audio
  inputs, scoped by connection and collection. Preserve missing targets, ensure
  imports require review, and expose add/edit/remove/enable controls with current
  monitoring state. Do not silently reuse a preflight microphone selection.
- Reuse meter timestamps; missing/frozen/invalid meters are unknown rather than
  silence. UUIDs never fall back to reused names. Use sustained thresholds,
  cooldown/recovery, reset on scope/config changes, and deduplicate observations.
- Maintain required identities/mute evidence without relying on an open Sources
  page or the main window being active. Audit detached Stats behavior: its app-owned
  event pump can outlive the shell, while the current health summary belongs to the
  shell. Decide and test how active monitoring/details are exposed there.
- Audit output-admission and output-observation timing during a prolonged GTK
  backlog. Current stop-expectation instants are captured when AppState applies
  events; preserve ingress/admission times if needed to keep the thirty-second
  correlation window independent of delayed GTK publication. Cover delayed events,
  rejected commands, unknown reads and scope retirement before declaring F06 done.
- Preserve the rest of the objective: F07 recording/media/bookmarks, F08 presets,
  F09 reports/export, F03 global shortcuts and F10 complete integration/docs remain.

### F06b selected-input monitoring verified

- Added workspace-owned `audio_alerts` rules with explicit input selection,
  enable/disable, separate silence/clipping choices and removal. Empty is the
  migration default. YAML imports preserve targets/choices but disable all rules
  for review. Rules remain independent of preflight microphone selection.
- AudioMonitor requires fresh, unambiguous catalog identity, audio mute evidence
  and meter receipt times. Known UUIDs follow renames and never fall back to reused
  names. Silence requires fifteen seconds at/below -50 dBFS post-fader or muted;
  clipping requires one second at full scale pre-fader while unmuted. Both recover
  after ten seconds of clear observations. Readings older than two seconds,
  invalid/missing channels, frozen/replayed timestamps, missing/ambiguous targets
  and duplicate rules cannot establish silence or recovery. Scope/configuration
  changes retire prior evidence; already raised unavailable incidents stay visible.
- InputLevels now retains malformed multiplier validity before the visual meter's
  existing silence conversion. Duplicate meter names are also invalid for health
  and preflight evidence. Visual meter behavior remains unchanged.
- The application-owned event-pump tick evaluates audio rules and requests existing
  read-only workspace observations when needed, at most once per ten seconds from
  this monitor, respecting pending/loading work. This works independently of page,
  window focus and shell lifetime. The main shell and detached Stats each show
  health incidents; both Live and Stats can open the report with no active warning.
  Detached health sources release on destruction/removal from the application.
- The editor keeps unavailable saved targets selected, appends newly observed
  choices without rebuilding edited widgets, reports unsaved/saving/saved/failure
  state, and supports Save-again retry through the ordered registry lane. Closed,
  session/collection-retired or externally changed drafts cannot save later.
- Seven audio evaluator tests, invalid/duplicate-meter tests, persistence/import
  and workspace isolation coverage are added. All nineteen health domain tests
  pass. Eight focused native tests pass, including failed-save retry, actual
  workspace persistence, missing/replaced targets, removal, stale/closed editor
  guards, 360x600 at font size 22, detached source lifetime, compact Ukrainian shell
  with/without an active warning, and new audio warnings after the shell closes.
- Validation passed: **716 regular Rust tests**, **85 ignored** (84 native plus
  locale), **all 84 isolated native GTK tests**, all eight locales, formatting,
  type check, all-target Clippy with denied warnings, production build and
  `git diff --check`. No jobs remain live. Logs:
  `/tmp/scenedeck-health-audio-{tests,native-all,clippy-final,check-final,build,locale,fmt}.log`.
- All eight locales and user/configuration/architecture/protocol docs are updated.
  No real OBS, keyring, portal registrations, broadcasts, commits, pushes or
  deployments were performed. Unrelated `QUALITY_REVIEW.md` and `TEMP.md` remain
  untouched; all implementation changes are uncommitted. The durable goal remains
  active; the next required increment is the output timing audit below.

### Continue F06 after audio monitoring

- The delayed-publication output timing audit from the F06a notes remains required:
  output admission and status timestamps currently originate when GTK applies
  their events. Preserve source admission/observation times so a prolonged GTK
  backlog cannot extend the thirty-second expected-stop window. Check ordering
  against command failures, newer transport echoes and readbacks; never feed an
  older workspace output status into health without an observation fence.
- Preserve the passing audio behavior when doing this: invalid/unknown readings
  cannot clear incidents; current connection/collection/registry identity controls
  rules; detached Stats keeps monitoring and releases all subscriptions on close.
- F06 is **not complete** until that audit and its regression checks pass. F07
  recording/media/bookmarks, F08 presets, F09 reports/export, F03 global shortcuts
  and F10 complete integration/Help/site verification remain in the active goal.


### F06c output timing and ordering verified — F06 complete

- Output status events carry source request-start/transport-receipt time, and
  admitted commands carry controller-admission time captured before executor or
  publication waits. The thirty-second correlation window no longer starts when
  GTK drains the event queue.
- Current output observations and read failures share per-output time fences in
  AppState. Fresh unknown reads do not imply stopped output; older reads cannot
  undo a newer echo or unknown result. Workspace observations retain separate
  stream/record request-start times and feed the same fences/health evaluator.
  Failed normal resync reads now publish unavailable evidence, with repeated
  identical error notices coalesced.
- Delayed recording file evidence survives a newer running-state observation,
  without rolling current output state or a newer latest path backward. These
  file facts refresh result consumers independently of state changes. A completed
  StopRecord response now emits only `RecordingStopObserved` file evidence; only
  status reads and transport events establish current recording state. Its command
  admission time conservatively orders the file path behind a later restart. A
  gated fake-client regression holds the response across an observed restart and
  proves that it cannot stop the new recording or replace its newer file path.
- Admitted pending output intent is connection-owned across collection changes;
  it cannot be discarded while its physical output operation continues. Session
  replacement still retires old commands, and output time fences reset only with
  the connection. Existing scoped snapshot/revision fences remain in force.
- Focused checks pass: 34 regular output tests, four sender/lifecycle cases,
  all-target Clippy, and the isolated GTK stale-read/failure test. Coverage includes
  expired and pre-admission stop observations applied in one delayed GTK batch,
  admission before executor/publication wait, wire-level stale readback after an
  actual OBS echo, late file evidence after restart, workspace failure/ordering,
  collection versus connection retirement, and unknown reads never creating stops.
- Final validation passed: **721 regular Rust tests**, **86 ignored** (85 native
  plus locale), **all 85 isolated native GTK tests**, all eight locales, formatting,
  type check, production build, all-target Clippy with denied warnings and
  `git diff --check`. Final logs: `/tmp/scenedeck-health-output-final-*.log`;
  Clippy: `/tmp/scenedeck-health-output-file-clippy.log`. No jobs remain live.
  F06 is complete. No real OBS, keyring, portal registrations, broadcasts, commits,
  pushes or deployments occurred. The overall durable goal remains active.

### Next milestone: F07 recording and media workflow

- Extend existing controls rather than duplicate them: recording pause/resume
  lives in `src/ui/widgets/transport_bar.rs`; media progress/seek widgets and
  in-place media polling updates live in `src/ui/widgets/operating_panel.rs`.
  `domain::control::MediaStatus` already carries optional duration/cursor ms,
  populated by the OBS adapter. Display progress and remaining time with explicit
  unavailable/indefinite handling, preserving focus/seek edits during polling.
- Add named timestamp bookmarks with visible session/recording timing semantics,
  preserving scope across reconnects and distinguishing actual observations from
  requests. The existing `record_active_since` is a local UI clock and must not be
  presented as an exact encoded-file offset, especially across pauses/reconnects.
  Inspect OBS timing fields before choosing the recording-position model. Make
  missing/stale timing explicit and retain evidence boundaries for F09 export.
- F08 presets, F09 optional reports/history/export, F03 global shortcuts and F10
  integration/Help/site verification remain required. Keep the full goal active.


### F07a media progress verified

- Sources now displays observed position, duration, remaining time and progress
  using existing media reads. No local playback extrapolation or new polling.
  Missing/invalid cursor and unknown/non-positive duration remain unavailable;
  stale/dirty/failed/disconnected workspace observations cannot show progress.
- Readouts update in place separately from the seek draft. All eight catalogs
  and user/architecture/protocol/operating docs updated. Native checks cover seek
  drafts/focus, paused/buffering/unknown data, staleness and narrow large-text layout.
- Validation passed: **722 regular Rust tests**, **87 ignored** (86 native plus
  locale), both targeted native Sources tests (new media timing interaction/layout
  and existing long-path/dropdown/draft preservation), all eight locale loads,
  formatting, check, all-target Clippy with denied warnings, production build and
  `git diff --check`. Logs: `/tmp/scenedeck-media-progress-*.log`. No jobs remain
  live. The full 85-test native suite passed at the preceding F06 checkpoint; only
  the two affected native tests were rerun for this localized Sources increment.
  F07 remains In progress: timestamp bookmarks and recording timing are next.
- Protocol reviewed: GetRecordStatus exposes outputDuration/timecode. Its meaning
  is output time, not an independently verified per-file offset after splitting.
  Future bookmark UI must label observed output time separately from local session
  time, preserve pauses/reconnects/unknown evidence and never infer a saved file.


### Resume F07 after media progress

- Implement named timestamp bookmarks as the next user-visible slice. Capture the
  local session timestamp at the user's action; label any OBS output-duration
  sample separately, with its observation age. Do not turn the existing
  `record_active_since` wall clock (which includes pauses) into a file offset.
- `src/obs/mapper.rs::map_record_status` currently discards the dependency's
  duration/timecode fields. Before retaining duration on `OutputStatus`, account
  for equality-based state change logging in `AppState::apply_output_event` and
  the other literal constructors. Ordinary clock advancement must not dirty the
  workspace or create a new state-change log every poll. An explicit one-shot
  bookmark timing read is another option; choose the smallest complete path.
- Current status observations are fenced by source time. A transport event has no
  duration; do not silently reuse a duration from before pause/restart/unknown
  state. If sampling asynchronously, retain click time and sample time separately,
  reject old connection/collection replies, and settle pending bookmarks on loss.
- Keep bookmark context (connection, collection, scene, session) with the record.
  Reconnect starts a new observation session rather than extending an assumed
  recording. Retain prior bookmarks for review with explicit unavailable timing;
  F09 will add opt-in bounded history and exports. Do not claim a saved file from
  a bookmark or acknowledged chapter command. Existing chapter controls are OBS
  mutations and must remain distinct from local operator bookmarks.
- Add UI management/review and all eight locales, cover pause/resume, missing
  duration, delayed reads, reconnect and scope changes with isolated tests, and
  update docs. F07 remains In progress until bookmarks and timing are verified.
- Continue afterward with F09 reports/history/export, F08 audio presets, F03 global
  shortcuts, and F10 integration/Help/site checks. Keep the durable goal active.


### Latest durable checkpoint

- F01, F02, F04, F05 and F06 complete. F03 application palette/bindings complete,
  global shortcuts pending. F07a media progress verified; F07 bookmarks next.
- All source changes are uncommitted. No real OBS, keyring, portal registration,
  broadcast, commit, push, release or deployment occurred. Unrelated
  `QUALITY_REVIEW.md` and `TEMP.md` remain untouched.
- Goal is active, not complete. Continue from the F07 resume notes above without
  restarting completed work or asking again for authorization to implement it.


### F07b bookmarks verified — F07 complete

- Added locally persisted named bookmarks, available from Live, Stats (including
  detached Stats) and the command palette. Capture stores UTC click time, local
  connection-session elapsed time, original connection/collection/scene context,
  and a separate observed recording duration/paused state/sample age when fresh.
  No new OBS request or mutation. Existing pause/resume/chapter controls remain.
- GetRecordStatus duration now survives mapping, while transport/fallback states
  contain no timing. Accepted source timestamps fence samples; five-second maximum
  sample age, unknown transitions and pending workspace operations prevent guessed
  recording time. Clock ticks do not dirty the workspace, log state changes or
  invalidate disconnect confirmation. Session time includes pauses; reconnect
  starts a new session. File splits do not imply a verified per-file offset.
- `bookmarks.json` uses a versioned archive, non-reused persisted IDs, a 2,000-note
  limit without eviction and a 16 MiB load/write bound. Startup reads off GTK;
  atomic FIFO writes report save failures and support retry. Corrupt or unsupported
  files are preserved and block edits. Notes can be renamed/removed offline.
- All eight locale catalogs and user/configuration/architecture/protocol docs are
  updated. Four focused core tests and both new native tests pass. Native coverage
  includes actual disk capture/rename/delete/reload, failed-save retry, corrupt-load
  recovery, retained editing drafts/focus, scope and closed-callback guards, offline
  review, palette access and 360x600 Ukrainian layout at font size 22. The form and
  history scroll together so the controls remain reachable at large text sizes.
- Final validation passed: **726 regular Rust tests**, **89 ignored** (88 native
  plus locale), **all 88 isolated native GTK tests**, all eight locale loads,
  formatting, check, production build, all-target Clippy with denied warnings and
  `git diff --check`. Logs: `/tmp/scenedeck-bookmark-final-*.log`; Clippy:
  `/tmp/scenedeck-bookmark-clippy.log`; focused native: `/tmp/scenedeck-bookmark-native.log`.
  No jobs remain live. F07 is complete; the overall goal is still active.


### Next after F07: F09 session reports and optional history

- Preserve the full remaining goal: F09 reports/export, F08 audio presets, later
  F03 global shortcuts and F10 integration/Help/site verification. Bookmarks are
  explicit user-created notes and remain saved independently of optional history.
- Existing evidence sources: `WorkspaceState.activity` (200 records),
  `OutputResults` (24 records with Requested/Acknowledged/Observed/Failed/
  Unconfirmed states), `StatsHistory` (120 samples), `HealthMonitor`, session
  retirement records and the new bookmark archive. These bounded views cannot be
  copied only at session end to claim complete history. Capture accepted changes
  as they occur, with explicit gaps/omissions and connection/collection context.
- Add an opt-in configuration preference, bounded background persistence and
  retention controls; default automatic disk history off. Keep passwords and
  connection secrets out of records/exports. Reuse existing background lanes and
  shutdown barriers rather than blocking GTK or writing from widget refreshes.
- Provide a compact current/post-session report and CSV/JSON save flows, including
  scene/output events, performance incidents, observed paths and bookmarks. Preserve
  requested/acknowledged/observed distinctions; acknowledge cannot establish a
  saved file, and a late file result does not establish current inactive state.
- Session clock integration lives in `controller/state/bookmarks.rs`; consider a
  shared session identity for reports without relabeling existing bookmark data.
  New reconnects must not join unrelated sessions merely because endpoint names
  match. Local bookmark context and sample age should survive report export.
- Use pure export/retention tests and isolated GTK tests for opt-in/out, stale
  readbacks, reload, failures/retry, disconnect/finalization and file exports.
  CSV text needs correct quoting and protection against spreadsheet formulas;
  JSON should retain exact structured values. Test real temporary files, no
  production OBS or user files. Revisit Help/site together in F10.


### Latest checkpoint after F07

- F01, F02, F04, F05, F06 and F07 are complete and verified. F03 application
  palette/bindings are complete; global shortcuts remain. F09 optional session
  reports/export is next, followed by F08 audio presets, F03 global shortcuts and
  F10 complete integration/Help/site validation. Use the F09 notes above.
- No real OBS, keyring, portal registrations, broadcasts, commits, pushes,
  deployments or releases occurred. All code is uncommitted; unrelated
  `QUALITY_REVIEW.md` and `TEMP.md` remain untouched. No background checks are live.
- This continuation made implementation and verification progress. The durable
  goal remains active and must not be marked complete while F08/F09/F10 and global
  shortcuts are unfinished.

### F09a: current/recent session reports and explicit export

- Implemented a bounded in-memory collector in `controller/state/session_reports`
  and pure report models in `domain/session_report`. The app-owned event pump
  collects before/after accepted publications and health ticks, independently of
  the selected page or surviving main shell. Eight connection sessions are kept;
  each has at most 4,096 events and 2,048 telemetry samples. Omission counts,
  activity cursor gaps and all-sample CPU/FPS aggregates remain visible.
- Output results preserve distinct requested, acknowledged, observed, failed and
  unconfirmed snapshots. Direct stream/record acknowledgements are recorded without
  manufacturing output state or file evidence. Reset captures final uncertainty
  before clearing caches. Generation routing attaches late terminal audit to the
  original retained report instead of the replacement connection.
- Collection context accompanies event records. Statistics use source observation
  timestamps and sanitize invalid/nonfinite values. A repeated tick does not add a
  new sample. Alert removal is labelled cleared, without a claim of recovery.
  Bookmark references resolve the separate archive's latest edits/deletions.
- Stats (including detached Stats) and the command palette open Session reports.
  Previous/Next selects a retained session; Refresh snapshot is explicit so review
  and export use stable data. The viewer shows the newest 150 events/50 bookmarks;
  JSON and CSV export the complete retained snapshot to the exact selected local
  path through atomic FIFO background writes. CSV protects spreadsheet formulas.
  Export failures stay visible and participate in shutdown failure reporting;
  a successful explicit retry resolves the same target/report failure.
- Added all new labels in all eight locales and updated user/configuration and
  architecture documentation. This slice creates no automatic history file.
  F09 remains **In progress** until opt-in history and retention are implemented.
- Final verification passed: **735 regular Rust tests**, **89 isolated native GTK
  tests**, all eight locale loads, `cargo check`, `cargo build`, Clippy all targets/
  all features with warnings denied, formatting and `git diff --check`. The new
  native test covers empty state, snapshot navigation, export failure/retry, real
  temporary JSON, Ukrainian large-text 360×600 layout and retired callbacks.
  The strengthened shell/popout lifecycle test verifies report collection and
  finalization with only detached Stats remaining. Exec session **92281** ended
  with exit 0; logs are `/tmp/scenedeck-report-final-*.log`. No checks remain live.

### Remaining F09 increment: optional disk history

- Automatic persistence must default off and expose explicit retention controls.
  Use a bounded versioned archive, bounded reads/writes, ordered background I/O,
  startup reload, visible load/save failures and explicit retry. Preserve corrupt
  or newer-version files. Snapshot writes must not accidentally overwrite unrelated
  sessions whose IDs collide. Current report IDs include connection-start UTC ms,
  the bookmark session ID and an in-process counter; validate collision semantics
  when persisting/reloading, especially after clock changes or bookmark-load retry.
- Consider storing the saved connection's display name alongside its stable ID,
  captured from the actual connected profile rather than the Settings selection.
- Reports currently close on connection reset/replacement. For durable history,
  also finalize/flush on app shutdown via existing admission/draining barriers,
  retaining an explicit interrupted/incomplete state when a process crashes.
  Background checkpoints need bounded/coalesced work rather than a new queued file
  snapshot on every 50 ms tick. Disabling history must stop future writes without
  deleting existing history unless the user explicitly selects clear history.
- Bookmarks stay independently saved and editable. Persist references in history;
  only explicit exports embed resolved bookmark content. Do not resurrect removed
  notes from a history reload. Keep Requested/Acknowledged/Observed boundaries,
  omission counts and terminal generation ownership intact.
- Test opt-in/out, retention, restart/reload, corrupt/newer files, clock/ID collisions,
  failures/retry, shutdown, old-session evidence, and detached-window operation.
  F08 audio presets, F03 global shortcuts and F10 Help/site/integration remain after
  F09. Do not mark the overall goal complete at this checkpoint.

### Latest durable checkpoint after F09a

- F09a is implemented and fully verified. F09 remains in progress: the next
  concrete work is opt-in disk history, retention, reload/retry and shutdown
  integration as listed above. Then continue F08 audio presets, later F03 global
  shortcuts and F10 Help/site/integration. The overall goal remains active.
- No Rust/Fluent code changed after final validation. All work remains uncommitted;
  unrelated `QUALITY_REVIEW.md` and `TEMP.md` were not changed. No real OBS, keyring,
  desktop shortcut registration, broadcasts, pushes, releases or deployments.
- This continuation made implementation and verification progress. Do not mark the
  goal complete or blocked; no external/user input is needed for the next increment.

### F09b: opt-in history, retention and recovery

- Added `HistoryPolicy` (`enabled: false`, eight retained sessions by default,
  bounded to 1–32), with explicit Apply controls in Settings and the report viewer.
  Enabling includes the current retained report and future connections. Disabling
  stops new checkpoints and keeps existing history. Applying a smaller limit prunes
  oldest sessions by receipt order, so backwards wall clocks do not reorder them.
- `storage/report_history` reads a bounded, versioned 64 MiB archive at
  `$XDG_STATE_HOME/scenedeck/session-history.json` (fallback `~/.local/state`). It
  rejects corrupt/newer/invalid/special-file data, serializes cooperating writers
  with a regular lock file, checks expected disk contents, and replaces atomically.
  No lock/file is created by the default disabled path. Session IDs include process
  identity and sub-millisecond time, and are checked against loaded IDs. Upsert
  rejects an ID belonging to another connection/session context.
- Startup now has a named `StartupData` structure and reads history off GTK.
  Previously open reports are marked interrupted without an invented end time.
  Desired and confirmed disk snapshots remain separate, and reload merges locally
  owned reports without overwriting another writer's sessions. Bookmarks remain
  references into the separate archive. Connection display names are captured from
  the actual connection profile when available.
- The app-owned history writer admits one operation at a time, with roughly
  ten-second checkpoints and earlier writes at session boundaries. Forced flushes
  chain the newest snapshot after an in-flight operation; no per-tick file queue.
  Main Close/Quit admits a checkpoint; shutdown finalizes the observation report
  and marks pending output-result tickets unconfirmed before the existing drain.
  Late terminal audits can reach an original retained history report after it has
  left the eight-session memory ring.
- Separate history/configuration error state and failure tickets support Reload
  and retry. Clear requires confirmation, preserves bookmark/current-memory data,
  excludes the current connection from future automatic checkpoints, and retains
  clear intent across read/retry. Fixed a GTK response reentrancy bug caught by the
  new native test: the one-shot guard must be set before closing the confirmation.
- Exact telemetry round-tripping is required by expected-file comparison. A local
  compiled probe demonstrated `112.21135131417657` changing by one bit with the
  default JSON parser, which would manufacture a conflict on the next checkpoint.
  Enabled the existing serde_json `float_roundtrip` feature (no version/lockfile
  change) and added a real-file consecutive-write regression.
- Initial checks: seven new pure history tests passed, then the 742-test full
  regular suite and Clippy passed after replacing the complex startup tuple.
  All three focused report/history GTK tests passed, covering opt-in/off, retention,
  interrupted reload, history/config errors and retry, confirmed clear, large-text
  Ukrainian layout, stable export snapshots, latest-snapshot shutdown draining and
  conflict recovery that preserves another writer's sessions.
- Final validation after the numeric-roundtrip fix passed: **743 regular Rust
  tests**, **91 isolated native GTK tests**, all eight locale loads, build, check,
  Clippy all targets/all features with warnings denied, formatting and diff checks.
  Exec session **74635** ended with exit 0; logs are
  `/tmp/scenedeck-history-final-*.log`. No checks remain live. F09 remains in
  progress until the final lifecycle audit below is addressed.

### F09 final lifecycle audit (resolved in F09c below)

- Inspect and test the actual last-window/app Quit path as well as the helper-level
  shutdown test. The main window has a persistence close guard, but a detached
  Stats/report window may be the last one, and the fallback Quit path currently
  calls `app.quit()` directly. The shutdown hook drains writes and logs errors,
  but verify failure/retry remains available when only a detached window exists.
- The new late-terminal routing supports disk-retained reports beyond the memory
  ring. Add a targeted ownership regression that evicts the original session from
  memory, delivers its terminal generation after reconnects, and checks only its
  archived report changes. Verify clear/retention cannot resurrect removed history.
- Complete the feature's final evidence review and keep F08 audio presets, F03
  global shortcuts and F10 Help/site/integration in scope afterward. The previous
  goal turn and this continuation both made implementation/verification progress.
- Concrete lifecycle concern from source inspection: `ui::report_history::ACTIVE`
  is weak, and `ui::event_pump` destroys its receiver/state closure on last-window
  removal. The shutdown hook must not depend on a page timer incidentally keeping
  AppState alive. Ensure final checkpoint admission/ownership happens before that
  release, without retaining destroyed shell widgets or breaking the existing
  assembled-window lifetime test. A failure during a last detached-window close
  must leave a usable retry/cancel path. Do not treat helper tests alone as proof
  that this actual application lifetime is covered.

### Checkpoint after F09b (superseded by F09c below)

- Optional history, retention, startup reload, conflict-safe atomic writes,
  separate settings/history failure recovery, clear confirmation and history
  viewer integration are implemented and validated. The new numeric roundtrip
  regression passes, and no Rust/Fluent code changed after the final full checks.
- Continue next with the F09 final lifecycle audit above: actual last detached
  window Close/Quit ownership, failure/cancel/retry, and late terminal evidence
  after eviction from the eight-session memory ring. Keep F09 In progress until
  those cases are proved. Then F08 audio presets, F03 global shortcuts and F10
  Help/site/integration remain; do not shrink or complete the overall goal.
- No background jobs remain. All changes are uncommitted; no real OBS, keyring,
  shortcut registration, broadcasts, commits, pushes, deployments or releases.
  `QUALITY_REVIEW.md` and `TEMP.md` were not changed. This turn made concrete
  implementation and verification progress; no user input is needed to continue.

### F09c exit ownership and late evidence

- The close guard now covers the last detached document and routes the Quit action
  through the same save barrier after the shell has closed. A temporary checkpoint
  holds application state until completion without retaining destroyed widgets.
  Main-window close keeps the current report active if another document survives.
- Final report snapshots copy outstanding requests as unconfirmed without changing
  live request tickets. Failed or delayed exits resume the same still-connected
  report and record the interruption; reconnects cannot reopen an old report.
  Retry reloads history, and informed abandonment suppresses additional automatic
  history writes during that exit attempt. Corrupt startup history with unsaved
  local reports now participates in the failure decision instead of silently exiting.
- Pure regressions cover late acknowledgements after cancellation, replaced
  sessions, archived terminal evidence after memory-ring eviction, and no
  resurrection after retention or explicit clearing. Native regressions exercise
  real assembled shell/Stats Close and Quit with a blocked history writer, state
  release, and detached failure/Cancel/Retry/abandon. All 12 focused application
  lifecycle tests passed (`/tmp/scenedeck-exit-native.log`, session 99875 exit 0).
- Final broad validation passed: **746 regular Rust tests**, **94 isolated native
  GTK tests**, all eight locale loads, check, build, Clippy all targets/all features
  with warnings denied, formatting and diff checks. Exec session **55618** ended
  with exit 0; logs are `/tmp/scenedeck-exit-final-*.log`. No jobs remain live and
  no Rust/Fluent code changed after those checks. F09 is complete.
- The overall program remains active. Continue with F08 below, then F03 global
  shortcuts and F10. This turn made concrete implementation and verification
  progress; there is no blocker or missing user decision. All changes remain
  uncommitted, with unrelated `QUALITY_REVIEW.md` and `TEMP.md` preserved.

### F08 implementation direction

- Persist named selected-input gain/mute snapshots in the scoped `SceneRegistry`;
  reuse `ActionTarget`/`resolve_input` for UUID identity and deliberate name-only
  fallback. Capture from fresh complete workspace audio/catalog observations, not
  the filtered active-scene Mixer display. The application already has complete
  reads in `controller::workspace::read_observation` and identity-aware OBS
  mutations in `ObsClient::execute_control_with_identity_if`.
- The current single-input `WorkspaceController::audio` admits up to 16 concurrent
  tasks and returns only a generic failure event. Do not implement preset Apply by
  firing that method repeatedly: large presets could silently exceed admission
  and would lack per-input acknowledgement/readback evidence. Add a bounded,
  scope-owned apply operation with explicit per-input outcomes, no reconnect
  replay, and identity/cancellation checks at each mutation boundary.
- Preview missing/unavailable inputs and gain/mute changes, explain shared-input
  effects, and require a current preview for Apply. Saving/renaming/deleting presets
  must retain workspace/generation ownership and the existing registry save lane.
  Preserve the registry's equality/serialization contracts when representing gain;
  reject invalid/nonfinite values instead of silently normalizing imported values.
- Keep F03 portal global shortcuts and F10 Help/site/integration pending after F08.
  No real OBS, keyring, portal registration, broadcasts, commits or publishing are
  part of fixture verification. No clarification or approval is currently needed.

### F08a capture, local management and live preview

- Added `domain::audio_preset` and workspace-owned `SceneRegistry::audio_presets`.
  Names are bounded to 80 characters, the map to 64 presets and each preset to
  1–64 unique input identities. Old registries load with no presets. Invalid or
  nonfinite gain, duplicate targets and malformed fields fail instead of silently
  changing imported settings. Gain is stored as a checked finite linear multiplier
  0–20, preserving silence, quiet values and positive dB gain independently of
  SceneDeck's current fader range. The current primary OBS protocol confirms
  this range: <https://github.com/obsproject/obs-websocket/blob/master/docs/generated/protocol.md#setinputvolume>.
- Added Mixer → Audio presets with selected-input capture, replace, rename, delete,
  saved/current comparison and missing/unavailable input labels. Captures come
  from the complete fresh workspace catalog/audio observations, never the filtered
  Mixer cards. Known UUIDs follow renames and reject reused names with new UUIDs.
  Missing selected inputs block replacement until explicitly deselected. The
  editor explains shared-input effects and retires on scope or concurrent preset
  changes. Registry writes/retries use the existing FIFO lane. All eight catalogs
  and configuration/user/architecture documentation have been updated.
- Two pure storage regressions passed (`/tmp/scenedeck-presets-storage.log`, exec
  77961 exit 0). Both focused native editor regressions passed
  (`/tmp/scenedeck-presets-editor-native.log`, exec 12275 exit 0), including UUID
  rename/name reuse, missing selections, exact gain capture, disk failure/retry,
  stale callbacks, workspace ownership and Ukrainian Sans 22 at 360×600. A test
  module import typo was corrected before that successful run. A subsequent
  one-line fix clears stale validation after a valid no-op Save; broad validation
  includes it.
- Broad validation passed: **748 regular Rust tests**, **96 isolated native GTK
  tests**, all eight locale loads, check, build, Clippy all targets/all features
  with warnings denied, formatting and diff checks. Exec **1271** ended with exit
  0; logs are `/tmp/scenedeck-presets-capture-*.log`. No jobs remain live and no
  Rust/Fluent code changed after validation. F08 remains In progress: this UI
  captures and reviews presets but has no Apply button yet.
- All changes remain uncommitted. No real OBS, keyring, portal registration,
  broadcasts, commits, pushes or publishing were used. Unrelated
  `QUALITY_REVIEW.md` and `TEMP.md` were preserved. Continue directly with F08b
  below; this is a verified partial milestone, not completion of F08 or the goal.

### F08b implementation requirements (completed below)

- Implement an explicit bounded apply operation through NavigationContext and
  AppController, with a current preview and before-values. Reuse the OBS identity
  resolver and mutation gates. Add a direct linear-gain adapter for presets:
  converting through the current −100–0 dB fader helper would lose valid stored
  gain. `obws` must remain confined to `src/obs/`.
- Resolve each target against a fresh catalog and verify its previewed values
  before writing. Missing/unavailable or changed inputs must be visible in the
  result rather than silently counted as applied. Preserve separate acknowledgement
  and observed-readback evidence; a timeout/retired socket is not successful Apply.
  Applying available inputs may partially succeed; do not claim atomicity or
  automatically retry after reconnect. Set gain before unmuting, and never unmute
  after a failed gain write. A requested mute can precede gain changes.
- Use one bounded sequential operation rather than the existing 16-task generic
  audio admission. Results need run/epoch/input identity, known partial progress,
  and integration with current session/collection retirement and late terminal
  evidence. Inspect `controller::lifecycle::OperationKind`/`OperationLease`,
  `AppState::apply_terminal_evidence`, and the application-owned event pump before
  choosing where progress lives. Keep in-flight outcomes available if the preset
  editor closes; a replacement setup must never inherit pending mutations.
- Add protocol fixtures for directional mute/gain, identity and scope fencing,
  readback/error/partial outcomes, then native Apply/review tests and final F08
  documentation. Preserve F03 global shortcuts and F10 Help/site/integration.
  No user input is required; the last goal turn and this one both made progress.


### F08b verified checkpoint — scoped Apply and per-input evidence

- Implemented Review → Apply with immutable before-values, 30-second freshness,
  capability checks, missing-input preview, Stop remaining, and results retained
  in AppState across editor closes. All eight locales and user/architecture/OBS
  documentation describe acknowledgement, readback, partial success and no replay.
- A single sequential worker handles up to 64 inputs. It re-resolves UUIDs, checks
  collection/topology/cancellation fences and before-values, and reads back each
  directional property write. Gain is confirmed before unmuting; mute can precede
  gain. The direct linear-gain adapter preserves 0–20 multipliers. Collection
  switch intent cancels unsent work immediately. Timeout retires the socket and
  cancels unsent rows without reconnect replay; GUI publication is outside the
  ten-second network deadline. Operation journal evidence retains input identity
  and late acknowledgements without overwriting terminal or newer runs.
- Protocol fixtures cover 64 inputs, UUID rename/reused names, changed review,
  missing targets, partial failure, readback failure, cancellation before/after
  gain acknowledgement, collection retirement and timeout. State tests cover late
  evidence and stale runs; a native test covers review fences, duplicate Apply and
  reopening results. A fixture initially emitted `{}` for void success instead of
  OBS's omitted/null response data; corrected before successful verification.
- Broad validation completed: **756 regular Rust tests**, **97 isolated native GTK
  tests**, all eight locale loads, check, build, Clippy all targets/all features
  with warnings denied, formatting and diff checks. Logs:
  `/tmp/scenedeck-presets-apply-final-*.log`. The sequential shell PID 1792617
  reached its final formatting step and exited; no jobs remain live. Its original
  exec session identifier was lost during compaction, so each log and process
  termination were checked directly. No source edits followed this validation.
- F08 is complete. Continue with F03 capability-gated desktop portal shortcuts,
  then F10 integration, Help/site claims and final acceptance audit. Overall goal
  remains active. No real OBS, keyring, portal registrations, broadcasts, commits,
  pushes or publishing were used. Unrelated QUALITY_REVIEW.md and TEMP.md remain.


### F03 desktop shortcuts — verified checkpoint

- Added explicit GlobalShortcuts portal registration for up to 64 enabled,
  non-conflicting workspace bindings. Uses asynchronous GIO already in the
  dependency graph; v1 capability is checked before registration. Desktop
  assignments are authoritative and shown as returned, including partial/empty
  grants. Stable SHA-256 IDs include connection/endpoint/collection and UUID-based
  intentions, independent of scene order, rename display text or local keys.
- Portal calls/signals pin the unique bus owner. Subscriptions precede requests;
  scoped guards close sessions and outstanding permission dialogs. Cancellation,
  disconnect/switch intent, binding/workspace changes, window closure, revocation
  and owner loss retire dispatch. No auto-registration/reconnect replay. Held keys
  dispatch once, modal/text editing suppresses them, and corresponding local keys
  stand down while a desktop binding is active to prevent double dispatch.
- Added one pure identity/conflict regression and two native fake-portal fixtures
  covering fast responses, capability absence, subset grants, permission cancel,
  Stop during a pending dialog, assignment/session revocation, owner loss, repeat,
  modal suppression, local duplicate suppression and scope/switch-intent fencing.
  Existing binding-manager layout test found an initial width regression; wrapped
  labels inside the scrolling body fixed it. A new fixture's transient Live handle
  and unregistered test application were corrected before passing verification.
- Complete checkpoint passed: **757 regular Rust tests**, **99 isolated native GTK
  tests**, all eight locale loads, check, build, Clippy all targets/all features
  with denied warnings, formatting and diff checks. Sequential exec **36428**
  exit 0, logs `/tmp/scenedeck-global-final-*.log`. No real portal or OBS was used.
  F03 is complete. The overall goal remains active for F10.

### F10 final documentation and integration audit — in progress

- Corrected stale rename and storage descriptions, added show-tool guidance in
  Help for all eight locales, updated README/codebase/operating/configuration/user
  guides and the manual acceptance matrix. The product site describes source-build
  features without claiming old release downloads already contain them.
- Site gates passed: locked install without scripts, **29 Node tests**, static
  references/syntax/provenance checks and npm audit (zero vulnerabilities), exec
  **15167** exit 0. Python gates: **49 tests, 2 expected skips**, advisory exception
  expiry checker passed, exec **23075** exit 0. Cargo audit and cargo-deny passed
  (execs **80530**, **21802**, exit 0), with the repository's existing allowed
  warnings and time-limited advisory exception; this is not a zero-advisory claim.
  Logs `/tmp/scenedeck-integration-{site-*,python,exceptions,rust-audit,rust-deny}.log`.
- F10 Help Rust/Fluent changes were applied only after the F03 verification shell
  ended. Run the final Rust/locale/native checks on these edits and inspect final
  acceptance coverage before marking the feature program or goal complete.
  All changes remain uncommitted; QUALITY_REVIEW.md and TEMP.md are untouched.

- Final audit found a retained text-entry focus could suppress background global
  actions after another app gained focus. Gate text-entry suppression on actual
  main-window activation; keep visible modal suppression. Added a native two-window
  check for active typing versus inactive remembered focus. The first final run
  (exec 73688) passed regular/check/Clippy/build/locale steps but was intentionally
  stopped during native checks (exit 143) before this fix; do not count it as a
  complete final gate. Rerun final checks on the corrected code below.

- The native focus regression first failed against the old guard, then passed
  after the active-window condition was applied (exec 26190 exit 0,
  `/tmp/scenedeck-global-focus-native2.log`). A further compatibility review found
  the current host Registry API must associate an unsandboxed peer with its app ID
  before other portal calls. See the primary documentation:
  <https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.host.portal.Registry.html>.
  Added a dedicated D-Bus peer with Register(`io.scenedeck.app`), asynchronous
  lookup/cleanup, and compatibility with missing Registry or sandbox-derived IDs.
  The fake portal now requires registration before property reads/CreateSession
  and verifies old/sandbox fallback behavior. Host desktop-entry requirements are
  documented. No real desktop registration was made.
- The second final run (exec 62506) was intentionally stopped during native checks
  before the identity adapter edit (exit 143); its completed regular/check/Clippy/
  build/locale steps are partial evidence, not a completed final gate. Continue
  with the focused identity fixture and final complete suite on the corrected code.


### Final acceptance — complete (2026-11-05)

All ten milestones are implemented and verified. Final review retained every item
from the combined request:

- Saved connections, isolated show workspaces, collection-specific curation and
  UUID-preserving rename behavior; search, editable tags, favorites and named decks.
- Command palette, stable customizable local bindings, opt-in capability-gated
  desktop shortcuts, and configurable Live quick actions.
- Preflight collection/scenes/microphone/disk/manual checks and sustained,
  cross-page health alerts, including selected-input audio rules.
- Recording pause/resume and media transport integrated with progress/remaining
  time and local timestamp bookmarks with explicit timing semantics.
- Selected-input audio presets with current preview, missing-input handling,
  shared-input notice, scoped sequential Apply, per-input acknowledgement/readback
  evidence, partial outcomes, cancellation and no reconnect replay.
- Optional bounded persistent session history, reports and CSV/JSON exports;
  scope-preserving events, output evidence, bookmarks, retention and save recovery.
- Eight localized catalogs, in-app Help, user/developer/configuration docs, manual
  acceptance guidance and product-site source-build claims updated.

The final sequential verification **exec 78610 exited 0** after **757 regular Rust
tests**, **99 isolated native GTK tests**, all eight locale loads, `cargo check`,
production `cargo build`, all-target/all-feature Clippy with `-D warnings`, format
and diff checks. Logs are `/tmp/scenedeck-complete-*.log`. This run includes the
active-window focus fix, dedicated host-registered portal peer, and final Help
translations. No Rust/Fluent edits followed it. The focused host-identity fixtures
also passed (exec 56346 exit 0, `/tmp/scenedeck-global-identity-native.log`).

Independent integration checks passed on unchanged inputs: **29 site tests**, site
static/provenance validation, npm audit with zero vulnerabilities; **49 Python
verification tests (2 expected skips)**; advisory-expiry validation; cargo-audit and
cargo-deny under the existing repository policy. Dependency maintenance warnings
and the existing basic-toml exception expiring 2026-11-20 remain documented in
`docs/advisory-exceptions.md`; they were neither added nor suppressed by this work.
Logs remain under `/tmp/scenedeck-integration-*.log` as described above.

Verification used disposable OBS protocol fixtures, a fake desktop portal on an
isolated D-Bus session, and isolated native GTK processes. It does not certify an
actual OBS installation, every desktop backend, packaged sandbox permissions or
real broadcast performance. Host desktop shortcuts require the installed
`io.scenedeck.app.desktop` identity and a supporting portal. The manual test plan
records those environment-specific smoke checks without claiming they were run.

No required implementation work or verification jobs remain. The complete change
set is uncommitted and ready for review. No commits, pushes, releases, deployment,
real keyring/portal registration or production OBS mutations were performed.
Unrelated `QUALITY_REVIEW.md` and `TEMP.md` were preserved. The durable goal can now
be marked complete; new work should begin from this final checkpoint.
