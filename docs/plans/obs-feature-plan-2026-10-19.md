> Historical feature plan and delivery record. Superseded by [current PLAN.md](../../PLAN.md). Relative file links have been rebased; task contracts and prior results are retained.

# SceneDeck development plan

**Current design revision:** user feedback supersedes the previous Live layout
and rejects rehearsal baselines, saved setup expectations and manual show cues.
Follow the [scene-wall redesign brief](../../docs/ideas/live-scene-wall.md) for the next
implementation. It specifies scene-card previews, a topbar Studio toggle,
conditional transition controls, compact audio and outputs including virtual
camera, and a dedicated Sources destination. There is no Show page. The delivery
record below describes the earlier implementation; the redesign and feature
removal have not yet been applied to application code.

**Implementation status:** all five OBS-only feature waves have working implementations. See [implementation and verification](../../docs/verification/2026-10-19/README.md) and the [operating guide](../../docs/obs-operating-workspace.md).

Planning and implementation date: **2026-10-19**. Baseline: `003d9ba1aa72cdbddb16884c9eb0a8b97704418b`, SceneDeck 0.4.0. Both research passes and the original task history remain preserved.

## 1. Current state and next action

| State | Evidence |
|---|---|
| Implemented | Source inspection/search/visibility/usage; setup checks; rehearsal comparison; UUID-bound watched audio; recording/replay; Studio; still images; media/filter/text/advanced audio; manual cues; activity/chart context; adaptive desktop UI |
| Verified | Rust checks, local WebSocket regression fixtures, isolated locale/display tests, disposable OBS 32.2.2, GTK interactions and screenshots; exact coverage is in the verification report |
| Tracking | [tasks/todo.md](../../tasks/todo.obs-features-history.md) distinguishes delivered features from a complete release qualification |
| Preserved | Earlier research, deferred ideas, original task contracts and checklist history |

No feature implementation group from these five waves remains queued. Remaining release qualification is explicitly listed in the report: native-speaker editorial review, Orca speech output, a real remote-host/network matrix, and compatible recording formats for successful split/chapter operations. These are coverage limits, not claims of completed validation. No release or broadcast was performed.

The detailed contracts below are retained as the original planning record. Architecture was consolidated into cohesive workspace/control/workflow modules rather than creating a separate controller and storage file for every feature. See the implementation map in the operating guide for actual paths and tradeoffs.

## 2. Scope and product decisions

Build a Linux desktop OBS controller that makes recurring operations faster and their effects easier to understand.

| Delivery wave | Visible outcome | Detailed tasks |
|---|---|---|
| 1 | Complete scene-item control and improve existing operation | [Operating core](#wave-1) |
| 2 | Explain shared-source usage and check explicit expectations | [Context and preflight](#wave-2) |
| 3 | Compare rehearsal settings and improve audio work | [Repeatability](#wave-3) |
| 4 | Add the remaining focused native OBS controls | [Native control depth](#wave-4) |
| 5 | Add cues and investigation context when validated | [Conditional workflows](#wave-5) |

Wave 4 contains independent feature groups. Replay can move forward after Wave 1; Studio can move forward after scope handling. Waves 2–3 do not depend on Studio or screenshots. UI accessibility tasks with no dependencies can move forward at any time.

**Included:** native OBS WebSocket requests/events; local preferences, expectations, baselines and cue data. Preserve the GTK4/libadwaita shell, scene roles/order, Mixer, Graph, Doctor and Stats.

**Excluded:** chat; platform accounts/APIs; vendor/plugin requests; integrations with Companion or other controllers; remote asset transfer; a full scene/composition editor; arbitrary scripts/macros; a dashboard framework. A capability does not enter scope merely because a vendor transports it over OBS WebSocket.

Planning defaults resolve the earlier open choices:

1. **Audience:** one Linux operator controlling one local or remote OBS instance. No multi-OBS orchestration.
2. **Disconnect:** preserve existing stream/record shutdown behavior, name it explicitly, and apply the relevant stop confirmations. Do not silently extend that shutdown set to newly added outputs. Detach-only would be a separate behavior change.
3. **Scene actions:** inspection never changes Program. Direct mode retains immediate switching. Studio mode uses explicit staging and Take.
4. **Readiness/baselines:** fresh reads, explicit unknowns, scope-bound results. Preflight does not block Start or repair settings. Baseline capture/comparison is read-only toward OBS.
5. **Feature depth:** explicit setters and focused controls first. No all-settings snapshots, transactional restore, T-bar, timed cues or automatic output/media actions.

## 3. Sources and continuity

- [First research](../../docs/obs-only-feature-research.md): current features, UI/UX and initial parity gaps.
- [First review](../../docs/obs-only-research-review.md): disconnect risk and scope discipline.
- [Second research](../../docs/obs-controller-feature-opportunities.md): pinned OBS Blade/obs-web/Companion evidence and seven proposed differentiating workflows.
- [Original UI plan](../../tasks/plan.first-pass-history.md) and [original checklist history](../../tasks/todo.obs-features-history.md#first-pass-history): retained, not marked completed by this document.
- [CLAUDE.md](../../CLAUDE.md), [architecture](../../docs/architecture.md), [OBS integration](../../docs/obs-websocket.md) and [CI](../../.github/workflows/ci.yml): implementation boundaries and required checks.

This root `PLAN.md` is the current execution plan requested by the user. The old `tasks/plan.md` is retained with a pointer here. The current completion list is in `tasks/todo.md`; historical unchecked boxes remain historical, not duplicate active assignments.

Task-management conventions are applied to the existing repository checklist. No personal productivity workspace, `TASKS.md`, dashboard, external tracker, commit or release is created by this planning task.

### Mapping the first plan into this plan

| Previous work | Current task IDs |
|---|---|
| U1 / U2 / U3 | D01 / D02 / Q01 |
| U4 / U5 / U6 | Q02 / Q03 / Q04 |
| U7 / U8 / U9 | Q05 / Q06 / Q07 |
| C1 source visibility; C2a pause; C2b replay | S01–S07; O01–O03; R01–R06 |
| C3 Studio; C4 images; C5 media; C6 filters | T01–T08; I01–I03; M01–M03; F01–F03 |
| L1 / L2 locale work | Explicit LA/LB tasks for every delivered wave |
| New source context / preflight / baseline | U01–U03 / P01–P06 / B01–B06 |
| New audio watchlist / cues / timeline | W01–W03 / C01–C06 / H01–H03 |

## 4. Execution rules

1. Pick one ready task from the checklist. Read its dependencies and actual callers before editing. Recheck the file list; split it if more than five files are needed.
2. Implement one small change, then run focused checks before expanding. Aim to check nontrivial logic before accumulating roughly 100 changed lines.
3. Keep the project compilable after each task. A new command/event variant must include its exhaustive consumers in the same task.
4. Finish the group checkpoint before exposing an incomplete feature or moving into unrelated work. Mark tasks complete only after acceptance and applicable definition-of-done checks pass.
5. Record evidence and the next task in the checklist. Leave a reviewable checkpoint; do not assume commits, pushes or releases are authorized.

A task is a bounded work unit. Two or three units commonly form one complete vertical feature slice because the repository separates OBS, domain, controller, GTK and localization. Internal adapter/state tasks are labeled as support work; they are not user-facing feature completion.

Before the first code edit, run Q once and record any existing failures with the current commit and test environment. Do not attribute a pre-existing failure to a new feature or silently relax the checks.

Keep related support changes in one working branch until their consumer and tests exist. Do not add broad `allow(dead_code)` annotations or stub success handlers to pass a partial checkpoint. If a support unit cannot pass on its own, it remains unfinished within the current group until the complete path passes. Do not commit or release it as complete.

Unfinished UI remains unexposed. A feature-flag framework is unnecessary. If a future delivery requires merging incomplete work, use a single explicit temporary flag with a removal task and test both states.

### Standard verification Q

Every task's **Verify** field adds specific cases to this common requirement:

- Run the relevant inline tests after logic changes. Example: `cargo test --workspace --all-features --locked controller::state`. Add meaningful cases for new behavior and verify the filter actually runs them; a zero-test result is not evidence.
- Run `cargo check --workspace --all-features --locked` after code changes and `cargo fmt --all -- --check` before a checkpoint.
- At each completed group, run the full test/Clippy checks below. A task cannot be marked complete while required checks fail.
- For UI changes, exercise the stated rendered interaction. For OBS mutations, test failures and external state changes through the existing fake-client/local-WebSocket seams, then use disposable live OBS data.
- Repeat a successful check only after relevant code changes or new unresolved evidence. Layout/text-only work needs rendered verification, not tests mirroring literal widget construction.

```sh
cargo fmt --all -- --check
cargo check --workspace --all-features --locked
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

Locale tests mutate global state. Run them separately after the wave's catalog tasks:

```sh
cargo test --workspace --all-features --locked -- --ignored --test-threads=1
```

Before release, the existing dependency audit and deny jobs must also pass under repository policy. No new dependency is planned. Do not run real public streams as unattended tests.

### Scope estimates and companion work

**XS:** one file. **S:** two files. **M:** three to five files. File lists include module registration and known exhaustive routing. They are forecasts, not permission to omit an affected caller. A newly discovered sixth file triggers a visible split/update before coding.

Tests live inline in the touched modules. New user-facing copy includes English in the core UI task. The other seven locale catalogs are explicit LA/LB companion work, not hidden in a five-file estimate. Documentation and rendered/manual release evidence belong to LC. Development may use English fallback; release must restore locale parity.

## 5. Architecture and correctness contracts

### Preserve current boundaries

GTK owns widgets and `Rc<RefCell<AppState>>`. Tokio owns OBS work. Pages use `NavigationContext::dispatch`; only `src/obs/` imports `obws`. Keep the bounded event channel and current polling/update model. Do not create another store, event bus or generic command engine.

Clone shared handles before awaiting; never hold a standard mutex across an await. New asynchronous work belongs to a session/visible-target lifetime. Command failure is not a connection failure. Keep local file I/O on the existing background/serialized path.

### Target identity and observed state

| Contract | Required behavior |
|---|---|
| Item identity | Owner scene/group plus item ID and current scope; input-name deduplication is not scene-item identity |
| Source identity | Prefer reported UUIDs for new bindings when available; names remain labels/fallbacks requiring explicit rebinding when uncertain |
| Scope | Configured endpoint, connection generation, collection identity/generation, request sequence and selected target as appropriate |
| Usage paths | Cache each container once but retain each parent occurrence; shared nested children can affect several paths |
| Freshness | Record observation time/interval and failed subreads; invalidate during scope changes; a set of reads is not a frozen snapshot |

A write checked immediately before sending can still race another OBS client. Explicit target/direction, server errors and authoritative rereads reduce mistakes; they do not establish transactions or compare-and-set semantics.

Input volume, mute, filters, text and media state are source-wide. Item visibility belongs to its owner; a reused owner can affect several scene appearances. Structural usage, reported source activity, actual pixel visibility and audible output are different claims.

### Persistence and result semantics

Use backward-compatible defaults for new preferences. Baselines/cues use versioned local documents and fixed XDG filenames. Preserve malformed/unknown-version user data; show errors instead of overwriting with empty defaults. Serialize writes per file and expose failed saves.

Baseline capture and comparison read fresh selected properties. No full restore or rollback claim. Replay acknowledgement and observed save are different evidence; the saved event does not identify its requester. Paths refer to the OBS host.

`GetVersion.availableRequests` gates unavailable methods. Actual input/output configuration can still reject a supported request. OBS audio monitoring is playback on the OBS host. Native screenshots are still images, not a video/audio transport.

## 6. Detailed work

Task statuses live only in [tasks/todo.md](../../tasks/todo.obs-features-history.md). Each acceptance list below defines completion; it is not a second progress ledger. **Conditional** tasks require the stated product/technical evidence before starting.

<a id="wave-1"></a>
## Wave 1 — Operate confidently and inspect sources

**Outcome:** Explicit disconnect behavior, recording pause, OBS disk space, source visibility/search, and a usable desktop shell.

### Work group: Existing operating controls

<a id="d01"></a>
### D01 — Make stop-and-disconnect explicit

**Type:** Core work · **Scope:** M, 3 files · **Estimate:** 60–120 active minutes.

Reuse the existing output confirmation pattern. This plan chooses explicit stop-and-disconnect as the smallest change; detach-only remains a separately scoped product change.

**Depends on:** None (follow the selected wave order).

**Acceptance:**
- Label the action as stopping stream/recording before disconnecting; preserve the existing controller behavior.
- Honor enabled stop-confirmation preferences; unknown output state requires confirmation. Cancel sends no command.
- Re-evaluate displayed output state before dispatch; do not silently extend shutdown to replay or virtual camera.

**Verify:** Q plus Test the confirmation decision in ui::pages::live; manually check stream-only, recording-only, both, inactive, unknown and Cancel.

**Files likely touched:** `src/ui/sidebar.rs`, `src/ui/pages/live.rs`, `i18n/en/scenedeck.ftl`.

<a id="d02"></a>
### D02 — Explain an empty Live deck

**Type:** Core work · **Scope:** S, 2 files · **Estimate:** 30–60 active minutes.

Give the operator an obvious recovery path without changing scene curation.

**Depends on:** None (follow the selected wave order).

**Acceptance:**
- Distinguish disconnected, no OBS scenes, and scenes excluded by Primary-role filtering.
- Offer Organize scenes for the role-filtered state; mouse and keyboard open Inventory.
- Do not assign roles or change OBS automatically.

**Verify:** Q plus Render the three states and a normal populated deck; verify navigation selection and existing scene shortcuts.

**Files likely touched:** `src/ui/pages/live.rs`, `i18n/en/scenedeck.ftl`.

<a id="q01"></a>
### Q01 — Put operating pages first

**Type:** Core work · **Scope:** S, 2 files · **Estimate:** 20–40 active minutes.

Reuse the existing shared page list and index mapping.

**Depends on:** None (follow the selected wave order).

**Acceptance:**
- Navigation reads Live, Mixer, Stats, Inventory, Graph, Doctor, Settings, Help.
- Sidebar selection, Help links and programmatic navigation open the correct page.

**Verify:** Q plus Manual navigation through every page; run existing UI navigation tests if present. No snapshot test of literal ordering.

**Files likely touched:** `src/ui/window.rs`, `src/ui/sidebar.rs`.

**Checkpoint after D01 / D02 / Q01:** run Q, confirm each listed acceptance condition, and keep existing flows usable. If this group is internal support, keep its unfinished controls unexposed until the next integration group passes. Record evidence and the next task in the checklist.

### Work group: Disk space and capabilities

<a id="d03"></a>
### D03 — Retain OBS recording disk space

**Type:** Core work · **Scope:** M, 3 files · **Estimate:** 30–60 active minutes.

Use the existing stats polling response. Do not add a local filesystem probe or another polling loop.

**Depends on:** None (follow the selected wave order).

**Acceptance:**
- Carry the OBS-reported disk-space field through ObsStats without changing existing counter semantics.
- Represent invalid/nonfinite/negative readings as unavailable.
- Record last successful observation time in state; update all ObsStats literals and fixtures in these modules.

**Verify:** Q plus Run domain::stats, obs::mapper and controller::state tests; include valid, zero, invalid and missing-evidence cases.

**Files likely touched:** `src/domain/stats.rs`, `src/obs/mapper.rs`, `src/controller/state.rs`.

<a id="d04"></a>
### D04 — Display disk space with host context

**Type:** Core work · **Scope:** M, 3 files · **Estimate:** 30–60 active minutes.

Start with useful evidence. Do not estimate remaining recording time from an unvalidated constant bitrate.

**Depends on:** [D03](#d03).

**Acceptance:**
- Label the value as OBS recording disk space, with unambiguous units.
- Unavailable is distinct from zero; stale/disconnected values are not shown as fresh.
- Expose the value in Stats; keep the status bar compact rather than adding a mandatory wide label.

**Verify:** Q plus Render zero, unavailable, large values and disconnected state; compare against OBS on another host.

**Files likely touched:** `src/ui/pages/stats.rs`, `src/ui/widgets/status_bar.rs`, `i18n/en/scenedeck.ftl`.

<a id="k01"></a>
### K01 — Keep the server capability list

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 45–90 active minutes.

Extend the existing connection result. Avoid a new capability registry or dependency.

**Depends on:** None (follow the selected wave order).

**Acceptance:**
- Retain GetVersion.availableRequests and supported image formats in connected-session state.
- Clear capability state on reconnect/disconnect; update ConnectionInfo test literals.
- Unsupported differs from request failure; the presence of a request does not imply output configuration support.

**Verify:** Q plus Run mapper, session_controller and state tests with restricted and complete capability responses.

**Files likely touched:** `src/controller/event.rs`, `src/obs/mapper.rs`, `src/controller/session_controller.rs`, `src/controller/state.rs`, `src/ui/window.rs`.

**Checkpoint after D03 / D04 / K01:** run Q, confirm each listed acceptance condition, and keep existing flows usable. If this group is internal support, keep its unfinished controls unexposed until the next integration group passes. Record evidence and the next task in the checklist.

### Work group: Scope and freshness

<a id="k02"></a>
### K02 — Define scoped observations for new controls

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Introduce the smallest scope record needed by source inspection and later checks. Do not rewrite all existing state.

**Depends on:** [K01](#k01).

**Acceptance:**
- Define endpoint, connection generation, collection identity/generation and observation time for new feature state.
- Invalidate new-feature selections/results on session replacement; an old response cannot become current after reconnect.
- Keep request sequence separate from source identity; test reconnecting to the same endpoint/name.

**Verify:** Q plus Run domain::obs and controller lifecycle/state tests; inject delayed old-session results.

**Files likely touched:** `src/domain/obs.rs`, `src/controller/state.rs`, `src/controller/session_controller.rs`, `src/controller/event.rs`, `src/ui/window.rs`.

<a id="k03"></a>
### K03 — Invalidate observations during collection changes

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Do not wait until a new scene list has already replaced the old one to invalidate source targets.

**Depends on:** [K02](#k02).

**Acceptance:**
- Map the collection-changing boundary before replacement completes; use conservative invalidation where unavailable.
- New controls are unavailable until the new collection context is ready.
- Handle leave-and-return to a same-named collection without accepting old replies.

**Verify:** Q plus Map OBS event fixtures and replay collection-changing/changed sequences; verify repeated names and failed refresh.

**Files likely touched:** `src/obs/event.rs`, `src/obs/mapper.rs`, `src/controller/refresh_controller.rs`, `src/controller/event.rs`, `src/ui/window.rs`.

<a id="k04"></a>
### K04 — Bind new operations to the current session

**Type:** Core work · **Scope:** M, 4 files · **Estimate:** 60–120 active minutes.

Complete the runtime half of K02/K03 before exposing source mutations. Keep OBS authoritative; scope checks are not server-side transactions.

**Depends on:** [K03](#k03).

**Acceptance:**
- New requests carry captured scope and target identity; reject work that became stale before dispatch.
- Associate new reads/polls with session lifetime and cancellation; do not replay writes after reconnect.
- Late completion may be ignored without mutating a replacement session or clearing its pending state.

**Verify:** Q plus Use delayed fake/server requests around disconnect and collection changes; confirm task cancellation and no queued-write replay.

**Files likely touched:** `src/obs/client.rs`, `src/controller/app_controller.rs`, `src/controller/session_controller.rs`, `src/controller/refresh_controller.rs`.

**Checkpoint after K02 / K03 / K04:** run Q, confirm each listed acceptance condition, and keep existing flows usable. If this group is internal support, keep its unfinished controls unexposed until the next integration group passes. Record evidence and the next task in the checklist.

### Work group: Recording pause

<a id="o01"></a>
### O01 — Add explicit recording pause and resume commands

**Type:** Core work · **Scope:** M, 4 files · **Estimate:** 60–120 active minutes.

Extend the already implemented record status path; no new event family is needed merely to represent paused state.

**Depends on:** [D01](#d01), [K04](#k04).

**Acceptance:**
- Wrap PauseRecord and ResumeRecord; route explicit desired state rather than ToggleRecordPause.
- Reuse recording pending/failure handling and the output fake client; reject conflicting recording operations.
- Refresh actual status after completion/failure; pause never becomes a disconnect error.

**Verify:** Q plus Add fake-client cases for explicit pause/resume, duplicate requests, failure and Stop interaction; run controller::output_controller and controller::app_controller tests.

**Files likely touched:** `src/obs/client.rs`, `src/controller/output_controller.rs`, `src/controller/command.rs`, `src/controller/app_controller.rs`.

<a id="o02"></a>
### O02 — Expose Pause and Resume beside recording status

**Type:** Core work · **Scope:** M, 4 files · **Estimate:** 45–90 active minutes.

Use the existing persistent output area rather than adding another page.

**Depends on:** [O01](#o01).

**Acceptance:**
- Show Pause only for active recording and Resume for paused recording; keep Stop distinct.
- Disable conflicting actions while pending; external pause/resume changes update the control.
- Name controls accessibly and preserve recording path/status. Label whether elapsed time includes pauses; do not imply recorded-file duration from a wall-clock timer.

**Verify:** Q plus Live OBS: start, pause, resume, stop; repeat with an external OBS action and a rejected request. Verify keyboard focus.

**Files likely touched:** `src/ui/sidebar.rs`, `src/ui/pages/live.rs`, `src/ui/window.rs`, `i18n/en/scenedeck.ftl`.

<a id="o03"></a>
### O03 — Verify pause ordering and recovery

**Type:** Core work · **Scope:** M, 4 files · **Estimate:** 45–90 active minutes.

Concentrate tests on state races and output recovery, not widget implementation details.

**Depends on:** [O02](#o02).

**Acceptance:**
- Cover response-before-event and event-before-response without leaving a false paused/active display.
- A failed pause does not strand the pending guard or prevent a later Stop.
- Record reproducible manual cases for unsupported recording modes and disconnect during an action.

**Verify:** Q plus Run controller::output_controller, controller::app_controller and controller::state tests; execute the recording section against OBS.

**Files likely touched:** `src/controller/output_controller.rs`, `src/controller/app_controller.rs`, `src/controller/state.rs`, `docs/manual-test-plan.md`.

**Checkpoint after O01 / O02 / O03:** run Q, confirm each listed acceptance condition, and keep existing flows usable. If this group is internal support, keep its unfinished controls unexposed until the next integration group passes. Record evidence and the next task in the checklist.

### Work group: Read a scene's source items

<a id="s01"></a>
### S01 — Read owned scene items without audio deduplication

**Type:** Core work · **Scope:** M, 4 files · **Estimate:** 60–120 active minutes.

Read-only support slice. Cache container contents once while preserving distinct parent paths. New file: src/domain/source.rs.

**Depends on:** [K04](#k04).

**Acceptance:**
- Add typed item records with owner, item ID, source identity, kind, enabled state and path.
- Read the selected scene and groups/nested containers with bounded concurrency; retain duplicate occurrences.
- Return partial coverage and failed containers explicitly; do not use unwrap_or(true) as evidence.

**Verify:** Q plus Add inline fixtures for duplicate inputs, groups, repeated nested scenes, cycles and failed child reads; run domain::source and obs::client tests.

**Files likely touched:** `src/domain/source.rs`, `src/domain/mod.rs`, `src/obs/client.rs`, `src/obs/mapper.rs`.

<a id="s02"></a>
### S02 — Route scoped source reads into application state

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Wire the actual read path before adding a visible inspector. Include every exhaustive AppEvent match in window.rs.

**Depends on:** [S01](#s01).

**Acceptance:**
- Add explicit inspect/refresh commands and scoped loading/success/failure results.
- Preserve selected scene independently from Program; stale results cannot overwrite current selection.
- Represent empty, partial, unavailable and failed states separately.

**Verify:** Q plus Test slow scene A followed by fast scene B, reconnect and same-name collection replacement; run controller::state and controller::app_controller.

**Files likely touched:** `src/controller/command.rs`, `src/controller/event.rs`, `src/controller/app_controller.rs`, `src/controller/state.rs`, `src/ui/window.rs`.

<a id="s03"></a>
### S03 — Show a read-only source inspector on Live

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Deliver the first complete source-inspection flow. New file: src/ui/widgets/source_inspector.rs.

**Depends on:** [S02](#s02).

**Acceptance:**
- Show inspected scene, owner breadcrumbs, source rows, coverage and Retry.
- Provide an explicit inspect-scene selection; inspecting never switches Program.
- Use native keyboard navigation and source-specific accessible names; no mutation controls yet.

**Verify:** Q plus Render normal, empty, partial and failed snapshots; inspect an inactive scene while Program remains unchanged.

**Files likely touched:** `src/ui/widgets/source_inspector.rs`, `src/ui/widgets/mod.rs`, `src/ui/pages/live.rs`, `src/ui/window.rs`, `i18n/en/scenedeck.ftl`.

**Checkpoint after S01 / S02 / S03:** run Q, confirm each listed acceptance condition, and keep existing flows usable. Demonstrate the read-only inspector end to end. Record evidence and the next task in the checklist.

### Work group: Change item visibility

<a id="s04"></a>
### S04 — Route explicit scene-item visibility writes

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Extend the existing source feature path; do not create a generic action engine.

**Depends on:** [S03](#s03).

**Acceptance:**
- Address writes by owning container and item ID, never by deduplicated input name.
- Use the scoped source result path for pending/failure/reconciliation; reread actual state.
- Reject a removed or replaced target; preserve the intended enabled/disabled direction.

**Verify:** Q plus Fixture-test duplicate item IDs in different owners, shared nested children, rejected writes and stale scope.

**Files likely touched:** `src/obs/client.rs`, `src/controller/command.rs`, `src/controller/app_controller.rs`, `src/domain/source.rs`, `src/controller/state.rs`.

<a id="s05"></a>
### S05 — Add visibility controls with scope feedback

**Type:** Core work · **Scope:** M, 4 files · **Estimate:** 45–90 active minutes.

Keep item visibility separate from input-wide mute/filter/media operations.

**Depends on:** [S04](#s04).

**Acceptance:**
- Show enabled state, pending and failure on the affected row.
- Explain that a reused owning container can affect several scene paths.
- Keyboard selection remains inspection; activating the dedicated visibility control performs the write.

**Verify:** Q plus Live OBS: toggle two copies of one input separately, then a child in a shared nested scene; verify the actual affected output.

**Files likely touched:** `src/ui/widgets/source_inspector.rs`, `src/ui/pages/live.rs`, `src/ui/window.rs`, `i18n/en/scenedeck.ftl`.

<a id="s06"></a>
### S06 — Keep source state current after external edits

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Reuse existing event categories where adequate. Add detailed events only when needed to preserve ownership or ordering.

**Depends on:** [S05](#s05).

**Acceptance:**
- Invalidate or refresh relevant items on external create/remove/rename/reorder/visibility changes.
- Coalesce refresh bursts; bound concurrent reads and preserve row focus when identity survives.
- Discard older reads after a newer mutation/event; stale rows cannot remain interactive under a new scene label.

**Verify:** Q plus Replay event bursts and reordered responses; verify external OBS edits, item removal during a click and reconnect.

**Files likely touched:** `src/obs/event.rs`, `src/obs/mapper.rs`, `src/controller/refresh_controller.rs`, `src/controller/event.rs`, `src/ui/window.rs`.

**Checkpoint after S04 / S05 / S06:** run Q, confirm each listed acceptance condition, and keep existing flows usable. Demonstrate an actual visibility change and authoritative recovery after failure. Record evidence and the next task in the checklist.

### Work group: Find and read source relationships

<a id="s07"></a>
### S07 — Search within the inspected scene

**Type:** Core work · **Scope:** M, 3 files · **Estimate:** 30–60 active minutes.

Deliver local search first. A universal command palette is outside this slice.

**Depends on:** [S06](#s06).

**Acceptance:**
- Filter by source name and owner path while retaining breadcrumbs.
- Enter inspects a result; it never hides an item or changes Program.
- Clearing search restores rows and selection predictably; long names remain discoverable.

**Verify:** Q plus Keyboard-only search in a nested collection; verify duplicate names, no matches and clearing the query.

**Files likely touched:** `src/ui/widgets/source_inspector.rs`, `src/ui/pages/live.rs`, `i18n/en/scenedeck.ftl`.

<a id="q07"></a>
### Q07 — Provide a readable Graph relationship list

**Type:** Core work · **Scope:** M, 3 files · **Estimate:** 45–90 active minutes.

Carry forward first-pass U9 without building a keyboard graph editor.

**Depends on:** None (follow the selected wave order).

**Acceptance:**
- Expose scenes, edges and classifications in a keyboard-accessible textual view.
- List and canvas use the same snapshot; empty and cyclic graphs remain readable.
- Keep existing canvas navigation and node positions.

**Verify:** Q plus Compare list/canvas with nested and cyclic fixtures; keyboard and screen-reader traversal.

**Files likely touched:** `src/ui/pages/graph.rs`, `i18n/en/scenedeck.ftl`, `docs/manual-test-plan.md`.

**Checkpoint after S07 / Q07:** run Q, confirm each listed acceptance condition, and keep existing flows usable. If this group is internal support, keep its unfinished controls unexposed until the next integration group passes. Record evidence and the next task in the checklist.

### Work group: Compact desktop shell

<a id="q02"></a>
### Q02 — Collapse navigation without hiding active outputs

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Use the existing assets/scenedeck.css and libadwaita navigation split view. Choose the breakpoint from rendered measurements.

**Depends on:** [D01](#d01), [O02](#o02).

**Acceptance:**
- Add measured libadwaita breakpoint behavior for navigation.
- Active output status/actions stay reachable when navigation collapses.
- Reuse one output action/update path; switching widths does not duplicate commands or listeners.

**Verify:** Q plus Render 1100×740, 800×600 and proposed 640×600 target; test active recording/streaming and large text.

**Files likely touched:** `src/ui/window.rs`, `src/ui/sidebar.rs`, `src/ui/pages/live.rs`, `assets/scenedeck.css`, `i18n/en/scenedeck.ftl`.

<a id="q03"></a>
### Q03 — Fit header and health context to available width

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Preserve searchable selectors and existing status data; do not redesign the entire theme.

**Depends on:** [Q02](#q02).

**Acceptance:**
- Profile/collection selection and health details remain reachable at compact widths.
- Long names and large text do not hide active output state or create ambiguous truncation.
- Detailed health can move into a popover; avoid duplicating complete control trees.

**Verify:** Q plus Rendered width/large-text matrix with long localized labels; keyboard through collapsed and expanded presentations.

**Files likely touched:** `src/ui/window.rs`, `src/ui/sidebar.rs`, `src/ui/widgets/status_bar.rs`, `assets/scenedeck.css`, `i18n/en/scenedeck.ftl`.

**Checkpoint after Q02 / Q03:** run Q, confirm each listed acceptance condition, and keep existing flows usable. If this group is internal support, keep its unfinished controls unexposed until the next integration group passes. Record evidence and the next task in the checklist.

### Release: locale catalogs

<a id="l1a"></a>
### L1A — Wave 1: synchronize English, German, Swiss German and Spanish

**Type:** Release companion · **Scope:** M, 4 files · **Estimate:** 30–60 active minutes.

Run when the selected wave scope and UI copy are complete. Conditional features may be omitted explicitly; omitted work is not marked implemented.

**Depends on:** [D01](#d01), [D02](#d02), [Q01](#q01), [D03](#d03), [D04](#d04), [K01](#k01), [K02](#k02), [K03](#k03), [K04](#k04), [O01](#o01), [O02](#o02), [O03](#o03), [S01](#s01), [S02](#s02), [S03](#s03), [S04](#s04), [S05](#s05), [S06](#s06), [S07](#s07), [Q07](#q07), [Q02](#q02), [Q03](#q03).

**Acceptance:**
- Use only finalized strings from the selected completed feature groups; preserve message variables.
- Synchronize these catalogs and record translation uncertainty for review.
- No speculative or unimplemented product labels enter the release.

**Verify:** Q plus Compare keys/variables and render long labels for selected flows; full isolated locale parity follows the next batch.

**Files likely touched:** `i18n/en/scenedeck.ftl`, `i18n/de/scenedeck.ftl`, `i18n/de-CH/scenedeck.ftl`, `i18n/es/scenedeck.ftl`.

<a id="l1b"></a>
### L1B — Wave 1: synchronize the remaining locale catalogs

**Type:** Release companion · **Scope:** M, 4 files · **Estimate:** 30–60 active minutes.

Explicit companion task; the seven non-English catalogs are not hidden in a feature's file count.

**Depends on:** [L1A](#l1a).

**Acceptance:**
- Match the same finalized keys/variables in all remaining catalogs.
- Run locale parity separately from other tests and keep fallback/translation status honest.

**Verify:** Q plus cargo test --workspace --all-features --locked -- --ignored --test-threads=1; large-text and long-label spot checks.

**Files likely touched:** `i18n/it/scenedeck.ftl`, `i18n/pl/scenedeck.ftl`, `i18n/pt-PT/scenedeck.ftl`, `i18n/uk/scenedeck.ftl`.

**Checkpoint after L1A / L1B:** All selected release strings have matching keys/variables; isolated locale parity passes.

### Release: documentation and verification

<a id="l1c"></a>
### L1C — Wave 1: document the implemented behavior

**Type:** Release companion · **Scope:** M, 5 files · **Estimate:** 45–90 active minutes.

Change only relevant sections; retain historical research as research, not current product documentation.

**Depends on:** [L1B](#l1b).

**Acceptance:**
- Document only completed controls, native API usage, state ownership and local data changes.
- Record reproducible live OBS, failure, keyboard and compact-layout cases for the selected scope.
- Explain material limits: shared sources, freshness, remote paths and read-only behavior where relevant.

**Verify:** Q plus Cross-check docs against actual commands/widgets and the selected acceptance matrix. Check relative links and git diff --check.

**Files likely touched:** `docs/obs-websocket.md`, `docs/user-guide.md`, `docs/manual-test-plan.md`, `docs/architecture.md`, `docs/configuration.md`.

<a id="l1d"></a>
### L1D — Wave 1: verify and record the releasable result

**Type:** Release companion · **Scope:** M, 3 files · **Estimate:** 60–120 active minutes.

This is a reviewable release candidate, not permission to publish. If no conditional feature in Wave 5 is selected, defer its release tasks too.

**Depends on:** [L1C](#l1c).

**Acceptance:**
- All selected task acceptance, full code checks, isolated locales and required CI jobs pass.
- An operator verifies the complete flow against disposable live OBS; record tested versions, scope and evidence.
- Update codebase/product descriptions only for shipped behavior; no commit, push or release occurs without its own authorization.

**Verify:** Q plus Run Q's full commands and required CI dependency checks; perform this wave's outcome gate and keyboard/large-text/compact matrix.

**Files likely touched:** `docs/codebase-overview.md`, `docs/manual-test-plan.md`, `site/index.html`.

**Checkpoint after L1C / L1D:** All selected acceptance criteria and required checks pass; the implemented flow is ready for review.

**Wave 1 outcome gate:** Record/pause/resume/stop and inspect/toggle the correct nested item with keyboard and mouse. External edits, failure, reconnect and compact layout stay understandable.


<a id="wave-2"></a>
## Wave 2 — Understand source usage and check the setup

**Outcome:** Used in… context plus a small factual preflight grounded in fresh OBS observations.

### Work group: Source usage context

<a id="u01"></a>
### U01 — Index all selected source usage paths

**Type:** Core work · **Scope:** M, 3 files · **Estimate:** 60–120 active minutes.

Extend the inspected source records; do not replace the existing scene-only Graph model.

**Depends on:** [S06](#s06).

**Acceptance:**
- Build a bounded reverse index from input/source identity to owning items and ancestor paths.
- Preserve repeated uses and disabled ancestors without duplicating mixer faders.
- Mark incomplete scans and distinguish structural membership from reported activity.

**Verify:** Q plus Test a diamond-shaped graph, duplicate items, cycles, disabled ancestors, globals and one failed container.

**Files likely touched:** `src/domain/source.rs`, `src/obs/client.rs`, `src/obs/mapper.rs`.

<a id="u02"></a>
### U02 — Refresh usage on demand with visible coverage

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Collection-wide discovery is on demand; disclose incomplete data instead of an empty 'Used nowhere' claim.

**Depends on:** [U01](#u01).

**Acceptance:**
- Provide scoped usage loading/results through the established source state path.
- Cancel/ignore obsolete scans and coalesce invalidation after topology changes.
- Do not continuously rescan the full collection while unused.

**Verify:** Q plus Test cancelled scans, topology changes mid-read and same-name collection replacement; record request count for a representative collection.

**Files likely touched:** `src/controller/command.rs`, `src/controller/event.rs`, `src/controller/app_controller.rs`, `src/controller/state.rs`, `src/ui/window.rs`.

<a id="u03"></a>
### U03 — Expose Used in details at the point of control

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Deliver the first differentiating workflow without adding warning dialogs to every fader movement.

**Depends on:** [U02](#u02).

**Acceptance:**
- Show usage paths and item-wide versus source-wide effect scope beside relevant controls.
- A usage link inspects the target scene/Mixer without switching Program.
- Do not claim pixel visibility or audibility from graph membership.

**Verify:** Q plus Live OBS: inspect one mic and one nested overlay used in several scenes; verify navigation, labels and keyboard access.

**Files likely touched:** `src/ui/widgets/source_inspector.rs`, `src/ui/widgets/audio_card.rs`, `src/ui/pages/mixer.rs`, `src/ui/navigation.rs`, `i18n/en/scenedeck.ftl`.

**Checkpoint after U01 / U02 / U03:** run Q, confirm each listed acceptance condition, and keep existing flows usable. If this group is internal support, keep its unfinished controls unexposed until the next integration group passes. Record evidence and the next task in the checklist.

### Work group: Preflight evidence

<a id="p01"></a>
### P01 — Model explicit preflight expectations

**Type:** Core work · **Scope:** M, 4 files · **Estimate:** 60–120 active minutes.

Start with one required microphone and factual context. New files: domain/preflight.rs and services/preflight_service.rs.

**Depends on:** [K04](#k04), [D03](#d03).

**Acceptance:**
- Represent required input present/unmuted, expected scene and output observations with Pass/Fail/Unknown.
- Carry scope, observation times, coverage and evidence independently from structural diagnostic severity.
- Silence is not failure; no automatic repair or Start blocking.

**Verify:** Q plus Pure table-driven cases for missing, muted, unsupported, failed, stale and satisfied expectations.

**Files likely touched:** `src/domain/preflight.rs`, `src/domain/mod.rs`, `src/services/preflight_service.rs`, `src/services/mod.rs`.

<a id="p02"></a>
### P02 — Read a fresh bounded setup observation

**Type:** Core work · **Scope:** M, 3 files · **Estimate:** 60–120 active minutes.

Keep the query scoped to chosen expectations. No full scene-editor snapshot is required.

**Depends on:** [P01](#p01).

**Acceptance:**
- Read requested input state, profile/collection, output state and disk space; retain failures per observation.
- Record the observation interval and mark changes during the read as stale/incomplete.
- Do not reuse the audio scanner's skipped failures as proof of completeness.

**Verify:** Q plus Fake-server tests for partial reads, delayed results and a collection change during collection; assert reads only.

**Files likely touched:** `src/obs/client.rs`, `src/obs/mapper.rs`, `src/domain/preflight.rs`.

<a id="p03"></a>
### P03 — Route the Check setup operation

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Complete the runtime path before exposing a status that could be mistaken for readiness.

**Depends on:** [P02](#p02).

**Acceptance:**
- Add explicit pending/result/failure handling for preflight without changing connection state.
- Newer checks supersede old replies; reconnect/collection changes invalidate the displayed result.
- Evaluate current observations through the pure preflight service.

**Verify:** Q plus Run controller tests for overlapping checks, failed reads and scope replacement; assert no mutation dispatch.

**Files likely touched:** `src/controller/command.rs`, `src/controller/event.rs`, `src/controller/app_controller.rs`, `src/controller/state.rs`, `src/ui/window.rs`.

**Checkpoint after P01 / P02 / P03:** run Q, confirm each listed acceptance condition, and keep existing flows usable. If this group is internal support, keep its unfinished controls unexposed until the next integration group passes. Record evidence and the next task in the checklist.

### Work group: Preflight interaction

<a id="p04"></a>
### P04 — Configure the small expectation set

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Use explicit fields, not a rule-language editor. Empty expectations produce facts, not a universal 'ready' badge.

**Depends on:** [P03](#p03).

**Acceptance:**
- Let the user choose a required microphone and optional expected opening scene for the current endpoint/collection.
- Existing config loads with defaults; identical names in another scope do not inherit expectations.
- Saving reports errors through the established background persistence path.

**Verify:** Q plus Config round-trip/migration tests and Doctor keyboard flow; switch between collections sharing the same names.

**Files likely touched:** `src/storage/config.rs`, `src/controller/state.rs`, `src/ui/pages/doctor.rs`, `src/ui/persist.rs`, `i18n/en/scenedeck.ftl`.

<a id="p05"></a>
### P05 — Display actionable Check setup results

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Keep operational checks distinct from Doctor's current cached structural rerun.

**Depends on:** [P04](#p04).

**Acceptance:**
- Live shows a concise summary; Doctor shows evidence, age, unknowns and relevant control links.
- Existing structural findings remain labeled structural and reflect an explicitly refreshed or age-labeled graph.
- Checking or opening a result never starts outputs or changes Program.

**Verify:** Q plus Seed a muted mic, missing input, old graph and failed read; correct each through the linked UI without returning to OBS.

**Files likely touched:** `src/ui/pages/doctor.rs`, `src/ui/pages/live.rs`, `src/ui/window.rs`, `src/ui/navigation.rs`, `i18n/en/scenedeck.ftl`.

<a id="p06"></a>
### P06 — Add selected-item visibility expectations

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Extend only the small known expectation set. A timed microphone activity test remains deferred.

**Depends on:** [P05](#p05), [S06](#s06).

**Acceptance:**
- Bind an optional required visible item to owner/item identity and scope.
- Treat missing owner, failed read and disabled ancestor separately.
- Report shared-container scope; no promise that an enabled item is visually unobscured.

**Verify:** Q plus Pure and OBS fixtures for duplicated sources, hidden ancestor, deleted item and shared nested child; manual correction flow.

**Files likely touched:** `src/domain/preflight.rs`, `src/services/preflight_service.rs`, `src/obs/client.rs`, `src/ui/pages/doctor.rs`, `i18n/en/scenedeck.ftl`.

**Checkpoint after P04 / P05 / P06:** run Q, confirm each listed acceptance condition, and keep existing flows usable. Demonstrate the fresh check and correction flow without any automatic mutation. Record evidence and the next task in the checklist.

### Release: locale catalogs

<a id="l2a"></a>
### L2A — Wave 2: synchronize English, German, Swiss German and Spanish

**Type:** Release companion · **Scope:** M, 4 files · **Estimate:** 30–60 active minutes.

Run when the selected wave scope and UI copy are complete. Conditional features may be omitted explicitly; omitted work is not marked implemented.

**Depends on:** [U01](#u01), [U02](#u02), [U03](#u03), [P01](#p01), [P02](#p02), [P03](#p03), [P04](#p04), [P05](#p05), [P06](#p06).

**Acceptance:**
- Use only finalized strings from the selected completed feature groups; preserve message variables.
- Synchronize these catalogs and record translation uncertainty for review.
- No speculative or unimplemented product labels enter the release.

**Verify:** Q plus Compare keys/variables and render long labels for selected flows; full isolated locale parity follows the next batch.

**Files likely touched:** `i18n/en/scenedeck.ftl`, `i18n/de/scenedeck.ftl`, `i18n/de-CH/scenedeck.ftl`, `i18n/es/scenedeck.ftl`.

<a id="l2b"></a>
### L2B — Wave 2: synchronize the remaining locale catalogs

**Type:** Release companion · **Scope:** M, 4 files · **Estimate:** 30–60 active minutes.

Explicit companion task; the seven non-English catalogs are not hidden in a feature's file count.

**Depends on:** [L2A](#l2a).

**Acceptance:**
- Match the same finalized keys/variables in all remaining catalogs.
- Run locale parity separately from other tests and keep fallback/translation status honest.

**Verify:** Q plus cargo test --workspace --all-features --locked -- --ignored --test-threads=1; large-text and long-label spot checks.

**Files likely touched:** `i18n/it/scenedeck.ftl`, `i18n/pl/scenedeck.ftl`, `i18n/pt-PT/scenedeck.ftl`, `i18n/uk/scenedeck.ftl`.

**Checkpoint after L2A / L2B:** All selected release strings have matching keys/variables; isolated locale parity passes.

### Release: documentation and verification

<a id="l2c"></a>
### L2C — Wave 2: document the implemented behavior

**Type:** Release companion · **Scope:** M, 5 files · **Estimate:** 45–90 active minutes.

Change only relevant sections; retain historical research as research, not current product documentation.

**Depends on:** [L2B](#l2b).

**Acceptance:**
- Document only completed controls, native API usage, state ownership and local data changes.
- Record reproducible live OBS, failure, keyboard and compact-layout cases for the selected scope.
- Explain material limits: shared sources, freshness, remote paths and read-only behavior where relevant.

**Verify:** Q plus Cross-check docs against actual commands/widgets and the selected acceptance matrix. Check relative links and git diff --check.

**Files likely touched:** `docs/obs-websocket.md`, `docs/user-guide.md`, `docs/manual-test-plan.md`, `docs/architecture.md`, `docs/configuration.md`.

<a id="l2d"></a>
### L2D — Wave 2: verify and record the releasable result

**Type:** Release companion · **Scope:** M, 3 files · **Estimate:** 60–120 active minutes.

This is a reviewable release candidate, not permission to publish. If no conditional feature in Wave 5 is selected, defer its release tasks too.

**Depends on:** [L2C](#l2c).

**Acceptance:**
- All selected task acceptance, full code checks, isolated locales and required CI jobs pass.
- An operator verifies the complete flow against disposable live OBS; record tested versions, scope and evidence.
- Update codebase/product descriptions only for shipped behavior; no commit, push or release occurs without its own authorization.

**Verify:** Q plus Run Q's full commands and required CI dependency checks; perform this wave's outcome gate and keyboard/large-text/compact matrix.

**Files likely touched:** `docs/codebase-overview.md`, `docs/manual-test-plan.md`, `site/index.html`.

**Checkpoint after L2C / L2D:** All selected acceptance criteria and required checks pass; the implemented flow is ready for review.

**Wave 2 outcome gate:** Find a shared input, inspect every known usage path, then identify and correct seeded setup mistakes. Partial/unknown evidence never becomes an all-clear result.


<a id="wave-3"></a>
## Wave 3 — Compare rehearsal state and stabilize audio work

**Outcome:** Read-only audio baselines, conditional individual audio watching, and existing Mixer/keyboard/accessibility improvements.

### Work group: Rehearsal baseline storage

<a id="b01"></a>
### B01 — Define selected audio baselines and comparison rules

**Type:** Core work · **Scope:** S, 2 files · **Estimate:** 60–120 active minutes.

New file: src/domain/baseline.rs. This is selected operational state, not an OBS scene-collection backup.

**Depends on:** [P03](#p03).

**Acceptance:**
- Represent versioned selected-input mute/gain observations, endpoint/collection binding and observation interval.
- Define unchanged/changed/missing/unreadable/stale outcomes with an explicit gain tolerance.
- Reject cross-scope matching and require rebinding when identity is unresolved; no raw settings blobs.

**Verify:** Q plus Pure tests for equal values, gain tolerance, source rename/removal, missing UUID fallback and identical names in another collection.

**Files likely touched:** `src/domain/baseline.rs`, `src/domain/mod.rs`.

<a id="b02"></a>
### B02 — Persist baselines without overwriting unreadable data

**Type:** Core work · **Scope:** M, 4 files · **Estimate:** 60–120 active minutes.

New file: src/storage/baseline.rs. Reuse atomic.rs; no database, cloud service or storage framework.

**Depends on:** [B01](#b01).

**Acceptance:**
- Store a versioned baselines.json under the existing XDG application directory using atomic writes.
- Use a dedicated serialized write lane; fixed filenames never derive directly from source names.
- Loading corrupt/unsupported data reports an error and blocks replacement unless explicitly resolved.

**Verify:** Q plus Inline storage round-trip, malformed/versioned file, failed write and ordered save cases using temporary directories.

**Files likely touched:** `src/storage/baseline.rs`, `src/storage/mod.rs`, `src/storage/xdg.rs`, `src/ui/background_io.rs`.

<a id="b03"></a>
### B03 — Load scoped baseline state on demand

**Type:** Core work · **Scope:** M, 4 files · **Estimate:** 45–90 active minutes.

Use the existing background I/O path rather than sending local persistence through AppCommand.

**Depends on:** [B02](#b02).

**Acceptance:**
- Load the selected scope's baseline off GTK; show loading/empty/read-error separately.
- Late file reads cannot select a baseline for a different current scope.
- Keep last saved data distinct from unsaved edits and failed saves.

**Verify:** Q plus State fixtures for scope changes during load; manual Doctor opening with missing, valid and malformed files.

**Files likely touched:** `src/controller/state.rs`, `src/ui/pages/doctor.rs`, `src/ui/persist.rs`, `i18n/en/scenedeck.ftl`.

**Checkpoint after B01 / B02 / B03:** run Q, confirm each listed acceptance condition, and keep existing flows usable. If this group is internal support, keep its unfinished controls unexposed until the next integration group passes. Record evidence and the next task in the checklist.

### Work group: Capture and compare rehearsal state

<a id="b04"></a>
### B04 — Read fresh selected-input baseline values

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Reuse the read contracts from preflight only where their semantics match; avoid an arbitrary snapshot engine.

**Depends on:** [B03](#b03), [P02](#p02).

**Acceptance:**
- Read only explicitly selected inputs through the scoped observation path.
- Preserve per-property failures and observation interval; do not copy cached Mixer values as fresh evidence.
- A scope change during read invalidates the capture/comparison result; no OBS writes occur.

**Verify:** Q plus Delayed-server cases for mixed success, source deletion, renamed input and collection replacement; assert the request list contains no mutations.

**Files likely touched:** `src/obs/client.rs`, `src/controller/command.rs`, `src/controller/event.rs`, `src/controller/app_controller.rs`, `src/ui/window.rs`.

<a id="b05"></a>
### B05 — Capture an intentional audio baseline

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Start with one active baseline per endpoint/collection and explicit replacement. Multiple-version history is deferred.

**Depends on:** [B04](#b04).

**Acceptance:**
- Let the operator select inputs, name a baseline and review fresh values before saving.
- Incomplete capture remains explicit; never silently replace a good baseline with failed values.
- Report persistence failure and preserve the previous saved baseline.

**Verify:** Q plus Capture mute/gain from live OBS, change scope during reading, simulate save failure and reload the saved baseline.

**Files likely touched:** `src/controller/state.rs`, `src/ui/pages/doctor.rs`, `src/ui/persist.rs`, `src/storage/baseline.rs`, `i18n/en/scenedeck.ftl`.

<a id="b06"></a>
### B06 — Show a read-only rehearsal difference report

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Deliver repeatability before adding repair writes. Selected-property restore requires a later decision and new task breakdown.

**Depends on:** [B05](#b05).

**Acceptance:**
- Compare a fresh observation with the selected saved baseline and show before/after values.
- Show missing, unreadable and stale separately from unchanged; include observation time and scope.
- No Restore button or OBS mutation is present in this milestone.

**Verify:** Q plus Run domain::baseline tests and a real rehearsal/change/compare flow; inspect outbound requests for read-only behavior.

**Files likely touched:** `src/domain/baseline.rs`, `src/controller/state.rs`, `src/ui/pages/doctor.rs`, `src/ui/window.rs`, `i18n/en/scenedeck.ftl`.

**Checkpoint after B04 / B05 / B06:** run Q, confirm each listed acceptance condition, and keep existing flows usable. Demonstrate capture and comparison with zero OBS writes. Record evidence and the next task in the checklist.

### Work group: Stable watched audio

<a id="w01"></a>
### W01 — Persist individually watched inputs

**Type:** Conditional · **Scope:** M, 3 files · **Estimate:** 45–90 active minutes.

Start only after observing a real need beyond pinned-scene Mixer mode. Retain existing Mixer behavior.

**Depends on:** [S06](#s06), [K04](#k04).

**Acceptance:**
- Add ordered watched-input bindings scoped to endpoint/collection with backward-compatible defaults.
- Keep watch selection distinct from Active/Selected/Pinned scene mode and session-only fader locks.
- Resolve rename/missing identity explicitly; never silently bind to another collection.

**Verify:** Q plus Config/domain round-trip tests, old-config load, duplicate prevention and scope replacement cases.

**Files likely touched:** `src/domain/mixer.rs`, `src/storage/config.rs`, `src/controller/state.rs`.

<a id="w02"></a>
### W02 — Refresh watched-input state independently from the active scene

**Type:** Conditional · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Reuse existing meter events. An inactive source without meter samples is unavailable, not proven silent.

**Depends on:** [W01](#w01).

**Acceptance:**
- Read watched input state even when it is not in the inspected scene; reuse input-wide commands.
- Track meter freshness independently from zero signal and scene membership.
- Handle source removal and external changes without reordering surviving watched controls.

**Verify:** Q plus Tests for scene changes, absent meter batches, delayed source state and removal; compare with live OBS.

**Files likely touched:** `src/obs/client.rs`, `src/controller/app_controller.rs`, `src/controller/refresh_controller.rs`, `src/controller/state.rs`, `src/ui/window.rs`.

<a id="w03"></a>
### W03 — Expose Watch controls and a stable audio strip

**Type:** Conditional · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Ship only if a representative workflow demonstrates value beyond pinning a whole scene.

**Depends on:** [W02](#w02).

**Acceptance:**
- Add Watch/Unwatch and a compact strip with fixed order across scene changes.
- Reuse existing mute/fader/lock widgets and accessible source labels.
- Expose missing/inactive/stale status without automatic gain changes, auto-unmute or constant warning noise.

**Verify:** Q plus Operate Intro/Interview/Break by keyboard; confirm control positions, external mute state and meter freshness.

**Files likely touched:** `src/ui/widgets/audio_card.rs`, `src/ui/pages/live.rs`, `src/ui/pages/mixer.rs`, `src/ui/persist.rs`, `i18n/en/scenedeck.ftl`.

**Checkpoint after W01 / W02 / W03:** run Q, confirm each listed acceptance condition, and keep existing flows usable. If this group is internal support, keep its unfinished controls unexposed until the next integration group passes. Record evidence and the next task in the checklist.

### Work group: Existing desktop interactions

<a id="q04"></a>
### Q04 — Prioritize Mixer mode and target

**Type:** Core work · **Scope:** M, 3 files · **Estimate:** 45–90 active minutes.

Carry forward U6 without replacing the Mixer.

**Depends on:** None (follow the selected wave order).

**Acceptance:**
- Keep mode and target scene primary; put grouping/search options in a secondary area.
- Preserve Apply-search semantics and saved selection preferences.
- Loading, failure, Retry and no-target states stay visible and keyboard reachable.

**Verify:** Q plus Manual Active/Selected/Pinned flows at default and compact widths; verify Apply-search and persisted settings.

**Files likely touched:** `src/ui/pages/mixer.rs`, `assets/scenedeck.css`, `i18n/en/scenedeck.ftl`.

<a id="q05"></a>
### Q05 — Add keyboard Inventory reordering

**Type:** Core work · **Scope:** M, 3 files · **Estimate:** 45–90 active minutes.

Reuse the current reorder helper instead of maintaining a second ordering model.

**Depends on:** None (follow the selected wave order).

**Acceptance:**
- Provide move-up/down actions using the existing order model and persistence.
- Preserve focus and prevent movement past list ends.
- Drag order, keyboard order and Live shortcuts agree after restart.

**Verify:** Q plus Keyboard and drag reorder comparison, boundary cases, restart and save failure.

**Files likely touched:** `src/ui/pages/inventory.rs`, `i18n/en/scenedeck.ftl`, `docs/manual-test-plan.md`.

<a id="q06"></a>
### Q06 — Expose audio control names and readable meter values

**Type:** Core work · **Scope:** M, 4 files · **Estimate:** 60–120 active minutes.

Carry forward U8; new watched controls must use the same accessible components.

**Depends on:** None (follow the selected wave order).

**Acceptance:**
- Fader, mute and lock accessible names identify their source.
- Custom meters expose meaning/value through an accessible or bounded-rate textual alternative.
- Do not announce every meter frame or reduce visual metering responsiveness.

**Verify:** Q plus Inspect accessibility tree and use Orca with stereo inputs; keyboard/fine-adjustment regression check.

**Files likely touched:** `src/ui/widgets/audio_card.rs`, `src/ui/widgets/volume_meter.rs`, `i18n/en/scenedeck.ftl`, `docs/manual-test-plan.md`.

**Checkpoint after Q04 / Q05 / Q06:** run Q, confirm each listed acceptance condition, and keep existing flows usable. If this group is internal support, keep its unfinished controls unexposed until the next integration group passes. Record evidence and the next task in the checklist.

### Release: locale catalogs

<a id="l3a"></a>
### L3A — Wave 3: synchronize English, German, Swiss German and Spanish

**Type:** Release companion · **Scope:** M, 4 files · **Estimate:** 30–60 active minutes.

Run when the selected wave scope and UI copy are complete. Conditional features may be omitted explicitly; omitted work is not marked implemented.

**Depends on:** [B01](#b01), [B02](#b02), [B03](#b03), [B04](#b04), [B05](#b05), [B06](#b06), [Q04](#q04), [Q05](#q05), [Q06](#q06).

**Acceptance:**
- Use only finalized strings from the selected completed feature groups; preserve message variables.
- Synchronize these catalogs and record translation uncertainty for review.
- No speculative or unimplemented product labels enter the release.

**Verify:** Q plus Compare keys/variables and render long labels for selected flows; full isolated locale parity follows the next batch.

**Files likely touched:** `i18n/en/scenedeck.ftl`, `i18n/de/scenedeck.ftl`, `i18n/de-CH/scenedeck.ftl`, `i18n/es/scenedeck.ftl`.

<a id="l3b"></a>
### L3B — Wave 3: synchronize the remaining locale catalogs

**Type:** Release companion · **Scope:** M, 4 files · **Estimate:** 30–60 active minutes.

Explicit companion task; the seven non-English catalogs are not hidden in a feature's file count.

**Depends on:** [L3A](#l3a).

**Acceptance:**
- Match the same finalized keys/variables in all remaining catalogs.
- Run locale parity separately from other tests and keep fallback/translation status honest.

**Verify:** Q plus cargo test --workspace --all-features --locked -- --ignored --test-threads=1; large-text and long-label spot checks.

**Files likely touched:** `i18n/it/scenedeck.ftl`, `i18n/pl/scenedeck.ftl`, `i18n/pt-PT/scenedeck.ftl`, `i18n/uk/scenedeck.ftl`.

**Checkpoint after L3A / L3B:** All selected release strings have matching keys/variables; isolated locale parity passes.

### Release: documentation and verification

<a id="l3c"></a>
### L3C — Wave 3: document the implemented behavior

**Type:** Release companion · **Scope:** M, 5 files · **Estimate:** 45–90 active minutes.

Change only relevant sections; retain historical research as research, not current product documentation.

**Depends on:** [L3B](#l3b).

**Acceptance:**
- Document only completed controls, native API usage, state ownership and local data changes.
- Record reproducible live OBS, failure, keyboard and compact-layout cases for the selected scope.
- Explain material limits: shared sources, freshness, remote paths and read-only behavior where relevant.

**Verify:** Q plus Cross-check docs against actual commands/widgets and the selected acceptance matrix. Check relative links and git diff --check.

**Files likely touched:** `docs/obs-websocket.md`, `docs/user-guide.md`, `docs/manual-test-plan.md`, `docs/architecture.md`, `docs/configuration.md`.

<a id="l3d"></a>
### L3D — Wave 3: verify and record the releasable result

**Type:** Release companion · **Scope:** M, 3 files · **Estimate:** 60–120 active minutes.

This is a reviewable release candidate, not permission to publish. If no conditional feature in Wave 5 is selected, defer its release tasks too.

**Depends on:** [L3C](#l3c).

**Acceptance:**
- All selected task acceptance, full code checks, isolated locales and required CI jobs pass.
- An operator verifies the complete flow against disposable live OBS; record tested versions, scope and evidence.
- Update codebase/product descriptions only for shipped behavior; no commit, push or release occurs without its own authorization.

**Verify:** Q plus Run Q's full commands and required CI dependency checks; perform this wave's outcome gate and keyboard/large-text/compact matrix.

**Files likely touched:** `docs/codebase-overview.md`, `docs/manual-test-plan.md`, `site/index.html`.

**Checkpoint after L3C / L3D:** All selected acceptance criteria and required checks pass; the implemented flow is ready for review.

**Wave 3 outcome gate:** Capture selected inputs, change OBS, compare fresh values and explain the differences without writing to OBS. Existing Mixer modes and shortcuts still work.


<a id="wave-4"></a>
## Wave 4 — Complete native OBS operating controls

**Outcome:** Replay/result feedback, Studio/Take, optional measured still previews, and focused media/filter/text/audio detail controls.

### Work group: Replay state

<a id="r01"></a>
### R01 — Add replay state and native adapters

**Type:** Core work · **Scope:** M, 3 files · **Estimate:** 60–120 active minutes.

Internal support slice. Normalize native events in R02, not in an unmatched enum addition here.

**Depends on:** [K04](#k04), [D01](#d01).

**Acceptance:**
- Model replay state separately from stream/record status and initialize it as unknown.
- Wrap explicit status/start/stop/save operations with unsupported/unconfigured errors.
- Leave controls unexposed until event routing and command guards are complete.

**Verify:** Q plus Adapter/domain fixtures for inactive, active, unavailable and rejected native operations.

**Files likely touched:** `src/domain/output.rs`, `src/controller/state.rs`, `src/obs/client.rs`.

<a id="r02"></a>
### R02 — Route replay lifecycle and saved events

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

State storage exists from R01. This is still internal wiring; controls arrive in R05.

**Depends on:** [R01](#r01).

**Acceptance:**
- Normalize replay lifecycle and saved-path events and route every new enum variant.
- Read replay state when connecting/refreshing and store scoped observations without affecting recording.
- Reject stale session results; saved events retain their observed origin.

**Verify:** Q plus Event fixtures for replay transitions, saved paths, external actions and reconnect; exhaustive routing compiles.

**Files likely touched:** `src/obs/event.rs`, `src/obs/mapper.rs`, `src/controller/refresh_controller.rs`, `src/controller/event.rs`, `src/ui/window.rs`.

<a id="r03"></a>
### R03 — Route guarded replay commands

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Finish command behavior before exposing controls. Result-tray semantics follow in R04.

**Depends on:** [R02](#r02).

**Acceptance:**
- Route explicit replay Start/Stop/Save with independent pending guards and scoped outcomes.
- Reconcile actual status after failure and never convert a command error into a disconnect.
- Do not add replay to the existing stream/record disconnect shutdown set.

**Verify:** Q plus Fake-client tests for duplicate operations, unavailable replay, delayed replies, failure recovery and disconnect.

**Files likely touched:** `src/controller/command.rs`, `src/controller/output_controller.rs`, `src/controller/app_controller.rs`, `src/controller/event.rs`, `src/ui/window.rs`.

**Checkpoint after R01 / R02 / R03:** run Q, confirm each listed acceptance condition, and keep existing flows usable. If this group is internal support, keep its unfinished controls unexposed until the next integration group passes. Record evidence and the next task in the checklist.

### Work group: Replay results

<a id="r04"></a>
### R04 — Model bounded output results without false correlation

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Use native saved-path evidence; acknowledgement alone is not a saved file.

**Depends on:** [R03](#r03).

**Acceptance:**
- Represent requested, acknowledged, observed saved, failed and unconfirmed separately.
- Handle events before responses and responses before events; no event implies a unique requester.
- Bound result history and preserve explicit gaps/unconfirmed results across disconnect.

**Verify:** Q plus Replay delayed/event-first/response-first fixtures, another client's save and a missing saved event.

**Files likely touched:** `src/domain/output.rs`, `src/controller/output_controller.rs`, `src/controller/event.rs`, `src/controller/state.rs`, `src/ui/window.rs`.

<a id="r05"></a>
### R05 — Expose replay controls and the result tray

**Type:** Core work · **Scope:** M, 4 files · **Estimate:** 45–90 active minutes.

Start with a small bounded tray, not an asset browser.

**Depends on:** [R04](#r04).

**Acceptance:**
- Show distinct Start, Stop and Save replay controls with pending/error state; keep them reachable in compact layout.
- Show recent results and copy the OBS-host path; never try opening that path locally by default.
- Show recent results and clearly state what continues on disconnect; keyboard use remains straightforward.

**Verify:** Q plus Save multiple replays with one rejected request and one delayed completion; verify copying a remote-host path.

**Files likely touched:** `src/ui/sidebar.rs`, `src/ui/pages/live.rs`, `src/ui/window.rs`, `i18n/en/scenedeck.ftl`.

<a id="r06"></a>
### R06 — Validate output interaction as a whole

**Type:** Core work · **Scope:** M, 4 files · **Estimate:** 60–120 active minutes.

Do not start real broadcasts as part of unattended validation.

**Depends on:** [R05](#r05), [O03](#o03).

**Acceptance:**
- Stream, recording pause/stop and replay operations keep independent state and pending guards.
- Reconnect, failures and external actions do not leave stuck controls.
- Document what continues and what stops on disconnect.

**Verify:** Q plus Run output/controller tests plus a live stream/record/replay matrix using a disposable OBS configuration.

**Files likely touched:** `src/controller/output_controller.rs`, `src/controller/app_controller.rs`, `src/controller/state.rs`, `docs/manual-test-plan.md`.

**Checkpoint after R04 / R05 / R06:** run Q, confirm each listed acceptance condition, and keep existing flows usable. If this group is internal support, keep its unfinished controls unexposed until the next integration group passes. Record evidence and the next task in the checklist.

### Work group: Studio observations

<a id="t01"></a>
### T01 — Read Preview, Program and transition state

**Type:** Core work · **Scope:** M, 4 files · **Estimate:** 60–120 active minutes.

New file: src/domain/studio.rs. Start without image polling.

**Depends on:** [K04](#k04).

**Acceptance:**
- Model direct/studio mode, separate Preview/Program identities and transition availability/duration.
- Use native reads; represent unavailable state rather than guessing from Program.
- Capture scope and request ordering for the new state.

**Verify:** Q plus Adapter and domain fixtures for direct mode, Studio mode, missing preview and unavailable transition data.

**Files likely touched:** `src/domain/studio.rs`, `src/domain/mod.rs`, `src/obs/client.rs`, `src/obs/mapper.rs`.

<a id="t02"></a>
### T02 — Map Studio changes from OBS

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Read-only event wiring precedes Stage/Take.

**Depends on:** [T01](#t01).

**Acceptance:**
- Map mode, Preview, Program and transition changes into the Studio state path.
- External mode changes invalidate incompatible actions and stale selection.
- Normalize transition state without treating a queued request as completed.

**Verify:** Q plus Event replay cases for remote mode changes, transition-in-progress and scene removal; exhaustive event routing compiles.

**Files likely touched:** `src/obs/event.rs`, `src/obs/mapper.rs`, `src/controller/refresh_controller.rs`, `src/controller/event.rs`, `src/ui/window.rs`.

<a id="t03"></a>
### T03 — Display explicit Direct and Studio context

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Read-only Studio context is an integration checkpoint; complete T04–T06 before releasing Studio operation.

**Depends on:** [T02](#t02).

**Acceptance:**
- Display separate Preview and Program labels/tallies with text as well as color.
- Direct mode keeps current immediate switching; Studio context is read-only until the Stage/Take tasks connect.
- Do not enable unimplemented Take controls or silently reinterpret an action during a mode change.

**Verify:** Q plus Render both modes and external changes; inspect focus and shortcut hints.

**Files likely touched:** `src/controller/state.rs`, `src/ui/pages/live.rs`, `src/ui/widgets/scene_card.rs`, `src/ui/window.rs`, `i18n/en/scenedeck.ftl`.

**Checkpoint after T01 / T02 / T03:** run Q, confirm each listed acceptance condition, and keep existing flows usable. If this group is internal support, keep its unfinished controls unexposed until the next integration group passes. Record evidence and the next task in the checklist.

### Work group: Stage and Take

<a id="t04"></a>
### T04 — Route explicit Stage and Take commands

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Use native OBS Studio Mode; no local simulation of isolation.

**Depends on:** [T03](#t03).

**Acceptance:**
- Route explicit SetStudioMode, Stage and Take; Stage changes Preview only and actions retain their requested direction.
- Track pending/failure through the existing Studio result path; reconcile with actual OBS state.
- Reject incompatible mode/transition state without sending a different action.

**Verify:** Q plus Fake-server tests for stage-without-program-change, stale mode, rejected Take and two rapid Take requests.

**Files likely touched:** `src/obs/client.rs`, `src/controller/command.rs`, `src/controller/app_controller.rs`, `src/domain/studio.rs`, `src/controller/state.rs`.

<a id="t05"></a>
### T05 — Wire scene cards and shortcuts to explicit mode semantics

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Do not introduce a global shortcut system.

**Depends on:** [T04](#t04).

**Acceptance:**
- Show a distinct Take action; scene click/shortcut stages only in the clearly indicated Studio mode.
- Shortcuts do not execute through text input; mode changes update visible hints before new actions.
- Keep selection/inspection distinct from staging and taking.

**Verify:** Q plus Keyboard/mouse parity in both modes, focus in text fields and mode change between selection and activation.

**Files likely touched:** `src/ui/pages/live.rs`, `src/ui/widgets/scene_card.rs`, `src/ui/hotkeys.rs`, `src/services/hotkey_service.rs`, `i18n/en/scenedeck.ftl`.

<a id="t06"></a>
### T06 — Route transition choice and duration changes

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Keep setters separate from the next UI slice; T-bar control is deferred.

**Depends on:** [T05](#t05).

**Acceptance:**
- Add explicit supported transition and duration commands using the existing Studio result path.
- Validate transition capabilities and duration; preserve fixed-duration behavior.
- Reconcile failures/external state without clearing a newer confirmed value.

**Verify:** Q plus Adapter/controller tests for supported and fixed-duration transitions, stale scope and failed settings.

**Files likely touched:** `src/obs/client.rs`, `src/controller/command.rs`, `src/controller/app_controller.rs`, `src/domain/studio.rs`, `src/controller/state.rs`.

**Checkpoint after T04 / T05 / T06:** run Q, confirm each listed acceptance condition, and keep existing flows usable. If this group is internal support, keep its unfinished controls unexposed until the next integration group passes. Record evidence and the next task in the checklist.

### Work group: Transition interface

<a id="t07"></a>
### T07 — Expose transition choice, duration and mode controls

**Type:** Core work · **Scope:** M, 4 files · **Estimate:** 60–120 active minutes.

Complete Studio operation before considering still-image previews.

**Depends on:** [T06](#t06).

**Acceptance:**
- Show supported transition controls and actual Studio mode with pending/failed feedback.
- Keep Preview, Program and Take explicit at wide and compact widths.
- Warn contextually that shared-source edits can affect Program; do not claim rehearsal isolation.

**Verify:** Q plus Live OBS choose transition/duration, stage and Take; repeat with external mode changes and a rejected setter.

**Files likely touched:** `src/ui/pages/live.rs`, `src/ui/window.rs`, `src/ui/hotkeys.rs`, `i18n/en/scenedeck.ftl`.

<a id="t08"></a>
### T08 — Verify Direct and Studio operation end to end

**Type:** Core work · **Scope:** M, 4 files · **Estimate:** 60–120 active minutes.

This is the Studio release gate; snapshots are optional, not a prerequisite.

**Depends on:** [T07](#t07).

**Acceptance:**
- Preview selection never changes Program until a deliberate Take in Studio mode.
- Direct click/shortcut semantics remain unchanged, including protection from text-entry shortcuts.
- Remote mode changes, transition-in-progress, failures and reconnect leave clear controls and state.

**Verify:** Q plus Focused domain/controller/hotkey tests and a full live OBS matrix in both modes.

**Files likely touched:** `src/controller/app_controller.rs`, `src/domain/studio.rs`, `src/services/hotkey_service.rs`, `docs/manual-test-plan.md`.

**Checkpoint after T07 / T08:** run Q, confirm each listed acceptance condition, and keep existing flows usable. If this group is internal support, keep its unfinished controls unexposed until the next integration group passes. Record evidence and the next task in the checklist.

### Work group: Optional still previews

<a id="i01"></a>
### I01 — Measure a bounded screenshot prototype

**Type:** Conditional · **Scope:** S, 2 files · **Estimate:** 60–120 active minutes.

Risk-first spike. Do not choose a universal refresh rate without measurements.

**Depends on:** [T08](#t08), [K01](#k01).

**Acceptance:**
- Capture one visible target at modest resolution using a supported format.
- Measure OBS render timing, request latency and bytes at explicitly recorded rates.
- Record a go/no-go decision for periodic previews; on-demand capture is an acceptable outcome.

**Verify:** Q plus Prototype against actual OBS with one Program/Preview target and a busy scene; record baseline versus capture measurements.

**Files likely touched:** `src/obs/client.rs`, `docs/manual-test-plan.md`.

<a id="i02"></a>
### I02 — Route cancellation-aware snapshot reads

**Type:** Conditional · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Only proceed after I01 supports a useful performance budget. Add lifecycle support as a separate slice if these five files are insufficient.

**Depends on:** [I01](#i01).

**Acceptance:**
- Allow at most one in-flight read per visible target and reject outdated images.
- Stop capture when hidden, disconnected or superseded; decode away from GTK.
- Track target, observation time and failure; avoid crowding control events.

**Verify:** Q plus Delayed/reordered snapshot fixture, target switching, hidden page and disconnect checks; inspect task ownership.

**Files likely touched:** `src/domain/studio.rs`, `src/controller/command.rs`, `src/controller/event.rs`, `src/controller/app_controller.rs`, `src/ui/window.rs`.

<a id="i03"></a>
### I03 — Show honest snapshot previews

**Type:** Conditional · **Scope:** M, 4 files · **Estimate:** 45–90 active minutes.

Release only the measured on-demand or low-rate mode.

**Depends on:** [I02](#i02).

**Acceptance:**
- Show correctly labeled Program/Preview stills with age and stale/error status.
- Keep scene actions usable without images and preserve compact layout.
- Do not advertise full-motion video or controller-side audio monitoring.

**Verify:** Q plus Rendered target/age checks and repeat I01's measurements with the final UI under control/meter load.

**Files likely touched:** `src/controller/state.rs`, `src/ui/pages/live.rs`, `src/ui/window.rs`, `i18n/en/scenedeck.ftl`.

**Checkpoint after I01 / I02 / I03:** run Q, confirm each listed acceptance condition, and keep existing flows usable. If this group is internal support, keep its unfinished controls unexposed until the next integration group passes. Record evidence and the next task in the checklist.

### Work group: Selected media transport

<a id="m01"></a>
### M01 — Read and control supported media inputs

**Type:** Core work · **Scope:** M, 4 files · **Estimate:** 60–120 active minutes.

Start with supported native media inputs. Do not add playlists, remote file transfer or plugin actions.

**Depends on:** [S06](#s06), [K04](#k04).

**Acceptance:**
- Represent media state, duration/cursor and supported actions for a selected known source.
- Wrap Play/Pause/Restart/Stop and validated seek; reject unknown duration and invalid cursor values.
- Keep ownership context separate from shared input transport.

**Verify:** Q plus Adapter tests for ordinary file media, ended/paused source, unsupported input, missing duration and out-of-range seek.

**Files likely touched:** `src/domain/source.rs`, `src/obs/client.rs`, `src/obs/mapper.rs`, `src/controller/state.rs`.

<a id="m02"></a>
### M02 — Route scoped media requests and visible-only refresh

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Use a concrete media command path and existing session lifetime, not an automation scheduler.

**Depends on:** [M01](#m01).

**Acceptance:**
- Route media status/actions with target scope, pending state and authoritative reread.
- Refresh only the visible selected transport; bound/cancel polling and ignore superseded targets.
- External media changes appear through supported events or the bounded read policy.

**Verify:** Q plus Tests for changing selection during seek, rejected action, ended media and hidden/disconnected polling cancellation.

**Files likely touched:** `src/controller/command.rs`, `src/controller/event.rs`, `src/controller/app_controller.rs`, `src/controller/refresh_controller.rs`, `src/ui/window.rs`.

<a id="m03"></a>
### M03 — Expose contextual media transport

**Type:** Core work · **Scope:** M, 4 files · **Estimate:** 60–120 active minutes.

Keep media beside the selected source; no separate media-management application.

**Depends on:** [M02](#m02).

**Acceptance:**
- Show only supported transport controls and valid seek positions.
- Explain source-wide effects and distinguish inspected scene from Program.
- Show pending/failure/stale values; keyboard seeking cannot trigger scene shortcuts.

**Verify:** Q plus Live OBS media play/pause/restart/seek while inspecting another scene; check shared input effects and focus.

**Files likely touched:** `src/ui/widgets/source_inspector.rs`, `src/ui/pages/live.rs`, `src/ui/window.rs`, `i18n/en/scenedeck.ftl`.

**Checkpoint after M01 / M02 / M03:** run Q, confirm each listed acceptance condition, and keep existing flows usable. If this group is internal support, keep its unfinished controls unexposed until the next integration group passes. Record evidence and the next task in the checklist.

### Work group: Filter toggles

<a id="f01"></a>
### F01 — Read filters and wrap explicit enable state

**Type:** Core work · **Scope:** M, 4 files · **Estimate:** 45–90 active minutes.

No dynamic form generator or arbitrary filter setting editor.

**Depends on:** [S06](#s06).

**Acceptance:**
- List filter identity/name/enabled state on the selected source.
- Wrap explicit SetSourceFilterEnabled and reread state after failure/success.
- Keep unknown/unsupported source and removed filter distinct.

**Verify:** Q plus Adapter fixtures for empty filters, disabled filter, removal during write and rejected command.

**Files likely touched:** `src/domain/source.rs`, `src/obs/client.rs`, `src/obs/mapper.rs`, `src/controller/state.rs`.

<a id="f02"></a>
### F02 — Route scoped filter operations

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

An explicit refresh is acceptable for the initial details pane; do not imply every filter setting is tracked.

**Depends on:** [F01](#f01), [K04](#k04).

**Acceptance:**
- Route read/toggle results without changing connection state.
- Invalidate on source changes; define explicit refresh/on-show refresh for outside edits.
- Prevent an older read from overwriting a newer confirmed filter state.

**Verify:** Q plus Controller cases for event/read ordering, target replacement and failed toggles; verify no writes while disconnected.

**Files likely touched:** `src/controller/command.rs`, `src/controller/event.rs`, `src/controller/app_controller.rs`, `src/controller/refresh_controller.rs`, `src/ui/window.rs`.

<a id="f03"></a>
### F03 — Expose source filter controls

**Type:** Core work · **Scope:** M, 4 files · **Estimate:** 45–90 active minutes.

Deliver the narrow frequent operation before discussing a full filter editor.

**Depends on:** [F02](#f02).

**Acceptance:**
- Show filter toggles with shared-source scope, last-read state and Refresh.
- Pending/failure state belongs to the affected filter; names are accessible.
- Do not expose unsupported arbitrary filter properties.

**Verify:** Q plus Toggle filters on a source shared by two scenes; externally edit then refresh; test keyboard and rejected mutation.

**Files likely touched:** `src/ui/widgets/source_inspector.rs`, `src/ui/pages/live.rs`, `src/ui/window.rs`, `i18n/en/scenedeck.ftl`.

**Checkpoint after F01 / F02 / F03:** run Q, confirm each listed acceptance condition, and keep existing flows usable. If this group is internal support, keep its unfinished controls unexposed until the next integration group passes. Record evidence and the next task in the checklist.

### Work group: Built-in text editing

<a id="x01"></a>
### X01 — Read and update supported text-source content

**Type:** Core work · **Scope:** M, 4 files · **Estimate:** 60–120 active minutes.

Do not assume input settings constitute a generic property schema.

**Depends on:** [S06](#s06).

**Acceptance:**
- Recognize supported built-in text kinds and retain only needed text/edit metadata.
- Use SetInputSettings with overlay=true and only changed text fields.
- File-backed or unsupported text modes are explicitly read-only until separately supported.

**Verify:** Q plus Fixtures for supported Linux/other-host built-in kinds, file-backed text, Unicode/multiline content and rejected writes.

**Files likely touched:** `src/domain/source.rs`, `src/obs/client.rs`, `src/obs/mapper.rs`, `src/controller/state.rs`.

<a id="x02"></a>
### X02 — Route explicit text reads and saves

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

No optimistic closing of the editor or silent overwrite of known external changes.

**Depends on:** [X01](#x01), [K04](#k04).

**Acceptance:**
- Keep unsaved text local until Save; capture source identity and scope.
- Refresh before save to detect a changed base value and show a conflict instead of silently replacing it.
- Failed saves retain the draft; acknowledge the remaining race because OBS has no compare-and-set for this property.

**Verify:** Q plus Tests for selection/scope change, external edit, failed save and success followed by authoritative reread.

**Files likely touched:** `src/controller/command.rs`, `src/controller/event.rs`, `src/controller/app_controller.rs`, `src/controller/state.rs`, `src/ui/window.rs`.

<a id="x03"></a>
### X03 — Expose a small text editor in source details

**Type:** Core work · **Scope:** M, 4 files · **Estimate:** 45–90 active minutes.

Exclude styling, font browsing and browser-source configuration.

**Depends on:** [X02](#x02).

**Acceptance:**
- Provide explicit Save/Cancel, shared-source scope and visible failure/conflict.
- Warn about unsaved edits on changing the target; do not send text on each keystroke.
- Text entry never triggers scene shortcuts.

**Verify:** Q plus Edit a lower third with Unicode/multiline text; cancel, fail a save and change sources with a draft.

**Files likely touched:** `src/ui/widgets/source_inspector.rs`, `src/ui/pages/live.rs`, `src/ui/window.rs`, `i18n/en/scenedeck.ftl`.

**Checkpoint after X01 / X02 / X03:** run Q, confirm each listed acceptance condition, and keep existing flows usable. If this group is internal support, keep its unfinished controls unexposed until the next integration group passes. Record evidence and the next task in the checklist.

### Work group: Advanced audio details

<a id="a01"></a>
### A01 — Read and write native audio detail fields

**Type:** Core work · **Scope:** M, 4 files · **Estimate:** 60–120 active minutes.

Use installed obws types inside the adapter only. Monitoring plays on the OBS machine, not the controller.

**Depends on:** [S06](#s06), [K01](#k01), [K04](#k04).

**Acceptance:**
- Represent monitoring, sync offset, balance and six-track assignment for the selected input.
- Validate native ranges and handle unsupported operations individually.
- Preserve ordinary mute/volume and global/shared input identity.

**Verify:** Q plus Adapter tests for monitor modes, boundary sync offsets, balance range and track mask mapping.

**Files likely touched:** `src/domain/audio.rs`, `src/obs/client.rs`, `src/obs/mapper.rs`, `src/controller/state.rs`.

<a id="a02"></a>
### A02 — Route audio detail reads and explicit setters

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Do not build a routing matrix or generic settings framework.

**Depends on:** [A01](#a01), [K04](#k04).

**Acceptance:**
- Route field-specific pending/failure outcomes and authoritative rereads.
- Refresh details on open/Refresh and relevant native events where already exposed; show observation age.
- No automatic gain/sync/routing changes follow a meter value or scene switch.

**Verify:** Q plus Tests for target replacement, rejected settings and concurrent ordinary volume adjustment.

**Files likely touched:** `src/controller/command.rs`, `src/controller/event.rs`, `src/controller/app_controller.rs`, `src/controller/refresh_controller.rs`, `src/ui/window.rs`.

<a id="a03"></a>
### A03 — Expose a compact audio details sheet

**Type:** Core work · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Track assignment changes can affect recordings; make the target and current values explicit.

**Depends on:** [A02](#a02).

**Acceptance:**
- Keep ordinary faders compact; secondary detail controls live in a source-labeled sheet.
- Explain OBS-host monitoring and source-wide track/sync effects.
- Unsupported fields and failed writes remain understandable; keyboard operation works.

**Verify:** Q plus Live OBS monitoring/sync/track checks on a disposable setup; verify shared-source effects and keyboard/large-text use.

**Files likely touched:** `src/ui/widgets/audio_card.rs`, `src/ui/pages/mixer.rs`, `src/ui/pages/live.rs`, `src/ui/window.rs`, `i18n/en/scenedeck.ftl`.

**Checkpoint after A01 / A02 / A03:** run Q, confirm each listed acceptance condition, and keep existing flows usable. If this group is internal support, keep its unfinished controls unexposed until the next integration group passes. Record evidence and the next task in the checklist.

### Release: locale catalogs

<a id="l4a"></a>
### L4A — Wave 4: synchronize English, German, Swiss German and Spanish

**Type:** Release companion · **Scope:** M, 4 files · **Estimate:** 30–60 active minutes.

Run when the selected wave scope and UI copy are complete. Conditional features may be omitted explicitly; omitted work is not marked implemented.

**Depends on:** [R01](#r01), [R02](#r02), [R03](#r03), [R04](#r04), [R05](#r05), [R06](#r06), [T01](#t01), [T02](#t02), [T03](#t03), [T04](#t04), [T05](#t05), [T06](#t06), [T07](#t07), [T08](#t08), [M01](#m01), [M02](#m02), [M03](#m03), [F01](#f01), [F02](#f02), [F03](#f03), [X01](#x01), [X02](#x02), [X03](#x03), [A01](#a01), [A02](#a02), [A03](#a03).

**Acceptance:**
- Use only finalized strings from the selected completed feature groups; preserve message variables.
- Synchronize these catalogs and record translation uncertainty for review.
- No speculative or unimplemented product labels enter the release.

**Verify:** Q plus Compare keys/variables and render long labels for selected flows; full isolated locale parity follows the next batch.

**Files likely touched:** `i18n/en/scenedeck.ftl`, `i18n/de/scenedeck.ftl`, `i18n/de-CH/scenedeck.ftl`, `i18n/es/scenedeck.ftl`.

<a id="l4b"></a>
### L4B — Wave 4: synchronize the remaining locale catalogs

**Type:** Release companion · **Scope:** M, 4 files · **Estimate:** 30–60 active minutes.

Explicit companion task; the seven non-English catalogs are not hidden in a feature's file count.

**Depends on:** [L4A](#l4a).

**Acceptance:**
- Match the same finalized keys/variables in all remaining catalogs.
- Run locale parity separately from other tests and keep fallback/translation status honest.

**Verify:** Q plus cargo test --workspace --all-features --locked -- --ignored --test-threads=1; large-text and long-label spot checks.

**Files likely touched:** `i18n/it/scenedeck.ftl`, `i18n/pl/scenedeck.ftl`, `i18n/pt-PT/scenedeck.ftl`, `i18n/uk/scenedeck.ftl`.

**Checkpoint after L4A / L4B:** All selected release strings have matching keys/variables; isolated locale parity passes.

### Release: documentation and verification

<a id="l4c"></a>
### L4C — Wave 4: document the implemented behavior

**Type:** Release companion · **Scope:** M, 5 files · **Estimate:** 45–90 active minutes.

Change only relevant sections; retain historical research as research, not current product documentation.

**Depends on:** [L4B](#l4b).

**Acceptance:**
- Document only completed controls, native API usage, state ownership and local data changes.
- Record reproducible live OBS, failure, keyboard and compact-layout cases for the selected scope.
- Explain material limits: shared sources, freshness, remote paths and read-only behavior where relevant.

**Verify:** Q plus Cross-check docs against actual commands/widgets and the selected acceptance matrix. Check relative links and git diff --check.

**Files likely touched:** `docs/obs-websocket.md`, `docs/user-guide.md`, `docs/manual-test-plan.md`, `docs/architecture.md`, `docs/configuration.md`.

<a id="l4d"></a>
### L4D — Wave 4: verify and record the releasable result

**Type:** Release companion · **Scope:** M, 3 files · **Estimate:** 60–120 active minutes.

This is a reviewable release candidate, not permission to publish. If no conditional feature in Wave 5 is selected, defer its release tasks too.

**Depends on:** [L4C](#l4c).

**Acceptance:**
- All selected task acceptance, full code checks, isolated locales and required CI jobs pass.
- An operator verifies the complete flow against disposable live OBS; record tested versions, scope and evidence.
- Update codebase/product descriptions only for shipped behavior; no commit, push or release occurs without its own authorization.

**Verify:** Q plus Run Q's full commands and required CI dependency checks; perform this wave's outcome gate and keyboard/large-text/compact matrix.

**Files likely touched:** `docs/codebase-overview.md`, `docs/manual-test-plan.md`, `site/index.html`.

**Checkpoint after L4C / L4D:** All selected acceptance criteria and required checks pass; the implemented flow is ready for review.

**Wave 4 outcome gate:** Complete each feature's real OBS path independently. No full editor, generic action engine, plugin API, or misleading completion state is introduced.


<a id="wave-5"></a>
## Wave 5 — Add validated show workflows

**Outcome:** Conditional manual scene cues and recent operations beside health charts.

**Start condition:** validate demand for each workflow separately. Cues and activity history are independent; neither is required to finish Waves 1–4.

### Work group: Manual cue data

<a id="c01"></a>
### C01 — Define repeated scene cues and local progress

**Type:** Conditional · **Scope:** M, 3 files · **Estimate:** 60–120 active minutes.

New file: src/domain/cue.rs. Do not reuse registry scene_order, which removes duplicates.

**Depends on:** [T08](#t08), [K04](#k04).

**Acceptance:**
- Give each cue its own ID, target scene binding and plain note; allow repeated scenes.
- Keep selected/current/next cue distinct from observed Program.
- External scene changes cannot advance progress or falsely confirm execution.

**Verify:** Q plus Pure cases for repeated scene targets, removed target, already-current target and external Program changes.

**Files likely touched:** `src/domain/cue.rs`, `src/domain/mod.rs`, `src/controller/state.rs`.

<a id="c02"></a>
### C02 — Persist cue lists using existing atomic storage

**Type:** Conditional · **Scope:** M, 4 files · **Estimate:** 60–120 active minutes.

New file: src/storage/cue.rs. Use fixed filenames and existing atomic writes.

**Depends on:** [C01](#c01).

**Acceptance:**
- Store versioned cues.json with endpoint/collection bindings and stable cue IDs.
- Serialize writes through a dedicated lane and preserve malformed/unknown-version data on load failure.
- Do not store passwords, arbitrary actions, scripts or OBS settings blobs.

**Verify:** Q plus Inline storage tests for repeated scene targets, corruption, failed writes and ordered saves.

**Files likely touched:** `src/storage/cue.rs`, `src/storage/mod.rs`, `src/storage/xdg.rs`, `src/ui/background_io.rs`.

<a id="c03"></a>
### C03 — Edit and load a simple running order

**Type:** Conditional · **Scope:** M, 4 files · **Estimate:** 60–120 active minutes.

Keep editing separate from live execution. Validate demand for structured shows before starting C01.

**Depends on:** [C02](#c02).

**Acceptance:**
- Add/edit/reorder/remove scene cues and notes with keyboard alternatives.
- Load/save in background with errors visible; scope switches cannot replace the active list with a late result.
- Leave all OBS state unchanged while editing.

**Verify:** Q plus Create Opening/Interview/Main/Break/Main; restart and verify separate notes/order; simulate save/load failure.

**Files likely touched:** `src/ui/pages/live.rs`, `src/ui/persist.rs`, `src/controller/state.rs`, `i18n/en/scenedeck.ftl`.

**Checkpoint after C01 / C02 / C03:** run Q, confirm each listed acceptance condition, and keep existing flows usable. If this group is internal support, keep its unfinished controls unexposed until the next integration group passes. Record evidence and the next task in the checklist.

### Work group: Manual cue execution

<a id="c04"></a>
### C04 — Execute one deliberate cue action

**Type:** Conditional · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

No timers, branching, multi-command transactions or automatic media/output actions.

**Depends on:** [C03](#c03), [T08](#t08).

**Acceptance:**
- Go/Stage invokes one explicit existing scene action using bound scope; Next remains local and deliberate.
- Do not infer request identity from an OBS Program event or require an event when already on target.
- Failures/disconnect/target removal preserve progress and explain the result.

**Verify:** Q plus Controller fixtures for already-current target, external change, rejected command and disconnect while pending.

**Files likely touched:** `src/domain/cue.rs`, `src/controller/command.rs`, `src/controller/app_controller.rs`, `src/controller/state.rs`, `src/ui/window.rs`.

<a id="c05"></a>
### C05 — Expose Current and Next cue controls

**Type:** Conditional · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Studio staging does not isolate shared sources. Retain the existing scene deck as an alternative.

**Depends on:** [C04](#c04).

**Acceptance:**
- Show cue note and target; distinguish Go, Next, Stage and Take by current mode.
- Keep global Program context visible when cue position differs.
- Text editing suppresses execution shortcuts; external mode changes update available actions.

**Verify:** Q plus Keyboard/mouse walkthrough with repeated scenes in Direct and Studio modes; remove a scene during operation.

**Files likely touched:** `src/ui/pages/live.rs`, `src/ui/hotkeys.rs`, `src/services/hotkey_service.rs`, `src/ui/window.rs`, `i18n/en/scenedeck.ftl`.

<a id="c06"></a>
### C06 — Validate a complete manual show sequence

**Type:** Conditional · **Scope:** M, 3 files · **Estimate:** 60–120 active minutes.

Do not add automation to rescue an unclear manual interaction.

**Depends on:** [C05](#c05).

**Acceptance:**
- Run a representative sequence without unintended scene/output/media operations.
- Record behavior after external Program changes, failures and reconnect.
- Confirm the cue workflow is useful enough to keep; otherwise retain the simpler scene deck.

**Verify:** Q plus Focused cue/controller tests plus a rehearsed five-cue live OBS walkthrough with a second operator changing OBS.

**Files likely touched:** `src/domain/cue.rs`, `src/controller/app_controller.rs`, `docs/manual-test-plan.md`.

**Checkpoint after C04 / C05 / C06:** run Q, confirm each listed acceptance condition, and keep existing flows usable. If this group is internal support, keep its unfinished controls unexposed until the next integration group passes. Record evidence and the next task in the checklist.

### Work group: Recent operations beside Stats

<a id="h01"></a>
### H01 — Model bounded recent operations with timestamps

**Type:** Conditional · **Scope:** M, 4 files · **Estimate:** 60–120 active minutes.

New file: src/domain/activity.rs. Session-only storage; no event-sourcing framework or permanent database.

**Depends on:** [K04](#k04).

**Acceptance:**
- Define bounded scene/output/failure records with local-request versus observed-event provenance.
- Timestamp stats samples and activity on a comparable time basis; identify disconnect gaps and counter resets.
- Exclude per-frame meters and coalesce continuous-control noise.

**Verify:** Q plus Pure tests for eviction, timestamp ordering, restart/counter reset and provenance.

**Files likely touched:** `src/domain/activity.rs`, `src/domain/mod.rs`, `src/domain/stats.rs`, `src/controller/state.rs`.

<a id="h02"></a>
### H02 — Record relevant commands and OBS observations

**Type:** Conditional · **Scope:** M, 5 files · **Estimate:** 60–120 active minutes.

Use only the events needed for the chosen diagnostic story.

**Depends on:** [H01](#h01).

**Acceptance:**
- Record local request intent separately from authoritative OBS events and failure results.
- Do not attribute another client's action to SceneDeck; mark unobserved intervals.
- Bound event work and avoid blocking the control channel behind history processing.

**Verify:** Q plus Controller/event fixtures for external scene switches, delayed failures, duplicate observations and reconnect gaps.

**Files likely touched:** `src/controller/command.rs`, `src/controller/event.rs`, `src/controller/app_controller.rs`, `src/controller/refresh_controller.rs`, `src/ui/window.rs`.

<a id="h03"></a>
### H03 — Show recent operations alongside health charts

**Type:** Conditional · **Scope:** M, 4 files · **Estimate:** 60–120 active minutes.

Persistence/export and automatic root-cause claims stay deferred.

**Depends on:** [H02](#h02).

**Acceptance:**
- Show a bounded Recent activity list; selecting an entry locates its chart time.
- Distinguish correlation from cause and show observation gaps.
- Keyboard and screen-reader users can access the event text without the chart.

**Verify:** Q plus Live OBS scene/output actions around a synthetic load change; verify timestamps, bounded rows and readable empty/gap states.

**Files likely touched:** `src/ui/pages/stats.rs`, `src/ui/widgets/chart.rs`, `src/ui/window.rs`, `i18n/en/scenedeck.ftl`.

**Checkpoint after H01 / H02 / H03:** run Q, confirm each listed acceptance condition, and keep existing flows usable. If this group is internal support, keep its unfinished controls unexposed until the next integration group passes. Record evidence and the next task in the checklist.

### Release: locale catalogs

<a id="l5a"></a>
### L5A — Wave 5: synchronize English, German, Swiss German and Spanish

**Type:** Conditional · **Scope:** M, 4 files · **Estimate:** 30–60 active minutes.

Run when the selected wave scope and UI copy are complete. Conditional features may be omitted explicitly; omitted work is not marked implemented.

**Depends on:** None (follow the selected wave order).

**Acceptance:**
- Use only finalized strings from the selected completed feature groups; preserve message variables.
- Synchronize these catalogs and record translation uncertainty for review.
- No speculative or unimplemented product labels enter the release.

**Verify:** Q plus Compare keys/variables and render long labels for selected flows; full isolated locale parity follows the next batch.

**Files likely touched:** `i18n/en/scenedeck.ftl`, `i18n/de/scenedeck.ftl`, `i18n/de-CH/scenedeck.ftl`, `i18n/es/scenedeck.ftl`.

<a id="l5b"></a>
### L5B — Wave 5: synchronize the remaining locale catalogs

**Type:** Conditional · **Scope:** M, 4 files · **Estimate:** 30–60 active minutes.

Explicit companion task; the seven non-English catalogs are not hidden in a feature's file count.

**Depends on:** [L5A](#l5a).

**Acceptance:**
- Match the same finalized keys/variables in all remaining catalogs.
- Run locale parity separately from other tests and keep fallback/translation status honest.

**Verify:** Q plus cargo test --workspace --all-features --locked -- --ignored --test-threads=1; large-text and long-label spot checks.

**Files likely touched:** `i18n/it/scenedeck.ftl`, `i18n/pl/scenedeck.ftl`, `i18n/pt-PT/scenedeck.ftl`, `i18n/uk/scenedeck.ftl`.

**Checkpoint after L5A / L5B:** All selected release strings have matching keys/variables; isolated locale parity passes.

### Release: documentation and verification

<a id="l5c"></a>
### L5C — Wave 5: document the implemented behavior

**Type:** Conditional · **Scope:** M, 5 files · **Estimate:** 45–90 active minutes.

Change only relevant sections; retain historical research as research, not current product documentation.

**Depends on:** [L5B](#l5b).

**Acceptance:**
- Document only completed controls, native API usage, state ownership and local data changes.
- Record reproducible live OBS, failure, keyboard and compact-layout cases for the selected scope.
- Explain material limits: shared sources, freshness, remote paths and read-only behavior where relevant.

**Verify:** Q plus Cross-check docs against actual commands/widgets and the selected acceptance matrix. Check relative links and git diff --check.

**Files likely touched:** `docs/obs-websocket.md`, `docs/user-guide.md`, `docs/manual-test-plan.md`, `docs/architecture.md`, `docs/configuration.md`.

<a id="l5d"></a>
### L5D — Wave 5: verify and record the releasable result

**Type:** Conditional · **Scope:** M, 3 files · **Estimate:** 60–120 active minutes.

This is a reviewable release candidate, not permission to publish. If no conditional feature in Wave 5 is selected, defer its release tasks too.

**Depends on:** [L5C](#l5c).

**Acceptance:**
- All selected task acceptance, full code checks, isolated locales and required CI jobs pass.
- An operator verifies the complete flow against disposable live OBS; record tested versions, scope and evidence.
- Update codebase/product descriptions only for shipped behavior; no commit, push or release occurs without its own authorization.

**Verify:** Q plus Run Q's full commands and required CI dependency checks; perform this wave's outcome gate and keyboard/large-text/compact matrix.

**Files likely touched:** `docs/codebase-overview.md`, `docs/manual-test-plan.md`, `site/index.html`.

**Checkpoint after L5C / L5D:** All selected acceptance criteria and required checks pass; the implemented flow is ready for review.

**Wave 5 outcome gate:** A representative operator can run repeated cues or investigate a real event sequence. Reject unsupported attribution, automatic advancement and unobserved history claims.


## 7. Dependency and sequencing summary

```text
Existing behavior: D01/D02/Q01
Capability and scope: K01, then K02/K03/K04
  Source reads: S01/S02/S03
    Visibility and external edits: S04/S05/S06
      Local search: S07
      Usage context: U01/U02/U03
      Media / filters / text: M / F / X groups

Fresh observations: P01/P02/P03
  Preflight UI: P04/P05/P06
  Rehearsal baseline: B01 through B06

Existing output path + scope: O01/O02/O03 and R01 through R06
Studio observations: T01/T02/T03
  Stage/Take: T04/T05
  Transitions and validation: T06/T07/T08
    Optional images: I01/I02/I03
    Conditional cues: C01 through C06

Watched audio: W01/W02/W03 (conditional)
Advanced audio details: A01/A02/A03 (does not require watching)
Activity alongside Stats: H01/H02/H03 (conditional)
```

The dependency graph names technical prerequisites. Wave placement expresses product priority. Do not invent a dependency on the entire UI redesign, a dashboard builder, complete telemetry history or full collection indexing.

**Parallel work:** pure domain rules, existing UI accessibility tasks and documentation for completed behavior can proceed independently. Changes to `command.rs`, `event.rs`, `state.rs`, `app_controller.rs`, `refresh_controller.rs` and `window.rs` need one owner or explicit sequencing. Assign agents a named task and file ownership, not an entire shared layer.

## 8. Acceptance scenarios to keep throughout development

| Scenario | Required result | Main tasks |
|---|---|---|
| One input appears twice in one scene | Item visibility targets the chosen owned item; audio remains one input | S01–S06 |
| A nested scene/group appears through multiple parents | Usage retains all paths; child visibility explains shared ownership | S01, U01–U03 |
| A child read fails or a meter goes quiet | Unknown/incomplete is distinct from disabled/silent | P02, W02 |
| Collection changes to another with the same names | Old selections/results/baselines cannot silently bind | K02–K04, B01–B06 |
| Another client changes state during a read/write | Current observation wins; failures reconcile without false success | S06, O03, R04, T08, X02 |
| Replay save event precedes acknowledgement | Display observed save without later regression or invented requester | R04–R06 |
| OBS is remote | Disk space, monitoring and file paths are labeled as OBS-host values | D04, A03, R05 |
| Source is edited while inspecting Preview | Explain input/owner-wide effects; never promise isolation | U03, T07, M03, F03, X03 |
| User types in a source/cue field | Scene/Take/Go shortcuts do not fire | T05, X03, C05 |
| Compact window, long labels, large text | Active outputs and primary controls stay reachable | Q02/Q03 and every feature UI |

## 9. Deferred backlog, preserved explicitly

These remain valid research candidates. They are not hidden implementation requirements or lost ideas.

| Candidate | Why deferred | Evidence needed before a small task breakdown |
|---|---|---|
| Virtual camera | Useful for a specific workflow; lower priority than source/record control | A real OBS-camera operating use case |
| Saved named OBS endpoints | Helpful for several machines; current target is one session | Repeated endpoint switching and a keyring migration contract |
| Native extra canvases | Native capability can fit the boundary, but expands identity/UI scope | Confirmed multi-canvas workflow and supported OBS versions; no Aitum/vendor calls |
| OBS registered hotkey favorites / command palette | Existing local scene shortcuts already cover part of the need | Repeated native actions that explicit controls/search cannot cover |
| Record split, chapters and local moment notes | Configuration/time-base details need focused validation | Recording format support and a clear desired marker workflow |
| Selected-property restore | A baseline diff is useful without writes | Read-only baseline use proves demand; define conflicts and partial failure first |
| Durable session history/export | Current live Stats and conditional recent activity may suffice | Concrete retention/export requirement and bounded storage policy |
| Timed microphone activity check | Can produce false confidence/noise | A deliberate operator-driven test with honest missing-sample handling |
| Baseline visibility comparison | Item identity/lifetime makes persistence more complex than mute/gain | Stable item binding behavior after scene edits and collection reloads |
| Generic filter forms / transformations / full editor | Large UI and source-kind surface | A specific recurring operation, not a desire to duplicate OBS |
| Automated cues / macros / bulk repair | Multi-write failures and live side effects exceed the chosen scope | A separate explicit specification; no claim of transactionality |

Chat and third-party/vendor integrations are excluded, not queued for later.

## 10. Risks, stop conditions and recovery

| Risk | Mitigation | Stop/replan trigger |
|---|---|---|
| False readiness or stale baseline | Fresh scoped reads, observation interval, unknown/partial outcomes | A failed/missing read can still yield Pass |
| Wrong item/source affected | Owner/item identity; source-wide scope labels; explicit desired state | A duplicate or shared-container fixture targets the wrong operation |
| Reply from a previous session accepted | Session/collection generations; request ordering and cancellation | Delayed old work changes current widgets or pending state |
| UI gets harder to operate | Context beside controls; native widgets; few persistent sections | Primary actions disappear at target width or focus is lost during refresh |
| Image/discovery work burdens OBS | Bounded work, visible-target lifetime and measured screenshot budget | Measured render/latency regression exceeds the agreed I01 budget |
| New persistence destroys user data | Versioned files, atomic ordered writes, preserve unreadable originals | Corruption/version mismatch silently produces a writable empty document |
| Tasks become large or speculative | Five-file review units, two-hour timeboxes, group checkpoints | Sixth file, broader subsystem change, or uncertain contract needs a split |

Before a persistent-data change, keep a test fixture for the old format. Reverting code must not delete user files. A schema change needs an explicit backward-read or version-error path; do not silently downgrade and overwrite.

If a task stalls, record the failing assumption and a smallest reproduction. Resume from that evidence rather than restarting the research. Do not expand a failing feature into a framework.

## 11. Implementation outcome and qualification

The five planned feature waves are implemented. The screenshot prototype met the bounded fixture budget and the optional still-image UI was implemented. Media, filter, built-in text and advanced audio controls are also implemented. Native virtual-camera, split-file and chapter controls are capability gated.

[Verification report](../../docs/verification/2026-10-19/README.md) records the actual checks, OBS behavior, implementation adaptations and untested cases. [Task tracking](../../tasks/todo.obs-features-history.md) keeps full release qualification separate from feature delivery. The deferred backlog in section 9 remains outside this delivery.

Important implementation choices:

- New OBS operations use one scoped workspace controller, epoch/sequence rejection and cancellation. Existing stream/record lifecycle controllers remain in place.
- OBS snapshots refresh on relevant events and a ten-second connected cadence. Expanded selected-media details use a separate one-second visible-only read. Usage scans are bounded to 64 scenes and 4096 inspected items and report partial coverage.
- Baselines and setup observations share a bounded aggregate read. Only explicitly selected complete audio values are captured. This trades extra reads for shared factual evidence; selected-only acquisition remains a performance optimization.
- Item expectations mean enabled through at least one inspected path; unscanned owners remain Unknown. This does not establish visual unobscuredness or Program isolation.
- Registered arbitrary OBS hotkeys, macros, chat, vendor requests, third-party integrations, baseline restoration and full scene editing were not added.
- Registry YAML moved to a maintained safe parser after the dependency gate identified the legacy unmaintained backend. Existing exports and metadata roundtrips remain tested.
