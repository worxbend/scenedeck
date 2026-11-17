# SceneDeck deep review and restructuring plan

Date: 2026-11-14. Commit reviewed: `31d512f` (main, v0.5.3 plus the scoped show workspaces feature).
Scope: the whole crate (`src/`, about 74k lines), with an emphasis on code smells, file restructuring and domain-driven design.

This document is the handoff for agents who will do the work later, one task per session, without this conversation. Read section 2 before any task. Each task in section 5 can be done on its own once its `depends_on` tasks are merged.

Inputs: 13 verified reviewer reports (299 findings, each re-checked by an adversarial verifier against the code). The raw reports are in [`findings/`](findings/). Every finding ID below (for example `controller-state-03`) resolves to an entry in one of those JSON files, with evidence, recommendation and verifier note. Where this plan and a reviewer disagree, the plan wins: section 8.3 lists the conflicts and how each was settled against the code.

---

## 1. Executive summary

### 1.1 Health

The code is in good shape on the basics. Baseline from the initial review:

- `cargo clippy -D warnings` is clean.
- 774 unit tests pass, plus 100 ignored ones (native GTK tests and the i18n locale-parity test).
- No `unsafe`, no TODOs.
- The Tokio side is mostly healthy: task tracking is bounded, no `Mutex` guard is held across `.await`, and refreshes are coalesced.

The debt is structural. Lines of code by module:

| Module | Lines |
|---|---|
| ui | 37.7k |
| controller | 15.9k |
| domain | 7.0k |
| storage | 4.4k |
| obs | 3.9k |
| services | 2.5k |
| infra | 0.4k |

The most recent feature commit (9488c86: 138 files, +28k lines) added behavior by growing the existing hubs:

- 9 more fields on AppState;
- 6 new `impl AppState` files;
- 13 flat modules at the `ui/` root.

### 1.2 Top 10 problems

1. **AppState is a god aggregate in the wrong layer.** It has 47 fields across about 9 bounded contexts, plus `WorkspaceState` with 31 fields, and it lives in `controller/`. Page code writes its fields directly: 559 `borrow_mut()` calls in the crate, about 308 of them on AppState. Nothing enforces borrow discipline, so a double borrow panics at runtime.
   - Findings: architecture-global-01, controller-state-01, controller-state-02, recent-change-review-04, controller-state-21.
2. **The narrow page port is not narrow.** CLAUDE.md says pages see only `switch_to_page()` and `dispatch()`. In fact `NavigationContext` exposes the raw `Rc<RefCell<AppState>>` (`ui/navigation.rs:20`), global shortcuts and the registry binding.
   - Findings: architecture-global-02, ui-shell-05, recent-change-review-13.
3. **Storage has no anti-corruption layer.** The serde DTO `SceneRegistry` is the live show-curation aggregate across five contexts. Domain structs are the on-disk schema, and UI pages hold curation rules inside GTK closures.
   - Findings: architecture-global-03, storage-infra-01, domain-services-ddd-05, recent-change-review-06, ui-pages-05.
4. **The OBS anti-corruption layer leaks in both directions.**
   - `obs/` imports `controller::event::ConnectionInfo`.
   - Every obws error becomes a String.
   - Request-name strings leak into UI capability checks.
   - The domain carries OBS wire shapes (`from_mul`, `is_text_kind`).
   - `obs/client.rs` has a 24-arm, 290-line dispatcher.
   - Findings: obs-acl-01, obs-acl-02, obs-acl-04, obs-acl-05, domain-services-ddd-07, uiw-05.
5. **God files that mix many responsibilities.** Eighteen files are over 1,000 lines (one of them, `controller/final_regression_tests.rs`, is all tests). The worst:
   - `controller/state.rs` 3088;
   - `ui/window.rs` 2698, with a 590-line `apply_event` switch over 45 events;
   - `ui/widgets/operating_panel.rs` 2361, a full page living in `widgets/`;
   - `ui/pages/inventory.rs` 2126;
   - `ui/pages/mixer.rs` 2003;
   - `app.rs` 1345, with a 260-line close-guard state machine held in boolean Cells.
6. **Presentation lives inside the domain.** There are 141 `fl!` calls in 12 domain files, and CSS classes and GTK icon names sit on domain enums. The obvious fix (move labels to `ui/`) is blocked: the controller calls these labels, and `controller/` may not import `ui/`.
   - Findings: domain-services-ddd-01, controller-state-08.
7. **Primitive obsession on identity and values.** Examples:
   - `SceneId`/`InputId` are String aliases, and identity is modelled four ways.
   - The "unambiguous identity" rule is copied by hand in four places.
   - Endpoints are `host:port` text formatted in five places, and that text is also a workspace key on disk.
   - The OBS password is a plain `String` inside a `Debug + Clone` struct.
   - Findings: domain-services-ddd-02, domain-services-ddd-03, security-03, security-04.
8. **The UI synchronizes by polling.**
   - About 15 long-lived glib timers, plus about 20 dialog timers.
   - Three perpetual frame-tick callbacks keep the frame clock running at display refresh rate while idle.
   - A 50 ms event pump, with a producer that busy-retries every 2 ms.
   - `collect_report` runs about 60 times a second.
   - Findings: performance-01, performance-02, performance-03, performance-05.
9. **Correctness and security bugs that the restructuring must not bury:**
   - OBS scene names render as Pango markup;
   - manual Refresh drops output readback failures;
   - the close-failure Retry does not re-admit most writers;
   - scene switching depends on optional curation readiness;
   - a newer config is moved aside or down-converted;
   - single-flight queues strand requests.
   - Findings: security-01, ctrl-03, recent-change-review-02, recent-change-review-01, storage-infra-03, ctrl-04.
10. **The safety net is thinner than the test count suggests.**
    - Native tests are addressed by string literal, so a moved test can pass vacuously.
    - One test scans its own source file.
    - OBS wire tests are scattered across hand-written loopback servers.
    - Large inline test modules (50-90% of several god files) hide the real production size.
    - No ratchet stops the debt from growing.
    - Findings: ui-shell-01, tests-structure-12, tests-structure-01, tests-structure-07, architecture-global-11.

**Time-critical:** the `RUSTSEC-2026-0374` exception in `deny.toml` expires on **2026-11-20**, six days from this review. After that date every CI run fails. Task P0-T01 handles it and must come first.

### 1.3 Strategy: gradual, always green

- **Nets before moves.** Phase 0 adds the missing safety nets:
  - drift-proof native tests;
  - golden fixtures for every persisted file;
  - a scripted OBS mock that counts requests;
  - characterization tests;
  - ratchet fitness functions that pass today and fail if any debt grows.
- **Fixes before restructuring.** Phase 1 lands the behavior fixes as small `fix:` commits. Later restructuring commits can then be pure moves, with an unchanged test count.
- **Mechanical before semantic.** Phase 2 pulls inline tests out of god files. Phase 3 removes the inverted edges that would otherwise cause module cycles. Without that, moving AppState out of `controller/` creates a `controller <-> state` cycle.
- **Inside out.** Next comes the domain kernel and bounded contexts (Phase 4), then the storage ACL (5), the OBS ACL (6), Tokio orchestration (7), AppState slices (8), the UI shell (9) and the pages (10).
- **Every step compiles and passes the four CI commands.** Moves keep `pub use` shims for one PR. Each task is one PR, and a pure move is never mixed with a behavior change.
- **Fitness functions as ratchets.** Every phase lowers a ceiling in `architecture_tests` (borrow counts, `fl!` in the domain, production file sizes, thread_locals, ui->storage imports), so the debt cannot come back.

Size of the plan: 189 tasks in 12 phases (counts per phase in section 5). It covers all 298 confirmed or adjusted findings; the one refuted finding is in section 8.

---

## 2. Implementer ground rules

Read this section before every task. These rules override any task text that seems to conflict with them.

### 2.1 Definition of done (every task)

Run the four CI commands from CLAUDE.md. All must pass.

```sh
cargo fmt --all -- --check
cargo check --workspace --all-features
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Abbreviations used in task verification fields:

| Code | Meaning |
|---|---|
| **SV** | Standard verification: the four commands above. Always required. |
| **NV** | Native GTK tests: `python3 tools/test-native.py` (after P0-T02: `--filter <substring>` for a subset, but run the full set before merging). Required when any file under `src/ui/` or `src/app.rs` changes. Never run native tests with plain cargo. |
| **IV** | i18n parity: `cargo test --workspace --all-features infra::i18n::tests::every_shipped_locale_parses_and_loads -- --exact --ignored --test-threads=1`. Required when any `.ftl` file or `fl!` call changes. |
| **TC** | Test-count guard for pure moves. Run `cargo test --workspace --all-features -- --list 2>/dev/null \| grep -c ': test$'` (and the same with `--ignored`) before and after, and put both numbers in the PR description. They must be equal, unless the task explicitly deletes or table-drives tests; then list each removed test. |

CI also runs `cargo audit` and `cargo deny check advisories bans licenses sources`. Run them when `Cargo.toml` or `Cargo.lock` changes.

### 2.2 Change rules

1. **One task = one PR.** Big tasks may be several stacked PRs (marked "x2 PRs" or "per batch"). Each PR must pass SV on its own.
2. **Pure moves are separate from behavior changes.**
   - A move PR changes paths, `use` lines and visibility only, and the test count stays the same (TC).
   - If a test has to change for a move to compile, the move changed behavior. Stop and split the behavior change into its own PR.
   - Conventional Commit types: pure moves are `refactor:`, behavior fixes `fix:`, tests only `test:`, docs `docs:`, dependency or CI changes `build:`/`ci:`. Never mix `refactor:` and `fix:` in one commit (recent-change-review-28).
3. **Size.** Aim for under 600 changed lines per PR, not counting pure moves. A pure move can be larger, as long as it is only a move.
4. **Module layout.** A module is either `x.rs` with children in `x/`, or `x/mod.rs`, never both. When a task lists `x/mod.rs` but `x.rs` already exists (for example `ui/persist.rs` after P9-T04 adds `ui/persist/registry_sync.rs`), keep `x.rs` as the module root and add children under `x/`. Two parallel tasks that both create the first child of the same module (for example P1-T13 and P1-T16 under `domain/connection/`, or P7-T08 and P7-T16 under `controller/refresh/`) conflict on the module root file; the plan orders them, so rebase rather than duplicating the `mod` declaration.
5. **Shims.** When moving a public item, leave a `pub use old::path::Item;` for one PR, then remove it in the next PR of the same task.
   - Do not use `#[deprecated]` for transition delegates. `-D warnings` turns the lint into an error at every remaining call site. Migrate all callers in the same commit, or add a scoped `#[allow(deprecated)]` with a removal task.
6. **Threading rules (CLAUDE.md, keep them):**
   - Widget mutation and AppState stay on the GTK thread.
   - OBS work stays on Tokio.
   - Never hold a `std::sync::Mutex` guard across `.await`.
   - Config and Secret Service access from the controller run on the blocking pool.
   - Local persistence is not routed through AppCommand/AppEvent.
   - A failed command must not reuse connection-level error events.
7. **Borrow rule (from P0-T08 on).** Inside a `nav.read(..)` or `nav.update(..)` closure, never call:
   - `dispatch`;
   - a persist function;
   - another page's refresher;
   - anything that borrows AppState again.

   Compute a value inside the closure, drop the borrow, then act on the value.
8. **i18n.** Every new user-visible string uses `fl!(LANGUAGE_LOADER, "id")` and is added to all 8 locales: `de`, `de-CH`, `en`, `es`, `it`, `pl`, `pt-PT`, `uk` under `i18n/<locale>/scenedeck.ftl`.
   - `every_translation_has_the_same_messages_as_english` fails on a missing key. Removing a key also means removing it everywhere.
   - English-only strings in the controller or obs adapter are findings, not patterns to copy.
9. **Persisted formats.** Any change to a serde type in `domain/` or `storage/` must keep the P0-T05 golden fixtures green. If a fixture has to change, the PR is a format change: say so in the title and describe the migration and backward-read path.
10. **Behavior changes that need the maintainer** are marked "MAINTAINER DECISION" in the task. Do not start those without an answer recorded in the PR (see 8.2).
11. **Errors.** Do not add new `unwrap`/`expect` in production code. Do not swallow errors (`.ok()`, `let _ =`) unless a comment says why the failure is harmless. Use `infra::sync::lock_or_recover` (P0-T09) for std mutexes, except in places P0-T10 documents otherwise.

### 2.3 When structure changes

- **Update `src/architecture_tests/` in the same PR.** That means new rules, moved exemptions and lowered ratchet ceilings. A ceiling may only go down.
- **Update `docs/architecture.md` and `docs/codebase-overview.md`** for every module you add, move or delete. Update CLAUDE.md when a "Common changes" recipe or a stated rule changes. Examples: the New-page recipe after P9-T08, the event pump description after P9-T13, the NavigationContext statement after P9-T12.
- **Native test names are computed from `module_path!()` after P0-T02.** Before P0-T02 is merged, do not move any file that contains native tests.
- **`#[path]` test mounts and `isolated_test_child` names embed paths.** Grep for the old path after each move. The main ones are `ui::persist::tests::` (six sites) and `services::doctor_format::tests::...` (`doctor_format.rs:329`).
- **Check `site/index.html`** when a user-visible feature changes (CLAUDE.md product-site rule).

### 2.4 How to read a task

Each task lists:

- **why**: the finding IDs it addresses (look them up in `findings/*.json`);
- **files**: with the line ranges reviewers verified at `31d512f`. Lines drift as earlier tasks land, so re-locate items by name, not by number;
- **steps**;
- **acceptance criteria**;
- **verification**;
- **depends_on**;
- **size**: S under 200 changed lines, M 200-600, L larger (pure moves only);
- **risk**;
- **parallelizable with**.

Paths written as `src/state/...` assume P8-T01 has moved `controller/state` there. Before P8-T01 the same files are at `src/controller/state/...`.

Paths written as `src/domain/<context>/...` (for example `domain/audio/mixer.rs`, `domain/workspace/operator_action.rs`, `domain/outputs/output_result.rs`) assume P4-T12/P4-T13 have created the context directories. Before then the item lives in the flat file (`domain/mixer.rs`, `domain/operator_action.rs`, `domain/output_result.rs`). A task that only edits an existing item may work on the flat path; a task that creates a new file inside a context directory must wait for P4-T12 or P4-T13 (critic pass: these dependencies are explicit for P4-T25 and P10-T09).

---

## 3. Target architecture

### 3.1 Quality attributes and the decision

Quality attributes, ranked for this product (a desktop tool that runs for hours during a live show):

1. Operator safety: the right scene and output, with no stale command reaching OBS.
2. Responsiveness of the GTK thread.
3. Evolvability: features keep arriving.
4. Testability without a running OBS.
5. Idle resource use.

Options considered:

| Option | Verdict |
|---|---|
| (a) Keep layers as they are, only split files. | Cheapest. It leaves the AppState hub, the missing ACLs and presentation in the domain. Rejected: the main costs come from cross-context coupling, not file length. |
| (b) Modular monolith by bounded context inside the existing layers. Domain contexts as sub-modules, AppState as per-context slices, ACLs at `obs/` and `storage/`, and a GTK-free presentation module. | **Chosen.** Fits one crate and one small team, keeps the current threading model, and every step can land behind shims. |
| (c) Split into several crates (domain, obs adapter, storage, app). | Strongest enforcement, but premature. It needs (b) first anyway, because today the cycles (obs->controller, state<->controller) block it. Revisit after Phase 8. |

**Decision:** option (b). Record it in `docs/adr/0001-bounded-contexts.md` (P4-T01). The revisit trigger for (c): Phase 8 done and compile times or team size become a problem.

### 3.2 Bounded contexts and ubiquitous language

| Context | Owns | Today scattered in | Target home |
|---|---|---|---|
| **Session** | OBS connection lifecycle, endpoint, connection profiles, credentials, server info and capabilities, session generation, retirement | `controller/{session_controller,lifecycle}.rs`, `controller/state.rs` connection fields, `storage/config/connections.rs`, `storage/secret.rs` | `domain/connection/`, `domain/obs.rs`, `controller/session/`, `src/state/session.rs` |
| **Program** | Scenes, scene inventory, program and preview, studio mode, transitions, scene graph and dependency policy, deck slots, scene hotkeys | `domain/{scene,role,graph,registry,hotkey,obs}.rs`, `controller/state.rs` topology fields, `ui/pages/live.rs` slot logic | `domain/program/`, `src/state/collection.rs` |
| **Audio** | Inputs, gain/dB, meters, mixer selection, audio presets and runs, audio settings | `domain/{audio,meter,mixer,audio_preset}`, `services/audio_service.rs`, `controller/state/mixer.rs`, `ui/pages/mixer.rs` rules | `domain/audio/`, `services/audio_debounce.rs`, `src/state/{active_audio,mixer}.rs` |
| **Outputs** | Stream, record, replay buffer, virtual cam, output commands and results, confirmations, recording evidence | `domain/{output,output_result}.rs`, `controller/output_controller.rs`, state output fields, `ui/pages/live.rs` policy | `domain/outputs/`, `controller/output/`, `src/state/outputs.rs` |
| **Operating workspace** | Source inventory, source detail, operator actions, action bindings, quick actions, shortcuts, the operating view's read fence | `domain/{control,operator_action,operator_input,action_shortcut,icon}.rs`, `controller/workspace.rs`, `WorkspaceState`, `ui/action_catalog.rs` resolution | `domain/workspace/`, `controller/workspace/`, `src/state/workspace/` |
| **Show curation** | Per-workspace curation: scene roles, order, accent, icons, retained scenes, decks, filters; the show-workspace store | `storage/registry.rs` (`SceneRegistry`), `storage/show_workspaces.rs`, `controller/state/show_registry.rs`, `ui/persist.rs`, `ui/pages/inventory.rs` rules | `domain/curation/` (aggregate), `storage/registry/` (DTO + IO), `src/state/local_store.rs` |
| **Diagnostics** | Doctor findings, health alerts, preflight checks, stats health severity | `domain/{diagnostic,health/,preflight}.rs`, `services/doctor_service.rs`, `ui/pages/stats.rs` thresholds | `domain/diagnostics/` |
| **Reporting** | Activity log, session reports, report history, bookmarks, CSV export | `domain/{workflow,session_report,bookmark,stats}.rs`, `controller/state/{session_reports,report_history,bookmarks}.rs`, `storage/{report_history,bookmarks,report_export}.rs` | `domain/session/`, `src/state/{session_reports,report_history,bookmarks}.rs` |
| **Appearance** | Theme mode and id, density, language, motion, custom CSS preference | `domain/appearance.rs`, `ui/theme.rs` | `domain/appearance/`, `ui/theme/` |

Persistence (`storage/`), the OBS adapter (`obs/`), localization (`presentation/`, `infra/i18n`) and the GTK shell are technical layers, not contexts.

**Glossary** (goes into `docs/ubiquitous-language.md` in P4-T01). Today "workspace" has four meanings and "registry" three (architecture-global-07). The target terms:

| Term | Meaning | Today's identifiers | Target identifier |
|---|---|---|---|
| Show workspace | Persisted curation for one (endpoint, scene collection) pair | `show_workspaces::RegistryTarget::Workspace`, `StoredWorkspace`, `WorkspaceKey` | `ShowWorkspace`, `ShowWorkspaceId`, `WorkspaceKey` (domain) |
| Show curation | The curation content of a show workspace | `SceneRegistry` (storage DTO used as the aggregate), `domain::registry::SceneRegistrySnapshot` | `ShowCuration` (domain), `SceneRegistryDto` (storage) |
| Legacy registry | Pre-workspace `registry.json` | `RegistryTarget::File`, `registry.json` | unchanged on disk; `LegacyRegistry` in code |
| Workspace selection | Which show workspace is active, plus revision and generation | `ShowRegistryState` | `ShowWorkspaceSelection` |
| Operating view | The live, read-fenced view of the source inventory and controls | `WorkspaceState` | `OperatingView` (optional rename; at least document it) |
| Operating read / observation | A controller read of OBS for the operating view | `WorkspaceRequest`, `WorkspaceObservation` | `OperatingRead`, `OperatingObservation` |
| Production scope | (endpoint, scene collection) binding used to fence commands | `ScopeBinding`, `workflow_scope()` | `ProductionScope { endpoint, collection }` (keep the serde field names) |
| Endpoint | host + port of an OBS WebSocket server | `String` "host:port" in 5 places | `ObsEndpoint { host: ObsHost, port: NonZeroU16 }`, `EndpointKey` |
| Connection profile | A saved connection with id and name | `SavedConnection`, `connection_id: String`, `"default"` literal | `ConnectionId` (newtype, `legacy_default()`) |
| Scene / input / source reference | Name plus optional OBS uuid | `SceneId = String`, `InputId = String`, `ActionTarget{name, uuid}` | `SceneName`, `InputName`, `ObsUuid`, `ObsRef<N>` |
| Capability | An OBS request the server supports | `available_requests: Vec<String>` and string literals | `ObsCapabilities` with named `KnownRequests` |
| Generation / Revision | Equality fence (wrapping) / ordering fence (fail-closed) | assorted `u64` with 4 overflow policies | `Generation`, `Revision` (domain/revision.rs) |
| Inventory read warning | A partial-read warning on the source inventory | `WorkspaceWarning` | `InventoryReadWarning` |

The rename rule: Rust identifiers only, one concept per PR. File names on disk, serde keys and user-visible i18n keys do not change.

### 3.3 Target module tree

New or moved items are marked. Existing names are kept where they already fit.

```text
src/
  lib.rs                       run() only
  app/                         composition root (P9-T09)
    mod.rs                     run(): runtime, startup, GTK app, shutdown (~60 lines)
    startup.rs                 StartupData, load_startup (config + registry only; no keyring)
    runtime.rs                 build_runtime, shutdown_controller
    activation.rs              connect activate/shutdown, build_ui
  infra/
    error.rs                   AppError (structured variants; Clone)
    error_text.rs              user_message (callable from controller/obs; NOT in ui)
    i18n.rs                    Fluent loader, takes a locale code                (P3-T08)
    logging.rs
    clock.rs                   unix_ms(), Now { instant, unix_ms }        (P3-T02, P3-T05)
    sync.rs                    lock_or_recover                             (P0-T09)
    perf.rs                    perf-trace counters                         (P0-T15)
    portal/global_shortcuts.rs D-Bus portal client, PortalError           (P9-T18)
  domain/                      pure, no gtk/glib/obws/storage/ui/controller; no fl! at the end
    mod.rs                     context modules + kernel only
    kernel/
      identity.rs              SceneName, InputName, ObsUuid, ObsRef<N>, Resolution, resolve, targets_distinct
      observation.rs           fresh_within, Observed<T>
      label.rs                 OperatorLabel (Trimmed / Exact policies)
      keys.rs                  Modifiers (+consts), KeySymbol, KeyStroke
      scope.rs                 ProductionScope (ex ScopeBinding)
      string_enum.rs           string_enum_serde! macro
    revision.rs                Generation, Revision                        (P3-T06)
    capabilities.rs            ObsCapabilities, KnownRequests              (P4-T06)
    obs.rs                     ObsNamedList, ConnectionInfo, ObsServerInfo, RetirementReason
    connection/                ObsHost, ObsEndpoint, EndpointKey, ConnectionId, ObsPassword, CredentialStore
    program/                   scene, role, graph, policy (GraphDependencyPolicy::classify), deck, hotkey/{slots,config}, studio
    audio/                     gain (LinearGain, Decibels, FADER_RANGE, PRESET_RANGE), input (+InputPatch), meter, mixer (MixerPreference, MixerSession, filter/group, refresh_target), preset/{mod,run,library}, settings (AudioMonitoring, AudioTracks)
    outputs/                   output, output_result (RecordingStopTicket), command, policy (OutputAction, OutputSafety, confirmations), per_output
    workspace/                 operator_action (target(), stable_key()), operator_input, action_shortcut (try_new, matching_binding), source (ControlSnapshot, SourceItem, SourceDetail, ...), resolution (ResolutionContext, resolve), audio_rule, icon keys, request DTOs (OperatingRead/Observation, MixerAudioRequest, ThumbnailRequest)
    curation/                  ShowCuration, SceneCuration, DeckViews, OperatorSetup, AudioSetup, AccentColor, IconKey, SceneDeckView (ex domain/registry.rs snapshot types)
    diagnostics/               diagnostic, doctor, health/{mod,sustained,stats_alerts,output_alerts,audio_alerts}, preflight/{plan,report,review,checks}
    session/                   activity (ActivityLog), session_report (+ReportId, ReportEndReason, HealthAlertKind, SessionHistory), bookmark (BookmarkLog), stats (StatsHistory, HealthSeverity, BitrateSampler)
    appearance/                theme, language, motion, preference
    control.rs                 transitional ControlAction enum + From impls (until nested)
    test_support.rs            cfg(test) shared fixtures
  presentation/                GTK-free localized labels for domain enums; depends on domain + infra::i18n   (P4-T09)
    outputs.rs program.rs session.rs audio.rs hotkey.rs workspace.rs diagnostics.rs appearance.rs
  services/                    pure logic over domain values
    meter_service.rs hotkey_service.rs audio_debounce.rs graph_layout.rs
  storage/                     serde DTOs + file IO; converts to and from domain at its boundary
    error.rs                   StorageError (Missing, Busy, ...)
    json_store.rs              read_doc / replace_doc
    lock.rs                    inter-process <file>.lock
    atomic.rs small_file.rs xdg.rs secret.rs (KeyringCredentialStore)
    config.rs + config/{connections, preflight, migrations, file, presentation, dto}.rs
    registry.rs + registry/{dto/, scene_entry, curation, ordering, rules, json_file, yaml_export, import, decks, identity}.rs
    show_workspaces.rs + show_workspaces/{dto, legacy}.rs
    report_history.rs bookmarks.rs report_export.rs (+ CsvRow)
    fixtures/                  golden files (cfg(test))
    test_support.rs            TempDir (cfg(test))
  obs/                         the only obws importer; converts obws types at the boundary
    mod.rs error.rs (ObsError) identity.rs requests.rs (RequestGate, Admission) event.rs
    client.rs                  ObsClient struct, connect(&ObsEndpoint), lifecycle, request/mutation, get_version
    client/                    concurrency, outputs, collections, scenes, thumbnails, workspace_snapshot, source_detail (child modules: private fields visible)
    control/                   mod (dispatcher), studio, outputs, media, items, text, audio_controls
    mapper/                    mod, event, output, scene, media, audio, catalog, session, time
    audio.rs sources.rs operator_inputs.rs audio_presets.rs video_settings.rs scene_item_state.rs
    testing/                   cfg(test): wire, scripted (ScriptedObs), stateful (StatefulObs), payloads, client_ext
  controller/                  Tokio orchestration; no gtk/glib/ui; no state (after P8-T16)
    mod.rs app_controller.rs (router) dependencies.rs event_sender.rs observations.rs
    command/                   AppCommand families                          (P7-T14)
    event/                     AppEvent families, Publication, OutputEvent  (P7-T12/13)
    error.rs                   CommandError, SendRejected                   (P7-T05)
    runtime/                   budgets.rs, single_flight.rs, config.rs
    session/                   mod (ex session_controller), scope, identity, task_set, clocks, client_slot, journal, retirement, runner, invalidation, event_translation, event_ingress
    output/                    mod, port (OutputPort), pending, readback, shutdown
    refresh/                   snapshots, stats, manual, mixer_audio
    workspace.rs + workspace/  types, job (JobFence), read, audio, media, guards, audio_presets, lifecycle_tests
    thumbnails.rs meter_feed.rs
    test_support/              cfg(test): events, deps, state
  state/                       GTK-thread application state; no gtk/glib/ui; may use controller::{event, command} messages   (P8-T01)
    mod.rs                     AppState: slices + cross-slice transitions only
    session.rs                 ObsSessionState (+ cached scope, scope_rev)
    collection.rs active_audio.rs outputs.rs telemetry.rs mixer.rs(+tests) local_store.rs revisions.rs reducer.rs action_context.rs
    workspace/                 mod, fence, controls, inspection, readback, audit, reducer
    session_reports.rs + session_reports/{collect, collector, tests}.rs
    report_history.rs bookmarks.rs audio_presets.rs show_registry.rs (ShowWorkspaceSelection)
    persistence/snapshot.rs    DurableSnapshot<T, E> (pure)
    tests/                     fixtures + per-context tests
  ui/                          GTK only
    mod.rs navigation.rs (narrow port) scope.rs connection.rs output_flow.rs dialogs.rs markup.rs invalidation.rs responsive.rs registry_status.rs startup_notice.rs test_support.rs
    shell/                     window, split_view, lifecycle, header (HeaderChrome), header_selectors, sidebar, welcome, page_registry, close_guard, exit_policy, ticks/{workspace_tick, output_clock}, events/{mod, effects, session, output, scene_catalog, obs_scope, audio, workspace, topology, telemetry, feedback}
    presenters/                css classes, icon names, output_status, audio (format_db), diagnostics (doctor_format), connection
    persist/ (+persistence/coordinator.rs)  registry journal, writer, import, scope, sync, config
    background_io/             lanes, config_batch, ledger, flush, test_isolation
    theme/                     catalog, custom_css, providers, desktop, motion, tokens
    kit/                       labels, buttons, feature_window
    operator/ reports/ audio/ preflight/   feature groups (P9-T20)
    pages/                     live/, mixer/, inventory/, sources/, stats/, graph/, settings/, doctor.rs, topology.rs, help.rs
    widgets/                   audio_card/, chart/, cairo/, meter_hub.rs, volume_meter.rs, enum_combo.rs, status_page.rs, alert.rs, a11y.rs, ...
  architecture_tests/          mod, scanner, scanner_tests, rules            (P0-T04)
```

### 3.4 Dependency rules (allowed edges)

Edges use the `crate::` paths a module may import. Everything not listed is forbidden.

| From | May import | Notes |
|---|---|---|
| `infra` | `infra` only (external crates allowed) | leaf module; no `crate::domain` after P3-T08 (today `infra/i18n.rs:18` imports `domain::appearance::Language`, a domain <-> infra cycle) |
| `domain` | `domain`, `infra::{error, clock}` (temporarily `infra::i18n` until P4-T10) | no gtk/glib/obws/storage/services/controller/state/ui/presentation |
| `services` | `domain`, `infra` | no storage, no gtk |
| `presentation` | `domain`, `infra::i18n` | no gtk/glib/ui/controller/state/storage/obs |
| `storage` | `domain`, `infra` | no controller/obs/services/ui/state; no `LANGUAGE_LOADER` (after P5-T10) |
| `obs` | `domain`, `infra` | the only obws importer; no controller/storage/services/ui/state |
| `controller` | `domain`, `services`, `storage`, `obs`, `infra`, `presentation` | no gtk/glib/ui; no `state` (after P8-T16) |
| `state` | `domain`, `services`, `presentation`, `infra`, `controller::{event, command}`; `storage` only from `state/local_store.rs` | no gtk/glib/ui |
| `ui` | everything except `obs` and obws; `controller::AppController` only from `ui/shell/window.rs` and `ui/navigation.rs` | pages/widgets: no `nav.state` after P9-T12 |
| `app` | everything | composition root |

**Architecture test rules to add.** All use the existing `violations(scope, forbidden, exempt)` helper. P0-T04 extends it with exempt prefixes and a `ratchet` helper.

| Rule (exact test name) | Definition | Lands in |
|---|---|---|
| `obs_adapter_depends_only_on_domain_and_infra` | `violations("obs", [crate::controller, crate::ui, crate::storage, crate::services, crate::state], ..)` (KNOWN_DEBT exemption for obs/client.rs and obs/mapper.rs until P3-T01) | P0-T07, tightened P3-T01 |
| `storage_depends_only_on_domain_and_infra` | `violations("storage", [crate::controller, crate::obs, crate::services, crate::ui, crate::state])` | P0-T07 |
| `domain_is_innermost` | `violations("domain", [crate::controller, crate::ui, crate::obs, crate::storage, crate::services, crate::state, crate::presentation])` | P0-T07 (presentation part after P4-T09) |
| `services_depend_only_on_domain` | `violations("services", [crate::controller, crate::ui, crate::obs, crate::storage, crate::state])` | P0-T07 |
| `production_files_stay_within_budget` | ratchet on production lines per file (today's sizes as ceilings, 800 for new files) | P0-T07; ceilings lowered by every split |
| `app_state_borrows_ratchet` | per-file ceiling for `.state.borrow_mut()`, `.state.borrow()`, `nav.state` under `src/ui` | P0-T07; reaches 0 for pages/widgets in P9-T11 |
| `thread_local_ratchet` | per-file count of `thread_local!` under `src/ui` | P0-T07; lowered by P9-T16 |
| `domain_presentation_ratchet` / `domain_has_no_presentation` | count of `fl!(`/`LANGUAGE_LOADER` in `src/domain`, then ban plus `css_class` and `-symbolic` | P0-T07, P4-T11 |
| `presentation_is_gtk_free` | `violations("presentation", [gtk4, glib, adw, crate::ui, crate::controller, crate::state, crate::storage, crate::obs])` | P4-T09 |
| `nothing_below_presentation_uses_it` | domain, services, obs, storage must not import `crate::presentation` | P4-T09 |
| `background_layers_hold_no_gtk_or_ui` | extend the scope list with `state` | P8-T01 (same PR as the move) |
| `controller_does_not_reach_into_state` | `violations("controller", [crate::state, crate::ui])` | P8-T16 |
| `state_reaches_storage_only_through_local_store` | `violations("state", [crate::storage], exempt=[state/local_store.rs])` (check that sub-path scopes work) | P8-T09 |
| `no_clock_reads_in_state` | forbid `Instant::now`, `SystemTime::now`, `unix_ms(` under `src/state` | P3-T05 |
| `ui_storage_imports_ratchet` | ui -> storage import count, exempt `ui/background_io*` and `ui/persist*` | P5-T07 |
| `request_names_stay_in_obs` | no OBS request-name string literals outside `src/obs` and `domain/capabilities.rs` | P4-T06 |
| `pages_use_the_navigation_port` | no `nav.state` under `ui/pages` and `ui/widgets` | P9-T12 |
| `markup_safe_widgets` | no `adw::Toast::new` / `StatusPage::builder` outside `ui/markup.rs` and `ui/widgets/status_page.rs` | P1-T01 |
| `inventory_page_uses_native_list_rows` | replaces the include_str self-scan | P0-T03 |
| `infra_is_a_leaf` | `violations("infra", [crate::domain, crate::services, crate::storage, crate::obs, crate::controller, crate::state, crate::ui, crate::presentation])` | P3-T08 |

### 3.5 Value objects and newtypes to introduce

| Value object | Replaces | Where | Task |
|---|---|---|---|
| `SceneName`, `InputName`, `ObsUuid`, `ObsRef<N>`, `Resolution` | `SceneId`/`InputId` String aliases, `ActionTarget`, 4 copies of resolve | `domain/kernel/identity.rs` | P4-T02..P4-T05 |
| `ObsHost`, `ObsEndpoint`, `EndpointKey`, `ConnectionId` | host String, 5 `format!("{}:{}")`, 6 `"default"` literals | `domain/connection/` | P4-T22 |
| `ObsPassword` (redacting Debug) | `Option<String>` in AppState and secret APIs | `domain/connection/credential.rs` | P1-T13 |
| `ObsCapabilities`, `KnownRequests` | `available_requests: Vec<String>` + 5 helpers | `domain/capabilities.rs` | P4-T06 |
| `TransitionDuration`, `VolumeDb`, `AudioSyncOffset`, `AudioBalance`, `AudioTracks` | raw i64/f64/f32/[bool;6] in ControlAction, range checks in obs adapter | `domain/audio`, `domain/control` | P4-T07 |
| `LinearGain`, `Decibels` | f64 meaning either; AudioService forwarders | `domain/audio/gain.rs` | P4-T08 |
| `Generation`, `Revision` | 4 overflow policies on u64 counters | `domain/revision.rs` | P3-T06 |
| `Now` | hidden `Instant::now`/`unix_ms` in reducers | `infra/clock.rs` | P3-T05 |
| `OperatorLabel` | two divergent name rules | `domain/kernel/label.rs` | P4-T19 |
| `AccentColor`, `IconKey` | String accent/icon | `domain/curation`, `domain/icon` | P5-T09 |
| `HealthSeverity` | stats-page thresholds + chart Severity classification | `domain/session/stats.rs` | P4-T20 |
| `OutputAction`, `OutputSafety` | page-local enum + storage OutputConfig in page | `domain/outputs/policy.rs` | P4-T20 |
| `ReportEndReason`, `HealthAlertKind`, `ReportId` | localized strings persisted in reports | `domain/session/session_report.rs` | P4-T21, P8-T12 |
| `BookmarkSession`, `Observed<T>`, `InspectionToken`, `HealthKey` | anonymous tuples and a 0 sentinel | `src/state`, `domain/kernel` | P8-T13 |
| `OperationPhase`, `WorkspaceOutcome` | 4-bool evidence struct, bool soup in WorkspaceLoaded | `controller/session/journal.rs`, `controller/event` | P7-T04 |
| `ObsError`, `CommandError`, `StorageError`, `PortalError`, `CredentialError` | `Result<_, String>`, `Result<(), ()>`, stringly AppError | adapters and controller | P5-T01, P6-T05, P7-T05, P9-T18 |
| `MixerStructureKey`, `PresentationOwner` | Debug-format String signatures, duplicated owner tuples | `ui/pages/mixer`, `src/state` | P10-T08 |
| `UiEffects`, `PageSet` | interleaved reduce/render in apply_event | `ui/shell/events/effects.rs` | P9-T05 |
| `CloseState`, `ExitStep` | 4 boolean Cells in CloseGuard | `ui/shell/close_guard.rs`, `exit_policy.rs` | P9-T09 |

### 3.6 Aggregates and their boundaries

- **ShowCuration** (Show curation context). Root of scene curation, deck views, operator setup and audio setup for one show workspace. All mutations go through methods that keep its invariants:
  - rename;
  - set role, accent and icon;
  - move_scene, with the filter guard;
  - remove_retained, remove_stale_entry, stale_entries.

  `ShowWorkspaceSelection` (in `state/local_store.rs`) holds the active ShowCuration plus the revision and generation, and bumps them internally. Storage only maps it to and from `SceneRegistryDto`.
- **ObsSession** (Session). Status, endpoint, connection id, capabilities and the cached `ProductionScope` with `scope_rev`. It is rebuilt only by the session transitions (connect, reset, collection change, config endpoint change).
- **OutputsState** (Outputs). Two `OutputChannel`s plus `RecordingEvidence`. `apply(&OutputEvent) -> Option<OutputTransition>` keeps the ordering rule: path evidence first, then the Instant fence.
- **OperatingView** (ex WorkspaceState). Read fence, controls, inspection, readback and audit. `invalidate` carries over the activity log and keeps the revision monotonic.
- **ReportsState / SessionHistory / BookmarkLog** (Reporting). Reports collect through a read-only `ReportContext`. History and bookmarks are domain aggregates. Storage archives are DTOs.
- **PresetLibrary** (Audio). Owns presets and plans, and reuses the existing `valid_name` and `PRESET_LIMIT` rules.
- **SessionScope** (Tokio side). Identity, task set, clocks, client slot, journal and retirement. The composite is thin, and the lock order is documented.

Cross-aggregate transitions live only on `AppState` in `state/mod.rs`:

- `apply_connected`;
- `reset_obs_session`;
- `invalidate_collection_runtime`;
- `apply_terminal_evidence`;
- `note_command_intent`;
- `apply_input_patch`;
- `complete_workspace`.

### 3.7 Anti-corruption layers

**At `obs/`.**

- **Types.** No obws type crosses out of `obs/`, and every conversion happens in `obs/mapper/`.
- **Errors.** Errors cross as `ObsError`, mapped to `AppError` at the edge.
- **Identities.** Identities are parsed once in `obs/identity.rs`; nil uuids become `None`.
- **Capabilities.** Capabilities cross as `ObsCapabilities`.
- **Server info.** Server info crosses as `ObsServerInfo`. The obs layer never stamps placeholder connection ids or endpoints.
- **Wire facts.** Wire facts stay in the adapter: meter multipliers, OBS kind ids, request names and the non-audio message quirk.
- **No presentation.** No `fl!` and no `user_message` inside `obs/`.
- **Testing.** One scripted loopback server in `obs/testing/`. Controller tests use `StatefulObs`'s typed vocabulary, not raw `op` frames.

**At `storage/`.**

- **DTOs.** Each persisted file has a DTO in storage, with `TryFrom<Dto> for Domain` and `From<&Domain> for Dto`. Examples: `SceneRegistryDto`, the config DTO, `HistoryArchiveDto`, `BookmarkArchiveDto`, and DTOs for ActionBinding, OperatorPin, AudioRule, AudioPreset and ScopeBinding.
- **No file-format attributes in the domain.** No `deny_unknown_fields`, `try_from` or legacy `rename` on domain structs. Closed-set string enums are the documented exception.
- **Errors.** Storage returns `StorageError`, never `io::Error::other(String)`.
- **Golden fixtures.** P0-T05 fixtures pin every format.
- **Concurrency.** Writes go through one json_store helper under an inter-process lock.
- **Reads.** Reads have no side effects.

### 3.8 How AppState, AppCommand/AppEvent and event application split per context

**AppState** becomes slices:

```text
AppState {
  session: ObsSessionState,      // status, endpoint, capabilities, scope cache
  collection: CollectionState,   // inventory, graph, topology, profiles, collections
  active_audio: ActiveAudioState,
  mixer: MixerState,
  outputs: OutputsState,
  telemetry: TelemetryState,
  health: HealthMonitor,
  workspace: OperatingView,
  reports: ReportsState, history: ReportHistoryState, bookmarks: BookmarksState, presets: PresetState,
  local: LocalStore,             // config, ShowWorkspaceSelection(ShowCuration), startup notice
  page: Page,
  revisions: StateRevisions,
}
```

Slice fields are private. Slices expose intent methods and accessors. Child modules that read another slice get `pub(super)` accessors.

**AppCommand** becomes a router over families: `AppCommand { Session(..), Obs(..), Workspace(..), Audio(..), Output(..), Media(..) }` (P7-T14). `AppController::handle` keeps one line per family.

**AppEvent** gets families plus a single `Publication { generation, collection, revision, payload }` envelope that replaces the two boxed envelopes (P7-T13), and `AppEvent::Output(OutputEvent)`. Collection scoping stays a per-variant method (P7-T12). Deriving it from the family would change fencing.

**Event application** (today the 590-line `window.rs::apply_event`) becomes `ui/shell/events/`:

- `mod.rs` holds the exhaustive match and the Observation reconcile with its recursive re-dispatch, keeping today's order.
- Each context file (`session`, `output`, `scene_catalog`, `obs_scope`, `audio`, `workspace`, `topology`, `telemetry`, `feedback`) renders `UiEffects` returned by `AppState::apply(event, now)` in `state/reducer.rs`.
- The state transition is pure and unit-testable without GTK.
- Registry persistence and reconciliation leave the event path for `ui/persist/registry_sync.rs` (P9-T04).
- Change notification is `StateRevisions` diffed per batch into `UiInvalidation` (P8-T15, P9-T14). This replaces the polling timers.

---

## 4. God file decomposition catalogue

"Prod" is the production part, measured up to the first top-level `#[cfg(test)]` module, at `31d512f`. Several files interleave `#[cfg(test)]` items with production code; those are noted. Target sizes are rough estimates after the listed tasks. Every split keeps the existing `x.rs` + `x/` module layout where one exists (Rust 2018 style), instead of renaming to `mod.rs`.

### 4.1 `src/controller/state.rs`: 3088 lines (prod 1-1246, tests 1247-3088)

- **Responsibilities found:**
  - `Page` (34-96) with presentation;
  - `ObsStatus` (97-126) with label and css;
  - `TopologyStatus` (127-160);
  - `ActiveAudioStatus` (161), `update_active_since` (182);
  - `WorkspaceState` with 31 fields (192-436): fence, controls, inspection, screenshot, readback, audit;
  - `AppState` with 47 fields (437-499) and an impl at 501-1246, covering connection, collection/topology, outputs (4 parallel stream/record pairs, 8 mirrored setters), telemetry, active audio, input echo patching (4 copies), persistence caches, password, page and workspace;
  - cross-slice transitions: `apply_connected` 753, `reset_obs_session` 853, `accepts_session_publication` 882, `apply_terminal_evidence` 896, `workflow_scope` 788-796.
- **Targets:**

  | Target | Takes |
  |---|---|
  | `src/state/mod.rs` | AppState root and cross-slice transitions |
  | `session.rs` | ObsStatus, connection fields, workflow_scope cache |
  | `collection.rs` | topology, inventory, graph |
  | `active_audio.rs` | active audio |
  | `outputs.rs` | OutputChannel x2, RecordingEvidence, apply |
  | `telemetry.rs` | stats |
  | `workspace/{fence, controls, inspection, readback, audit, reducer}.rs` | WorkspaceState and the reducers from ui/event_reconciliation.rs |
  | `local_store.rs` | config, curation, startup notice |
  | `revisions.rs` | StateRevisions |
  | `reducer.rs` | `AppState::apply -> UiEffects` |
  | `tests/` | the test module |

  Presentation moves to `src/presentation` (labels) and `ui/presenters` (css/icon). Pure rules move to the domain: `SceneInventory::advance`, `ActiveClock`, `ControlAvailability`.
- **Order:**
  1. P2-T01: tests out.
  2. P3-T05, P3-T06, P3-T07.
  3. P8-T01: move to `src/state`.
  4. P8-T02 telemetry.
  5. P8-T03/T04 outputs.
  6. P8-T05 input patch.
  7. P8-T06 rules.
  8. P8-T07 collection and audio.
  9. P8-T08 session.
  10. P8-T09 local store.
  11. P8-T10/T11 workspace.
  12. P8-T15 revisions.
  13. P8-T16 lock.
- **Safety net:**
  - inline tests (49);
  - the P0-T12 golden AppEvent sequence;
  - `every_visible_workspace_mutation_advances_the_revision`;
  - output tests (ex 1358, 1478, 1550);
  - NV after P8-T10/T11.
- **Size:** mod.rs about 300 lines; each slice under 300.

### 4.2 `src/ui/window.rs`: 2698 lines (prod 1-1582, tests 1583-2698)

- **Responsibilities found:**
  - `build_main_window` 63-272: split view, teardown, action names;
  - `add_pages` 281, including the 200 ms registry/workspace poller 296-375;
  - `install_event_polling` 412-469, with collect_report called twice per event;
  - output clock 475-507;
  - header 513-610;
  - welcome dialog 611;
  - `apply_event` 664-1253 (46 arms, recursion at 706/718/755);
  - `apply_output_event` 1340;
  - status bar and refreshers;
  - `StreamingChrome` 1493;
  - `PageRefreshers` 1499;
  - `EventUiContext` 1515;
  - `apply_color_scheme` 1574;
  - a pure mixer predicate at 1378-1401;
  - 1100 lines of cross-context native tests.
- **Targets:**
  - `ui/shell/{window, split_view, lifecycle, header, header_selectors, sidebar, welcome, page_registry, ticks/{workspace_tick, output_clock}}.rs`;
  - `ui/shell/events/{mod, effects, session, output, scene_catalog, obs_scope, audio, workspace, topology, telemetry, feedback}.rs`;
  - `apply_color_scheme` to `ui/theme`;
  - the mixer predicate to `state/mixer.rs`;
  - tests to the matching `events/*` test files.
- **Order:**
  1. P2-T02, P2-T06.
  2. P9-T01.
  3. P9-T02, P9-T03.
  4. P9-T04, P9-T05.
  5. P9-T06, P9-T07, P9-T08.
  6. P10-T14 (color scheme).
- **Safety net:**
  - NV: window render and native tests, with paths made drift-proof by P0-T02;
  - P0-T12.
- **Size:** `events/mod.rs` under 150 lines; each context handler under 250; `shell/window.rs` about 80.

### 4.3 `src/ui/widgets/operating_panel.rs`: 2361 lines (prod 1-1613, tests 1614-2361)

- **Responsibilities found:**
  - the whole Sources page;
  - `LocalState` drafts (17-92);
  - `build` 94;
  - media polling 107-176, with two 1 s timers;
  - `SourcesContent` 317, `sources_content_from_state` 509;
  - `update_status` 548;
  - `populate` 678-1096 (about 400 lines);
  - `build_source_detail` 1098-1492 (about 400 lines);
  - numeric drafts 1494-1553;
  - helpers 1554-1613.
  - Widget-name string registry and tree visits are used as data binding, keyed by localized labels (locale bug risk).
- **Targets:** `ui/pages/sources/{mod, content, status, browser, drafts, focus, detail/{mod, filters, text_editor, media, media_progress, audio}}.rs` plus `ui/kit`. Delete the `pages/sources.rs` shim.
- **Order:** P0-T13, P2-T08, P10-T01, then P10-T02, P10-T03, P10-T04.
- **Safety net:**
  - P0-T13 section tests;
  - existing native tests (converted to `native_test` in P0-T14);
  - `widgets/lifetime_tests.rs`.
- **Size:** no file over 500 lines; browser and detail under 350 each.

### 4.4 `src/ui/pages/inventory.rs`: 2126 lines (prod 1-1505, tests 1506-2126)

- **Responsibilities found:**
  - presenter `read` 76;
  - `build` 131;
  - `update_rows` 213, with recursive widget-name lookups and 10 unwraps;
  - `populate` 330;
  - `preserve_breakpoint_focus` 462;
  - save-status 100 ms poller 528-572;
  - scene row 598;
  - role cell 697;
  - options popover 751;
  - accent 898;
  - drag reorder 960;
  - stale group 1019;
  - `reordered_scene_ids` 1152;
  - icon and accent setters 1330/1346;
  - YAML import and export 1384-1477;
  - curation rules inside closures.
  - The test at 1853 scans its own source.
- **Targets:**
  - `ui/pages/inventory/{mod, presentation, view, scene_row, scene_options, accent, reorder, stale, yaml_transfer, save_status}.rs`;
  - curation rules to `ShowCuration` (P5-T06);
  - `preserve_breakpoint_focus` to `ui/widgets/breakpoint_focus.rs` if shared.
- **Order:** P0-T03, P0-T13, P2-T08, P5-T06, P10-T05, P10-T06, P9-T19 (save status push).
- **Safety net:** P0-T13 section tests, plus `assert_native_list` (1887).
- **Size:** each file under 400 lines.

### 4.5 `src/ui/pages/mixer.rs`: 2003 lines (prod 38-1223, tests 1224-1787, layout tests 1788-2003)

- **Responsibilities found:**
  - `MixerBinding` tuple 31-45;
  - `MixerPageHandle` 50-62 (view handles plus signatures plus request tracker);
  - Debug-format structure key 173-221;
  - `build` 315;
  - `populate` 355-558, which holds a borrow across widget building with a manual `drop(state)`;
  - row builders 560-700;
  - duplicated group-by 757-792;
  - `filter_inputs` 894-913;
  - inspection 290-313 and 941-1049 (env var read on every call);
  - request dedupe 1116-1194.
- **Targets:**
  - `ui/pages/mixer/{mod, handle, structure_key, controls, body, rows, requests, inspection, view_model}.rs`;
  - `filter_inputs`/`group_inputs` to `domain/audio/mixer.rs`;
  - the request claim to `state/mixer.rs`.
- **Order:** P2-T08, P8-T14, P10-T07, then P10-T08, P10-T09.
- **Safety net:**
  - `native_mixer_echoes_preserve_cards_drafts_focus_and_popovers` (1802);
  - native test at 1923;
  - `mixer_structure_ignores_audio_echoes...` (1240-1259).
- **Size:** each file under 400 lines.

### 4.6 `src/controller/final_regression_tests.rs`: 1981 lines (all tests)

- **Responsibilities found:**
  - 35 tests mounted by `#[path]` in `app_controller.rs:460-462`;
  - a `Plan` flag-bag mock speaking raw OBS frames;
  - boolean-triple helper functions;
  - direct private-field access and replacement.
- **Targets:** each test group moves to its owning module's tests: workspace, obs requests, refresh, state, session terminal, output.
- **Order:** P0-T06, P2-T09 (seams and StatefulObs), P2-T10.
- **Safety net:** TC (35 tests).
- **Size:** the file is deleted.

### 4.7 `src/storage/registry.rs`: 1556 lines (prod about 692, tests 693-1556)

- **Responsibilities found:**
  - SceneRegistry with 12 fields over 5 contexts (23-72);
  - mutation impl blocks 167-476;
  - accent parse 505/518;
  - RuleConfig 530-585;
  - JSON/YAML IO 586-692.
  - Test-only production items at 151-165, 478-502, 622-637.
- **Targets:**
  - `storage/registry/{dto/, scene_entry, curation, ordering, rules, json_file, yaml_export, import}.rs` next to `decks.rs` and `identity.rs`;
  - behavior to `domain/curation` ShowCuration.
- **Order:** P0-T05, P2-T05, P1-T19, P5-T03, P5-T04, P5-T05, P5-T06, P5-T07, P5-T08, P5-T09.
- **Safety net:** golden fixtures (P0-T05) and the field-inventory test.
- **Size:** registry.rs under 200 lines; each submodule under 250.

### 4.8 `src/ui/pages/live.rs`: 1554 lines (prod 1-976, tests 977-1554)

- **Responsibilities found:**
  - `OutputAction` 27;
  - responsive ticks 137-152 and 270-284 (perpetual);
  - output start/stop and disconnect flows 310-604, owned by sidebar and transport bar;
  - policy functions 358/375/380/524/533;
  - slot projection 606-654;
  - `rebuild_scene_cards` 659-802, which clones the registry before its fast-path check;
  - `rebuild_audio_cards` 830-967.
- **Targets:**
  - `ui/pages/live/{mod, scene_deck, audio_strip, responsive}.rs`;
  - `ui/output_flow.rs` and `ui/dialogs.rs`;
  - policy to `domain/outputs/policy.rs`;
  - slots to `domain/program/deck.rs`.
- **Order:** P2-T08, P4-T20, P4-T23, P9-T07, P9-T19, P10-T10.
- **Safety net:** native `short_windows_collapse_audio_until_expanded`, plus the slot ordering tests (around 1307-1392; confirm the range).
- **Size:** each file under 400 lines.

### 4.9 `src/ui/theme.rs`: 1553 lines (prod 1-615, tests 616-1553)

- **Responsibilities found:**
  - 8 thread_locals (14-24);
  - catalog (42-96 and 529-614, four edits per theme);
  - ThemeManager 128-275;
  - custom CSS validator 279-307 (trust boundary);
  - desktop signals 320-398;
  - motion CSS 400-475;
  - providers 477-527.
- **Targets:** `ui/theme/{mod, catalog, custom_css, providers, desktop, motion, tokens}.rs` plus `theme/{contrast_tests, stylesheet_tests, native_tests}.rs`.
- **Order:** P2-T07, P1-T18, P10-T20, P10-T21.
- **Safety net:** theme tests, with native child-test names renamed.
- **Size:** each file under 250 lines.

### 4.10 `src/controller/state/mixer.rs`: 1396 lines (prod 1-452, tests 453-1396)

- **Responsibilities found:**
  - request fencing types (6-199);
  - Middle Man forwarders (201-220);
  - the UI view model `MixerVisible*`/`MixerInspection*` (34-103, 411-451);
  - refresh-target rule (319-365);
  - input updates (373-408);
  - `MixerAudioRequest` (108-119), which is imported by the controller in production.
- **Targets:**
  - `state/mixer.rs` (fencing and MixerState);
  - `domain/audio/mixer.rs` (refresh target);
  - `ui/pages/mixer/view_model.rs`;
  - `domain/workspace` (MixerAudioRequest, in P3-T03).
- **Order:** P2-T02, P3-T03, P8-T14.
- **Safety net:** `cargo test --all-features mixer` with an unchanged count, then NV.
- **Size:** under 300 lines.

### 4.11 `src/obs/client.rs`: 1395 lines (prod 1-316 + 517-1097; tests 317-515, 1099-1392, `#[path]` 1393-1395)

- **Responsibilities found:**
  - transport handle, two gates, session metadata and observation counter (52-60);
  - connect with the "default" placeholder (69-106);
  - reads for outputs, collections and scenes;
  - control snapshot (7 sequential reads, 517-575);
  - thumbnails (577-630);
  - source detail (632-761, up to 9 sequential reads);
  - the 24-arm dispatcher (781-1074);
  - helpers 1077-1097.
- **Targets:**
  - `obs/client.rs` (struct, connect, lifecycle, request/mutation, get_version);
  - child modules `obs/client/{concurrency, outputs, collections, scenes, thumbnails, workspace_snapshot, source_detail}.rs` (child modules keep private-field access);
  - `obs/control/{mod, studio, outputs, media, items, text, audio_controls}.rs`;
  - `obs/mapper/*`;
  - `obs/identity.rs`, `obs/error.rs`.
- **Order:** P0-T06, P2-T04, P6-T01, P6-T02, P6-T03, P6-T04, P6-T05, P6-T07, P6-T08, P6-T09.
- **Safety net:**
  - ScriptedObs wire tests for every request (including the ones never scripted before: StopStream, Start/StopRecord, SetStudioModeEnabled, transitions, replay, vcam, audio advanced setters and getters, SetCurrentProfile/SceneCollection);
  - `cargo test --all-features obs:: -- --list | wc -l` unchanged after each move.
- **Size:** client.rs about 200 lines; each child module under 250.

### 4.12 `src/app.rs`: 1345 lines (prod 1-456, tests 457-1345)

- **Responsibilities found:**
  - `StartupData` 23;
  - `run` 32-159, which joins startup I/O and a synchronous keyring read before the first frame;
  - shutdown nested MainLoop 97-115;
  - document windows 162/166;
  - CloseGuard 170-278 (four bool Cells);
  - `install_persistence_close_guard` 281-340;
  - `show_persistence_close_failure` 342-428 (borrowed i18n keys);
  - `build_ui` 431-455.
- **Targets:** `app/{mod, startup, runtime, activation}.rs` and `ui/shell/{close_guard, exit_policy}.rs`.
- **Order:** P1-T04, P1-T15, P2-T07, P9-T09.
- **Safety net:**
  - the close/persistence native tests (around 574-1305);
  - new ExitPolicy table tests.
- **Size:** app/mod.rs about 60 lines; close_guard under 250.

### 4.13 `src/controller/app_controller.rs`: 1279 lines (prod 1-458, `#[path]` 460-462, tests 464-1279)

- **Responsibilities found:**
  - command routing;
  - inline dB math (195-205);
  - three different cancellation sequences;
  - refresh queue (219-284);
  - mixer audio single-flight (293-358);
  - client slot reads;
  - config loading with Default on JoinError (449-458).
  - Test-only items at 8-33 and 419.
- **Targets:**
  - `app_controller.rs` (router);
  - `refresh/{manual, mixer_audio}.rs`;
  - `runtime/{single_flight, config, budgets}.rs`;
  - `command/`;
  - `app_controller/tests/`.
- **Order:** P2-T03, P1-T02, P1-T05, P1-T06, P7-T14, P7-T16.
- **Safety net:** app_controller tests (they drive `handle`, so they stay AppController integration tests).
- **Size:** under 250 lines.

### 4.14 `src/ui/pages/stats.rs`: 1160 lines (prod 1-912, tests 913-1160)

- **Responsibilities found:**
  - thresholds 34-62;
  - StatsCards 66;
  - build 84;
  - popout lifecycle 172-286;
  - gauges 352-470;
  - charts 471-585;
  - cards 589-723;
  - activity 724-844 (duplicated marker code);
  - severity classifiers 848-912.
- **Targets:**
  - `ui/pages/stats/{mod, popout, gauges, charts, cards, activity}.rs`;
  - thresholds and classifiers to `domain/session/stats.rs` (HealthSeverity);
  - `fps_target` (853) stays in ui.
- **Order:** P2-T08, P4-T20, P10-T11.
- **Safety net:** stats tests (1074-1157 move with the classifiers).
- **Size:** each file under 300 lines.

### 4.15 `src/ui/action_catalog.rs`: 1155 lines (prod 1-666, tests 667-1155)

- **Responsibilities found:**
  - input helpers 35-65 and resolution 148-288 (domain policy using OBS request strings);
  - `observed_label` 66-147;
  - entry and label builders 289-618 (labels written twice);
  - execute 619-666, with `unreachable!()`.
- **Targets:**
  - `domain/workspace/resolution.rs` plus `state/action_context.rs` (ResolutionContext);
  - `ui/action_catalog/{mod, labels, execute}.rs`;
  - later `ui/operator/catalog`.
- **Order:** P2-T08, P4-T06, P4-T24, P4-T25, P9-T20.
- **Safety net:** tests 795-1137, ported to the domain.
- **Size:** each file under 300 lines.

### 4.16 `src/ui/widgets/audio_card.rs`: 1123 lines (prod 1-884, tests 885-1123)

- **Responsibilities found:**
  - constants 23-38;
  - AudioCardHandle 42;
  - AudioTargetScope 70-99 (workflow_scope allocation per fader event);
  - VolumeChangeContext 101;
  - parts 168-350;
  - VolumeControls 351-454;
  - build and build_compact 455-616.
  - Direct AppState write for the lock toggle (312-319).
  - A `#[cfg(test)]` hook in production (863-864).
- **Targets:** `ui/widgets/audio_card/{mod, scope, volume, parts}.rs`.
- **Order:** P2-T08, P8-T08 (cached scope), P10-T17.
- **Safety net:**
  - `audio_callbacks_reject_another_session_collection_and_endpoint` (1079);
  - native tests.
- **Size:** each file under 350 lines.

### 4.17 `src/ui/pages/settings.rs`: 1057 lines (prod 1-873, tests 874-1057)

- **Responsibilities found:**
  - an upward import of `ui::window` (25);
  - the startup notice is consumed during build (48);
  - 7 copy-pasted enum ComboRows;
  - ThemeMode index tables (109-113, 126-130);
  - a dead `build_obs_group` (448-450);
  - PasswordWrites (456-497);
  - `obs_status_text` re-borrows (731-761).
- **Targets:**
  - `ui/pages/settings/{mod, appearance, language, outputs, hotkeys, rows, status}.rs` (next to the existing `settings/` directory);
  - `ui/widgets/enum_combo.rs`;
  - `settings/connections/password_writes.rs`.
- **Order:** P2-T08, P9-T11, P10-T14, P10-T15.
- **Safety net:** settings tests and native tests.
- **Size:** each file under 300 lines.

### 4.18 `src/ui/persist.rs`: 1033 lines (prod 1-590, tests 591-1033)

- **Responsibilities found:**
  - registry write journal 23-205 (thread_local 42);
  - import transaction gate 207-271 (thread_local 207);
  - `registry_availability_message` 273;
  - scope selection with the legacy migration rule 289-354 (expect at 317);
  - config saves 457-589.
  - Test-only items at 122 and 406.
- **Targets:**
  - `ui/persist/{mod, config, registry_journal, registry_writer, registry_import, registry_scope, registry_sync}.rs`;
  - `ui/registry_status.rs`;
  - the pure `RegistryJournal` and `DurableSnapshot` core in `state/persistence/`;
  - the legacy rule to `storage/show_workspaces`.
- **Order:** P2-T05, P2-T07, P5-T12, P9-T04, P9-T16.
- **Safety net:**
  - new pure journal tests;
  - existing `isolated_test_child` tests (rename the six embedded paths).
- **Size:** each file under 250 lines.

### 4.19 Other large files (800-1000 lines) touched by the plan

| File | Lines | Plan |
|---|---|---|
| `src/ui/pages/graph.rs` | 968 (tests 788) | P10-T12, P10-T13: topology presenter, shared Cairo, Pango canvas, graph_layout service |
| `src/controller/workspace.rs` | 924 (tests 565/577) | P7-T10: types, job (JobFence), read, audio, media, guards; facade stays |
| `src/ui/background_io.rs` | 894 (tests 378) | P9-T17: lanes, config_batch, ledger, flush, test_isolation |
| `src/controller/output_controller.rs` | 812 (first cfg(test) at 389 is the Fake trait; production continues to 561) | P7-T06, P7-T07: port, pending, readback, shutdown |
| `src/ui/sidebar.rs` | 800 (tests 548) | P9-T07: shell/sidebar, shell/header_selectors, ui/connection |
| `src/ui/action_shortcuts.rs` | 798 (tests 563) | P10-T19: domain matching rules plus chord/manager/editor |
| `src/controller/session_controller.rs` | 792 (tests 318) | P7-T02, P7-T03, P7-T07: session/mod, runner, budgets; teardown takes a shutdown plan |
| `src/storage/config.rs` | 771 (tests 409) | P1-T08, P1-T09, P5-T10: migrations, file, presentation (notice) to ui, dto |
| `src/ui/audio_presets.rs` | 713 | P10-T19: PresetLibrary in domain, editor/view split |
| `src/domain/health.rs` (+ events 274, audio 473) | 576 | P4-T17 |
| `src/domain/preflight.rs` | 504 | P4-T16 |
| `src/controller/lifecycle.rs` | 504 | P0-T10, P7-T02, P7-T03: session/{scope, identity, task_set, clocks, client_slot, journal, retirement} |
| `src/architecture_tests.rs` | 681 | P0-T04: scanner, scanner_tests, rules |
| `src/controller/refresh_controller.rs` | 610 | P7-T08, P7-T09: refresh/{snapshots, stats}, session/{invalidation, event_translation, event_ingress} |
| `src/ui/report_history.rs` | 541 | P9-T16: pure history decisions to state; window to ui/reports |
| `src/controller/state/session_reports.rs` | 412 | P8-T12: collect.rs, collector.rs, ReportContext |
| `src/domain/control.rs` | 331 | P4-T14: data structs per context, nested ControlAction |
| `src/obs/mapper.rs` | 561 | P6-T04: mapper/ directory |
| `src/obs/regression_tests.rs` | 706 | P2-T04: tests next to each capability; file deleted |

---

## 5. Phased plan

Tasks are numbered `P<phase>-T<nn>`. Pick any task whose `depends_on` tasks are merged. Inside a phase, tasks marked as parallelizable can run in separate sessions at the same time. Cross-phase dependencies are explicit; for example, P7-T13 waits for P9-T02, and P4-T18 waits for P1-T10.

### 5.1 Phase overview

| Phase | Title | Tasks | Goal |
|---|---|---|---|
| P0 | Urgent fix and safety nets | 15 | No structural change. Every later move depends on these nets. Do P0-T01 first: it is time-critical. |
| P1 | Correctness, security and data-safety fixes | 25 | Small `fix:` commits that change behavior on purpose. They come before the restructuring so the restructuring commits stay pure moves. |
| P2 | Pull inline tests out of god files (pure moves) | 11 | Mechanical: `x.rs` plus `x/tests.rs`. The test count must stay the same (TC). After this, each god file shows its real production size. |
| P3 | Boundary prerequisites (break the inverted edges) | 8 | Small moves that remove the edges blocking the big splits: obs->controller, ui/state->controller clock, command/event->state, infra->domain. |
| P4 | Domain kernel, value objects and bounded contexts | 25 | Builds the ubiquitous language in code. Each context move uses `pub use` shims so callers keep compiling. |
| P5 | Storage anti-corruption layer | 14 | Storage keeps serde DTOs. The domain gets real aggregates. Golden fixtures from P0-T05 guard every step. |
| P6 | OBS anti-corruption layer | 11 | Split obs/client.rs by request category, type the errors and identities, and finish the mapper. |
| P7 | Controller orchestration (Tokio side) | 16 | Split SessionScope, type the errors, remove the test seam from production and split the long async routines. |
| P8 | Application state decomposition (AppState slices) | 16 | Move AppState out of controller/ and break it into context slices with private fields and intent methods. |
| P9 | UI shell, navigation port and persistence services | 21 | Split window.rs and app.rs, narrow NavigationContext, replace pollers and thread_locals. |
| P10 | UI pages and widgets | 24 | Split each page and widget god file along the view/presenter/rules seams. |
| P11 | Documentation sync and deferred items | 3 | Close the loop: docs, ADRs, and items deliberately left to the end. |
| | **Total** | **189** | |

### 5.2 Critical path and parallel tracks

```text
P0-T01 (deadline 2026-11-20)
P0 nets: T02 native names -> T03/T13/T14 ; T04 -> T07 ratchets ; T05 goldens ; T06 ScriptedObs -> T11 ; T08 borrow helpers ; T09 -> T10 poison
   |
P1 fixes (independent of each other; most need only P0-T05/T06/T09)
   |
P2 test extraction (one file per PR; parallel)      P3 prerequisites: T01 ConnectionInfo, T02 clock, T03 request DTOs, T05 Now, T06 revisions, T08 infra leaf
   |                                                   |
P4 domain kernel + contexts (T02 identity -> T03/T04/T05; T09 -> T10 -> T11 presentation; T12 -> T13 context dirs -> T14..T19, T23)
   |                     \
P5 storage ACL            P6 OBS ACL (T01 -> T02 -> T03/T04/T05; T09 -> T10)
   |                     /
P7 controller (T01 -> T02 -> T03 ; T06 -> T07 -> T08 -> T09 ; T12 -> T13 after P9-T02)
   |
P8 state: T01 move (serialize: no other open PRs touching state) -> T02..T15 slices -> T16 lock
   |
P9 shell: T02 -> T03 -> T04/T05/T06 -> T07/T08 ; T10 -> T11 -> T12 ; T13 ; T14 -> T15 ; T16 -> T17 ; T19 anytime after P0-T15
   |
P10 pages (each page track independent) -> P11 docs
```

Parallel tracks that rarely conflict:

- **(A)** storage: P1-T08/T09/T17..T23, then P5.
- **(B)** obs: P0-T06, then P2-T04, then P6.
- **(C)** domain: P4.
- **(D)** UI fixes: P1-T01/T03/T07/T24, then P9-T19, then P10.

Serialize: P8-T01 (wide import churn), P2-T06 (window tests), P9-T02/T03 (window router).

### 5.3 Checkpoints

After each phase, a human checks:

- All four CI commands, NV and IV are green on main.
- The ratchet ceilings went down (or stayed the same for P0/P1).
- `docs/architecture.md` and `docs/codebase-overview.md` match the tree.
- A manual smoke run with OBS (docs/manual-test-plan.md core flows): connect, switch scene, mixer fader, start and stop recording, close with a failing save.

Phase-specific checks:

- P0: the new rules fail when debt is added (try one, then revert).
- P1: each fix has a test that fails without it.
- P2 and P3: TC numbers are equal across the phase.
- P8: the P0-T12 golden sequence is unchanged.
- P9 and P10: measure the section 6.2 performance numbers and record them in the PR.

### Phase 0 - Urgent fix and safety nets

No structural change. Every later move depends on these nets. Do P0-T01 first: it is time-critical.

| ID | Title | Size | Depends on |
|---|---|---|---|
| [P0-T01](#p0-t01) | Resolve or renew the RUSTSEC-2026-0374 advisory exception before 2026-11-20 | S | - |
| [P0-T02](#p0-t02) | Make native GTK test names drift-proof and give test-native.py a filter | M | - |
| [P0-T03](#p0-t03) | Replace the inventory source-text self-scan test | S | - |
| [P0-T04](#p0-t04) | Split architecture_tests.rs into scanner and rules; add a typed Forbidden matcher | M | - |
| [P0-T05](#p0-t05) | Golden wire-format fixtures for every persisted file | M | - |
| [P0-T06](#p0-t06) | Promote obs_fixture into a scripted loopback OBS (ScriptedObs) that matches by requestType and counts requests | M | - |
| [P0-T07](#p0-t07) | Add ratchet fitness functions that pass today | M | P0-T04 |
| [P0-T08](#p0-t08) | Borrow-discipline helpers on NavigationContext (no migration yet) | S | - |
| [P0-T09](#p0-t09) | One poison policy for std Mutex: lock_or_recover helper and the silent-drop sites | S | - |
| [P0-T10](#p0-t10) | Decide the poison policy for SessionScope locks per mutex, with a test | S | P0-T09 |
| [P0-T11](#p0-t11) | Characterization tests for run_event_loop: one per ObsEvent variant | M | P0-T06 |
| [P0-T12](#p0-t12) | Golden AppEvent-sequence test for AppState reducers | M | - |
| [P0-T13](#p0-t13) | Section characterization tests for operating_panel and inventory before their splits | M | P0-T02 |
| [P0-T14](#p0-t14) | Shared test support modules (controller, ui, storage) | M | P0-T02 |
| [P0-T15](#p0-t15) | Opt-in performance instrumentation (perf-trace feature) | S | - |

#### P0-T01

**Resolve or renew the RUSTSEC-2026-0374 advisory exception before 2026-11-20**

- **Why:** `security-12` (medium)
- **Commit type:** `build:` | **Size:** S | **Risk:** High if missed: every CI run fails after 2026-11-20
- **Depends on:** none | **Parallelizable with:** everything
- **Files:**
  - `deny.toml`
  - `tools/check-advisory-exceptions.py`
  - `tools/test_advisory_exceptions.py`
  - `docs/advisory-exceptions.md`
  - `Cargo.lock`
- **Steps:**
  1. Check crates.io for an i18n-config release that drops basic-toml 0.1.10 (cargo-i18n PR #176).
  2. If one exists: `cargo update -p i18n-config`, delete the ignore entry in deny.toml:6-8, and remove the exception record and its checker constants (EXPIRES, REVIEWED, PARENTS in tools/check-advisory-exceptions.py) together with their test in tools/test_advisory_exceptions.py.
  3. If none exists: renew the expiry with the same reachability note. Update deny.toml reason text, the `EXPIRES` constant and the literal `2026-11-20` checks in tools/check-advisory-exceptions.py:58-71, docs/advisory-exceptions.md, and tools/test_advisory_exceptions.py. Do not widen the ignore.
  4. Commit as `build:`.
- **Acceptance criteria:**
  - [ ] `cargo deny check advisories bans licenses sources` passes.
  - [ ] `python3 tools/check-advisory-exceptions.py` and `python3 -m unittest tools/test_advisory_exceptions.py` pass.
  - [ ] CI green on a date after 2026-11-20 (simulate by checking the new expiry).
- **Verification:** SV + `cargo deny check advisories bans licenses sources` + `cargo audit` (see 2.1)

#### P0-T02

**Make native GTK test names drift-proof and give test-native.py a filter**

- **Why:** `ui-shell-01` (high), `tests-structure-13` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Medium: touches every native test call site
- **Depends on:** none | **Parallelizable with:** P0-T03, P0-T05, P0-T06
- **Files:**
  - `src/ui/mod.rs (test_support::native_test, 258-299)`
  - `tools/test-native.py`
  - `every caller of native_test (ui/window.rs, app.rs, ui/sidebar.rs, ui/navigation.rs, ui/widgets/*, ui/pages/*)`
- **Steps:**
  1. Today native_test compares SCENEDECK_NATIVE_TEST_CHILD with a string literal and re-runs `--exact <literal>`. If a test moves and the literal is stale, the child runs zero tests and the outer test passes: a vacuous green.
  2. Change native_test to take `module_path!()` plus the fn name (a `native_test!(fn_name, body)` macro using `concat!(module_path!(), "::", stringify!(fn_name))`).
  3. In the child branch, assert that the child's libtest output reports exactly `1 passed`. Fail loudly on `0 passed`.
  4. Rename `mixer_page_minimum_height_does_not_grow_with_the_card_count` to `native_mixer_page_minimum_height_...` so every native test has the `native_` prefix (TS13).
  5. In tools/test-native.py classify ignored tests by a `native_` fn-name prefix instead of the module prefix tuple at line 19, and add `--filter <substring>`.
  6. Migrate every caller in one commit (mechanical).
- **Acceptance criteria:**
  - [ ] Renaming a native test fn without touching anything else still runs it (path computed at compile time).
  - [ ] A deliberately wrong path makes the outer test fail.
  - [ ] `python3 tools/test-native.py --filter mixer` runs only mixer native tests.
- **Verification:** SV + NV (see 2.1)

#### P0-T03

**Replace the inventory source-text self-scan test**

- **Why:** `tests-structure-12` (medium), `ui-pages-25` (low)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** P0-T02, P0-T05
- **Files:**
  - `src/ui/pages/inventory.rs 1851-1870`
  - `src/architecture_tests.rs`
- **Steps:**
  1. The test at inventory.rs:1853-1854 does `include_str!("inventory.rs")` and splits on `#[cfg(test)]`. It breaks on any split.
  2. Delete it. Keep the positive assertion through the existing native widget-tree check (assert_native_list, inventory.rs:1887).
  3. Express the negative part (no forbidden list widget) as a scanner rule in architecture_tests scoped to `ui/pages/inventory` (works for both `inventory.rs` and a future `inventory/` dir).
- **Acceptance criteria:**
  - [ ] No `include_str!` of a .rs file remains under src/ui/pages.
  - [ ] `git mv inventory.rs inventory/mod.rs` would compile and pass (try it locally, then revert).
- **Verification:** SV + NV (see 2.1)

#### P0-T04

**Split architecture_tests.rs into scanner and rules; add a typed Forbidden matcher**

- **Why:** `tests-structure-06` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** P0-T02, P0-T03, P0-T05
- **Files:**
  - `src/architecture_tests.rs -> src/architecture_tests/{mod.rs, scanner.rs (18-366), scanner_tests.rs (367-547), rules.rs (548-681)}`
- **Steps:**
  1. Pure move first (one commit).
  2. Then replace the special case at line 213 (`*forbidden == "AppController" && ...`) with `enum Forbidden { PathPrefix(&str), AnySegment(&str) }`.
  3. Replace the obws rule's output filter (`!hit.starts_with("  obs/")`, 566-575) with a real exempt-prefix parameter: `violations(scope, forbidden, exempt_prefixes)`.
  4. Add a `ratchet(scope, pattern, ceilings: &[(&str, usize)])` helper that counts occurrences per file and fails when a count rises above its ceiling or a new file appears. P0-T07 uses it.
- **Acceptance criteria:**
  - [ ] Same 7 rule tests plus scanner self-tests pass.
  - [ ] Scanner test count unchanged (TC).
- **Verification:** SV + TC (see 2.1)

#### P0-T05

**Golden wire-format fixtures for every persisted file**

- **Why:** `storage-infra-14` (medium)
- **Commit type:** `test:` | **Size:** M | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** P0-T02, P0-T03, P0-T04, P0-T06
- **Files:**
  - `src/storage/fixtures/ (new)`
  - `src/storage/config.rs tests`
  - `src/storage/registry.rs tests`
  - `src/storage/show_workspaces.rs tests`
  - `src/storage/report_history.rs tests`
  - `src/storage/bookmarks.rs tests`
  - `src/storage/report_export.rs tests`
- **Steps:**
  1. Add fixtures via include_str!: config.json (current version, plus a legacy file with `custom_css.path` and `theme_mode`), registry.json, show-workspaces.json, registry YAML export, a session report JSON, bookmarks.json.
  2. Tests: parse fixture -> serialize -> compare as serde_json::Value (key order free).
  3. YAML: HashMap fields (scenes, scene_uuids, inputs) serialize in random order. Keep at most one entry per HashMap in the YAML fixture, or compare parsed values.
  4. Add a field-inventory test that enumerates every Vec/Map field of SceneRegistry and fails when a field is added without a fixture update (security-19 phase 0 import-policy test).
- **Acceptance criteria:**
  - [ ] Every storage format has a round-trip golden test.
  - [ ] Renaming any serde field in domain/ or storage/ makes a golden test fail.
- **Verification:** SV (see 2.1)

#### P0-T06

**Promote obs_fixture into a scripted loopback OBS (ScriptedObs) that matches by requestType and counts requests**

- **Why:** `obs-acl-15` (medium), `performance-21` (low), `architecture-global-14` (medium), `tests-structure-01` (high)
- **Commit type:** `test:` | **Size:** M | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** P0-T02..P0-T05
- **Files:**
  - `src/obs/obs_fixture.rs -> src/obs/testing/{mod.rs, wire.rs, scripted.rs}`
  - `src/obs/mod.rs`
  - `src/obs/client.rs control_tests (1107-1145 mock_client)`
  - `src/obs/regression_tests.rs:4`
  - `src/controller/workspace/audio_presets/tests.rs:2`
- **Steps:**
  1. Declare `#[cfg(test)] pub(crate) mod testing;` in obs/mod.rs; move obs_fixture.rs to testing/wire.rs (listen/accept/version) and add `accept_with_identify` (the connect test asserts on the Identify frame).
  2. Add ScriptedObs: responders keyed by requestType, typed `Reply::Ok(Value) | Reply::Err{code, comment}` (replace the `__error/__code/__comment` keys), an `Arc<Mutex<HashMap<String, usize>>>` request counter, and an `ordered()` mode that keeps today's strict order for existing tests.
  3. Move mock_client and unresponsive_client (regression_tests.rs 381-408) into scripted.rs. Keep `pub(crate) use` shims at `obs::client::control_tests::mock_client` so both external importers compile; remove the shims in P2-T04.
  4. Add helpers response(), event(), request_data().
- **Acceptance criteria:**
  - [ ] All obs and controller tests pass unchanged (TC).
  - [ ] A new test can assert `counts["GetVideoSettings"] == 1`.
- **Verification:** SV + TC (see 2.1)

#### P0-T07

**Add ratchet fitness functions that pass today**

- **Why:** `architecture-global-11` (medium), `tests-structure-05` (high), `architecture-global-12` (medium)
- **Commit type:** `test:` | **Size:** M | **Risk:** Low
- **Depends on:** P0-T04 | **Parallelizable with:** P0-T05, P0-T06
- **Files:**
  - `src/architecture_tests/rules.rs`
- **Steps:**
  1. Using P0-T04's helpers add (names are exact):
  2. `obs_adapter_depends_only_on_domain_and_infra`: violations("obs", [crate::controller, crate::ui, crate::storage, crate::services], exempt=[obs/client.rs, obs/mapper.rs] as KNOWN_DEBT for ConnectionInfo until P3-T01).
  3. `storage_depends_only_on_domain_and_infra`: violations("storage", [crate::controller, crate::obs, crate::services, crate::ui]) - passes today (the bookmarks.rs:89 edge is test-only).
  4. `domain_is_innermost`: violations("domain", [crate::controller, crate::ui, crate::obs, crate::storage, crate::services]).
  5. `services_depend_only_on_domain`: violations("services", [crate::controller, crate::ui, crate::obs, crate::storage]).
  6. `production_files_stay_within_budget`: ratchet on production lines (up to the first top-level `#[cfg(test)] mod`) with ceilings = today's sizes, and a hard cap of 800 for files not in the list. Treat interleaved cfg(test) items carefully (registry.rs 153/479/490/622/631, obs/client.rs 317/1099).
  7. `app_state_borrows_ratchet`: per-file ceilings for `.state.borrow_mut()` / `.state.borrow()` / `nav.state` under src/ui (today: window 66, operating_panel 46, live 29, inventory 28, mixer 26, report_history 21, audio_card 20, persist 20 borrow_mut).
  8. `thread_local_ratchet` for src/ui (persist.rs 42/207, background_io 154/245, session_reports 22, report_history 21, bookmarks 21, theme 15).
  9. `domain_presentation_ratchet`: count `fl!(` (141) and `LANGUAGE_LOADER` (12 files) under src/domain; ceiling only goes down.
- **Acceptance criteria:**
  - [ ] All new rules pass on main.
  - [ ] Adding one `nav.state.borrow_mut()` to any ui file fails the build.
- **Verification:** SV (see 2.1)

#### P0-T08

**Borrow-discipline helpers on NavigationContext (no migration yet)**

- **Why:** `controller-state-21` (medium), `architecture-global-02` (high), `ui-shell-05` (high)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** P0-T05..P0-T07
- **Files:**
  - `src/ui/navigation.rs (19-32, 136-211)`
- **Steps:**
  1. Add `fn read<R>(&self, f: impl FnOnce(&AppState) -> R) -> R` and `#[track_caller] fn update<R>(&self, f: impl FnOnce(&mut AppState) -> R) -> Result<R, ReentrantUpdate>`.
  2. update uses try_borrow_mut. On BorrowMutError: tracing::error! with the caller location, debug_assert!(false), and return Err. Never drop the write silently (that would be fake success).
  3. Do not migrate callers here. Document the rule: never call dispatch, persist or another page refresher inside a read/update closure.
  4. clippy disallowed_methods cannot target RefCell<AppState> only; rely on the P0-T07 ratchet instead.
- **Acceptance criteria:**
  - [ ] Unit test: nested update returns Err and logs; read inside read works.
  - [ ] No behavior change for existing callers.
- **Verification:** SV (see 2.1)

#### P0-T09

**One poison policy for std Mutex: lock_or_recover helper and the silent-drop sites**

- **Why:** `ctrl-02` (low), `ui-pages-18` (medium), `uiw-08` (medium)
- **Commit type:** `fix:` | **Size:** S | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** P0-T05..P0-T08
- **Files:**
  - `src/infra/sync.rs (new)`
  - `src/controller/output_controller.rs 173-196, 418-439`
  - `src/controller/session_controller.rs 239-243`
  - `src/controller/app_controller.rs 395-397`
  - `src/controller/refresh_controller.rs 170`
  - `src/controller/workspace.rs 134, 198, 253`
  - `src/controller/workspace/audio_presets.rs 14`
  - `src/controller/thumbnails.rs 49`
  - `src/ui/pages/settings.rs 456-497 (PasswordWrites)`
  - `src/ui/background_io.rs 199, 221`
- **Steps:**
  1. Add `pub(crate) fn lock_or_recover<T>(m: &Mutex<T>) -> MutexGuard<T>`: `m.lock().unwrap_or_else(PoisonError::into_inner)`, plus one tracing::error! on recovery.
  2. Replace `.lock().ok()?` and `if let Ok(..) = m.lock()` in the listed controller sites. Today a poisoned lock silently turns into 'command ignored' or 'disconnected'.
  3. Replace the GTK-thread `.lock().expect` in PasswordWrites and background_io config batch (199, 221).
  4. Do NOT touch controller/lifecycle.rs here (P0-T10 decides that per mutex).
- **Acceptance criteria:**
  - [ ] Unit test of lock_or_recover with a poisoned mutex.
  - [ ] grep `lock().ok()` under src/controller returns 0.
- **Verification:** SV (see 2.1)

#### P0-T10

**Decide the poison policy for SessionScope locks per mutex, with a test**

- **Why:** `ctrl-01` (medium), `architecture-global-13` (medium)
- **Commit type:** `fix:` | **Size:** S | **Risk:** Medium: lifecycle is the session fence
- **Depends on:** P0-T09 | **Parallelizable with:** P0-T05..P0-T08
- **Files:**
  - `src/controller/lifecycle.rs (19 lock().expect at 96-107, 150-152, 214, 226-230, 239-245, 249-259, 267, 298-303, 326-330, 350-355, 385-388, 400, 416)`
- **Steps:**
  1. Reviewers agree poisoning is practically unreachable today: every critical section is a field assignment. The only panics are the two overflow expects (151, 278). Recovery through into_inner is a semantic change (a poisoned journal may hold a half-applied update), so decide per mutex:
  2. owner/slot/tasks/terminal_reason: recover with lock_or_recover (values are replaced whole, never half-written).
  3. evidence (OperationJournal) and terminals.pending: keep panic, but document the invariant in a comment, OR convert to an explicit error path. Pick one and record it in the module doc.
  4. Keep the overflow expects (151 'session generation exhausted', 278 'operation IDs exhausted') or make them saturating; do not mix with lock changes.
  5. Document the lock order (owner->slot, tasks->owner, evidence->owner, evidence->terminals.pending) in the module doc.
  6. Test: poison `evidence` in a thread that panics, then assert accepts_scope/is_active still return.
- **Acceptance criteria:**
  - [ ] Module doc lists every mutex with its policy and the lock order.
  - [ ] New poison test passes.
- **Verification:** SV (see 2.1)

#### P0-T11

**Characterization tests for run_event_loop: one per ObsEvent variant**

- **Why:** `ctrl-08` (medium)
- **Commit type:** `test:` | **Size:** M | **Risk:** Low
- **Depends on:** P0-T06 | **Parallelizable with:** P0-T12, P0-T13
- **Files:**
  - `src/controller/refresh_controller.rs 409-586 (tests at 588)`
- **Steps:**
  1. Using ScriptedObs (P0-T06), drive each ObsEvent through run_event_loop and assert the published AppEvents, their order, the revision order (next_observation before invalidate for SceneRenamed 538-546 and InputsChanged 547-557), and the side effects (graph invalidation, audio refresh, collection advance).
  2. Cover InputLevelsUpdated taking the direct try_send path (531-537).
- **Acceptance criteria:**
  - [ ] Every ObsEvent arm has a test.
  - [ ] Tests pass on unchanged production code.
- **Verification:** SV (see 2.1)

#### P0-T12

**Golden AppEvent-sequence test for AppState reducers**

- **Why:** `controller-state-09` (high)
- **Commit type:** `test:` | **Size:** M | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** P0-T11, P0-T13
- **Files:**
  - `src/controller/state.rs tests (or state/tests/ after P2-T01)`
- **Steps:**
  1. Feed a fixed AppEvent sequence (connect, inventory, scene change, stream start, record start/stop with path, readback failure, terminal evidence, disconnect) into the same reducer calls window.rs uses (apply_scene_inventory, apply_output_event, set_obs_stats_at, apply_terminal_evidence).
  2. Snapshot the relevant AppState fields after each step as a Debug-free struct and compare to a checked-in expectation.
  3. This guards P8-T03..P8-T11.
- **Acceptance criteria:**
  - [ ] Test passes on main.
  - [ ] Swapping two reducer statements in apply_output_event makes it fail (sanity check, then revert).
- **Verification:** SV (see 2.1)

#### P0-T13

**Section characterization tests for operating_panel and inventory before their splits**

- **Why:** `tests-structure-16` (medium)
- **Commit type:** `test:` | **Size:** M | **Risk:** Low
- **Depends on:** P0-T02 | **Parallelizable with:** P0-T11, P0-T12
- **Files:**
  - `src/ui/widgets/operating_panel.rs tests (1614+)`
  - `src/ui/pages/inventory.rs tests (1506+)`
- **Steps:**
  1. Extracting a pure reduce_event is a behavior refactor, not a characterization step. Instead add native characterization tests per section: Sources browser rows, detail panel per source kind (text, media, audio), numeric drafts; inventory rows, role cell, options popover, reorder, stale group, YAML import/export, save status.
  2. Each test names the widget-tree facts that the split must keep (handles found, labels, sensitivity).
- **Acceptance criteria:**
  - [ ] At least one native test per section listed.
  - [ ] All pass via `tools/test-native.py --filter operating_panel|inventory`.
- **Verification:** SV + NV (see 2.1)

#### P0-T14

**Shared test support modules (controller, ui, storage)**

- **Why:** `tests-structure-09` (medium), `tests-structure-17` (low), `tests-structure-10` (medium), `tests-structure-18` (low)
- **Commit type:** `test:` | **Size:** M | **Risk:** Low
- **Depends on:** P0-T02 | **Parallelizable with:** P0-T11..P0-T13
- **Files:**
  - `src/controller/test_support/{mod.rs, events.rs, deps.rs, state.rs} (new, cfg(test))`
  - `src/ui/test_support.rs (moved out of src/ui/mod.rs:243-345)`
  - `src/ui/widgets/operating_panel.rs 1748-1790`
  - `src/storage/test_support.rs (new)`
  - `src/storage/bookmarks.rs:85-89`
  - `src/storage/registry.rs:954-963`
- **Steps:**
  1. controller/test_support/events.rs: TEST_DEADLINE, recv_event, recv_matching, wait_until (replaces the try_recv+yield_now loop at final_regression_tests.rs:226-237 and the sleep loops at app_controller.rs:744-752, output_controller.rs:598).
  2. deps.rs: promote StaticConfigProvider (controller/dependencies.rs:72-85) as the one fake AppConfigProvider; AppStateBuilder in state.rs (about 29 AppState::new call sites, 4 local app_state helpers).
  3. ui/test_support.rs: move native_test and helpers out of ui/mod.rs; add find_named, find_where, expect_named; delete the 10 private `fn find` copies over time (start with operating_panel, which spawns its own child with no deadline at 1757-1768; convert it to native_test).
  4. storage/test_support.rs: TempDir with Drop (or add tempfile as a dev-dependency; it is already in Cargo.lock). Replace the `crate::controller::workspace::unix_ms()` call in storage/bookmarks.rs:89 test, removing the only storage->controller edge.
- **Acceptance criteria:**
  - [ ] No test helper in storage imports crate::controller.
  - [ ] operating_panel native tests run under the 60 s native_test deadline.
- **Verification:** SV + NV + TC (see 2.1)

#### P0-T15

**Opt-in performance instrumentation (perf-trace feature)**

- **Why:** `performance-20` (medium)
- **Commit type:** `feat:` | **Size:** S | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** all P0
- **Files:**
  - `Cargo.toml [features]`
  - `src/infra/perf.rs (new)`
  - `src/ui/window.rs (install_event_polling 412-469)`
  - `src/ui/pages/live.rs (rebuild_scene_cards 659)`
  - `src/ui/pages/mixer.rs (refresh)`
  - `src/controller/state/session_reports.rs (collect_report 271)`
- **Steps:**
  1. Add feature `perf-trace` that enables tracing spans: apply_event (kind = variant name), rebuild_scene_cards, rebuild_audio_cards, MixerPageHandle::refresh, collect_report, stage_report_history.
  2. Add per-second counters (events drained, batches, meter frames dropped, collect_report calls) logged at debug every 10 s from infra/perf.rs.
  3. Benches: criterion cannot reach pub(crate) items in this single binary crate (lib.rs exposes only run()). Use #[ignore]d timing tests instead.
  4. Document the measurement recipes from section 6 in docs/developer-guide.md.
- **Acceptance criteria:**
  - [ ] `cargo build --features perf-trace` works; default build unchanged.
  - [ ] Counters visible with RUST_LOG=scenedeck=debug.
- **Verification:** SV + `cargo clippy --all-targets --features perf-trace -- -D warnings` (see 2.1)

### Phase 1 - Correctness, security and data-safety fixes

Small `fix:` commits that change behavior on purpose. They come before the restructuring so the restructuring commits stay pure moves.

| ID | Title | Size | Depends on |
|---|---|---|---|
| [P1-T01](#p1-t01) | Escape OBS-controlled text in Pango markup (StatusPage descriptions, Toasts, help rows) | S | P0-T04 |
| [P1-T02](#p1-t02) | Manual RefreshData must publish output readback failures | S | P0-T06 |
| [P1-T03](#p1-t03) | Separate scene switching from curation readiness | M | - |
| [P1-T04](#p1-t04) | Close-failure Retry re-admits every failure producer | M | - |
| [P1-T05](#p1-t05) | Single-flight queues must not strand requests when spawn_reserved refuses | M | P0-T09 |
| [P1-T06](#p1-t06) | Controller config reads must not hide failures | S | - |
| [P1-T07](#p1-t07) | One formatter for the output status-bar slot | S | - |
| [P1-T08](#p1-t08) | Protect newer config files: no move-aside, no down-convert | M | P0-T05 |
| [P1-T09](#p1-t09) | Make config reads pure; perform migration write-back explicitly at startup | S | P1-T08 |
| [P1-T10](#p1-t10) | Deterministic Doctor finding order | S | - |
| [P1-T11](#p1-t11) | Control snapshot uses the canonical scene order (preflight editor) | S | P0-T06 |
| [P1-T12](#p1-t12) | A full operation journal must not silently drop an output command | S | - |
| [P1-T13](#p1-t13) | ObsPassword newtype with redacting Debug; no password in the Debug+Clone AppState | M | - |
| [P1-T14](#p1-t14) | One CredentialStore port and a Secret-lane barrier before Connect | M | P1-T13 |
| [P1-T15](#p1-t15) | Do not block startup on the Secret Service; load the password lazily | M | P1-T14 |
| [P1-T16](#p1-t16) | Warn about plaintext ws:// to non-loopback hosts | S | P1-T13 |
| [P1-T17](#p1-t17) | XDG base-dir hardening: no CWD fallback, ignore relative XDG_* | S | - |
| [P1-T18](#p1-t18) | Reject relative custom-CSS paths | S | - |
| [P1-T19](#p1-t19) | Explicit YAML import budget and semantic caps | S | - |
| [P1-T20](#p1-t20) | Bound the show-workspace store (limit, LRU eviction, user visibility) | M | P0-T05 |
| [P1-T21](#p1-t21) | Inter-process lock for config, registry, workspace and bookmark writes | M | - |
| [P1-T22](#p1-t22) | Clear startup notice for symlinked config files | S | - |
| [P1-T23](#p1-t23) | Surface an invalid registry.json at startup | S | - |
| [P1-T24](#p1-t24) | Scene search is session view state, not a persisted write per keystroke | S | - |
| [P1-T25](#p1-t25) | Optional redaction for session report exports | S | - |

#### P1-T01

**Escape OBS-controlled text in Pango markup (StatusPage descriptions, Toasts, help rows)**

- **Why:** `security-01` (high), `ui-pages-02` (high)
- **Commit type:** `fix:` | **Size:** S | **Risk:** Low
- **Depends on:** P0-T04 | **Parallelizable with:** all P1
- **Files:**
  - `src/ui/markup.rs (new)`
  - `src/ui/widgets/status_page.rs (new)`
  - `src/ui/pages/mixer.rs 398-403, 733-746, 798-806, 808-823, 933-938`
  - `src/ui/pages/inventory.rs 337, 362`
  - `src/ui/pages/graph.rs 78, 88`
  - `src/ui/window.rs 681, 777, 886, 971, 1136-1145, 1342, 1424`
  - `src/ui/pages/help.rs 408-412`
  - `src/architecture_tests/rules.rs`
- **Steps:**
  1. Add `markup::escape(&str) -> glib::GString` and `markup::plain_toast(title) -> adw::Toast` (builder `.use_markup(false)`; Toast use-markup defaults to TRUE in libadwaita >= 1.4).
  2. Add `status_page(icon, title, literal_description) -> adw::StatusPage` that always escapes and adds `app-status-page`. Replace all 7 StatusPage::builder sites.
  3. Replace every Toast built from OBS names or error text with plain_toast.
  4. Set `use_markup(false)` on help.rs ExpanderRow (AdwPreferencesRow defaults use-markup=true).
  5. Add an architecture rule forbidding `adw::Toast::new` and `StatusPage::builder` outside the two helpers.
- **Acceptance criteria:**
  - [ ] A scene named `<b>x</b> & y` renders literally in the mixer status page and in a toast.
  - [ ] New tests for escape and plain_toast.
- **Verification:** SV + NV (see 2.1)

#### P1-T02

**Manual RefreshData must publish output readback failures**

- **Why:** `ctrl-03` (high)
- **Commit type:** `fix:` | **Size:** S | **Risk:** Low
- **Depends on:** P0-T06 | **Parallelizable with:** all P1
- **Files:**
  - `src/controller/app_controller.rs 256-262`
  - `src/controller/output_controller.rs 469-501`
  - `src/controller/refresh_controller.rs 345-348`
- **Steps:**
  1. Add `#[must_use]` to OutputReadbackErrors; clippy -D warnings then flags app_controller.rs:260.
  2. Publish the errors there as refresh_worker does at refresh_controller.rs:345-348.
  3. Test with ScriptedObs returning an error for GetStreamStatus, then RefreshData, asserting AppEvent::OutputReadbackFailed{kind: Stream}.
  4. Land as its own `fix:` commit. The shared full_snapshot_scan extraction is P7-T07.
- **Acceptance criteria:**
  - [ ] New test passes and fails without the fix.
- **Verification:** SV (see 2.1)

#### P1-T03

**Separate scene switching from curation readiness**

- **Why:** `recent-change-review-01` (high)
- **Commit type:** `fix:` | **Size:** M | **Risk:** Medium: changes documented behavior
- **Depends on:** none | **Parallelizable with:** all P1
- **Files:**
  - `src/ui/navigation.rs 111-122, 184-187`
  - `src/ui/pages/live.rs 673-677`
  - `src/ui/widgets/scene_card.rs 107-109`
  - `src/ui/hotkeys.rs 158-163`
  - `docs/feature-program.md 166-167`
  - `docs/user-guide.md`
- **Steps:**
  1. MAINTAINER DECISION FIRST (see open questions): hotkey gating on curation readiness is documented as intentional (docs/feature-program.md:166-167).
  2. Scope: cards and dispatch only. Gate SwitchPrimaryScene on `workspace.scene_action(scene).is_some()` and session currency, not on RegistryStatus::Ready.
  3. When the registry is not Ready, render cards in OBS inventory order with no roles; keep them sensitive.
  4. Remove the duplicate registry_import_busy() check (navigation.rs:114 and 185).
  5. Update docs if hotkey behavior changes.
- **Acceptance criteria:**
  - [ ] With a failed registry load, clicking a scene card switches the program scene.
  - [ ] Native test covers it.
- **Verification:** SV + NV (see 2.1)

#### P1-T04

**Close-failure Retry re-admits every failure producer**

- **Why:** `recent-change-review-02` (high)
- **Commit type:** `fix:` | **Size:** M | **Risk:** Medium
- **Depends on:** none | **Parallelizable with:** all P1
- **Files:**
  - `src/app.rs 404-425`
  - `src/ui/background_io.rs 318-345 (flush_pending, UNRESOLVED_FAILURES)`
  - `src/ui/bookmarks.rs 21-79`
  - `src/ui/session_reports.rs 22-23`
  - `src/ui/persist.rs`
  - `src/ui/report_history.rs`
- **Steps:**
  1. Today Retry calls only persist::retry_registry_writes() and report_history::retry_active(). Bookmark, session-report export, config-snapshot and worker-panic failures stay in UNRESOLVED_FAILURES, so the close barrier keeps failing.
  2. Minimal fix now: a ui-side `retry_all_durable_writes()` that calls each producer's retry (bookmarks, session report export, config snapshot) and clears worker-panic entries that have no retry by re-running their snapshot.
  3. The generic DurableStore/PersistenceCoordinator is P9-T14; keep this fix small.
  4. Native test: fail a bookmark save, press Retry after fixing the path, assert the window closes.
- **Acceptance criteria:**
  - [ ] Every producer of background_io::record_failure has a retry path reachable from the close dialog.
- **Verification:** SV + NV (see 2.1)

#### P1-T05

**Single-flight queues must not strand requests when spawn_reserved refuses**

- **Why:** `ctrl-04` (medium), `ctrl-21` (low)
- **Commit type:** `fix:` | **Size:** M | **Risk:** Medium
- **Depends on:** P0-T09 | **Parallelizable with:** all P1
- **Files:**
  - `src/controller/runtime/single_flight.rs (new)`
  - `src/controller/app_controller.rs 219-284, 293-358`
  - `src/controller/workspace.rs 226-258`
  - `src/controller/workspace/audio_presets.rs 13-35`
- **Steps:**
  1. Add SingleFlight<J> { offer(job) -> Admission{Started|Coalesced}, next() -> Option<J>, abandon() -> Option<J> } with unit tests.
  2. When spawn_reserved returns None: abandon() and publish the matching failure - MixerAudioInputsFailed (mixer), WorkspaceLoaded with Err (workspace), PresetRejected{epoch, run} (audio_presets.rs:35 today assigns None silently).
  3. RefreshQueue: keep its inactive branch but make it race-free: try spawn_reserved first, fall back to runtime.spawn when it returns None (do not check is_active() first).
  4. In audio_presets.rs replace `client.unwrap()` after the guard (31) with let-else.
  5. Test: end the FakeSessionRunner, then RefreshMixerSceneAudio, assert MixerAudioInputsFailed.
- **Acceptance criteria:**
  - [ ] Mixer never stays 'loading' after a session ends.
  - [ ] SingleFlight tests cover offer/coalesce/next/abandon.
- **Verification:** SV (see 2.1)

#### P1-T06

**Controller config reads must not hide failures**

- **Why:** `ctrl-19` (medium)
- **Commit type:** `fix:` | **Size:** S | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** all P1
- **Files:**
  - `src/controller/app_controller.rs 449-458`
  - `src/controller/dependencies.rs 56-62`
  - `src/controller/session_controller.rs 271`
- **Steps:**
  1. load_config_blocking returns Default on JoinError; change it to Result<AppConfig, AppError> (map JoinError to AppError::storage as session_controller.rs:282-284 does).
  2. Change AppConfigProvider::load_config to return LoadedConfig (or Result) so ConfigStartupNotice::ReadFailed reaches the caller instead of being dropped at dependencies.rs:60.
  3. On Err publish CommandFailed/GraphRefreshRejected and skip the scan. ObsSessionRunner logs the notice.
  4. Test with a panicking provider.
- **Acceptance criteria:**
  - [ ] A corrupt config.json no longer silently applies the default audio_inputs filter.
- **Verification:** SV (see 2.1)

#### P1-T07

**One formatter for the output status-bar slot**

- **Why:** `ui-shell-04` (high)
- **Commit type:** `fix:` | **Size:** S | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** all P1
- **Files:**
  - `src/ui/window.rs 475-507, 1317-1338`
  - `src/ui/pages/live.rs 500-522`
  - `src/ui/presenters/output_status.rs (new)`
  - `i18n/*/scenedeck.ftl`
- **Steps:**
  1. The 1 s clock writes `window-*-status-line` with elapsed whenever active_since is Some; the event path shows elapsed only if status.active. They overwrite each other.
  2. Move format_elapsed and output_label into ui/presenters/output_status.rs. The clock calls the same refresh_output_status_bar(kind) the event path uses.
  3. Delete elapsed_suffix and the two now-unused window-*-status-line keys from every locale (parity test).
- **Acceptance criteria:**
  - [ ] With active_since Some and active=false, clock and events show the same text.
  - [ ] IV passes.
- **Verification:** SV + NV + IV (see 2.1)

#### P1-T08

**Protect newer config files: no move-aside, no down-convert**

- **Why:** `security-09` (low), `storage-infra-03` (high), `storage-infra-10` (medium)
- **Commit type:** `fix:` | **Size:** M | **Risk:** Medium: touches config load path
- **Depends on:** P0-T05 | **Parallelizable with:** all P1
- **Files:**
  - `src/storage/config.rs 25-63, 234-255, 287-303, 317-334, 380-406`
  - `src/storage/registry.rs 22-72`
- **Steps:**
  1. Add a VersionProbe { version: Option<u32> } read before full parse. If version > CURRENT_VERSION: do not rename to .invalid, run with in-memory parse (ignore unknown fields), refuse writes with io::ErrorKind::Unsupported, and add ConfigStartupNotice::NewerVersion (new fl! key in every locale; rendering moves to ui in P5-T10).
  2. Skip the read-time migration rewrite for newer configs.
  3. Prefer refusing writes over `#[serde(flatten)] extra` (flatten interacts badly with skip_serializing/rename). For registry.json add a schema version field with a golden test (P0-T05 fixture updated).
  4. Tests: version 999 plus an unknown field stays byte-identical on disk after startup and after a settings change.
- **Acceptance criteria:**
  - [ ] Test: a config.json with `version: 999` and an unknown field is byte-identical on disk after startup and after a settings change (step 4).
  - [ ] Test: the same file is not renamed to `.invalid` and the NewerVersion notice is produced.
- **Verification:** SV + IV (see 2.1)

#### P1-T09

**Make config reads pure; perform migration write-back explicitly at startup**

- **Why:** `storage-infra-05` (high), `security-10` (low)
- **Commit type:** `fix:` | **Size:** S | **Risk:** Low
- **Depends on:** P1-T08 | **Parallelizable with:** all P1 except P1-T08
- **Files:**
  - `src/storage/config.rs 234-265`
  - `src/controller/dependencies.rs 56-61`
  - `src/app.rs (build_ui ~439)`
- **Steps:**
  1. read_config_from_path returns LoadedConfig { config, startup_notice, pending_write: Option<AppConfig> } and never writes or renames.
  2. Startup (GTK side) performs the pending write via ui::persist::persist_config so a failure reaches close-guard accounting; keep the .invalid preservation at startup only.
  3. FileAppConfigProvider (controller, off-lane) uses the side-effect-free read.
  4. Test: reading a legacy file leaves bytes on disk unchanged.
- **Acceptance criteria:**
  - [ ] grep shows no write/rename inside read_config_from_path.
- **Verification:** SV (see 2.1)

#### P1-T10

**Deterministic Doctor finding order**

- **Why:** `domain-services-ddd-16` (medium)
- **Commit type:** `fix:` | **Size:** S | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** all P1
- **Files:**
  - `src/domain/registry.rs 112-117, 146-149`
  - `src/domain/graph.rs 164-172`
  - `src/services/doctor_service.rs 30-47, 588-663`
- **Steps:**
  1. Confirm with maintainer (behavior change: row order).
  2. Add a secondary sort key (scene id, then edge) after the severity sort at doctor_service.rs:45, or switch SceneRegistrySnapshot.scenes and SceneGraph.edges to BTreeMap.
  3. Update the test at 589+ to a fixed expected order.
- **Acceptance criteria:**
  - [ ] Doctor output identical across process launches (test runs evaluate on two differently-seeded HashMaps).
- **Verification:** SV (see 2.1)

#### P1-T11

**Control snapshot uses the canonical scene order (preflight editor)**

- **Why:** `obs-acl-07` (medium)
- **Commit type:** `fix:` | **Size:** S | **Risk:** Low
- **Depends on:** P0-T06 | **Parallelizable with:** all P1
- **Files:**
  - `src/obs/client.rs 529-538`
  - `src/obs/mapper.rs 36-54`
  - `src/ui/preflight.rs 287-291`
  - `src/ui/preflight/editor.rs 37-42`
  - `src/ui/preflight/tests.rs 62`
  - `src/controller/workspace.rs 738`
- **Steps:**
  1. get_control_snapshot builds Scene without `.rev()` while map_scenes reverses; preflight shows scenes in reversed switcher order.
  2. Call map_scenes from get_control_snapshot. Do NOT drop ControlSnapshot.scenes (preflight review and editor read it).
  3. Behavior change: own `fix:` commit with a wire test (ScriptedObs) asserting order; update controller/workspace.rs:738 and preflight tests if they assume the old order.
- **Acceptance criteria:**
  - [ ] Preflight editor lists scenes in OBS switcher order.
- **Verification:** SV + NV (see 2.1)

#### P1-T12

**A full operation journal must not silently drop an output command**

- **Why:** `ctrl-23` (low)
- **Commit type:** `fix:` | **Size:** S | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** all P1
- **Files:**
  - `src/controller/lifecycle.rs 271-272, 332, 402`
  - `src/controller/output_controller.rs 88-94`
  - `src/controller/workspace.rs 138`
  - `src/controller/thumbnails.rs 94`
  - `src/controller/workspace/audio_presets.rs 188`
  - `src/controller/state.rs 937`
- **Steps:**
  1. operation() returns None on a full journal (272 `?`), so set_output clears pending and returns silently.
  2. Return Result<OperationLease, AdmissionRefused{Inactive, JournalFull}>; set_output publishes OutputCommandFailed on JournalFull (localized message, every locale).
  3. Name the literals: JOURNAL_CAPACITY (271), TERMINAL_HISTORY (332, and state.rs:937 RETIREMENT_HISTORY shares it), TASK_CAPACITY (402), WORKSPACE_QUEUE (workspace.rs:138), THUMBNAIL_BUDGET (thumbnails.rs:94), PRESET_REVIEW_WINDOW (audio_presets.rs:188).
- **Acceptance criteria:**
  - [ ] Test: fill the journal, send SetOutput, assert OutputCommandFailed.
- **Verification:** SV + IV (see 2.1)

#### P1-T13

**ObsPassword newtype with redacting Debug; no password in the Debug+Clone AppState**

- **Why:** `security-04` (medium), `controller-state-16` (medium), `storage-infra-20` (medium)
- **Commit type:** `fix:` | **Size:** M | **Risk:** Medium: credential flow
- **Depends on:** none | **Parallelizable with:** all P1 except P1-T14/T15/T16
- **Files:**
  - `src/domain/connection/credential.rs (new)`
  - `src/controller/state.rs 437-456, 505, 525`
  - `src/app.rs 26, 44-52, 444, 636`
  - `src/storage/secret.rs 14-49`
  - `src/controller/dependencies.rs 18, 42, 67`
  - `src/ui/pages/settings/connections.rs 113, 121, 146, 156, 249, 382`
  - `src/ui/pages/settings.rs 457-495`
- **Steps:**
  1. Add `pub struct ObsPassword(Box<str>)` with manual Debug printing `ObsPassword(***)`, no Display, `expose(&self) -> &str`. Defer zeroize until a cargo deny review.
  2. Use it in storage::secret signatures, ObsPasswordProvider, PasswordWrites and the settings editor.
  3. Remove AppState::obs_password and StartupData.password; the settings editor owns Option<ObsPassword> (see P1-T15 for lazy load). If removal is too big for one PR, first replace the field type and add a manual Debug for AppState, then remove.
  4. obws needs a String at connect: call expose() at the obs boundary only.
- **Acceptance criteria:**
  - [ ] `format!("{:?}", app_state)` (in a test) contains no password.
  - [ ] grep `obs_password: Option<String>` returns nothing.
- **Verification:** SV + NV (see 2.1)

#### P1-T14

**One CredentialStore port and a Secret-lane barrier before Connect**

- **Why:** `security-06` (medium), `recent-change-review-25` (low)
- **Commit type:** `fix:` | **Size:** M | **Risk:** Medium
- **Depends on:** P1-T13 | **Parallelizable with:** P1-T01..P1-T12
- **Files:**
  - `src/domain/connection/credential.rs`
  - `src/storage/secret.rs`
  - `src/controller/dependencies.rs 17-19`
  - `src/controller/session_controller.rs 270-273`
  - `src/ui/pages/settings/connections.rs 22-38, 236-275`
  - `src/ui/background_io.rs 19-46`
- **Steps:**
  1. Define `trait CredentialStore: Send + Sync { read, write, delete by ConnectionId }` (sync; called only from the blocking pool or the Secret lane). Implement once in storage/secret.rs as KeyringCredentialStore.
  2. Delete ConnectionSecrets/KeyringSecrets in connections.rs; inject the same Arc<dyn CredentialStore> into ControllerDependencies and into the settings page through its page-builder argument (NOT through NavigationContext - see RC13).
  3. Connect from settings: enqueue the connect behind pending Secret-lane writes (the lane is a FIFO SerialQueue) so Connect never reads a stale password. The race is plausible, not reproduced: add a test with a slow fake store.
- **Acceptance criteria:**
  - [ ] Exactly one keyring port in the codebase.
  - [ ] Slow-store test: Connect after Save uses the new password.
- **Verification:** SV + NV (see 2.1)

#### P1-T15

**Do not block startup on the Secret Service; load the password lazily**

- **Why:** `security-05` (medium), `ui-shell-08` (medium)
- **Commit type:** `perf:` | **Size:** M | **Risk:** Low
- **Depends on:** P1-T14 | **Parallelizable with:** P1-T01..P1-T12
- **Files:**
  - `src/app.rs 42-74`
  - `src/ui/pages/settings/connections.rs 112-114, 117-163`
- **Steps:**
  1. Remove the keyring read from app.rs startup (spawn(...).join() before the first frame).
  2. Settings editor calls load_password(id, generation) on SerialLane::Secret when first built, with busy(true) (today load_password runs only from change_selection 117-131).
  3. Controller already reads the password through ObsPasswordProvider at connect time; keep that.
  4. Optional in the same task only if small: load bookmarks and report history in background_io after the window shows; otherwise leave for P9-T08.
  5. Replace the English fallback strings at app.rs:71-72 with fl! keys.
- **Acceptance criteria:**
  - [ ] Startup with a locked keyring shows the window immediately.
  - [ ] Test seam: a startup test with a counting CredentialStore asserts zero reads before activation.
- **Verification:** SV + NV + IV (see 2.1)

#### P1-T16

**Warn about plaintext ws:// to non-loopback hosts**

- **Why:** `security-02` (medium)
- **Commit type:** `feat:` | **Size:** S | **Risk:** Low
- **Depends on:** P1-T13 | **Parallelizable with:** all P1
- **Files:**
  - `src/domain/connection/endpoint.rs (is_loopback)`
  - `src/ui/pages/settings/connections.rs`
  - `i18n/*/scenedeck.ftl`
  - `docs/obs-websocket.md`
  - `docs/user-guide.md`
- **Steps:**
  1. Add pure `is_loopback(host) -> bool` (no GTK) and a persistent warning row in the connection editor when the host is not loopback. New key `settings-connection-unencrypted-remote` in every locale.
  2. Docs: obs-websocket is plaintext; recommend a TLS proxy or SSH tunnel for remote hosts.
  3. TLS transport itself is deferred (P11-T03): it adds a rustls tree that must pass deny.toml, and OBS has no native wss.
- **Acceptance criteria:**
  - [ ] Warning shows for 192.168.x.x and hides for 127.0.0.1, ::1, localhost.
- **Verification:** SV + NV + IV (see 2.1)

#### P1-T17

**XDG base-dir hardening: no CWD fallback, ignore relative XDG_***

- **Why:** `security-15` (low), `storage-infra-19` (low)
- **Commit type:** `fix:` | **Size:** S | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** all P1
- **Files:**
  - `src/storage/xdg.rs 21-58`
- **Steps:**
  1. Write one private `base_dir(var, home_suffix) -> Option<PathBuf>` that ignores non-absolute XDG values and falls back to std::env::home_dir() (stable, consults getpwuid). Do NOT use glib (storage bans glib::).
  2. Minimal: keep PathBuf return and log a warning on fallback; full: return Option and run with in-memory defaults (about six callers of config_path/registry_path).
  3. Unit tests for relative XDG, unset HOME.
- **Acceptance criteria:**
  - [ ] No path resolves against the process CWD.
- **Verification:** SV (see 2.1)

#### P1-T18

**Reject relative custom-CSS paths**

- **Why:** `security-14` (low)
- **Commit type:** `fix:` | **Size:** S | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** all P1
- **Files:**
  - `src/ui/theme.rs 178-215 (load_custom_css)`
  - `src/domain/appearance.rs 280-290, 335-353`
  - `i18n/*/scenedeck.ftl`
- **Steps:**
  1. In ThemeManager::load_custom_css reject a non-absolute path with CustomCssLoad::Failed and a localized message (new key in every locale).
  2. Do NOT add a deserialize_with on the domain type that references storage::xdg (domain cannot import storage).
- **Acceptance criteria:**
  - [ ] Test: relative path returns Failed.
- **Verification:** SV + IV (see 2.1)

#### P1-T19

**Explicit YAML import budget and semantic caps**

- **Why:** `security-11` (low)
- **Commit type:** `fix:` | **Size:** S | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** all P1
- **Files:**
  - `src/storage/registry.rs 655-679`
- **Steps:**
  1. Use serde_saphyr::from_str_with_options with Options { budget: Some(Budget { max_aliases: 1_000, max_anchors: 1_000, max_recorded_anchor_bytes: 1<<20, max_nodes: 100_000, max_merge_keys: 100, ..Default::default() }), ..Default::default() }.
  2. Add validate_import(&SceneRegistry) with count/length caps per collection.
  3. Tests: alias bomb rejected; oversized list rejected.
- **Acceptance criteria:**
  - [ ] Test: an alias-bomb YAML (nested anchors over the `max_aliases` budget) returns `Err` without allocating past the budget, and the error maps to a localized import error, not a panic.
  - [ ] Test: a registry YAML with one collection over its `validate_import` cap is rejected and nothing is written.
- **Verification:** SV (see 2.1)

#### P1-T20

**Bound the show-workspace store (limit, LRU eviction, user visibility)**

- **Why:** `security-08` (low), `recent-change-review-08` (low), `storage-infra-16` (medium)
- **Commit type:** `fix:` | **Size:** M | **Risk:** Low
- **Depends on:** P0-T05 | **Parallelizable with:** all P1
- **Files:**
  - `src/storage/show_workspaces.rs 69-157`
  - `src/storage/small_file.rs 10`
- **Steps:**
  1. Add WORKSPACE_STORE_LIMIT and a boundary test that a store near the limit still loads.
  2. Add `last_used_unix_ms: u64` (serde default 0) to StoredWorkspace, updated on the next write after a load_or_create hit (not an extra write per read).
  3. MAX_WORKSPACES = 64 with LRU eviction that never evicts the target or legacy_owner; surface eviction in the UI (toast; new key in every locale).
  4. Per-workspace files are deferred until after P1-T21 (locking).
- **Acceptance criteria:**
  - [ ] MAX+1 evicts the oldest; target never evicted.
- **Verification:** SV + IV (see 2.1)

#### P1-T21

**Inter-process lock for config, registry, workspace and bookmark writes**

- **Why:** `storage-infra-06` (medium)
- **Commit type:** `fix:` | **Size:** M | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** all P1
- **Files:**
  - `src/storage/lock.rs (new, extracted from storage/report_history.rs 130-149)`
  - `src/storage/config.rs 380-405`
  - `src/storage/registry.rs 640-652`
  - `src/storage/show_workspaces.rs 50-66`
  - `src/storage/bookmarks.rs`
- **Steps:**
  1. Lock a separate `<file>.lock`, never the data file (atomic::write renames over the inode).
  2. Return a distinct Busy error; the UI shows a toast.
  3. Matters mainly for the snap build (NON_UNIQUE); outside snap GApplication forwards a second launch.
- **Acceptance criteria:**
  - [ ] Two processes writing config concurrently: one gets Busy, no torn file.
- **Verification:** SV (see 2.1)

#### P1-T22

**Clear startup notice for symlinked config files**

- **Why:** `storage-infra-11` (low)
- **Commit type:** `fix:` | **Size:** S | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** all P1
- **Files:**
  - `src/storage/small_file.rs 14-22`
  - `src/storage/config.rs 260-263, 383-390`
  - `src/storage/registry.rs 586-594`
  - `docs/configuration.md`
- **Steps:**
  1. Detect ELOOP (raw_os_error == libc::ELOOP) and show a specific notice instead of the generic read-failed one; document the no-symlink policy. Supporting symlinks is a product decision (open question).
- **Acceptance criteria:**
  - [ ] Symlinked config.json gives the specific notice.
- **Verification:** SV + IV (see 2.1)

#### P1-T23

**Surface an invalid registry.json at startup**

- **Why:** `storage-infra-21` (low)
- **Commit type:** `fix:` | **Size:** S | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** all P1
- **Files:**
  - `src/storage/registry.rs 586-594`
  - `src/app.rs 25, 51`
- **Steps:**
  1. First check whether StartupData.registry is still read now that workspaces are primary; if only load_or_create reads the legacy file, delete the startup read.
  2. Otherwise return a startup notice instead of a warn log.
- **Acceptance criteria:**
  - [ ] An invalid registry.json is either not read at startup or reported to the user.
- **Verification:** SV (see 2.1)

#### P1-T24

**Scene search is session view state, not a persisted write per keystroke**

- **Why:** `recent-change-review-07` (medium)
- **Commit type:** `perf:` | **Size:** S | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** all P1
- **Files:**
  - `src/ui/widgets/deck_controls.rs 52-62, 283-290`
  - `src/storage/show_workspaces.rs 50-66`
  - `src/ui/persist.rs 511-536`
- **Steps:**
  1. Today every search-changed rewrites the whole show-workspaces.json store.
  2. Keep the live query in AppState (not persisted); persist only named decks. If the product wants the query remembered, debounce 1 s through one coalescible snapshot like run_config_snapshot.
- **Acceptance criteria:**
  - [ ] Typing in scene search performs zero file writes (test with a counting store or by checking mtime).
- **Verification:** SV + NV (see 2.1)

#### P1-T25

**Optional redaction for session report exports**

- **Why:** `security-18` (low)
- **Commit type:** `feat:` | **Size:** S | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** all P1
- **Files:**
  - `src/storage/report_export.rs 1, 25-31, 73, 87-95`
  - `src/ui/session_reports.rs (export dialog)`
  - `i18n/*/scenedeck.ftl`
- **Steps:**
  1. Product decision on the default (open question). Add ExportOptions { redact_endpoint, redact_paths } and a checkbox in the export dialog (new keys in every locale).
- **Acceptance criteria:**
  - [ ] Export with redaction on contains no host, connection name or OBS paths.
- **Verification:** SV + NV + IV (see 2.1)

### Phase 2 - Pull inline tests out of god files (pure moves)

Mechanical: `x.rs` plus `x/tests.rs`. The test count must stay the same (TC). After this, each god file shows its real production size.

| ID | Title | Size | Depends on |
|---|---|---|---|
| [P2-T01](#p2-t01) | Move controller/state.rs tests out (1247-3088) and gather fixtures | L (pure move) | P0-T07 |
| [P2-T02](#p2-t02) | Move state/mixer.rs tests and the misplaced window mixer predicate | M | P2-T01, P0-T02 |
| [P2-T03](#p2-t03) | Move app_controller.rs tests into app_controller/tests/ | L (pure move) | P0-T07 |
| [P2-T04](#p2-t04) | Move obs/client.rs test modules and regroup regression_tests by capability | L (pure move) | P0-T06 |
| [P2-T05](#p2-t05) | Move storage test modules out and delete test-only production scaffolding | L (pure move) | P0-T05 |
| [P2-T06](#p2-t06) | Move ui/window.rs tests next to their subjects | L (pure move) | P0-T02, P2-T02 |
| [P2-T07](#p2-t07) | Move tests out of ui/theme.rs, ui/persist.rs, app.rs, ui/background_io.rs | L (pure move) | P0-T02 |
| [P2-T08](#p2-t08) | Move tests out of the UI page and widget god files | L (pure move) | P0-T02, P0-T03 |
| [P2-T09](#p2-t09) | Seams and a stateful OBS mock for final_regression_tests | M | P0-T06, P2-T03 |
| [P2-T10](#p2-t10) | Split final_regression_tests.rs by owning component | L (pure move) | P2-T09 |
| [P2-T11](#p2-t11) | Table-drive mirrored tests, move domain test helpers into test_support | M | P2-T01, P2-T03 |

#### P2-T01

**Move controller/state.rs tests out (1247-3088) and gather fixtures**

- **Why:** `controller-state-20` (medium), `tests-structure-07` (medium), `tests-structure-08` (medium)
- **Commit type:** `refactor:` | **Size:** L (pure move) | **Risk:** Low
- **Depends on:** P0-T07 | **Parallelizable with:** P2-T03..P2-T08
- **Files:**
  - `src/controller/state.rs`
  - `src/controller/state/tests/{mod,fixtures,outputs,health,connection,topology,workspace,telemetry,audio}.rs`
  - `src/controller/state/mixer.rs:455`
- **Steps:**
  1. Pure move. Fixtures are interleaved (1253, 1257, 1905-1980, 2426, 2503, 2893): collect all 17 `pub(super) fn` helpers by grep into state/tests/fixtures.rs (child of state, so private fields stay visible).
  2. Change mixer.rs:455 `use super::super::tests::*` to `use crate::controller::state::tests::fixtures::*` with fixtures `pub(in crate::controller::state)`.
  3. Split tests by name ranges (1595 health, 1745 connecting, 1832-1880 topology, 1989-2293 stream/record, ...).
- **Acceptance criteria:**
  - [ ] state.rs shrinks to about 1246 lines.
  - [ ] TC equal (49 tests in state.rs).
- **Verification:** SV + TC (see 2.1)

#### P2-T02

**Move state/mixer.rs tests and the misplaced window mixer predicate**

- **Why:** `tests-structure-11` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P2-T01, P0-T02 | **Parallelizable with:** P2-T03..P2-T08
- **Files:**
  - `src/controller/state/mixer.rs (tests 453-1396)`
  - `src/ui/window.rs 1378-1401, 2549-2698`
  - `src/controller/app_controller.rs 26-27, 655-685`
  - `src/obs/regression_tests.rs 252-318`
  - `src/controller/final_regression_tests.rs 970-1007`
- **Steps:**
  1. Move mixer tests to state/mixer/tests.rs (45 tests).
  2. Move should_rebuild_visible_mixer_for_input_event (window.rs:1378-1401, pure over AppState) into `impl AppState` in state/mixer.rs with its 11 mixer_input_event_* tests and their helpers (imports at window.rs:1590-1591).
  3. Move bitrate tests (app_controller.rs:656-672) to refresh_controller tests; split the DoctorService assertion (regression_tests.rs:308) into services/doctor_service tests; move the graph reducer test (final_regression_tests.rs:970-1007) to state tests.
- **Acceptance criteria:**
  - [ ] TC equal.
  - [ ] window.rs no longer defines a pure AppState predicate.
- **Verification:** SV + TC + NV (see 2.1)

#### P2-T03

**Move app_controller.rs tests into app_controller/tests/**

- **Why:** `architecture-global-21` (low)
- **Commit type:** `refactor:` | **Size:** L (pure move) | **Risk:** Low
- **Depends on:** P0-T07 | **Parallelizable with:** P2-T01, P2-T04..P2-T08
- **Files:**
  - `src/controller/app_controller.rs (tests 464-1279, #[path] final_regression_tests at 460-462)`
- **Steps:**
  1. Tests drive AppController::handle, so they stay AppController integration tests: app_controller/tests/{outputs,config,refresh}.rs.
  2. Keep the #[path = "final_regression_tests.rs"] include resolvable (paths are relative to the declaring file's directory) until P2-T10 splits it.
  3. Move test-only items at 8-33 and 419 with the tests.
- **Acceptance criteria:**
  - [ ] app_controller.rs production about 458 lines.
  - [ ] TC equal.
- **Verification:** SV + TC (see 2.1)

#### P2-T04

**Move obs/client.rs test modules and regroup regression_tests by capability**

- **Why:** `obs-acl-21` (low), `tests-structure-20` (low), `performance-22` (low)
- **Commit type:** `refactor:` | **Size:** L (pure move) | **Risk:** Low
- **Depends on:** P0-T06 | **Parallelizable with:** P2-T01..P2-T03, P2-T05..P2-T08
- **Files:**
  - `src/obs/client.rs (mod tests 317-515, control_tests 1099-1392, #[path] 1393-1395)`
  - `src/obs/regression_tests.rs`
  - `src/obs/{requests,audio,sources,operator_inputs}.rs`
- **Steps:**
  1. Move `mod tests` (317-515) so both impl ObsClient blocks sit together; move it to obs/client/tests.rs (child module keeps private access).
  2. Move control_tests to obs/client/control_tests.rs; drop the P0-T06 mock_client shim and update regression_tests.rs:4 and controller/workspace/audio_presets/tests.rs:2.
  3. Move regression groups next to their modules: requests (381-572, creates requests tests module), audio (126-197, 597-656), sources (200-336, 623-636 plus client.rs 1214-1274), operator_inputs (20-124), stats (575-594), gain/identity wire tests (339-378, 658-706). Compile-check each group: moved tests lose `use super::*` access to client.rs privates.
  4. Delete regression_tests.rs and the #[path] include.
- **Acceptance criteria:**
  - [ ] `cargo test --all-features obs:: -- --list | wc -l` identical before and after.
  - [ ] client.rs production about 1-316 + 517-1097 contiguous.
- **Verification:** SV + TC (see 2.1)

#### P2-T05

**Move storage test modules out and delete test-only production scaffolding**

- **Why:** `storage-infra-13` (medium), `recent-change-review-26` (low), `tests-structure-04` (high)
- **Commit type:** `refactor:` | **Size:** L (pure move) | **Risk:** Low
- **Depends on:** P0-T05 | **Parallelizable with:** P2-T01..P2-T04, P2-T06..P2-T08
- **Files:**
  - `src/storage/registry.rs (151-165, 478-502, 622-637; tests 693-1556)`
  - `src/storage/config.rs (tests 409-771)`
  - `src/ui/persist.rs 122-125, 406-414`
  - `src/controller/state.rs 943-946, 1112-1120, 1143-1151`
  - `src/controller/app_controller.rs 419-422`
- **Steps:**
  1. Port tests that use RegistryMutationError, mutate_registry, the free set_scene_icon, set_scene_role_from_path and remove_scene_entry_from_path to RegistryTarget::File(path).read()/write(); delete those cfg(test) items. Remove the mutate_registry assertion in every_registry_json_write_refuses_a_corrupt_original (registry.rs:844); the write_registry_to_path assertion at 843 keeps the guarantee.
  2. Move registry tests to storage/registry/tests/{curation,persistence,snapshot}.rs and config tests to storage/config/tests.rs.
  3. Delete AppState test-only set_*_command_failure and set_audio_levels helpers by switching tests to the real reducers; move persist.rs test-only items into its tests module.
- **Acceptance criteria:**
  - [ ] No `#[cfg(test)]` item remains in the production part of registry.rs, persist.rs, state.rs (outside tests modules).
  - [ ] TC equal or documented (deleted duplicates listed in PR).
- **Verification:** SV + TC (see 2.1)

#### P2-T06

**Move ui/window.rs tests next to their subjects**

- **Why:** `ui-shell-16` (medium)
- **Commit type:** `refactor:` | **Size:** L (pure move) | **Risk:** Low
- **Depends on:** P0-T02, P2-T02 | **Parallelizable with:** P2-T03..P2-T05, P2-T07, P2-T08
- **Files:**
  - `src/ui/window.rs (tests 1583-2698)`
  - `src/ui/window/tests/{native,render,events}.rs`
- **Steps:**
  1. Pure move into ui/window/tests/*.rs (paths stay under ui::, test-native.py keeps finding them; native_test paths are now computed by P0-T02).
  2. Group: registry/scene scenarios, workspace scenarios, audio/page-blink scenarios, assembled-window leak test. They move again with their handlers in P9-T02.
  3. Use StaticConfigProvider from P0-T14 instead of the local NoServices provider (2371-2376).
- **Acceptance criteria:**
  - [ ] window.rs production about 1582 lines.
  - [ ] TC equal; NV green.
- **Verification:** SV + TC + NV (see 2.1)

#### P2-T07

**Move tests out of ui/theme.rs, ui/persist.rs, app.rs, ui/background_io.rs**

- **Why:** `tests-structure-07` (medium), `tests-structure-19` (low)
- **Commit type:** `refactor:` | **Size:** L (pure move) | **Risk:** Low
- **Depends on:** P0-T02 | **Parallelizable with:** P2-T01..P2-T06, P2-T08
- **Files:**
  - `src/ui/theme.rs (616-1553)`
  - `src/ui/persist.rs (591-1033)`
  - `src/app.rs (457-1345)`
  - `src/ui/background_io.rs (378+)`
  - `docs/developer-guide.md`
- **Steps:**
  1. Standardize on `<module>/tests.rs` or `<module>/tests/<topic>.rs`; replace the six #[path] test mounts over P2 (app_controller 460-462, client 1393-1395, window 1583-1585, transport_bar 432, previews 292, workspace 566).
  2. Rename embedded child-test paths (persist.rs isolated_test_child names 'ui::persist::tests::...', six sites) as the files move; app.rs native tests stay under `app::tests::native_` or update test-native.py classification in the same commit.
  3. Document the convention in docs/developer-guide.md.
- **Acceptance criteria:**
  - [ ] No `#[path = ...]` test mount remains except where documented.
  - [ ] TC equal; NV green.
- **Verification:** SV + TC + NV (see 2.1)

#### P2-T08

**Move tests out of the UI page and widget god files**

- **Why:** `tests-structure-07` (medium)
- **Commit type:** `refactor:` | **Size:** L (pure move) | **Risk:** Low
- **Depends on:** P0-T02, P0-T03 | **Parallelizable with:** P2-T01..P2-T07
- **Files:**
  - `src/ui/pages/{inventory (1506+), mixer (1224+, layout 1788+), live (977+), stats (913+), settings (874+)}.rs`
  - `src/ui/widgets/{operating_panel (1614+), audio_card (885+)}.rs`
  - `src/ui/action_catalog.rs (667+)`
  - `src/ui/sidebar.rs (548+)`
  - `src/ui/action_shortcuts.rs (563+)`
- **Steps:**
  1. One file per commit; each becomes `x.rs` + `x/tests.rs` (and `x/native_tests.rs` where native tests exist). Native test names are path-computed after P0-T02.
  2. Run rustfmt so collapsed one-liners expand.
- **Acceptance criteria:**
  - [ ] Each listed file's production part is unchanged; TC equal; NV green.
- **Verification:** SV + TC + NV (see 2.1)

#### P2-T09

**Seams and a stateful OBS mock for final_regression_tests**

- **Why:** `tests-structure-02` (high), `tests-structure-03` (high), `tests-structure-15` (medium)
- **Commit type:** `test:` | **Size:** M | **Risk:** Low
- **Depends on:** P0-T06, P2-T03 | **Parallelizable with:** P2-T04..P2-T08
- **Files:**
  - `src/controller/final_regression_tests.rs 22-224`
  - `src/controller/app_controller.rs`
  - `src/obs/testing/stateful.rs (new)`
- **Steps:**
  1. Add narrow #[cfg(test)] seams on AppController for reads (controller.session.scope() at 416, 1547, 1558, 1818, 1923, 1944; refresh_task.take() at 1930, 1967) AND for the field replacement `controller.session = SessionController::with_runner(..)` at 1449/1535/1791.
  2. Replace the 8-field Plan flag bag (22-32) with obs::testing::StatefulObs speaking a typed vocabulary; controller tests stop building raw `op:5`/`op:7` frames (89, 303-308, 470-471).
  3. Replace bool triples with enums: mode_echo_fences_activation(at_final_gate, aba) at 653, mode_echo_with_full_gtk_queue at 753, terminal_case(acknowledged, full, reconnect) at 1512 -> GatePoint/Delivery/Channel/Followup. Keep names and assertions.
  4. Optional rule: no `"op":` string literal under src/controller tests.
- **Acceptance criteria:**
  - [ ] final_regression_tests.rs contains no raw protocol frames.
  - [ ] TC equal (35 tests).
- **Verification:** SV + TC (see 2.1)

#### P2-T10

**Split final_regression_tests.rs by owning component**

- **Why:** `tests-structure-02` (high)
- **Commit type:** `test:` | **Size:** L (pure move) | **Risk:** Low
- **Depends on:** P2-T09 | **Parallelizable with:** P2-T04..P2-T08
- **Files:**
  - `src/controller/final_regression_tests.rs (1981 lines)`
- **Steps:**
  1. Move groups to: workspace tests (cancellation), obs requests tests, refresh tests (event loop, graph), state topology tests, scene activation, audio snapshots, session terminal, output tests. Use `#[cfg(test)] mod tests;` children of the owning module (there is no controller/tests module).
  2. Delete the file and the #[path] include at app_controller.rs:460-462.
- **Acceptance criteria:**
  - [ ] TC equal.
- **Verification:** SV + TC (see 2.1)

#### P2-T11

**Table-drive mirrored tests, move domain test helpers into test_support**

- **Why:** `tests-structure-14` (medium), `domain-services-ddd-22` (low)
- **Commit type:** `test:` | **Size:** M | **Risk:** Low
- **Depends on:** P2-T01, P2-T03 | **Parallelizable with:** P2-T04..P2-T10
- **Files:**
  - `src/controller/state/tests/*.rs (7 stream/record twin pairs)`
  - `src/controller/app_controller/tests/*.rs (3 twin pairs)`
  - `src/controller/state/mixer/tests.rs`
  - `src/domain/test_support.rs (new)`
  - `src/domain/meter.rs 81-88, 141-157, 177-182, 215-220`
  - `src/domain/health.rs 290-306`
  - `src/domain/session_report.rs 242, 258`
- **Steps:**
  1. Parametrize stream/record twins over OutputKind; keep case labels in assertion messages. Adjust the TC guard: assert case counts per table instead.
  2. Create domain/test_support.rs (cfg(test)) with shared fixtures (health::tests::stats() etc.); make health's tests module private again; move `#[cfg(test)] pub` meter members into test_support where possible.
- **Acceptance criteria:**
  - [ ] No `pub(crate) mod tests` under src/domain.
  - [ ] Case count per table documented in the PR.
- **Verification:** SV (see 2.1)

### Phase 3 - Boundary prerequisites (break the inverted edges)

Small moves that remove the edges blocking the big splits: obs->controller, ui/state->controller clock, command/event->state, infra->domain.

| ID | Title | Size | Depends on |
|---|---|---|---|
| [P3-T01](#p3-t01) | Move ConnectionInfo into the domain; obs stops importing controller | S | P0-T07 |
| [P3-T02](#p3-t02) | Move unix_ms into infra/clock.rs | S | - |
| [P3-T03](#p3-t03) | Move request/observation DTOs out of controller/state and controller internals | S | - |
| [P3-T04](#p3-t04) | Shared budgets module for controller request deadlines | S | P1-T12 |
| [P3-T05](#p3-t05) | Inject time: a Now value for reducers; name the freshness constants | M | P3-T02, P2-T01 |
| [P3-T06](#p3-t06) | Revision and generation value objects with explicit overflow policy; document the two fences (ADR) | M | P2-T01, P2-T02 |
| [P3-T07](#p3-t07) | Small state hygiene: ObsStatus::is_connected, explicit re-exports | S | P2-T01 |
| [P3-T08](#p3-t08) | Make infra a leaf: i18n::init takes a locale tag, not domain::appearance::Language | S | P0-T07 |

#### P3-T01

**Move ConnectionInfo into the domain; obs stops importing controller**

- **Why:** `architecture-global-05` (medium), `obs-acl-04` (high), `security-16` (low), `recent-change-review-16` (medium)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** P0-T07 | **Parallelizable with:** P3-T02, P3-T03
- **Files:**
  - `src/controller/event.rs 10-18`
  - `src/domain/obs.rs`
  - `src/obs/client.rs 14, 185-195`
  - `src/obs/mapper.rs 12, 20-30`
  - `src/architecture_tests/rules.rs`
- **Steps:**
  1. Move ConnectionInfo as-is (String fields) to src/domain/obs.rs next to ObsNamedList; keep `pub use crate::domain::obs::ConnectionInfo;` in controller/event.rs for one PR (13 files reference it).
  2. Remove the KNOWN_DEBT exemption from obs_adapter_depends_only_on_domain_and_infra (P0-T07).
  3. Follow-up in the same task if small, else P6-T07: obs returns `ObsServerInfo { obs_version, websocket_version, capabilities, image_formats }`; session_controller composes ConnectionInfo from (ConnectionId, Endpoint, ObsServerInfo); map_version stops stamping 'default'/empty endpoint (mapper.rs:21-29).
- **Acceptance criteria:**
  - [ ] obs/ has zero imports of crate::controller (rule enforces it).
- **Verification:** SV (see 2.1)

#### P3-T02

**Move unix_ms into infra/clock.rs**

- **Why:** `architecture-global-06` (medium), `ctrl-22` (medium)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** P3-T01, P3-T03
- **Files:**
  - `src/controller/workspace.rs 569-575`
  - `src/infra/clock.rs (new)`
  - `about 30 call sites: ui/{bookmarks,report_history,navigation,session_reports,workspace,event_reconciliation,window}.rs, controller/state.rs (382, 758, 835, 854, 932), controller/state/{session_reports (66, 120, 213), bookmarks (84)}.rs`
- **Steps:**
  1. Create infra/clock.rs `pub(crate) fn unix_ms() -> u64`; re-export from controller::workspace for one PR; sweep callers; remove the re-export.
  2. Note: the storage/bookmarks.rs:89 caller is test-only (removed in P0-T14). The real problem is GTK-side state and ui depending on a Tokio controller module for a clock.
- **Acceptance criteria:**
  - [ ] No `controller::workspace::unix_ms` reference remains.
- **Verification:** SV (see 2.1)

#### P3-T03

**Move request/observation DTOs out of controller/state and controller internals**

- **Why:** `architecture-global-08` (medium)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** P3-T01, P3-T02
- **Files:**
  - `src/controller/state/mixer.rs 108-119 (MixerAudioRequest)`
  - `src/controller/command.rs 6-51`
  - `src/controller/event.rs 273, 327, 331`
  - `src/controller/app_controller.rs 48, 293`
  - `src/controller/workspace.rs 22-110 (WorkspaceRequest/WorkspaceObservation)`
  - `src/controller/thumbnails.rs (ThumbnailRequest)`
  - `src/domain/{mixer.rs, workspace.rs or control.rs}`
- **Steps:**
  1. Move MixerAudioRequest to src/domain/mixer.rs; ThumbnailRequest and WorkspaceRequest/WorkspaceObservation to domain (domain/workspace.rs; rename comes in P4-T01 glossary work).
  2. Keep `pub use` aliases for one PR. This is the hard prerequisite of P8-T01: without it, moving state creates a controller<->state cycle.
- **Acceptance criteria:**
  - [ ] grep `controller::state::MixerAudioRequest` in src/controller returns 0.
- **Verification:** SV (see 2.1)

#### P3-T04

**Shared budgets module for controller request deadlines**

- **Why:** `ctrl-23` (low)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** P1-T12 | **Parallelizable with:** P3-T01..P3-T03
- **Files:**
  - `src/controller/runtime/budgets.rs (new)`
  - `src/controller/session_controller.rs (REQUEST_BUDGET)`
  - `src/controller/output_controller.rs:11`
  - `src/controller/refresh_controller.rs:5`
  - `src/controller/workspace.rs:431`
- **Steps:**
  1. Move REQUEST_BUDGET (imported by three siblings from session_controller) and the named capacities from P1-T12 into controller/runtime/budgets.rs.
- **Acceptance criteria:**
  - [ ] No controller module imports a budget constant from session_controller.
- **Verification:** SV (see 2.1)

#### P3-T05

**Inject time: a Now value for reducers; name the freshness constants**

- **Why:** `controller-state-12` (medium), `recent-change-review-19` (medium), `domain-services-ddd-17` (low), `controller-state-13` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Medium: many signatures
- **Depends on:** P3-T02, P2-T01 | **Parallelizable with:** P3-T06
- **Files:**
  - `src/infra/clock.rs (Now)`
  - `src/controller/state.rs 186, 247-266, 319-348, 382, 398-405, 755-758, 835, 854, 932, 1168`
  - `src/controller/state/session_reports.rs 64-66, 120, 162-165, 213`
  - `src/controller/state/bookmarks.rs 82-86, 126`
  - `src/ui/event_reconciliation.rs 27, 74, 90, 160, 165`
  - `src/ui/navigation.rs 207-210`
  - `src/domain/stats.rs 162-179`
  - `src/architecture_tests/rules.rs`
- **Steps:**
  1. Add `struct Now { instant: Instant, unix_ms: u64 }` with `Now::capture()` (ui side). Pass it to every reducer that reads a clock today (apply_connected, invalidate_collection_runtime, reset_obs_session, apply_terminal_evidence, set_obs_stats_at - keep its separate `now` argument semantics -, update_active_since, prepare_report_exit, finish_report, bookmarks capture).
  2. Capture Now at each GTK entry point: event pump apply, NavigationContext::dispatch (note_command_intent), event_reconciliation adapters, page callbacks. One capture per entry.
  3. StatsHistory: make push_at(timestamp, ..) the only public entry; delete push() and migrate tests (domain/stats.rs 360-426, ui/pages/stats.rs 1037-1066).
  4. Name CONTROL_FRESHNESS in domain/control.rs (30 s at 256/327/331 - keep whole-second truncation: 256/327 use `<= 30`, 331 `> 30`), RETIREMENT_HISTORY (937), RECORDING_SAMPLE_MAX_AGE in domain/bookmark.rs (bookmarks.rs:126).
  5. The report id nonce (SystemTime + process::id, 162-168) takes injected values.
  6. Add a rule: no `Instant::now`, `SystemTime::now` or `unix_ms(` under src/controller/state (later src/state).
- **Acceptance criteria:**
  - [ ] Reducer tests use fixed Now values; no sleeps.
  - [ ] The new rule passes.
- **Verification:** SV (see 2.1)

#### P3-T06

**Revision and generation value objects with explicit overflow policy; document the two fences (ADR)**

- **Why:** `controller-state-11` (medium), `controller-state-06` (high)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P2-T01, P2-T02 | **Parallelizable with:** P3-T05
- **Files:**
  - `src/domain/revision.rs (new)`
  - `src/controller/state.rs 236, 368-370, 410, 1000, 1038-1041`
  - `src/controller/state/mixer.rs 131-134, 191-194, 620-638`
  - `src/controller/state/show_registry.rs 50, 84, 115`
  - `src/controller/state/session_reports.rs 58, 154`
  - `src/controller/state/audio_presets.rs 14`
  - `src/controller/observations.rs 64-75`
  - `docs/adr/ (new ADR)`
- **Steps:**
  1. Write an ADR first: the Observations revision fence orders controller-published events by wire revision; the Instant fence (state.rs:605-615) orders events against workspace readback. Both stay. HealthMonitor is downstream, not a fence. Observations reset on collection change (844) while the Instant fence survives (603-604) by design; document it.
  2. Add `Generation(u64)` for equality fences (wrapping OK: WorkspaceState::accepts uses ==, state.rs:433) and `Revision(u64)` for ordering fences (fail-closed or saturating: topology compares `<`, state.rs:1000).
  3. Migrate the counters listed; the mixer #[should_panic] tests (620-638) change in step with the chosen policy.
- **Acceptance criteria:**
  - [ ] One documented policy per counter kind; ADR merged.
- **Verification:** SV (see 2.1)

#### P3-T07

**Small state hygiene: ObsStatus::is_connected, explicit re-exports**

- **Why:** `controller-state-23` (low), `controller-state-18` (medium)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** P2-T01 | **Parallelizable with:** P3-T01..P3-T06
- **Files:**
  - `src/controller/state.rs 20-30`
  - `src/controller/state/{audio_presets.rs 1, 9; mixer.rs 4; bookmarks.rs 81, 107; show_registry.rs 55; session_reports.rs 102}`
- **Steps:**
  1. Add ObsStatus::is_connected() and replace the 5 `matches!(.., ObsStatus::Connected { .. })` sites.
  2. Replace `pub use mixer::*` with an explicit list (include the view-model types MixerVisibleAudioStatus, MixerVisibleRenderSource, MixerInspection*, MixerSceneRefreshTarget* until P8-T14) and `use super::*` in submodules with explicit uses (audio_presets needs fl!, LANGUAGE_LOADER, ObsStatus).
- **Acceptance criteria:**
  - [ ] No glob re-export in controller/state.
- **Verification:** SV (see 2.1)

#### P3-T08

**Make infra a leaf: i18n::init takes a locale tag, not domain::appearance::Language**

- **Why:** `architecture-global-11` (medium; its recommendation: "the real fix is making i18n take a locale code &str"). Added by the critic pass: today `src/infra/i18n.rs:18` imports `crate::domain::appearance::Language`, while the domain imports `infra::i18n` (141 `fl!` calls) and `infra::error`. That is a domain <-> infra module cycle, and section 3.4 had no rule for `infra`.
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** P0-T07 | **Parallelizable with:** P3-T01..P3-T07
- **Files:**
  - `src/infra/i18n.rs 7, 18, 63-90 (init), 210 (test-only loop over Language::ALL)`
  - `src/app.rs 438`
  - `src/ui/pages/settings.rs 429`
  - test call sites: `src/ui/bookmarks.rs 564`, `src/ui/session_reports.rs 505`, `src/ui/report_history/tests.rs 45`, `src/ui/audio_presets/tests.rs 49`, `src/ui/pages/inventory.rs 1957`, `src/ui/widgets/transport_bar_tests.rs 87`, `src/services/doctor_format.rs 346`
  - `src/architecture_tests/rules.rs`
- **Steps:**
  1. Change `pub(crate) fn init(language: Language)` to `pub(crate) fn init(locale_tag: Option<&str>)`. The body already branches on `language.locale_tag()` (i18n.rs:73), so callers pass `config.language.locale_tag()`. `None` keeps today's desktop/C-locale behavior.
  2. Migrate the production callers (app.rs:438, settings.rs:429) and the test callers in the same commit.
  3. The parity test at i18n.rs:210 may keep iterating `Language::ALL`: it is `#[cfg(test)]`, and the scanner exempts test-only imports.
  4. Add rule `infra_is_a_leaf`: `violations("infra", [crate::domain, crate::services, crate::storage, crate::obs, crate::controller, crate::state, crate::ui, crate::presentation], ..)`. P9-T18 later moves the portal client into `infra/portal/`; it must keep passing this rule (shortcut ids are computed by the caller, not in infra).
- **Acceptance criteria:**
  - [ ] `grep -n "crate::domain" src/infra/*.rs` matches only lines inside `#[cfg(test)]` code.
  - [ ] `infra_is_a_leaf` passes on the branch and fails if `use crate::domain::appearance::Language;` is re-added to i18n.rs (try it, then revert).
  - [ ] IV passes (locale selection unchanged).
- **Verification:** SV + NV + IV (see 2.1)

### Phase 4 - Domain kernel, value objects and bounded contexts

Builds the ubiquitous language in code. Each context move uses `pub use` shims so callers keep compiling.

| ID | Title | Size | Depends on |
|---|---|---|---|
| [P4-T01](#p4-t01) | ADR for bounded contexts and a ubiquitous-language glossary | S | - |
| [P4-T02](#p4-t02) | Shared kernel: OBS identity newtypes, ObsRef and a single Resolution rule | M | P0-T05 |
| [P4-T03](#p4-t03) | Replace the hand-rolled identity resolution copies | S | P4-T02 |
| [P4-T04](#p4-t04) | Replace SceneId/InputId String aliases with newtypes (domain and services) | M | P4-T02 |
| [P4-T05](#p4-t05) | Propagate identity newtypes through controller, obs and ui | M | P4-T04 |
| [P4-T06](#p4-t06) | ObsCapability set built at the OBS boundary | M | P3-T01 |
| [P4-T07](#p4-t07) | Value objects for control ranges; remove range checks from the OBS adapter | M | - |
| [P4-T08](#p4-t08) | Gain and decibel value objects; remove the AudioService Middle Man | M | - |
| [P4-T09](#p4-t09) | GTK-free presentation module for localized labels (part 1: create it, move session/output/role labels) | M | P0-T07 |
| [P4-T10](#p4-t10) | Presentation module part 2: icon, hotkey, control, meter, mixer, audio, appearance, diagnostics labels | M | P4-T09 |
| [P4-T11](#p4-t11) | Presentation part 3: CSS classes and icon names to ui/presenters; ban presentation in domain | M | P4-T10 |
| [P4-T12](#p4-t12) | Domain bounded-context directories (part 1): kernel macro, session, appearance, outputs | L (pure move) | P4-T02 |
| [P4-T13](#p4-t13) | Domain contexts (part 2): program, audio, workspace, diagnostics; then remove shims | L (pure move) | P4-T12 |
| [P4-T14](#p4-t14) | Split ControlAction: data structs out, category(), then nested commands | M | P4-T13 |
| [P4-T15](#p4-t15) | Move OBS wire facts out of the domain | S | P4-T08 |
| [P4-T16](#p4-t16) | Preflight split, MicrophoneChecks, and one freshness/meter-evidence rule | M | P4-T13 |
| [P4-T17](#p4-t17) | Health monitor split: sustained, stats/output/audio alerts, StatsObservation, PerOutput | M | P2-T11, P4-T13 |
| [P4-T18](#p4-t18) | Doctor rules into the domain; edge classification onto the policy | M | P4-T13, P1-T10 |
| [P4-T19](#p4-t19) | Domain invariants by construction: labels, presets, shortcuts, mixer preference, graph | M | P4-T13 |
| [P4-T20](#p4-t20) | Move output, disconnect and stats-severity policies out of UI pages | M | P4-T12 |
| [P4-T21](#p4-t21) | Typed session report health events and end reasons | M | P0-T05 |
| [P4-T22](#p4-t22) | Endpoint, host and connection id value objects | M | P0-T05 |
| [P4-T23](#p4-t23) | Deck slot projection as a domain function | S | P4-T13 |
| [P4-T24](#p4-t24) | OperatorAction::target(), one label builder, Page::ALL | S | - |
| [P4-T25](#p4-t25) | Operator action resolution out of the UI | M | P4-T06, P4-T24, P4-T13 |

#### P4-T01

**ADR for bounded contexts and a ubiquitous-language glossary**

- **Why:** `architecture-global-07` (medium), `domain-services-ddd-18` (low), `architecture-global-18` (medium)
- **Commit type:** `docs:` | **Size:** S | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** P4-T02..P4-T08
- **Files:**
  - `docs/adr/0001-bounded-contexts.md (new; check for an existing ADR convention first)`
  - `docs/ubiquitous-language.md (new)`
  - `docs/architecture.md`
  - `CLAUDE.md (NavigationContext and storage-conversion statements)`
- **Steps:**
  1. Record the contexts, glossary and dependency rules from section 3 of this review.
  2. Fix the two false statements now: CLAUDE.md says pages hold a NavigationContext exposing only switch_to_page() and dispatch() (navigation.rs:20 exposes raw state); docs/architecture.md:186 says storage converts to SceneRegistrySnapshot before services use it.
  3. Rename rules: Rust identifiers only, one concept per PR; on-disk names (show-workspaces.json, serde keys) and user-visible i18n keys stay unchanged (every locale would have to change together).
- **Acceptance criteria:**
  - [ ] ADR and glossary merged; CLAUDE.md no longer contradicts the code.
- **Verification:** SV (see 2.1)

#### P4-T02

**Shared kernel: OBS identity newtypes, ObsRef and a single Resolution rule**

- **Why:** `domain-services-ddd-02` (high), `domain-services-ddd-03` (high)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P0-T05 | **Parallelizable with:** P4-T01, P4-T06..P4-T08
- **Files:**
  - `src/domain/kernel/{mod.rs, identity.rs} (new)`
  - `src/domain/operator_action.rs 6-22`
  - `src/domain/operator_input.rs 61-80`
- **Steps:**
  1. Add newtypes SceneName, InputName (Deref<Target=str>, serde(transparent)) and ObsUuid (non-empty, non-nil).
  2. Add `ObsRef<N> { name: N, uuid: Option<ObsUuid> }` with ActionTarget::matches semantics; `type ActionTarget = ObsRef<String>` keeps serde shape (ActionTarget is persisted in OperatorPin, ActionBinding, AudioRule, PresetInput, PreflightPlan; P0-T05 goldens must stay green).
  3. Add `enum Resolution<'a, T> { Unique(&'a T), Missing, Ambiguous }` and `resolve(items, target)` reproducing operator_input::resolve_input (exactly one identity match AND exactly one input with that name).
  4. Add `targets_distinct(&[ObsRef])` for the different set-distinctness rule (AudioPreset::valid 62-76, AudioMonitor::observe 78-86).
  5. Table test first: rename-by-uuid, name reuse with new uuid, duplicate uuid, duplicate name, name-only targets.
- **Acceptance criteria:**
  - [ ] Kernel compiles with tests; goldens unchanged.
- **Verification:** SV (see 2.1)

#### P4-T03

**Replace the hand-rolled identity resolution copies**

- **Why:** `domain-services-ddd-03` (high)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** P4-T02 | **Parallelizable with:** P4-T04
- **Files:**
  - `src/ui/action_catalog.rs 172-187, 215-226`
  - `src/domain/preflight.rs 131-140, 163-170, 202-205`
  - `src/domain/health/audio.rs 94-103`
- **Steps:**
  1. Switch each copy to kernel::resolve. action_catalog's scene path returns None on ambiguity while preflight maps it to Unknown (139): the Resolution enum keeps both behaviors.
  2. Do not fold the set-distinctness checks into resolve().
- **Acceptance criteria:**
  - [ ] One resolution implementation; guarding tests operator_input.rs:86-126, preflight.rs:344-369, health/audio.rs:389-416 pass.
- **Verification:** SV (see 2.1)

#### P4-T04

**Replace SceneId/InputId String aliases with newtypes (domain and services)**

- **Why:** `domain-services-ddd-02` (high)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P4-T02 | **Parallelizable with:** P4-T03
- **Files:**
  - `src/domain/scene.rs 3-14`
  - `src/domain/audio.rs 39-42, 73-102`
  - `src/domain/control.rs 48-60, 221-285`
  - `src/domain/mixer.rs 120`
  - `src/services/*.rs`
- **Steps:**
  1. Module by module; SceneId 45 and InputId 49 references. serde(transparent) everywhere a value is persisted.
  2. Do NOT collapse AudioInput.id/name/display_name here: check obs/mapper.rs construction first (separate task if wanted).
- **Acceptance criteria:**
  - [ ] Domain and services compile with no `type SceneId = String`.
- **Verification:** SV (see 2.1)

#### P4-T05

**Propagate identity newtypes through controller, obs and ui**

- **Why:** `domain-services-ddd-02` (high)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P4-T04 | **Parallelizable with:** P4-T06..P4-T08
- **Files:**
  - `src/controller/**`
  - `src/obs/**`
  - `src/ui/**`
- **Steps:**
  1. Mechanical, compiler-guided. Split into several PRs by directory if the diff passes 600 lines.
- **Acceptance criteria:**
  - [ ] No remaining String alias for OBS identity.
- **Verification:** SV + NV (see 2.1)

#### P4-T06

**ObsCapability set built at the OBS boundary**

- **Why:** `domain-services-ddd-04` (high), `performance-12` (medium), `uiw-05` (high)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P3-T01 | **Parallelizable with:** P4-T02..P4-T05
- **Files:**
  - `src/domain/capabilities.rs (new)`
  - `src/obs/mapper.rs 27 (map_version)`
  - `src/controller/state.rs 447`
  - `src/ui/widgets/studio_controls.rs 52, 93-96, 214`
  - `src/ui/previews.rs 83-86`
  - `src/ui/widgets/operating_panel.rs 1560-1562`
  - `src/ui/widgets/transport_bar.rs 219-225`
  - `src/ui/audio_presets.rs 118-134`
  - `src/ui/action_catalog.rs 168`
- **Steps:**
  1. `ObsCapabilities { known: KnownRequests (u64 bitset newtype; bitflags is not a dependency), all: Arc<HashSet<String>> }` built in map_version from availableRequests.
  2. Named flags for every request the app checks (SetStudioModeEnabled, GetSourceScreenshot, SetSceneItemEnabled, SetSourceFilterEnabled, SetInputSettings, TriggerMediaInputAction, SetMediaInputCursor, SetInputAudioMonitorType/SyncOffset/Balance/Tracks, SetInputMute, SetInputVolume, SaveReplayBuffer, SetCurrentPreviewScene, SetCurrentProgramScene, ...). Keep `all` for Doctor and dynamic catalog checks.
  3. Replace the five ad hoc helpers (operating_panel::supported, transport_bar::supports, audio_presets::capable, action_catalog closure, studio_controls closure) and previews inline scan with `caps.contains(..)`.
  4. Mapper unit test availableRequests -> flags.
- **Acceptance criteria:**
  - [ ] No request-name string literal outside src/obs and domain/capabilities.rs (add a ratchet).
  - [ ] Doctor tests pass.
- **Verification:** SV + NV (see 2.1)

#### P4-T07

**Value objects for control ranges; remove range checks from the OBS adapter**

- **Why:** `obs-acl-03` (high), `domain-services-ddd-07` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** P4-T02..P4-T06
- **Files:**
  - `src/domain/control/values.rs or src/domain/audio.rs`
  - `src/domain/control.rs 237, 248-251, 265-280`
  - `src/obs/client.rs 846-847, 1008-1009, 1037-1038, 1050-1051`
  - `src/ui/widgets/studio_controls.rs 163`
  - `src/ui/widgets/operating_panel.rs 1395, 1404, 1418`
- **Steps:**
  1. Add TransitionDuration (50..=20000 ms), VolumeDb (MIN_VOLUME_DB..=MAX_VOLUME_DB, finite), AudioSyncOffset (-950..=20000), AudioBalance (finite 0..=1), AudioTracks([bool; 6]), each with TryFrom.
  2. Change the ControlAction fields (not serialized: derives Debug, Clone, PartialEq only).
  3. UI constructors handle TryFrom failure explicitly (clamp to the spin range or toast), never panic; derive spin ranges from the same constants.
  4. Move range tests to domain; keep one wire test (regression gain wire test) proving canonical values reach OBS; keep the SeekMedia fresh-duration check (927-933) in the adapter.
- **Acceptance criteria:**
  - [ ] Adapter contains no numeric range literal for these fields.
- **Verification:** SV + NV (see 2.1)

#### P4-T08

**Gain and decibel value objects; remove the AudioService Middle Man**

- **Why:** `domain-services-ddd-09` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** P4-T02..P4-T07
- **Files:**
  - `src/domain/audio/gain.rs (new)`
  - `src/domain/audio.rs 5-37`
  - `src/domain/meter.rs 99-110`
  - `src/services/audio_service.rs 19-78`
  - `src/services/audio_debounce.rs (new)`
  - `src/ui/widgets/audio_card.rs (31 call sites)`
  - `src/ui/pages/mixer.rs (5)`
- **Steps:**
  1. Add LinearGain and Decibels with named FADER_RANGE (0 dB max by design, audio.rs:8) and PRESET_RANGE (0..=20 linear, positive dB allowed, audio_preset.rs:13): the different ranges are intentional, keep both.
  2. One mul->dB formula (today audio::volume_mul_to_db and meter::db_from_mul).
  3. Move VolumeChangeDebouncer and its constants to services/audio_debounce.rs; turn the forwarders into newtype methods; delete AudioService after migrating its 36 call sites. format_db goes to ui/presenters/audio.rs.
  4. Do NOT derive volume_db from volume_mul (OBS reports both; separate test-first change if ever).
  5. Property test db->mul->db round trip over the fader range.
- **Acceptance criteria:**
  - [ ] services/audio_service.rs deleted.
- **Verification:** SV + NV (see 2.1)

#### P4-T09

**GTK-free presentation module for localized labels (part 1: create it, move session/output/role labels)**

- **Why:** `domain-services-ddd-01` (high), `controller-state-08` (medium), `ui-shell-18` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P0-T07 | **Parallelizable with:** P4-T02..P4-T08
- **Files:**
  - `src/presentation/{mod.rs, outputs.rs, program.rs, session.rs} (new)`
  - `src/domain/{output.rs 28-69, role.rs 27-86}`
  - `src/controller/state.rs 32-123 (Page, ObsStatus)`
  - `src/architecture_tests/rules.rs`
- **Steps:**
  1. Labels cannot go to src/ui: the controller calls OutputKind::label and OutputRunState::label (state.rs:670), PresetOutcome::description (audio_presets.rs:49) and ObsStatus::label (state.rs:755, 859, 861), and background_layers_hold_no_gtk_or_ui forbids crate::ui there.
  2. Create src/presentation/ (depends only on domain + infra::i18n; no gtk/glib). Move label()/description() as free functions or a `Label` trait implemented there, one enum per commit; move English-text test assertions with them.
  3. Rule: domain, services, obs, storage must not import crate::presentation; presentation must not import gtk/glib/ui/controller/storage/obs.
- **Acceptance criteria:**
  - [ ] domain_presentation_ratchet ceiling lowered.
- **Verification:** SV + IV (see 2.1)

#### P4-T10

**Presentation module part 2: icon, hotkey, control, meter, mixer, audio, appearance, diagnostics labels**

- **Why:** `domain-services-ddd-01` (high)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P4-T09 | **Parallelizable with:** P4-T11..P4-T16
- **Files:**
  - `src/presentation/{audio.rs, hotkey.rs, workspace.rs, diagnostics.rs, appearance.rs}`
  - `src/domain/{icon.rs, hotkey.rs 157-182, 398-435, control.rs 118-196, meter.rs 222-251, mixer.rs 22-36, 73-79, audio.rs 53-70, diagnostic.rs 22-63, graph.rs 21-59, appearance.rs 113-126, audio_preset/run.rs 71-113}`
- **Steps:**
  1. Continue one enum per commit. PresetOutcome::description hard-codes glyphs; keep its output identical.
  2. Language::display_name moves to presentation/appearance.rs.
- **Acceptance criteria:**
  - [ ] Zero fl!/LANGUAGE_LOADER under src/domain (ratchet at 0).
- **Verification:** SV + IV (see 2.1)

#### P4-T11

**Presentation part 3: CSS classes and icon names to ui/presenters; ban presentation in domain**

- **Why:** `domain-services-ddd-01` (high), `controller-state-08` (medium), `ui-shell-18` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P4-T10 | **Parallelizable with:** P4-T12..P4-T16
- **Files:**
  - `src/ui/presenters/{mod.rs, connection.rs, diagnostics.rs, graph.rs, audio.rs, icons.rs} (new)`
  - `src/domain/{audio.rs 63-70, diagnostic.rs 32-47, graph.rs 44-53, icon.rs (33 icon_name)}`
  - `src/controller/state.rs 115-122, 136-159, 1074-1084`
  - `src/ui/sidebar.rs 257, 449, 474-479`
  - `src/ui/widgets/status_bar.rs 206-207`
- **Steps:**
  1. Move AudioSourceScope::css_class, DiagnosticSeverity::icon_name/css_class, EdgeStatus::icon_name/css_class, ObsStatus::css_class (+ CONNECTION_CSS_CLASSES from one table), TopologyStatus::notice, active_audio_notice, Page::id/icon_name, IconChoice.icon_name (domain keeps the stable key).
  2. Add rule `domain_has_no_presentation`: no LANGUAGE_LOADER, `fl!(`, `css_class`, `-symbolic` under src/domain (the -symbolic part last).
- **Acceptance criteria:**
  - [ ] `domain_has_no_presentation` passes, and adding `fn css_class()` to any domain enum makes it fail (try, then revert).
- **Verification:** SV + NV (see 2.1)

#### P4-T12

**Domain bounded-context directories (part 1): kernel macro, session, appearance, outputs**

- **Why:** `architecture-global-12` (medium), `domain-services-ddd-18` (low)
- **Commit type:** `refactor:` | **Size:** L (pure move) | **Risk:** Low
- **Depends on:** P4-T02 | **Parallelizable with:** P4-T09..P4-T11
- **Files:**
  - `src/domain/mod.rs 3-27, 44-72`
  - `src/domain/{workflow.rs, bookmark.rs, session_report.rs, stats.rs, appearance.rs, output.rs, output_result.rs}`
- **Steps:**
  1. git mv into domain/session/ (ActivityLog from workflow.rs:13-57, bookmark, session_report, stats), domain/appearance/{theme,language,motion,preference}.rs, domain/outputs/{output,output_result,command}.rs with `pub use` shims in domain/mod.rs.
  2. string_enum_serde! is macro_rules: declare kernel before users or re-export with `pub(crate) use`.
  3. ScopeBinding -> kernel/scope.rs (keep wire field names; it is serde + deny_unknown_fields).
- **Acceptance criteria:**
  - [ ] All callers compile through shims; TC equal.
- **Verification:** SV + TC (see 2.1)

#### P4-T13

**Domain contexts (part 2): program, audio, workspace, diagnostics; then remove shims**

- **Why:** `architecture-global-12` (medium), `domain-services-ddd-18` (low)
- **Commit type:** `refactor:` | **Size:** L (pure move) | **Risk:** Low
- **Depends on:** P4-T12 | **Parallelizable with:** P4-T14..P4-T16
- **Files:**
  - `src/domain/{scene, role, registry, graph, hotkey, obs, audio, meter, mixer, audio_preset/, operator_action, operator_input, action_shortcut, icon, control, diagnostic, health/, preflight}.rs`
- **Steps:**
  1. domain/program/ (scene, role, registry policy, graph, hotkey, obs ObsNamedList, studio), domain/audio/ (gain, input, meter, mixer, preset/, settings), domain/workspace/ (operator_action, operator_input, action_shortcut, source types from control.rs, icon keys, audio_rule), domain/diagnostics/ (diagnostic, doctor, health/, preflight/).
  2. One context per commit with shims; final commit replaces old paths and deletes shims.
  3. Rename WorkspaceWarning -> InventoryReadWarning and `(u64,u64)` health scope -> WorkspaceGeneration (state.rs:957-960) here or in P8.
- **Acceptance criteria:**
  - [ ] domain/mod.rs lists only context modules and kernel.
- **Verification:** SV + TC (see 2.1)

#### P4-T14

**Split ControlAction: data structs out, category(), then nested commands**

- **Why:** `domain-services-ddd-08` (medium), `obs-acl-02` (high)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P4-T13 | **Parallelizable with:** P4-T15, P4-T16
- **Files:**
  - `src/domain/control.rs 7-22, 24-116, 198-297`
  - `18 files referencing ControlAction (ui/workspace.rs 40-77, ui/widgets/studio_controls.rs 163, operating_panel.rs 1395/1418, obs/client.rs)`
- **Steps:**
  1. Move data structs: StudioState -> program/studio.rs; GlobalToggle -> outputs/command.rs; AudioSettings, AudioMonitor (rename AudioMonitoring: health::audio::AudioMonitor exists) -> audio/settings.rs; ControlSnapshot, SourceItem, SourceDetail, SourceFilter, MediaStatus, MediaState -> workspace/source.rs. Re-export from control.rs.
  2. Add non-breaking ControlAction::category() (used by the obs dispatcher in P6-T02).
  3. Later commits: nest `ControlAction { Program(..), Output(..), Audio(..), Source(..) }` one context at a time with From impls, starting with Audio. No serde impact.
- **Acceptance criteria:**
  - [ ] control.rs holds only the transitional enum + From impls.
- **Verification:** SV + NV (see 2.1)

#### P4-T15

**Move OBS wire facts out of the domain**

- **Why:** `domain-services-ddd-07` (medium)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** P4-T08 | **Parallelizable with:** P4-T14, P4-T16
- **Files:**
  - `src/domain/control.rs 287-297 (is_text_kind)`
  - `src/domain/meter.rs 89-97, 104-110, 127-139 (from_mul)`
  - `src/obs/mapper.rs 183`
  - `src/obs/client.rs 652, 967`
- **Steps:**
  1. Move is_text_kind and its test (302-308) to obs/mapper.rs; replace it in the domain by a SourceCapability computed in the mapper.
  2. Move ChannelLevel::from_mul, db_from_mul and InputLevels::from_mul validity to obs/mapper.rs (only production caller 183); add InputLevels::new; keep a #[cfg(test)] from_mul builder in domain/test_support for tests in health/audio.rs, meter_service.rs, ui/preflight/tests.rs.
  3. Document gain tolerance on LinearGain (PresetInput::same_values f32 cast, run.rs:39-46).
- **Acceptance criteria:**
  - [ ] domain has no OBS kind ids or wire shapes.
- **Verification:** SV (see 2.1)

#### P4-T16

**Preflight split, MicrophoneChecks, and one freshness/meter-evidence rule**

- **Why:** `domain-services-ddd-11` (medium), `domain-services-ddd-10` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P4-T13 | **Parallelizable with:** P4-T14, P4-T15, P4-T17
- **Files:**
  - `src/domain/preflight.rs (504) -> domain/diagnostics/preflight/{plan,report,review,checks}.rs`
  - `src/domain/health.rs 143-149, 234-244`
  - `src/domain/health/audio.rs 104-118, 132-146`
  - `src/ui/preflight.rs 366-368`
- **Steps:**
  1. Characterization test first for the absent-vs-ambiguous microphone path (202-207).
  2. MicrophoneChecks { present, unmuted, signal } with iter() and From<MicrophoneChecks> for [CheckStatus; 3]; migrate ui/preflight.rs and ready() (216-223) in the same commit.
  3. Extract check_collection/scenes/disk/microphone and an explicit record_signal (no hidden self.signal_at mutation).
  4. kernel/observation.rs fresh_within (preflight 85-88 = health 146-149); SIGNAL_FLOOR_DB (-50 at preflight 191, health/audio 135); InputLevelSnapshot::trusted(); decide peak_db vs input_peak_db explicitly (ask maintainer). Keep preflight 2 s vs health METER_GAP windows as parameters; keep preflight minimum disk (user u64) and LOW_DISK_MIB (f64 alert) as separate concepts.
- **Acceptance criteria:**
  - [ ] evaluate is orchestration only (< 40 lines).
- **Verification:** SV + NV (see 2.1)

#### P4-T17

**Health monitor split: sustained, stats/output/audio alerts, StatsObservation, PerOutput**

- **Why:** `domain-services-ddd-12` (medium), `domain-services-ddd-13` (low), `performance-16` (low)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P2-T11, P4-T13 | **Parallelizable with:** P4-T16, P4-T18
- **Files:**
  - `src/domain/health.rs (576)`
  - `src/domain/health/events.rs`
  - `src/domain/health/audio.rs`
  - `src/controller/state.rs 1167-1168`
  - `src/controller/state/session_reports.rs 347`
  - `src/ui/health_alerts.rs 40`
- **Steps:**
  1. Follow the split: diagnostics/health/{mod, sustained, stats_alerts, output_alerts, audio_alerts}.rs; AudioRule -> workspace/audio_rule.rs; PerOutput<T> in outputs/ replaces [OutputWatch; 2] + index().
  2. StatsObservation parameter object (single production caller state.rs:1167-1168); invalidate_baseline(kind) replaces events.rs:105-111 reaching into private samples.
  3. HealthMonitor.audio is pub and read from session_reports.rs:347 and ui/health_alerts.rs:40: keep a type alias when renaming AudioMonitor -> AudioAlerts.
  4. Performance (PF16): precompute rule uniqueness and resolved names when rules or scope change; observe on new sequence OR 250 ms elapsed (silence detection needs time to pass).
  5. Optional cleanup (DD13, low): key audio alerts by AudioRuleId/ObsRef instead of index; no correctness bug today.
- **Acceptance criteria:**
  - [ ] observe_stats takes one parameter object; no cross-file private mutation.
- **Verification:** SV (see 2.1)

#### P4-T18

**Doctor rules into the domain; edge classification onto the policy**

- **Why:** `domain-services-ddd-15` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P4-T13, P1-T10 | **Parallelizable with:** P4-T17
- **Files:**
  - `src/services/graph_service.rs 11-53`
  - `src/services/doctor_service.rs 14-175`
  - `src/services/doctor_format.rs (329 --exact literal)`
  - `src/domain/registry.rs 39-110`
  - `src/ui/pages/{doctor.rs 141, graph.rs}`
  - `src/obs/regression_tests.rs 308 (or its new home)`
- **Steps:**
  1. classify_edge -> GraphDependencyPolicy::classify (program/policy.rs) with a forwarder; rules -> domain/diagnostics/doctor.rs keeping DoctorService::run as a forwarder; doctor_format -> ui/presenters/diagnostics.rs (only ui consumes it), updating the --exact test path literal at doctor_format.rs:329.
  2. Delete the old files; rewrite services/mod.rs docs.
- **Acceptance criteria:**
  - [ ] services/ holds only real services (meter, hotkey, debounce, graph layout).
- **Verification:** SV (see 2.1)

#### P4-T19

**Domain invariants by construction: labels, presets, shortcuts, mixer preference, graph**

- **Why:** `domain-services-ddd-06` (high), `domain-services-ddd-21` (low), `domain-services-ddd-19` (low), `domain-services-ddd-20` (low)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P4-T13 | **Parallelizable with:** P4-T17, P4-T18
- **Files:**
  - `src/domain/kernel/label.rs (new)`
  - `src/domain/audio_preset.rs 37-77`
  - `src/domain/audio_preset/run.rs 10-38`
  - `src/domain/action_shortcut.rs 8-12, 32-81`
  - `src/domain/hotkey.rs 48-88, 143-154`
  - `src/ui/actions.rs 22-49`
  - `src/domain/mixer.rs 104-134`
  - `src/domain/graph.rs 62-242`
  - `src/domain/bookmark.rs 35-43`
- **Steps:**
  1. OperatorLabel with two explicit policies (Trimmed max 120 for bookmarks, Exact max 80 for presets); do not unify without maintainer sign-off.
  2. AudioPreset private fields + try_new (grep `preset.inputs` first); PresetPlan::is_fresh(now) replaces elapsed() (run.rs:25).
  3. ActionShortcut::try_new(key, modifiers): the invariant needs both (non-F keys need ctrl/alt/logo).
  4. Modifiers::CTRL/ALT/SHIFT/SUPER consts + `with`; RESERVED_APP_SHORTCUTS in domain; ui/actions.rs derives accels via a key->accel mapping (`comma` vs `,`) or a test asserts the lists agree.
  5. MixerSelection -> MixerPreference (persisted) + MixerSession (search, locked_inputs); derive Default.
  6. graph.rs: tests to the end, extract reverse_adjacency/finish_order/components from cycle_edges, private edges behind from_edges/insert_edge (grep `SceneGraph {` and `.edges`).
- **Acceptance criteria:**
  - [ ] No public-field struct with a separate valid() remains in these modules.
- **Verification:** SV + NV (see 2.1)

#### P4-T20

**Move output, disconnect and stats-severity policies out of UI pages**

- **Why:** `architecture-global-09` (medium), `ui-pages-15` (medium), `ui-pages-07` (high), `ctrl-24` (low)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P4-T12 | **Parallelizable with:** P4-T17..P4-T19
- **Files:**
  - `src/ui/pages/live.rs 27, 358-392, 524-540`
  - `src/ui/pages/stats.rs 32-54, 846-912`
  - `src/ui/widgets/chart.rs 44-87`
  - `src/controller/event.rs 20-88`
  - `src/controller/output_controller.rs 16-18`
  - `src/domain/outputs/policy.rs (new)`
  - `src/domain/stats.rs`
- **Steps:**
  1. live.rs requires_disconnect_confirmation, output_running, disconnect_stops_outputs, requires_output_confirmation, output_action_for_active_state and OutputAction -> domain/outputs/policy.rs. OutputSafety becomes a domain value; storage converts OutputConfig. disconnect_label stays in ui (fl!).
  2. Stats thresholds (34-54), frame_budget_ms (860), fps/frame_time/dropped classifiers (867-912) -> domain/stats.rs `HealthSeverity {Unknown, Good, Warning, Critical}`. fps_target (853) stays in ui (reads AppState). chart::Severity keeps label/color as a ui extension over HealthSeverity. Do NOT merge with DiagnosticSeverity {Info, Warning, Error}.
  3. Stats gauge uses a cumulative ratio while domain/health uses windowed deltas: document the two semantics, do not force one constant.
  4. fallback_status_after_failed_output_command -> OutputStatus::settled_after_failed_command in domain/outputs; event.rs test module (90-216) moves to the end.
- **Acceptance criteria:**
  - [ ] live.rs and stats.rs contain no policy thresholds.
- **Verification:** SV + NV (see 2.1)

#### P4-T21

**Typed session report health events and end reasons**

- **Why:** `domain-services-ddd-14` (medium), `recent-change-review-09` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Medium: persisted format
- **Depends on:** P0-T05 | **Parallelizable with:** P4-T17..P4-T20
- **Files:**
  - `src/domain/session_report.rs 39-59`
  - `src/controller/state/session_reports.rs 336-388`
  - `src/controller/state.rs 755, 861`
  - `src/ui/report_history.rs 57, 78, 105`
  - `src/ui/session_reports.rs`
- **Steps:**
  1. Golden test first (P0-T05 report fixture).
  2. ReportEvent::Health carries `kind: HealthAlertKind` and `phase: Option<AlertPhase>` with serde snake_case keeping today's wire strings; mapping becomes `impl From<&Alert>`.
  3. Ended.reason today stores LOCALIZED labels (ObsStatus::Connecting.label(), fl!(history-app-closed)). Add `ReportEndReason { Reconnected, AppClosed, ExitRequested, Disconnected, Other(String) }` with a fallback so old reports deserialize; localize at render in ui/session_reports.rs. Maintainer must approve the content change.
- **Acceptance criteria:**
  - [ ] New reports store stable codes; old fixture still loads.
- **Verification:** SV + IV (see 2.1)

#### P4-T22

**Endpoint, host and connection id value objects**

- **Why:** `security-03` (medium), `recent-change-review-15` (medium), `ui-pages-19` (medium), `storage-infra-09` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P0-T05 | **Parallelizable with:** P4-T17..P4-T21
- **Files:**
  - `src/domain/connection/{mod.rs, endpoint.rs}`
  - `src/storage/config.rs 99-105, 161, 358`
  - `src/storage/config/connections.rs 40, 43, 61-86`
  - `src/storage/secret.rs 21`
  - `src/obs/client.rs 99-100`
  - `src/obs/mapper.rs 23`
  - `src/controller/state.rs 788-796`
  - `src/storage/show_workspaces.rs 10-15`
  - `src/ui/pages/settings/connections.rs 186-234`
  - `src/ui/persist.rs 317-328`
- **Steps:**
  1. ObsHost (Ip or lowercase Dns, trailing dot stripped, rejects `/ @ ? # [ ] :` and whitespace), ObsEndpoint { host, port: NonZeroU16 } with FromStr/Display (brackets IPv6), EndpointKey from ObsEndpoint::key() - the `host:port` text is an on-disk workspace key: keep the exact format for existing hosts (golden test).
  2. ConnectionId newtype with legacy_default()/is_legacy_default() replacing the 6 production `"default"` literals.
  3. Endpoint parse errors (EmptyHost, InvalidPort, EmptyName) each with a Fluent message in every locale; connections.rs save_fields uses it (removes port.unwrap() at 214).
- **Acceptance criteria:**
  - [ ] `format!("{}:{}"` for endpoints appears only inside ObsEndpoint.
- **Verification:** SV + NV + IV (see 2.1)

#### P4-T23

**Deck slot projection as a domain function**

- **Why:** `ui-pages-06` (high), `performance-04` (high)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** P4-T13 | **Parallelizable with:** P4-T17..P4-T22
- **Files:**
  - `src/ui/pages/live.rs 606-654 (tests 1307-1392 - confirm)`
  - `src/ui/hotkeys.rs 145, 161`
  - `src/domain/program/deck.rs (new)`
  - `src/storage/registry.rs (deck_view())`
- **Steps:**
  1. domain::registry::SceneRegistrySnapshot has no order/favorite/filter: add a domain input `SceneDeckView { ordered_ids, roles, favorites, filter }` built by storage `SceneRegistry::deck_view()` (or by ShowCuration after P5-T06).
  2. deck_projection(&SceneInventory, &SceneDeckView) -> Vec<DeckSlot> and scene_for_slot; re-export from live temporarily; migrate hotkeys and window.
  3. Do NOT use it for inventory: inventory lists all ordered+filtered scenes, live filters to is_live_switchable roles.
- **Acceptance criteria:**
  - [ ] live.rs keeps no slot ordering logic.
- **Verification:** SV + NV (see 2.1)

#### P4-T24

**OperatorAction::target(), one label builder, Page::ALL**

- **Why:** `uiw-04` (high), `uiw-18` (medium), `recent-change-review-10` (medium)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** P4-T17..P4-T23
- **Files:**
  - `src/domain/operator_action.rs`
  - `src/ui/action_catalog.rs 66-147, 289-461, 513-616, 619-666`
  - `src/ui/sidebar.rs 22-31`
  - `src/controller/state.rs 35-47`
- **Steps:**
  1. Add OperatorAction::target() -> Option<&ActionTarget>. The three matches differ (99-104 and 207-215 exclude Scene; 647-655 includes Scene but not OpenSource): keep that per call site with an explicit filter, or two methods.
  2. One action_label(state, action, display_name) used by entries and binding_entry (536-609 duplicates).
  3. Page::ALL; derive NAV_PAGES (8, no Sources - intentional) and the palette list (9) with explicit filters.
  4. Remove the unreachable!() at 634-637 with let-else.
- **Acceptance criteria:**
  - [ ] No duplicated label strings in action_catalog.
- **Verification:** SV + NV (see 2.1)

#### P4-T25

**Operator action resolution out of the UI**

- **Why:** `domain-services-ddd-04` (high), `recent-change-review-10` (medium), `uiw-04` (high), `architecture-global-09` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P4-T06, P4-T24, P4-T13 | **Parallelizable with:** P5
- **Files:**
  - `src/ui/action_catalog.rs 35-65, 148-288`
  - `src/domain/workspace/resolution.rs (new)`
  - `src/controller/state/action_context.rs (new; later src/state/)`
  - `tests ui/action_catalog.rs 795-1137`
- **Steps:**
  1. Resolution reads storage SceneRegistry and controller RegistryStatus/WorkspaceState today, so it cannot go to domain as is. Split: AppState::resolution_context() builds a `ResolutionContext` of domain values (capabilities, registry readiness, is_live_switchable roles, observed inputs, filters); `OperatorAction::resolve(&ctx) -> Option<ResolvedAction>` lives in domain/workspace/resolution.rs.
  2. observed_label returns a typed observed state; ui localizes.
  3. Port the tests to the domain; keep a thin ui test for rendering.
- **Acceptance criteria:**
  - [ ] action_catalog.rs keeps only rendering (labels) and execute (nav dispatch).
- **Verification:** SV + NV (see 2.1)

### Phase 5 - Storage anti-corruption layer

Storage keeps serde DTOs. The domain gets real aggregates. Golden fixtures from P0-T05 guard every step.

| ID | Title | Size | Depends on |
|---|---|---|---|
| [P5-T01](#p5-t01) | Storage error taxonomy and structured AppError variants | M | - |
| [P5-T02](#p5-t02) | One JSON document helper for validate-serialize-size-check-atomic-write | M | P0-T05, P5-T01 |
| [P5-T03](#p5-t03) | Split storage/registry.rs internally (no domain move yet) | L (pure move) | P2-T05 |
| [P5-T04](#p5-t04) | SceneRegistry fields grouped into contiguous sections | S | P5-T03 |
| [P5-T05](#p5-t05) | ShowCuration domain aggregate (part 1): types, mappers, golden parity | M | P5-T04, P4-T13 |
| [P5-T06](#p5-t06) | ShowCuration (part 2): move behavior from SceneRegistry and from GTK closures | M | P5-T05 |
| [P5-T07](#p5-t07) | ShowCuration (part 3): AppState and UI hold the domain aggregate; ratchet ui->storage | M | P5-T06, P0-T07 |
| [P5-T08](#p5-t08) | DTOs for persisted domain types; serde attributes leave the domain | M | P5-T05 |
| [P5-T09](#p5-t09) | AccentColor and IconKey value objects; remove dead LiveConfig role lists | S | P5-T05 |
| [P5-T10](#p5-t10) | Config storage split: migration table, notice rendering out of storage, sectioned AppConfig | M | P1-T09, P4-T22 |
| [P5-T11](#p5-t11) | SessionHistory and BookmarkLog domain aggregates; storage keeps archives as DTOs | M | P5-T08 |
| [P5-T12](#p5-t12) | Legacy workspace adoption rule moves to storage | S | P4-T22 |
| [P5-T13](#p5-t13) | CSV export with a typed row | S | - |
| [P5-T14](#p5-t14) | Typed store handles that capture their serial lane | S | P5-T11 |

#### P5-T01

**Storage error taxonomy and structured AppError variants**

- **Why:** `storage-infra-08` (medium), `storage-infra-07` (medium), `security-17` (low)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** P5-T02, P5-T03
- **Files:**
  - `src/storage/error.rs (new)`
  - `src/infra/error.rs 9-83`
  - `src/storage/{show_workspaces.rs 36-48, 81, 121-150; registry.rs 129-165; config.rs 380-390; bookmarks.rs 45-77; report_history.rs 34-36, 150-153; secret.rs 14-20}`
  - `i18n/*/scenedeck.ftl`
- **Steps:**
  1. StorageError with an explicit Missing variant (callers rely on NotFound: RegistryTarget::read, show_registry 'show workspace is missing'); convert one store at a time; stop flattening RegistryStorageError into io::Error::other.
  2. AppError: add AppError::Storage(StorageError), AppError::Credential(CredentialError{Locked, Unavailable, NotFound, Backend}) mapped in storage/secret.rs, AppError::Auth mapped in obs connect; keep sources as Arc<..> (AppError must stay Clone for AppEvent).
  3. Delete the cfg(test) Config variant and remove the 'error-config' key from every locale.
  4. user_message stays callable from controller and obs: move it to infra/error_text.rs if separated (NOT ui).
- **Acceptance criteria:**
  - [ ] No io::Error::other(RegistryStorageError..) left.
- **Verification:** SV + IV (see 2.1)

#### P5-T02

**One JSON document helper for validate-serialize-size-check-atomic-write**

- **Why:** `storage-infra-24` (low)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P0-T05, P5-T01 | **Parallelizable with:** P5-T03
- **Files:**
  - `src/storage/json_store.rs (new)`
  - `src/storage/{registry.rs 640-652, config.rs 380-405, bookmarks.rs 67-79, show_workspaces.rs 152-157, report_history.rs 124-158}`
- **Steps:**
  1. Trait with read_doc and replace_doc only, plus a before-replace hook (report_history compare-and-swap; workspace store refuse-corrupt). No generic repository.
- **Acceptance criteria:**
  - [ ] Five stores use the helper; behavior identical (goldens).
- **Verification:** SV (see 2.1)

#### P5-T03

**Split storage/registry.rs internally (no domain move yet)**

- **Why:** `security-19` (medium), `storage-infra-22` (low), `recent-change-review-06` (medium)
- **Commit type:** `refactor:` | **Size:** L (pure move) | **Risk:** Low
- **Depends on:** P2-T05 | **Parallelizable with:** P5-T01, P5-T02
- **Files:**
  - `src/storage/registry.rs (production ~692) -> storage/registry/{scene_entry.rs, curation.rs, ordering.rs, rules.rs, json_file.rs, yaml_export.rs, import.rs} next to existing decks.rs and identity.rs`
- **Steps:**
  1. Keep registry.rs as module root (2018 layout; decks/identity stay).
  2. Pure moves first; then update_scene_entry(scene, |e| -> bool) replacing the set_* get_mut/is_empty/remove dance (277-310, 318-343, 398-437, 449-475); compare before insert so clearing on a missing entry creates nothing (comment at 460-461).
  3. RuleConfig stays in storage (moving it to domain would make domain name storage types).
- **Acceptance criteria:**
  - [ ] registry.rs < 200 production lines.
- **Verification:** SV + TC (see 2.1)

#### P5-T04

**SceneRegistry fields grouped into contiguous sections**

- **Why:** `storage-infra-02` (high), `recent-change-review-06` (medium)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** P5-T03 | **Parallelizable with:** P5-T05
- **Files:**
  - `src/storage/registry.rs 22-72`
- **Steps:**
  1. Group the 12 fields into sections (curation: scenes, scene_uuids, retained_scenes, scene_order | views: scene_filter, decks | operator: action_bindings, quick_actions | audio: audio_alerts, audio_presets, inputs | rules) with #[serde(flatten)] keeping declaration order so key order is unchanged.
  2. Golden fixtures must be byte-stable for JSON (compare text, not only Value, for this task).
- **Acceptance criteria:**
  - [ ] Goldens unchanged.
- **Verification:** SV (see 2.1)

#### P5-T05

**ShowCuration domain aggregate (part 1): types, mappers, golden parity**

- **Why:** `architecture-global-03` (high), `storage-infra-01` (high), `recent-change-review-06` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P5-T04, P4-T13 | **Parallelizable with:** P5-T07..P5-T09
- **Files:**
  - `src/domain/curation/{mod.rs, show_curation.rs, accent.rs} (or grow domain/registry/ - decide once, see open questions)`
  - `src/domain/registry.rs (SceneRegistrySnapshot, SceneMetadata, GraphDependencyPolicy)`
  - `src/storage/registry/dto.rs`
- **Steps:**
  1. Do not keep two 'registry' vocabularies: either grow domain/registry.rs into domain/curation/ and move the snapshot types there, or the reverse. Record the choice in the ADR.
  2. Define ShowCuration { curation: SceneCuration, views: DeckViews, operator: OperatorSetup, audio: AudioSetup } with private fields.
  3. storage keeps SceneRegistryDto with field-for-field From/TryFrom so on-disk JSON/YAML stays identical (goldens).
- **Acceptance criteria:**
  - [ ] Round trip DTO -> ShowCuration -> DTO equals the fixture.
- **Verification:** SV (see 2.1)

#### P5-T06

**ShowCuration (part 2): move behavior from SceneRegistry and from GTK closures**

- **Why:** `architecture-global-03` (high), `storage-infra-01` (high), `ui-pages-05` (high)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P5-T05 | **Parallelizable with:** P5-T07..P5-T09
- **Files:**
  - `src/storage/registry/*.rs (rename, set_*, ordered_scene_ids 215, snapshot 252, policy 537)`
  - `src/ui/pages/inventory.rs 84-114, 1025-1047, 1086-1090, 1120-1169, 1315-1318, 1384-1431`
- **Steps:**
  1. Move rename/set_*/ordered_scene_ids/snapshot onto ShowCuration.
  2. Add remove_retained(&RetainedScene) -> bool, remove_stale_entry(&str) -> bool, move_scene(inventory_ids, source, target, Placement{Before, After}) -> bool with the filter guard inside, stale_entries(&SceneInventory) (computed twice today: 107-114 and 1025-1047).
  3. Unit tests for each; inventory.rs callbacks call them.
- **Acceptance criteria:**
  - [ ] inventory.rs has no curation rule logic.
- **Verification:** SV + NV (see 2.1)

#### P5-T07

**ShowCuration (part 3): AppState and UI hold the domain aggregate; ratchet ui->storage**

- **Why:** `architecture-global-03` (high), `storage-infra-01` (high), `controller-state-15` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P5-T06, P0-T07 | **Parallelizable with:** P5-T08, P5-T09
- **Files:**
  - `src/controller/state.rs 15-16, 453-456`
  - `ui files importing storage::registry (38 refs in 9 files)`
  - `src/architecture_tests/rules.rs`
- **Steps:**
  1. AppState.registry becomes ShowCuration; storage converts at read/write.
  2. Add `ui_storage_imports_ratchet` (95 refs in 20 ui files importing any storage module today) allowing ui/background_io.rs and ui/persist*.rs (CLAUDE.md sanctions GTK-side persistence).
- **Acceptance criteria:**
  - [ ] No ui page imports storage::registry.
- **Verification:** SV + NV (see 2.1)

#### P5-T08

**DTOs for persisted domain types; serde attributes leave the domain**

- **Why:** `domain-services-ddd-05` (high)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Medium: persisted formats
- **Depends on:** P5-T05 | **Parallelizable with:** P5-T09..P5-T12
- **Files:**
  - `src/storage/registry/dto/{action_binding,operator_pin,audio_rule,audio_preset}.rs`
  - `src/storage/show_workspaces/dto.rs`
  - `src/storage/report_export/dto.rs`
  - `src/domain/{audio_preset.rs 14-35, 80-131; health.rs 5-15; action_shortcut.rs 8-22; operator_action.rs 24-62; preflight.rs 6-39; appearance.rs 284-290; mixer.rs 104-121; workflow.rs 6-11}`
- **Steps:**
  1. TryFrom<Dto> for Domain and From<&Domain> for Dto; move deserialize_presets (audio_preset.rs:80) into the storage DTO; move the audio_preset.rs:99-131 round-trip test into storage.
  2. Closed-set string enums may keep serde in the domain: document the exception in the ADR.
  3. Only then extend domain_and_services_do_not_reach_into_storage to cover #[cfg(test)] code.
- **Acceptance criteria:**
  - [ ] No deny_unknown_fields / try_from / legacy rename attributes left on domain structs (except documented string enums).
- **Verification:** SV (see 2.1)

#### P5-T09

**AccentColor and IconKey value objects; remove dead LiveConfig role lists**

- **Why:** `storage-infra-04` (medium)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** P5-T05 | **Parallelizable with:** P5-T08, P5-T10..P5-T12
- **Files:**
  - `src/storage/registry.rs 96-104, 115-126, 504-528, 571-581`
  - `src/storage/config.rs 139-153`
  - `src/domain/icon.rs`
  - `docs/configuration.md`
- **Steps:**
  1. AccentColor with string serde and lenient parse (invalid hand edits become None); IconKey newtype next to icon_choice.
  2. LiveConfig role lists are never read: delete them (keep serde accepting the old keys) and fix the docs drift.
- **Acceptance criteria:**
  - [ ] 2 UI files use AccentColor; no dead config fields.
- **Verification:** SV + NV (see 2.1)

#### P5-T10

**Config storage split: migration table, notice rendering out of storage, sectioned AppConfig**

- **Why:** `storage-infra-15` (low), `storage-infra-12` (low), `storage-infra-09` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P1-T09, P4-T22 | **Parallelizable with:** P5-T08, P5-T09, P5-T11
- **Files:**
  - `src/storage/config.rs (771) -> storage/config/{presentation.rs, migrations.rs, file.rs} next to connections.rs/preflight.rs`
  - `src/ui/startup_notice.rs (new)`
  - `src/app.rs 439`
- **Steps:**
  1. Migration step table plus a separate normalize() (replace the while/match with a no-op step and unreachable arm, 305-374).
  2. ConfigStartupNotice rendering (fl! at 206-228) moves to ui/startup_notice.rs; add rule: storage must not name LANGUAGE_LOADER.
  3. AppConfig sections (15 fields, denormalized `obs` endpoint kept in sync by hand): keep deserialize_connections (37) and legacy_theme_mode rename+skip_serializing (61) working, proven by goldens. Move CustomCssPreference.legacy_path into the config DTO.
- **Acceptance criteria:**
  - [ ] storage has no fl!.
- **Verification:** SV + IV (see 2.1)

#### P5-T11

**SessionHistory and BookmarkLog domain aggregates; storage keeps archives as DTOs**

- **Why:** `recent-change-review-05` (high), `storage-infra-17` (medium), `performance-17` (low)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P5-T08 | **Parallelizable with:** P5-T09, P5-T10, P5-T12
- **Files:**
  - `src/controller/state/report_history.rs 3-26, 95-140`
  - `src/controller/state/bookmarks.rs 8-46`
  - `src/controller/state/show_registry.rs 5-25`
  - `src/storage/report_history.rs 37-108`
  - `src/storage/bookmarks.rs 30-51`
  - `src/domain/session_report.rs`
  - `src/domain/bookmark.rs`
  - `src/domain/show_workspace.rs (WorkspaceKey, ShowWorkspaceId)`
- **Steps:**
  1. SessionHistory { entries, upsert, retain } and SessionReport::validate in domain; BookmarkLog { next_id, next_session, entries; capture, rename, remove } in domain; WorkspaceKey/ShowWorkspaceId in domain (domain may define types storage uses).
  2. Storage keeps HistoryArchiveDto/BookmarkArchiveDto with mappers; domain validate returns a domain error that storage maps.
  3. Performance: hold reports as Arc<SessionReport> in the domain history, compare by per-report revision or Arc::ptr_eq instead of deep compare every 10 s (ui/report_history.rs:171, 181). Arc inside the DTO would need serde 'rc'; keep Arc on the domain side only.
- **Acceptance criteria:**
  - [ ] controller/state imports no storage archive types.
- **Verification:** SV (see 2.1)

#### P5-T12

**Legacy workspace adoption rule moves to storage**

- **Why:** `recent-change-review-15` (medium), `uiw-06` (high)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** P4-T22 | **Parallelizable with:** P5-T08..P5-T11
- **Files:**
  - `src/ui/persist.rs 316-328`
  - `src/storage/show_workspaces.rs`
- **Steps:**
  1. `RegistryTarget::legacy_source(&self, config: &AppConfig) -> Option<PathBuf>` (or LegacyAdoption::applies(&WorkspaceKey, &AppConfig)) in storage, unit-tested; uses ConnectionId::is_legacy_default (P4-T22).
  2. Replace the expect at persist.rs:316 with a logged refusal.
- **Acceptance criteria:**
  - [ ] persist.rs holds no legacy policy.
- **Verification:** SV (see 2.1)

#### P5-T13

**CSV export with a typed row**

- **Why:** `storage-infra-18` (low), `recent-change-review-24` (low)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** P5
- **Files:**
  - `src/storage/report_export.rs 64-177`
  - `src/domain/{workflow.rs (ActivityOrigin), output_result.rs, output.rs}`
- **Steps:**
  1. CsvRow struct with to_cells() as the single header source (replaces vec![String::new(); 14] and values[13] at 96-105).
  2. as_str() on ActivityOrigin, OutputResultStatus, OutputKind replaces the JSON trim hack (118-119). ui/session_reports.rs maps to localized labels - not a duplicate.
  3. Keep formula neutralization in cell() (35); guards csv_quotes_multiline_cells_and_neutralizes_formula_prefixes (189) and exports_keep_acknowledgement_separate_from_file_evidence_and_roundtrip (198).
- **Acceptance criteria:**
  - [ ] No positional index in the exporter.
- **Verification:** SV (see 2.1)

#### P5-T14

**Typed store handles that capture their serial lane**

- **Why:** `storage-infra-23` (low)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** P5-T11 | **Parallelizable with:** P5
- **Files:**
  - `src/ui/background_io.rs 49-58`
  - `src/storage/show_workspaces.rs 1, 10-15, 134-142`
- **Steps:**
  1. Do not move SerialLane into storage (completion callbacks return to GTK). Add ConfigStore/WorkspaceStore handles in ui/background_io.rs that carry their lane, so callers cannot pick the wrong lane.
  2. WorkspaceKey newtypes from P5-T11.
- **Acceptance criteria:**
  - [ ] No caller passes a SerialLane by hand for config/workspace writes.
- **Verification:** SV + NV (see 2.1)

### Phase 6 - OBS anti-corruption layer

Split obs/client.rs by request category, type the errors and identities, and finish the mapper.

| ID | Title | Size | Depends on |
|---|---|---|---|
| [P6-T01](#p6-t01) | Split obs/client.rs by request category (part 1): concurrency, outputs, collections | L (pure move) | P2-T04, P0-T06 |
| [P6-T02](#p6-t02) | Split obs/client.rs (part 2): scenes (merge scene_activation), thumbnails, workspace snapshot, source detail | L (pure move) | P6-T01 |
| [P6-T03](#p6-t03) | Control dispatcher: obs/control/{mod, studio, outputs, media, items, text, audio_controls} | M | P6-T02, P4-T14 |
| [P6-T04](#p6-t04) | Mapper as a directory; pull in the inline conversions | M | P6-T02 |
| [P6-T05](#p6-t05) | Typed ObsError; RetirementReason in the domain; localize in controller/ui | M | P6-T02 |
| [P6-T06](#p6-t06) | RequestGate: one watch channel and an injectable timeout | S | P2-T04 |
| [P6-T07](#p6-t07) | OBS identity: parse ObsUuid once; scene/input/source refs; fix nil uuid strings | M | P4-T02, P6-T03 |
| [P6-T08](#p6-t08) | ObsClient composition: endpoint at connect, server info, session-scoped observation counter | M | P3-T01, P4-T22, P6-T02 |
| [P6-T09](#p6-t09) | Cut OBS round trips: concurrent snapshot reads, stats join, video settings cache | M | P6-T02, P0-T06 |
| [P6-T10](#p6-t10) | Scene-item enable-state cache to remove the N+1 audio membership reads | M | P6-T09 |
| [P6-T11](#p6-t11) | Name OBS adapter constants; move test conveniences off the production type | S | P6-T02 |

#### P6-T01

**Split obs/client.rs by request category (part 1): concurrency, outputs, collections**

- **Why:** `obs-acl-01` (high), `obs-acl-19` (low), `performance-22` (low)
- **Commit type:** `refactor:` | **Size:** L (pure move) | **Risk:** Low
- **Depends on:** P2-T04, P0-T06 | **Parallelizable with:** P6-T03, P6-T05
- **Files:**
  - `src/obs/client.rs`
  - `src/obs/client/{concurrency.rs, outputs.rs, collections.rs} (child modules)`
- **Steps:**
  1. Use CHILD modules src/obs/client/*.rs: only `inner` is a pub(super) field; requests, images, endpoint, connection_id, observation are private and visible only to children. request/mutation are pub(super) methods.
  2. concurrency.rs: MAX_CONCURRENT_OBS_REQUESTS (24) + collect_bounded_ordered (26-49) + tests; re-export from client for audio.rs/operator_inputs.rs.
  3. outputs.rs: 244-314 (get_stream_status and get_stream_health send the same GetStreamStatus: share one mapping helper; no wasted round trip today).
  4. collections.rs: 212-242.
  5. One commit per category; `cargo test --all-features obs:: -- --list | wc -l` unchanged.
- **Acceptance criteria:**
  - [ ] `client.rs` no longer defines `collect_bounded_ordered`, `MAX_CONCURRENT_OBS_REQUESTS`, the stream/record status getters (ex 244-314) or the collection getters (ex 212-242); they live in `obs/client/{concurrency,outputs,collections}.rs`.
  - [ ] TC equal (`cargo test --all-features obs:: -- --list | wc -l` identical).
- **Verification:** SV + TC (see 2.1)

#### P6-T02

**Split obs/client.rs (part 2): scenes (merge scene_activation), thumbnails, workspace snapshot, source detail**

- **Why:** `obs-acl-01` (high), `obs-acl-09` (medium)
- **Commit type:** `refactor:` | **Size:** L (pure move) | **Risk:** Low
- **Depends on:** P6-T01 | **Parallelizable with:** P6-T03, P6-T05
- **Files:**
  - `src/obs/client/{scenes.rs, thumbnails.rs, workspace_snapshot.rs, source_detail.rs}`
  - `src/obs/scene_activation.rs`
- **Steps:**
  1. scenes.rs: get_scene_inventory (197-202) + scene_activation.rs merged then deleted.
  2. thumbnails.rs: 577-630 + thumbnail_dimensions (1077-1087) + tests.
  3. workspace_snapshot.rs: get_control_snapshot (517-575).
  4. source_detail.rs: get_source_detail (632-761) split into read_text_state, read_filters, read_media, read_audio_basic, read_audio_advanced (pure split, still sequential; concurrency in P6-T08). Add domain::control::is_media_kind; typed SourceDetailWarning (English warnings at 671-757 move to fl! via P6-T05).
  5. Wire tests for GetInputAudioBalance/SyncOffset/MonitorType/Tracks before the split (from P0-T06 list).
- **Acceptance criteria:**
  - [ ] client.rs holds struct, connect, lifecycle, request/mutation, endpoint, get_version.
- **Verification:** SV + TC (see 2.1)

#### P6-T03

**Control dispatcher: obs/control/{mod, studio, outputs, media, items, text, audio_controls}**

- **Why:** `obs-acl-02` (high)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P6-T02, P4-T14 | **Parallelizable with:** P6-T04
- **Files:**
  - `src/obs/client.rs 764-1097 (execute_control*, text_is_file_backed)`
  - `src/obs/control/*.rs (new)`
- **Steps:**
  1. control/mod.rs keeps the dispatcher; it resolves identity once, then delegates by ControlAction::category() (P4-T14). Name the audio file audio_controls.rs (avoid confusion with obs/audio.rs).
  2. Arms: studio (ActivateScene 795-806, SetStudioMode 824-827, SetProgram 828-836, Take, SetTransition, SetTransitionDuration 845-856), outputs (857-900), media (901-942, SeekMedia reads before write), items (SetItemEnabled 807-823, SetFilterEnabled 943-959), text (SetText 960-996 + text_is_file_backed + tests), audio_controls (997-1071 + invalid_audio_values_never_reach_obs test).
  3. First commit moves the match unchanged; then one category per commit. Sub-enums are P4-T14's later step.
- **Acceptance criteria:**
  - [ ] execute_control_with_identity_if < 60 lines.
- **Verification:** SV + TC (see 2.1)

#### P6-T04

**Mapper as a directory; pull in the inline conversions**

- **Why:** `obs-acl-08` (medium), `obs-acl-07` (medium), `obs-acl-16` (low), `obs-acl-18` (low)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P6-T02 | **Parallelizable with:** P6-T03
- **Files:**
  - `src/obs/mapper.rs (561) -> src/obs/mapper/{mod, event, output, scene, media, audio, catalog, session, time}.rs`
  - `src/obs/client/*.rs inline conversions (ObsNamedList 215-218/231-234, SourceFilter 684-691, MediaStatus 700-705, MonitorType 744-750/1025-1031, MediaAction 906-913)`
  - `src/obs/operator_inputs.rs 90-165`
  - `src/obs/audio.rs 46-55`
- **Steps:**
  1. Directory move first (only obs/client.rs and obs/event.rs import the mapper).
  2. Pull in each inline conversion with a wire-JSON test; unknown MonitorType maps to an explicit variant or is logged (today `_ => Off`).
  3. mapper::time::millis checked helper replaces `whole_milliseconds() as i64` (client 559, 703, 704, 743; operator_inputs 125-126); test -1, 0, i128 overflow.
  4. operator_inputs.rs: Infallible instead of `Ok::<_, AppError>` + expect (103, 156, 165); extract read_input_properties; dedupe the format! at 107/110.
  5. map_media_state visibility pub(super) like the others; map_output_state duplicate Unknown arm removed.
- **Acceptance criteria:**
  - [ ] No obws type is converted to a domain type outside src/obs/mapper.
- **Verification:** SV (see 2.1)

#### P6-T05

**Typed ObsError; RetirementReason in the domain; localize in controller/ui**

- **Why:** `obs-acl-05` (high), `obs-acl-17` (low), `obs-acl-10` (medium), `obs-acl-13` (low)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Medium
- **Depends on:** P6-T02 | **Parallelizable with:** P6-T04
- **Files:**
  - `src/obs/error.rs (new)`
  - `src/obs/requests.rs 16-39, 101-188`
  - `src/infra/error.rs 36-38`
  - `src/obs/{audio.rs 18-21, 42-44, 117-120, 188-221; sources.rs 37, 44; scene_activation (25-28) or client/scenes.rs; audio_presets.rs 13-62}`
  - `src/controller/refresh_controller.rs 576-583`
  - `src/controller/workspace.rs 380`
  - `src/controller/workspace/audio_presets.rs 42, 59, 206`
  - `i18n/*/scenedeck.ftl`
- **Steps:**
  1. `enum ObsError { Rejected{code, comment}, Retired(RetirementReason), Timeout, Superseded, Transport(String), InvalidIdentity(String), RejectedLocally(ControlRejection) }` with From<obws::error::Error> and From<ObsError> for AppError.
  2. RetirementReason -> domain/obs.rs; retirement_reason() returns Option<RetirementReason>; controller localizes with fl! (today the English reason becomes tx.terminal_reason, user-visible).
  3. Admission<T> { Sent(obws::Result<T>), Skipped(SkipReason), Deadline } in run_if_valid; Skipped(Invalidated) -> ObsError::Superseded (today a superseded skip reports Disconnected at 181).
  4. is_non_audio stays in the adapter (protocol quirk) but yields a typed variant.
  5. audio_presets adapter returns Result<_, AppError/ObsError>; user_message moves to the controller. Remove the fl! from scene activation.
  6. Replace substring assertions in tests (contains("timeout") etc.) with variant matches. New keys in every locale.
- **Acceptance criteria:**
  - [ ] obs/ contains no fl! and no user_message calls.
  - [ ] No test matches error substrings.
- **Verification:** SV + IV (see 2.1)

#### P6-T06

**RequestGate: one watch channel and an injectable timeout**

- **Why:** `obs-acl-12` (low)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** P2-T04 | **Parallelizable with:** P6-T01..P6-T05
- **Files:**
  - `src/obs/requests.rs 14, 31-39, 101-115`
  - `src/obs/requests tests (deadline test, ex regression 483-502)`
- **Steps:**
  1. Collapse `retired: watch::Sender<bool>` and `reason: Mutex<Option<RetirementReason>>` into `watch::Sender<Option<RetirementReason>>` with send_if_modified (Deadline wins). Removes the Mutex and both expects.
  2. Inject the timeout through a #[cfg(test)] constructor or a parameter; do not use start_paused with real loopback sockets (the 15 s connect timeout can fire early).
- **Acceptance criteria:**
  - [ ] Deadline test runs in < 1 s.
- **Verification:** SV (see 2.1)

#### P6-T07

**OBS identity: parse ObsUuid once; scene/input/source refs; fix nil uuid strings**

- **Why:** `obs-acl-06` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P4-T02, P6-T03 | **Parallelizable with:** P6-T04, P6-T05
- **Files:**
  - `src/obs/identity.rs (new)`
  - `src/obs/client.rs 787-836 (or control/mod.rs)`
  - `src/obs/{audio_presets.rs 38-45, operator_inputs.rs 94-113, audio.rs 194-200, sources.rs 135-148, 216-219}`
  - `src/domain/control.rs (SourceItem.input_uuid)`
  - `src/controller/workspace.rs 329-331`
- **Steps:**
  1. obs/identity.rs: input_ref, source_ref, scene_ref, optional_uuid over kernel ObsUuid (rejects nil); replace the four parse sites and the 13 uuid-or-name fallbacks.
  2. sources.rs:139 writes nil uuid strings into a HashMap<String,String> and SourceItem.input_uuid gets unwrap_or_default (empty string): make input_uuid Option<String> (domain change, check consumers) in a separate `fix:` commit with a test.
  3. SetItemEnabled and SetProgram ignore a supplied uuid (817, 833): reject a uuid for actions that cannot use it or carry identity inside the variant. SetTransition has no uuid in the API (not a defect).
- **Acceptance criteria:**
  - [ ] One identity parse path in obs/.
- **Verification:** SV (see 2.1)

#### P6-T08

**ObsClient composition: endpoint at connect, server info, session-scoped observation counter**

- **Why:** `obs-acl-14` (medium), `recent-change-review-16` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P3-T01, P4-T22, P6-T02 | **Parallelizable with:** P6-T03..P6-T07
- **Files:**
  - `src/obs/client.rs 52-60, 69-106, 169-195`
  - `src/controller/session_controller.rs 308`
  - `src/controller/refresh_controller.rs 196, 212, 268, 493-516`
- **Steps:**
  1. `connect(&ObsEndpoint, password)` removes with_connection_id and the hard-coded "default" (100). obs returns ObsServerInfo; session_controller composes ConnectionInfo (if not done in P3-T01).
  2. The observation counter is per connection and shared by clones: keep it documented as a session-scoped revision, or move it into a controller-side `LiveSession { client, observation }` passed to refresh tasks. Do not make it a session_controller field (refresh tasks hold only a client clone).
- **Acceptance criteria:**
  - [ ] ObsClient has no connection_id field.
- **Verification:** SV (see 2.1)

#### P6-T09

**Cut OBS round trips: concurrent snapshot reads, stats join, video settings cache**

- **Why:** `performance-08` (medium), `obs-acl-11` (medium), `obs-acl-09` (medium), `performance-07` (medium)
- **Commit type:** `perf:` | **Size:** M | **Risk:** Low
- **Depends on:** P6-T02, P0-T06 | **Parallelizable with:** P6-T03..P6-T08
- **Files:**
  - `src/obs/client/workspace_snapshot.rs (519-575)`
  - `src/obs/client/source_detail.rs`
  - `src/obs/client/outputs.rs (253-274)`
  - `src/controller/refresh_controller.rs 109-123`
  - `src/obs/client/thumbnails.rs (585-589)`
  - `src/obs/video_settings.rs (new)`
- **Steps:**
  1. Make ScriptedObs order-insensitive for these tests first (P0-T06 by-type mode).
  2. Control snapshot: after the scenes list, tokio::join! the six optional reads (per-field .ok() semantics kept); record OBS rejections as WorkspaceWarning::FacilityUnavailable (today swallowed; timeouts already retire the gate); use MAX_INSPECTED_ITEMS instead of 4096; checked cast at 559. Pure assemble() for unit tests.
  3. Source detail: join settings(+text defaults), filters, media, mute; volume and advanced audio stay sequential after mute (non-audio detection).
  4. Stats: tokio::join!(get_obs_stats, get_stream_health). Video settings: obws 0.15 has no VideoSettingsChanged event - cache with CurrentProfileChanged invalidation + reconnect + TTL (e.g. 30 s), shared across the requests and images gates; thumbnails reuse it.
  5. Measure (section 6): request counts per minute via ScriptedObs counters; snapshot wall time with 5 ms injected latency per request (hypothesis 35 ms -> ~10 ms).
- **Acceptance criteria:**
  - [ ] Request-count tests: one GetVideoSettings per TTL window; snapshot issues reads concurrently.
- **Verification:** SV (see 2.1)

#### P6-T10

**Scene-item enable-state cache to remove the N+1 audio membership reads**

- **Why:** `performance-11` (medium)
- **Commit type:** `perf:` | **Size:** M | **Risk:** Low
- **Depends on:** P6-T09 | **Parallelizable with:** P6-T07, P6-T08
- **Files:**
  - `src/obs/audio.rs 102-163`
  - `src/obs/sources.rs 88-96, 182`
  - `src/obs/mapper.rs 192`
  - `src/controller/refresh_controller.rs 353-372, 481-485, 509`
  - `src/obs/scene_item_state.rs (new)`
- **Steps:**
  1. Step 1: keep the last known program (from full refresh inventory and CurrentProgramSceneChanged) in RefreshInvalidation and use it on SceneItemVisibilityChanged; fall back to GetSceneList only when unknown (the None path is deliberate for nested scenes, comment 355-356).
  2. Step 2: extend ObsEvent::SceneItemVisibilityChanged with {scene, item_id, enabled} (obws event carries them); SceneItemEnabledCache keyed (scene, item_id); invalidate on SceneItemsChanged (created/removed/reindexed), CollectionChanging and event-stream lag recovery; query only misses.
  3. obws 0.15 SceneItem lacks sceneItemEnabled: track an upgrade (speculative).
  4. Test counting GetSceneItemEnabled for a 3-level nested scene.
- **Acceptance criteria:**
  - [ ] A visibility toggle costs about 1 request instead of items+2.
- **Verification:** SV (see 2.1)

#### P6-T11

**Name OBS adapter constants; move test conveniences off the production type**

- **Why:** `obs-acl-20` (low)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** P6-T02 | **Parallelizable with:** P6-T03..P6-T10
- **Files:**
  - `src/obs/client.rs 74-76, 204-210, 570, 606, 764-770, 1077-1087`
  - `src/obs/audio.rs 26-30`
  - `src/obs/testing/client_ext.rs (new)`
- **Steps:**
  1. Name: CONNECT_TIMEOUT (15 s wrapping obws connect_timeout), SCREENSHOT_SIZE 960x540, THUMBNAIL_BOUNDS 384x216, MAX_INSPECTED_ITEMS reuse.
  2. Move #[cfg(test)] set_current_program_scene, execute_control, get_all_audio_inputs into an ObsClientTestExt trait in obs/testing.
  3. Normalize visibility to pub(crate).
- **Acceptance criteria:**
  - [ ] No #[cfg(test)] method inside impl ObsClient.
- **Verification:** SV (see 2.1)

### Phase 7 - Controller orchestration (Tokio side)

Split SessionScope, type the errors, remove the test seam from production and split the long async routines.

| ID | Title | Size | Depends on |
|---|---|---|---|
| [P7-T01](#p7-t01) | ClientSlot value and constructor-injected scope | M | P0-T09 |
| [P7-T02](#p7-t02) | SessionScope split (part 1): journal and retirement | M | P0-T10, P7-T01 |
| [P7-T03](#p7-t03) | SessionScope split (part 2): task set with Notify-based quiesce, clocks, identity | M | P7-T02 |
| [P7-T04](#p7-t04) | OperationPhase and WorkspaceOutcome replace boolean flag soup | M | P7-T02 |
| [P7-T05](#p7-t05) | Typed CommandError in the controller; localize only at the AppEvent boundary; Fluent for controller strings | M | P6-T05 |
| [P7-T06](#p7-t06) | OutputPort trait replaces the cfg(test) Fake enum in production code | M | P7-T01 |
| [P7-T07](#p7-t07) | Output module split; shutdown plan passed in (no session<->output cycle) | M | P7-T06, P7-T03 |
| [P7-T08](#p7-t08) | Refresh split (part 1): snapshots, stats poll, BitrateSampler, invalidation worker | M | P0-T11, P7-T07 |
| [P7-T09](#p7-t09) | Refresh split (part 2): pure event translation and ingress | M | P7-T08 |
| [P7-T10](#p7-t10) | Workspace controller: JobFence and request() decomposition | M | P3-T04, P7-T05 |
| [P7-T11](#p7-t11) | Thumbnail controller goes through EventSender with named budgets | S | P7-T05 |
| [P7-T12](#p7-t12) | AppEvent: per-variant collection scoping method and observed() helper | S | P7-T08 |
| [P7-T13](#p7-t13) | AppEvent families and a Publication envelope; OutputEvent sub-enum | M | P7-T12, P9-T02 |
| [P7-T14](#p7-t14) | AppCommand families, sanitized dB conversion, explicit cancellation helpers | M | P4-T08, P2-T03 |
| [P7-T15](#p7-t15) | Meter levels as latest-value state, not GTK queue items | M | P7-T09 |
| [P7-T16](#p7-t16) | app_controller.rs split: manual refresh, mixer audio, runtime config | M | P1-T05, P1-T06, P7-T07, P2-T03 |

#### P7-T01

**ClientSlot value and constructor-injected scope**

- **Why:** `ctrl-11` (medium), `ctrl-12` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P0-T09 | **Parallelizable with:** P7-T03, P7-T04
- **Files:**
  - `src/controller/session/client_slot.rs (new)`
  - `src/controller/app_controller.rs 57, 78-101, 395-397`
  - `src/controller/output_controller.rs 35, 42-61, 184-188`
  - `src/controller/workspace.rs 75, 87-117, 134, 198, 253`
  - `src/controller/workspace/audio_presets.rs 14`
  - `src/controller/thumbnails.rs 24, 49`
  - `src/controller/lifecycle.rs 152, 230, 244`
- **Steps:**
  1. ClientSlot wraps Arc<Mutex<Option<ObsClient>>>: current() for the crate, install_if/clear restricted to the session module (pub(in crate::controller) until lifecycle moves into session/). Uses lock_or_recover. Replaces 7 read copies.
  2. OutputController::new and WorkspaceController::new take the scope and ClientSlot as parameters (no private SessionScope::initial); keep set_scope for rebinding. Tests pass SessionScope::initial explicitly (session_controller.rs 451, 515, 691, 734, 780; output_controller.rs 637, 704, 747, 795; workspace.rs 666, 885; workspace_lifecycle_tests 199, 257).
- **Acceptance criteria:**
  - [ ] Only lifecycle/session code writes the slot.
- **Verification:** SV (see 2.1)

#### P7-T02

**SessionScope split (part 1): journal and retirement**

- **Why:** `ctrl-05` (high), `architecture-global-13` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Medium
- **Depends on:** P0-T10, P7-T01 | **Parallelizable with:** P7-T03
- **Files:**
  - `src/controller/lifecycle.rs 31-128, 260-360`
  - `src/controller/session/{journal.rs, retirement.rs, scope.rs}`
- **Steps:**
  1. journal.rs: OperationKind, OperationEvidence, OperationLease, OperationJournal (per generation). retirement.rs: SessionRetirement, TerminalQueue (Arc shared across generations; next() clones it at 169).
  2. finish_evidence spans journal and retirement queue under nested locks (evidence -> terminals.pending, 303-330): keep ONE function that takes the journal entries and pushes to the queue so no new lock order appears.
  3. Keep SessionScope as a thin delegating composite; method bodies identical. Extend retirement_cancels_old_work_and_cannot_retire_a_new_generation (485-503) to install a client.
- **Acceptance criteria:**
  - [ ] lifecycle.rs < 300 lines; lock order doc updated.
- **Verification:** SV (see 2.1)

#### P7-T03

**SessionScope split (part 2): task set with Notify-based quiesce, clocks, identity**

- **Why:** `ctrl-05` (high), `ctrl-17` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Medium
- **Depends on:** P7-T02 | **Parallelizable with:** P7-T04, P7-T05
- **Files:**
  - `src/controller/session/{task_set.rs, clocks.rs, identity.rs}`
  - `src/controller/lifecycle.rs 129-433 (quiesce 383-393)`
  - `src/controller/session_controller.rs 119, 129, 218, 235`
- **Steps:**
  1. task_set.rs: active, failed (18, 173-179), cancelled, tasks, spawn/spawn_reserved/cancelled/quiesce/mark_failed. Keep returning JoinHandle (callers store and abort: app_controller.rs:279, workspace.rs 204, 258) - JoinSet does not fit.
  2. Replace the yield_now spin in quiesce with an AtomicUsize live count + Notify; decrement in a Drop guard inside the spawned wrapper (aborted tasks drop the future); enable-then-check like cancelled() (371-381).
  3. clocks.rs: collection, graph_revision, scene_activation_revision. identity.rs: generation, owner.
  4. Move lifecycle.rs -> session/scope.rs and session_controller.rs -> session/mod.rs once all facets have moved.
- **Acceptance criteria:**
  - [ ] quiesce has no busy loop; lifecycle tests pass.
- **Verification:** SV (see 2.1)

#### P7-T04

**OperationPhase and WorkspaceOutcome replace boolean flag soup**

- **Why:** `ctrl-15` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P7-T02 | **Parallelizable with:** P7-T03, P7-T05
- **Files:**
  - `src/controller/session/journal.rs (ex lifecycle.rs 49-60, 95-127, 297-319)`
  - `src/controller/event.rs 267-275`
  - `src/controller/state.rs 911-924`
- **Steps:**
  1. Add accessor methods first (state.rs:920-923 formats dispatched/acknowledged/uncertain for the audit).
  2. OperationPhase enum with forward-only transitions that also accepts Admitted -> Completed directly (audio path finishes without acknowledge, workspace.rs 173-176).
  3. WorkspaceLoaded carries WorkspaceOutcome { Read, Applied, Rejected, Unknown } instead of acknowledged/uncertain/action bools.
- **Acceptance criteria:**
  - [ ] No 4-bool evidence struct.
- **Verification:** SV (see 2.1)

#### P7-T05

**Typed CommandError in the controller; localize only at the AppEvent boundary; Fluent for controller strings**

- **Why:** `ctrl-13` (medium), `ctrl-14` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P6-T05 | **Parallelizable with:** P7-T03, P7-T04
- **Files:**
  - `src/controller/error.rs (new)`
  - `src/controller/workspace.rs 152-176, 205-212, 291-563, 805`
  - `src/controller/thumbnails.rs 59-125`
  - `src/controller/workspace/audio_presets.rs 13-77`
  - `src/controller/event_sender.rs 100-145`
  - `src/controller/lifecycle.rs 310-322 (finish_evidence English)`
  - `src/controller/event.rs 41`
  - `src/controller/output_controller.rs 320-322`
  - `i18n/*/scenedeck.ftl`
- **Steps:**
  1. CommandError (thiserror 2 is a dependency) carrying AppError/ObsError kinds; EventSender returns SendRejected{Stale, Closed} instead of Result<(), ()>; 'uncertain' derives from the error kind, not from now_or_never probes (workspace.rs 398-400, output_controller.rs 142).
  2. Replace English user-visible strings with stored error kinds rendered by fl! at the UI boundary: finish_evidence failure text (copied into last_*_command_error and the audit), event.rs:41 outcome-unknown suffix, workspace.rs 374 and 439, output_controller.rs 320-322, thumbnails messages. Not the cfg(test) Fake arms (262-266, 288-292) or refresh_controller.rs:332 (unseen).
  3. workspace.rs:805 test matches the variant instead of a substring. Coordinate AppEvent payload changes with P7-T12.
- **Acceptance criteria:**
  - [ ] No Result<_, String> in controller production code.
  - [ ] IV passes.
- **Verification:** SV + IV (see 2.1)

#### P7-T06

**OutputPort trait replaces the cfg(test) Fake enum in production code**

- **Why:** `ctrl-09` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P7-T01 | **Parallelizable with:** P7-T04, P7-T05
- **Files:**
  - `src/controller/output/port.rs (new)`
  - `src/controller/output_controller.rs 31-39, 140-146, 173-203, 219-387, 337-351, 389-390`
  - `src/controller/session_controller.rs 143-155, 169-192`
  - `src/controller/dependencies.rs`
  - `src/controller/refresh_controller.rs 345`
- **Steps:**
  1. Consumer-owned OutputPort trait implemented for ObsClient (controller already imports obs::client::ObsClient; obws stays in obs/). It covers commands AND status reads (OutputStatusReader is used by refresh_worker through refresh_output_statuses).
  2. Delete OutputCommandClient, OutputStatusReader, client_override; inject a port factory through ControllerDependencies; move the 'uncertain' decision into the port (`outcome_unknown_after(&self, &AppError) -> bool`).
  3. FakeOutputCommandClient moves under #[cfg(test)] next to the port.
- **Acceptance criteria:**
  - [ ] No `#[cfg(test)]` enum variant matched in production methods.
- **Verification:** SV (see 2.1)

#### P7-T07

**Output module split; shutdown plan passed in (no session<->output cycle)**

- **Why:** `ctrl-18` (medium), `ctrl-20` (medium), `recent-change-review-23` (low), `ctrl-03` (high)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P7-T06, P7-T03 | **Parallelizable with:** P7-T08
- **Files:**
  - `src/controller/output/{mod.rs, pending.rs, readback.rs, shutdown.rs}`
  - `src/controller/output_controller.rs 469-559`
  - `src/controller/session_controller.rs 138-224`
  - `src/controller/app_controller.rs 256-262`
- **Steps:**
  1. readback.rs: OutputReadbackErrors and refresh_output_statuses as one read_status(kind) loop over [Stream, Record] (507-558 duplicated).
  2. shutdown.rs: OutputController::shutdown_plan() -> OutputShutdown { confirm_stopped(), retire() }. AppController builds the plan and passes it to SessionController::disconnect, so session/ never imports output/ (output already depends on session scope/journal).
  3. Extract full_snapshot_scan(client, tx, audio_filter, valid) into controller/refresh/snapshots.rs used by both RefreshData and refresh_worker (the P1-T02 fix stays).
- **Acceptance criteria:**
  - [ ] session/ has no import of output/; disconnect < 40 lines.
- **Verification:** SV (see 2.1)

#### P7-T08

**Refresh split (part 1): snapshots, stats poll, BitrateSampler, invalidation worker**

- **Why:** `ctrl-25` (low), `ctrl-20` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P0-T11, P7-T07 | **Parallelizable with:** P7-T09
- **Files:**
  - `src/controller/refresh_controller.rs 17-387`
  - `src/controller/refresh/{snapshots.rs, stats.rs}`
  - `src/controller/session/invalidation.rs`
  - `src/domain/stats.rs (BitrateSampler)`
  - `src/controller/app_controller.rs 286-291`
- **Steps:**
  1. snapshots.rs (26-99, 187-291), stats.rs (101-185), invalidation worker (295-387).
  2. BitrateSampler::sample(now, bytes) in domain/stats.rs; it stays shared (Arc<Mutex<..>>) because manual RefreshStats (app_controller 287-289) and SessionController::clear_bitrate_sample (108, 165) use it.
  3. AppEvent::observed(revision, event) constructor replaces the 14 hand-built Observation envelopes; invalidate_graph returns (u64, AppEvent) so refresh_live_data_if reuses it (212-221 duplicate).
  4. Keep refresh/ free of session imports apart from EventSender (session -> refresh -> output).
- **Acceptance criteria:**
  - [ ] refresh_controller.rs only holds the event loop.
- **Verification:** SV + TC (see 2.1)

#### P7-T09

**Refresh split (part 2): pure event translation and ingress**

- **Why:** `ctrl-08` (medium), `architecture-global-22` (low), `performance-15` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Medium: fencing order
- **Depends on:** P7-T08 | **Parallelizable with:** P7-T10
- **Files:**
  - `src/controller/refresh_controller.rs 409-586`
  - `src/controller/session/{event_translation.rs, event_ingress.rs}`
- **Steps:**
  1. `fn translate(event: ObsEvent, ctx) -> Ingress { publications: Vec<AppEvent> (<= 2; no SmallVec dependency), effects: IngressEffects { graph_invalidation, refresh: RefreshKind, fence_scene_activation, advance_collection, meter } }`. invalidate_graph is not pure: carry a bool and let the executor call it.
  2. The executor replays effects in today's order: next_observation/observe_graph_revision BEFORE dirty.invalidate/invalidate_graph (SceneRenamed 538-546, InputsChanged 547-557). InputLevelsUpdated stays an explicit effect bypassing the bounded handoff.
  3. Keep the select! structure; move resync (445-468) into resynchronize().
  4. Table-test every ObsEvent variant (P0-T11 tests keep passing).
- **Acceptance criteria:**
  - [ ] run_event_loop < 60 lines.
- **Verification:** SV (see 2.1)

#### P7-T10

**Workspace controller: JobFence and request() decomposition**

- **Why:** `ctrl-07` (high)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Medium
- **Depends on:** P3-T04, P7-T05 | **Parallelizable with:** P7-T09, P7-T11
- **Files:**
  - `src/controller/workspace.rs 226-408 (+ types, read, audio, media, guards)`
  - `src/controller/workspace/{types.rs, job.rs, read.rs, audio.rs, media.rs, guards.rs, lifecycle_tests.rs}`
- **Steps:**
  1. Characterization test first: ActivateScene rejected after a StudioModeChanged activation bump.
  2. JobFence { cancellation, ticket, revision, expected_revision, tx } with SEPARATE methods per check level (the 8 copies differ on purpose: audio() skips revision; supersession 295-297 skips accepts_current_scope; dispatch guard adds the activation clause; final publish 387-388 skips scope because EventSender re-checks).
  3. run_job, verify_expected_text, settle_evidence, enum JobOutcome. Keep workspace.rs as facade (already file + dir layout); move workspace_lifecycle_tests.rs via #[path] to workspace/lifecycle_tests.rs; guards.rs needs runtime/budgets (P3-T04).
- **Acceptance criteria:**
  - [ ] request() < 60 lines; characterization test passes.
- **Verification:** SV (see 2.1)

#### P7-T11

**Thumbnail controller goes through EventSender with named budgets**

- **Why:** `ctrl-16` (medium)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** P7-T05 | **Parallelizable with:** P7-T10
- **Files:**
  - `src/controller/thumbnails.rs 21-125`
- **Steps:**
  1. Inject EventSender; publish through tx.send/send_if, or keep try_send with a fallback on Full (thumbnails stuck on loading when the GTK queue is full).
  2. Keep the deliberate drain on UI cancel (comments at 45 and 100: obws keeps unanswered receivers); retirement still aborts. Name THUMBNAIL_BUDGET; use CommandError.
  3. Routing via EventSender adds collection fencing (ThumbnailLoaded is not in event_sender.rs:9-23): verify the GTK-side generation check and that a fenced drop does not strand the loading tile.
- **Acceptance criteria:**
  - [ ] No raw SyncSender in thumbnails.rs.
- **Verification:** SV + NV (see 2.1)

#### P7-T12

**AppEvent: per-variant collection scoping method and observed() helper**

- **Why:** `ctrl-06` (high)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** P7-T08 | **Parallelizable with:** P7-T10, P7-T11
- **Files:**
  - `src/controller/event.rs 229-388`
  - `src/controller/event_sender.rs 9-23, 85-98, 152+`
- **Steps:**
  1. Move is_collection_independent into `impl AppEvent { fn collection_scoped(&self) -> bool }` as a PER-VARIANT match. Never derive it from a family: OutputCommandFailed and GraphRefreshRejected are independent while CommandFailed and PresetProgress are scoped.
  2. Guard: lifecycle_envelope_and_collection_admission_follow_the_same_policy (event_sender.rs:152+).
- **Acceptance criteria:**
  - [ ] event_sender.rs holds no variant policy table.
- **Verification:** SV (see 2.1)

#### P7-T13

**AppEvent families and a Publication envelope; OutputEvent sub-enum**

- **Why:** `ctrl-06` (high), `ui-shell-13` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** High churn
- **Depends on:** P7-T12, P9-T02 | **Parallelizable with:** P8
- **Files:**
  - `src/controller/event.rs -> event/{mod, envelope, lifecycle, outcome, observed, workspace, stats}.rs`
  - `src/ui/event_pump.rs`
  - `src/ui/window.rs (or ui/shell/events after P9-T02) 977-983, 1340-1356`
  - `src/controller/observations.rs 63-123`
- **Steps:**
  1. Publication { generation, collection, revision, payload } alongside SessionEvent and Observation (boxed envelopes 235-239, 313-316); coordinate the event pump change.
  2. Move families one at a time with `pub(crate) use` aliases; then delete the boxed variants.
  3. AppEvent::Output(OutputEvent { StatusUpdated, CommandPending, CommandSucceeded, CommandFailed, RecordingStopObserved, ReadbackFailed }) so apply_output_event takes &OutputEvent and the debug_assert!(false) re-dispatch in window.rs disappears (OutputCommandFailed toast stays special-cased).
  4. Guards: event_sender tests, observations tests, final regression tests (split), window render tests.
- **Acceptance criteria:**
  - [ ] No Box<AppEvent> inside AppEvent.
- **Verification:** SV + NV (see 2.1)

#### P7-T14

**AppCommand families, sanitized dB conversion, explicit cancellation helpers**

- **Why:** `ctrl-10` (medium), `architecture-global-08` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P4-T08, P2-T03 | **Parallelizable with:** P7-T10..P7-T12
- **Files:**
  - `src/controller/command.rs 5-51 -> controller/command/`
  - `src/controller/app_controller.rs 118-215, 132-146, 167-177, 378-388`
  - `20 ui files referencing AppCommand`
- **Steps:**
  1. Group AppCommand { Session, Obs, Workspace, Audio, Output, Media } with From shims.
  2. Replace inline dB code (195-205) with domain gain conversion (P4-T08). Table: 0.0 -> -100, 1.0 -> 0, 2.0 -> 0, NaN -> -100, +inf -> -100. The +inf case changes behavior (today 0 dB full gain): separate `fix:` commit.
  3. cancel_collection_scoped_work() / cancel_session_scoped_work() keeping the real differences (Connect retires the client, SetCurrentSceneCollection cancels the preset, shutdown aborts refresh_task).
- **Acceptance criteria:**
  - [ ] handle() is a router with one line per family.
- **Verification:** SV + NV (see 2.1)

#### P7-T15

**Meter levels as latest-value state, not GTK queue items**

- **Why:** `performance-06` (medium)
- **Commit type:** `perf:` | **Size:** M | **Risk:** Low
- **Depends on:** P7-T09 | **Parallelizable with:** P7-T13, P7-T14
- **Files:**
  - `src/controller/refresh_controller.rs 392-406, 531-537`
  - `src/controller/meter_feed.rs (new)`
  - `src/ui/event_pump.rs / ui/window.rs consumer`
- **Steps:**
  1. LevelsSlot via tokio::sync::watch (lock-free for the reader) written by the event loop instead of tx.try_send.
  2. Publish a zero-payload AppEvent::LevelsAvailable only on empty->full transition (AtomicBool), or have GTK read the slot once per batch.
  3. Test: fill the GTK channel with meter frames, assert a later CurrentSceneChanged is still admitted and the client is not retired.
  4. Measure: injected 3 s GTK stall with meters active; count backlog retirements (hypothesis 0).
- **Acceptance criteria:**
  - [ ] Meters cannot occupy GTK queue slots.
- **Verification:** SV + NV (see 2.1)

#### P7-T16

**app_controller.rs split: manual refresh, mixer audio, runtime config**

- **Why:** `ctrl-04` (medium), `ctrl-19` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P1-T05, P1-T06, P7-T07, P2-T03 | **Parallelizable with:** P7-T09..P7-T15
- **Files:**
  - `src/controller/app_controller.rs (production ~458)`
  - `src/controller/refresh/{manual.rs, mixer_audio.rs}`
  - `src/controller/runtime/config.rs`
- **Steps:**
  1. mixer_audio.rs gets refresh_mixer_scene_audio with the SingleFlight from P1-T05; manual.rs gets refresh_data with the race-free inactive fallback (comment 233-234: refresh must finish Doctor's loading state); runtime/config.rs gets load_config_blocking (Result from P1-T06).
- **Acceptance criteria:**
  - [ ] app_controller.rs < 250 production lines.
- **Verification:** SV (see 2.1)

### Phase 8 - Application state decomposition (AppState slices)

Move AppState out of controller/ and break it into context slices with private fields and intent methods.

| ID | Title | Size | Depends on |
|---|---|---|---|
| [P8-T01](#p8-t01) | Move AppState out of controller/ into src/state/ (pure move + GTK ban) | L (pure move) | P3-T01, P3-T02, P3-T03, P2-T01, P2-T02 |
| [P8-T02](#p8-t02) | TelemetryState slice | S | P8-T01, P3-T05 |
| [P8-T03](#p8-t03) | OutputsState slice: OutputChannel x2 + RecordingEvidence | M | P8-T01, P0-T12, P2-T11 |
| [P8-T04](#p8-t04) | Split apply_output_event; RecordingStopTicket value | M | P8-T03 |
| [P8-T05](#p8-t05) | One InputPatch for the four denormalized input copies | S | P8-T01 |
| [P8-T06](#p8-t06) | Pure rules from AppState to the domain | M | P8-T01, P3-T05 |
| [P8-T07](#p8-t07) | Collection/topology and active-audio slices; keep the fence asymmetry | M | P8-T03, P3-T06 |
| [P8-T08](#p8-t08) | Session slice with a cached workflow scope | M | P8-T07, P4-T22 |
| [P8-T09](#p8-t09) | LocalStore slice: persistence caches and show-workspace selection invariants | M | P8-T07, P5-T07 |
| [P8-T10](#p8-t10) | WorkspaceState split into fence, controls, inspection, screenshot, readback, audit | M | P8-T07 |
| [P8-T11](#p8-t11) | Move the workspace reducers from ui/event_reconciliation.rs into state | M | P8-T10, P8-T03 |
| [P8-T12](#p8-t12) | Session reports: ReportContext, phased collect, cheap no-op collection | M | P8-T02, P8-T03, P4-T21 |
| [P8-T13](#p8-t13) | Replace tuples and flag soups in state submodules | M | P8-T12 |
| [P8-T14](#p8-t14) | Mixer state: domain refresh target, no forwarders, view model to the mixer page, request claim in state | M | P8-T07, P2-T02 |
| [P8-T15](#p8-t15) | StateRevisions: per-slice change counters | S | P8-T03, P8-T07, P8-T08, P8-T09, P8-T10 |
| [P8-T16](#p8-t16) | Lock the state boundary: controller does not import state; remove the shim | S | P8-T01, P8-T14 |

#### P8-T01

**Move AppState out of controller/ into src/state/ (pure move + GTK ban)**

- **Why:** `architecture-global-01` (high), `controller-state-01` (critical), `recent-change-review-04` (high)
- **Commit type:** `refactor:` | **Size:** L (pure move) | **Risk:** Medium: wide import churn; land when few PRs are open
- **Depends on:** P3-T01, P3-T02, P3-T03, P2-T01, P2-T02 | **Parallelizable with:** none (serialize)
- **Files:**
  - `src/controller/state.rs + src/controller/state/ -> src/state/mod.rs (or src/state.rs) + src/state/`
  - `src/controller/mod.rs`
  - `src/lib.rs`
  - `src/architecture_tests/rules.rs (background_layers_hold_no_gtk_or_ui 669-681)`
- **Steps:**
  1. Prerequisites (hard): P3-T01 (ConnectionInfo), P3-T02 (unix_ms), P3-T03 (MixerAudioRequest). Without P3-T03 the move creates a controller<->state module cycle (command.rs:42, event.rs:327/331, app_controller.rs:48/293 import it).
  2. git mv; add a temporary `pub(crate) use crate::state;` shim in controller/mod.rs.
  3. SAME PR: add `violations("state", &gtk_forbidden, &[])` to background_layers_hold_no_gtk_or_ui; today AppState is GTK-free only because it sits under controller/.
  4. state still imports controller::event::AppEvent and controller::command::AppCommand (note_command_intent 566-571, apply_output_event 641-646): allowed edge state -> controller::{event, command}; nothing else from controller.
  5. In the same PR, re-scope every architecture rule that names `controller/state` to `state` (at least P3-T05's clock-read rule, which becomes `no_clock_reads_in_state`, and any ratchet ceilings keyed by `controller/state/...` file paths). A rule whose scope no longer matches any file passes vacuously.
  6. Update paths in this document's later tasks: `controller/state/...` means `src/state/...` from here on.
- **Acceptance criteria:**
  - [ ] Rule passes; TC equal; no other behavior change.
- **Verification:** SV + TC (see 2.1)

#### P8-T02

**TelemetryState slice**

- **Why:** `controller-state-04` (medium)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** P8-T01, P3-T05 | **Parallelizable with:** P8-T05, P8-T06
- **Files:**
  - `src/state/telemetry.rs (new)`
  - `src/state/mod.rs 487-498, 1143-1188`
  - `src/state/session_reports.rs 316-331`
- **Steps:**
  1. TelemetryState { latest: Option<TelemetrySample>, history } with record(sample, stream_active, now) and clear(). stream_bitrate_kbps depends on stream_status.active (1172): pass the bool in, no slice-to-slice dependency.
  2. Encoding derivation (1160-1166) -> domain::health::EncodingActivity::from_outputs.
  3. Give session_reports a pub(super) accessor for the group read.
- **Acceptance criteria:**
  - [ ] Telemetry fields private to the slice.
- **Verification:** SV (see 2.1)

#### P8-T03

**OutputsState slice: OutputChannel x2 + RecordingEvidence**

- **Why:** `controller-state-03` (high), `recent-change-review-23` (low), `ui-shell-11` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P8-T01, P0-T12, P2-T11 | **Parallelizable with:** P8-T02, P8-T05
- **Files:**
  - `src/state/outputs.rs (new)`
  - `src/state/mod.rs 465-474, 605-750, 1086-1141`
  - `src/ui/event_reconciliation.rs 116-138`
  - `src/ui/window.rs 939-969, 1317-1338, 475-507`
- **Steps:**
  1. OutputChannel { status, command_error, active_since, observed_at }; OutputsState { stream, record, recording: RecordingEvidence } with channel(kind)/channel_mut(kind).
  2. observe_readback(kind, Option<&OutputStatus>, at) covers accept + active clock + status, so event_reconciliation stops writing stream_active_since/record_active_since and calling set_*_status (fifth per-kind switch).
  3. Do not use #[deprecated] delegates (clippy -D warnings fails at remaining call sites): migrate all callers in the same commit or add a scoped #[allow(deprecated)].
  4. Guards: state tests 1989-2426 (now table-driven), P0-T12 golden sequence.
- **Acceptance criteria:**
  - [ ] No set_stream_*/set_record_* method pairs remain.
- **Verification:** SV + NV (see 2.1)

#### P8-T04

**Split apply_output_event; RecordingStopTicket value**

- **Why:** `controller-state-09` (high)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Medium
- **Depends on:** P8-T03 | **Parallelizable with:** P8-T05
- **Files:**
  - `src/state/outputs.rs`
  - `src/state/mod.rs 641-750`
  - `src/domain/outputs/output_result.rs`
- **Steps:**
  1. OutputsState::apply(&OutputEvent) -> Option<OutputTransition>; AppState handles health, activity log, report acks.
  2. Ordering: observe_recording_path runs BEFORE the Instant fence (658-662 precede 663; comment 656-657). When the fence rejects, still return Some(kind) (664). OutputTransition encodes 'fenced out but path evidence applied'.
  3. Guards: tests 1358, 1478, 1550 and the golden sequence.
- **Acceptance criteria:**
  - [ ] apply_output_event < 40 lines.
- **Verification:** SV (see 2.1)

#### P8-T05

**One InputPatch for the four denormalized input copies**

- **Why:** `controller-state-07` (high)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** P8-T01 | **Parallelizable with:** P8-T02..P8-T04
- **Files:**
  - `src/state/mod.rs 1190-1244`
  - `src/domain/audio/input.rs`
- **Steps:**
  1. domain::audio::InputPatch with apply_input / apply_mixer / apply_workspace_audio / apply_detail; AppState::apply_input_patch replaces apply_input_mute and apply_input_volume.
  2. Keep the asymmetry: detail.volume_db is patched, there is no detail volume_mul.
  3. Fix the doc comment at 1190-1195 ('two places' vs four). Extend the test at 2455 to cover workspace.detail. merge_workspace_audio stays in Observations (echo merge, not a stored copy).
- **Acceptance criteria:**
  - [ ] One patch path for mute/volume echoes.
- **Verification:** SV (see 2.1)

#### P8-T06

**Pure rules from AppState to the domain**

- **Why:** `controller-state-14` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P8-T01, P3-T05 | **Parallelizable with:** P8-T02..P8-T05
- **Files:**
  - `src/state/mod.rs 170-188, 247-355`
  - `src/state/mixer.rs 326-365`
  - `src/state/bookmarks.rs 124-147`
  - `src/domain/{scene.rs, outputs, control.rs, mixer.rs, bookmark.rs}`
- **Steps:**
  1. SceneInventory::advance (previous_scene_for_inventory_update), domain::outputs::ActiveClock (update_active_since with `now` parameter), domain::control::ControlAvailability (WorkspaceState control policy, using CONTROL_FRESHNESS), MixerSelection::refresh_target(&self, current: Option<&'a str>) with explicit lifetimes, RecordingTime::sample.
  2. Leave 1-line delegations; move tests.
- **Acceptance criteria:**
  - [ ] Each moved rule has a domain unit test.
- **Verification:** SV (see 2.1)

#### P8-T07

**Collection/topology and active-audio slices; keep the fence asymmetry**

- **Why:** `controller-state-01` (critical), `controller-state-06` (high), `recent-change-review-04` (high)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P8-T03, P3-T06 | **Parallelizable with:** P8-T08
- **Files:**
  - `src/state/{collection.rs, active_audio.rs}`
  - `src/state/mod.rs 836-880, 995-1025`
- **Steps:**
  1. CollectionState (TopologyStatus, inventory, graph, topology, profiles, scene_collections, apply_scene_inventory, set_topology_*, topology branch of reconcile_observation).
  2. ActiveAudioState (ActiveAudioStatus, audio fields, set_current_program, invalidate/apply_active_audio, levels, reconcile branches, audio_input_mut).
  3. Keep Observations reset on collection change and the Instant fence surviving it (ADR from P3-T06). Child modules (bookmarks, session_reports, audio_presets, show_registry, report_history) get pub(super) accessors for fields they read today.
- **Acceptance criteria:**
  - [ ] AppState fields for these contexts are private slices.
- **Verification:** SV (see 2.1)

#### P8-T08

**Session slice with a cached workflow scope**

- **Why:** `controller-state-01` (critical), `performance-10` (medium)
- **Commit type:** `perf:` | **Size:** M | **Risk:** Low
- **Depends on:** P8-T07, P4-T22 | **Parallelizable with:** P8-T09
- **Files:**
  - `src/state/session.rs (new)`
  - `src/state/mod.rs 750-880, 788-796`
  - `src/ui/widgets/audio_card.rs 91-98, 807, 833, 858-862, 1079`
  - `src/ui/previews.rs 81, 196`
  - `src/ui/pages/live.rs 685, 842`
  - `src/ui/widgets/operating_panel.rs 146`
- **Steps:**
  1. ObsSessionState { status, endpoint, connection_id, capabilities, scope: Rc<ScopeBinding>, scope_rev } rebuilt in apply_connected, reset_obs_session, SceneCollectionsUpdated, invalidate_collection_runtime.
  2. Disconnected fallback depends on config host/port edited directly by the settings page: either add one config setter that bumps scope_rev, or cache only the connected case and compute the disconnected fallback lazily.
  3. workflow_scope() returns Rc<ScopeBinding>; scope_rev() for hot-path equality (AudioTargetScope stores epoch + scope_rev). Commands still carry ScopeBinding by value.
  4. Test pairing each mutator with a rev bump; guard audio_callbacks_reject_another_session_collection_and_endpoint (audio_card.rs:1079).
- **Acceptance criteria:**
  - [ ] workflow_scope() allocates nothing when connected (heaptrack during a fader drag).
- **Verification:** SV + NV (see 2.1)

#### P8-T09

**LocalStore slice: persistence caches and show-workspace selection invariants**

- **Why:** `controller-state-15` (medium), `recent-change-review-14` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P8-T07, P5-T07 | **Parallelizable with:** P8-T08
- **Files:**
  - `src/state/local_store.rs (new)`
  - `src/state/mod.rs 15-16, 453-456`
  - `src/state/{session_reports.rs 178-187, report_history.rs 115, show_registry.rs}`
  - `src/ui/persist.rs 358-378, 511-536, 554-571`
  - `src/ui/window.rs 1027-1029`
  - `src/architecture_tests/rules.rs`
- **Steps:**
  1. LocalStore holds config, ShowCuration, show workspace selection, startup notice and theme accessors; other slices read through accessors.
  2. ShowRegistryState (rename ShowWorkspaceSelection) owns the registry snapshot and exposes edit(f) -> Option<EditTicket> and apply_reconciliation(&inventory) that bump revision and generation internally; fields private (today generation += 1 at persist.rs:368 and window.rs:1027-1029; revision at persist.rs:526, 569).
  3. Keep retry-on-unchanged-edit (documented in docs/configuration.md:215-216) unless the maintainer decides otherwise.
  4. Rule: crate::storage imports under src/state allowed only in state/local_store.rs (verify violations() accepts a sub-path scope).
- **Acceptance criteria:**
  - [ ] No ui code bumps show_registry.generation.
- **Verification:** SV + NV (see 2.1)

#### P8-T10

**WorkspaceState split into fence, controls, inspection, screenshot, readback, audit**

- **Why:** `controller-state-02` (high)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Medium
- **Depends on:** P8-T07 | **Parallelizable with:** P8-T11
- **Files:**
  - `src/state/workspace/{mod, fence, controls, inspection, screenshot, readback, audit}.rs`
  - `src/state/mod.rs 192-436`
- **Steps:**
  1. Rewrite invalidate (367-397) with explicit carry-over, not `..Default::default()`: revision must advance (render key; ReadFence::next_epoch bumps epoch, sequence, revision); keep `activity` (session_reports.rs:277-292 relies on monotonic sequence); output_results invalidated (not cleared); result and recording_stop_result cleared; next_operator_inspection kept, operator_inspections cleared.
  2. Guard: every_visible_workspace_mutation_advances_the_revision (state tests, ex 1813). If P11-T02 already removed the screenshot fields, skip screenshot.rs.
- **Acceptance criteria:**
  - [ ] WorkspaceState fields pub(super); invalidate test covers each carry-over.
- **Verification:** SV + NV (see 2.1)

#### P8-T11

**Move the workspace reducers from ui/event_reconciliation.rs into state**

- **Why:** `controller-state-05` (high), `recent-change-review-12` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P8-T10, P8-T03 | **Parallelizable with:** P8-T12
- **Files:**
  - `src/ui/event_reconciliation.rs 20-206`
  - `src/state/workspace/reducer.rs (new)`
  - `callers of .workspace.<field> writes (operating_panel 40, ui/workspace 19, action_catalog 19, operator_picker 12, window 11)`
- **Steps:**
  1. acknowledge, media_loaded, audio_failed, replay_saved -> WorkspaceState methods; complete_workspace touches outputs, health, merge_workspace_audio, workflow_scope -> `impl AppState`, delegating to OutputsState::observe_readback.
  2. Replace the bools with `enum CompletionKind { Refresh, Action { acknowledged }, Uncertain }`; store result enums, localize in ui.
  3. Then make workspace fields pub(super) and fix the external writers via intent methods; ui/event_reconciliation.rs keeps dispatch only (or disappears).
- **Acceptance criteria:**
  - [ ] grep `.workspace.[a-z_]* =` outside src/state returns 0.
- **Verification:** SV + NV (see 2.1)

#### P8-T12

**Session reports: ReportContext, phased collect, cheap no-op collection**

- **Why:** `controller-state-10` (high), `recent-change-review-18` (medium), `performance-02` (high), `recent-change-review-04` (high)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P8-T02, P8-T03, P4-T21 | **Parallelizable with:** P8-T13
- **Files:**
  - `src/state/session_reports.rs 136-210, 271-411`
  - `src/state/session_reports/collect.rs (new)`
  - `src/domain/session_report.rs (ReportId)`
- **Steps:**
  1. ReportsState::collect(&ReportContext) where ReportContext is a read-only view built by AppState (activity, outputs, scene, stats sample, alerts with resolved audio names, bookmark session).
  2. Phases collect_activity/outputs/scene/sample/health/sync_bookmarks bound to one `let Some(report) = current_mut() else { return }` (removes unwraps at 276, 280, 326, 392 and back_mut().unwrap() at 158, 161).
  3. ReportId::new(started, session, run, pid, nonce) with injected pid/nonce (P3-T05).
  4. Revision early-return: skip when activity.sequence(), OutputResults::revision(), HealthMonitor::revision() and bookmark revision are unchanged; iterate activity with skip_while(seq <= last); &'static str alert kinds. The collector reads AppState fields, so it lives in src/state/session_reports/collector.rs, not in domain.
  5. Golden ReportEvent-list test before the split; test that collect is a no-op when nothing changed.
- **Acceptance criteria:**
  - [ ] collect_report < 40 lines; no unwrap.
- **Verification:** SV (see 2.1)

#### P8-T13

**Replace tuples and flag soups in state submodules**

- **Why:** `controller-state-19` (medium), `controller-state-24` (low), `controller-state-25` (low), `recent-change-review-17` (medium), `controller-state-22` (low)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P8-T12 | **Parallelizable with:** P8-T14
- **Files:**
  - `src/state/bookmarks.rs 11-46, 82-87, 103-106`
  - `src/state/session_reports.rs 18, 116, 137, 391`
  - `src/state/mod.rs 194, 207, 212, 896-940`
  - `src/state/audio_presets.rs 4-144`
  - `src/state/report_history.rs 7-50, 121-140`
- **Steps:**
  1. BookmarkSession (replaces Option<(u64, u64, Instant)> and the session_id == 0 sentinel), Observed<T> (keep PartialEq/Debug; event_reconciliation tests compare it), InspectionToken, HealthKey struct.
  2. PresetState { run, serial } returning ActivityRecord; phase rule (Unconfirmed vs Cancelled) -> domain::audio_preset::PresetOutcome. Check preset_run ui readers before narrowing visibility.
  3. ReportHistory: `enum DiskState { Loading, LoadFailed, Idle, Saving{revision, then}, SaveFailed{revision, error} }` plus RetentionPolicy and Ownership; a SaveTracker with only the shared revision/saved_revision/save_error core (bookmarks has pending/failed_revision, history has more flags: do not over-generalize); retain_known_ids helper for the duplicated retain closures.
  4. apply_terminal_evidence: Display for OperationEvidence/SessionRetirement, ConnectionState::retire, OutputsState::apply_terminal; keep order (preset_terminal before output reset; collect_report before the audit log; report_terminal_audit before retirement push).
- **Acceptance criteria:**
  - [ ] No anonymous tuple domain concept in state.
- **Verification:** SV (see 2.1)

#### P8-T14

**Mixer state: domain refresh target, no forwarders, view model to the mixer page, request claim in state**

- **Why:** `controller-state-17` (medium), `architecture-global-17` (medium), `ui-pages-10` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P8-T07, P2-T02 | **Parallelizable with:** P8-T13
- **Files:**
  - `src/state/mixer.rs 7-452`
  - `src/domain/audio/mixer.rs`
  - `src/ui/pages/mixer/view_model.rs (new)`
  - `src/ui/pages/mixer.rs 50-62, 290-313, 754, 941-1049, 1116-1194, 1150`
  - `src/ui/window.rs 1153, 1163, 1386`
- **Steps:**
  1. Refresh target (52-65, 319-365) -> domain.
  2. Remove forwarders 201-220; callers: ui/pages/mixer.rs:1150 (loading), ui/window.rs:1153/1163, tests (window, pages/mixer, state, final regression 1310).
  3. MixerVisible*/MixerInspection* view model (34-103, 411-451) -> ui/pages/mixer/view_model.rs; window.rs:1386 imports it from there; move shared fixtures (visible_loaded_input etc.) with the tests.
  4. Request dedupe tracker -> MixerState::requested_scene + AppState::claim_mixer_scene_request(intent); port its 13 tests (1622-1786). Duplicate OBS requests are already prevented by status, so this is ownership, not a bug fix.
  5. Inspection stays compiled in (documented manual-test diagnostic SCENEDECK_MIXER_INSPECT=1, docs/manual-test-plan.md:361-365, 406-410); cache the env flag in a OnceLock and keep the eprintln output contract.
- **Acceptance criteria:**
  - [ ] state/mixer.rs < 300 production lines.
- **Verification:** SV + NV (see 2.1)

#### P8-T15

**StateRevisions: per-slice change counters**

- **Why:** `performance-03` (high)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** P8-T03, P8-T07, P8-T08, P8-T09, P8-T10 | **Parallelizable with:** P8-T13, P8-T14
- **Files:**
  - `src/state/revisions.rs (new)`
  - `slice mutators`
- **Steps:**
  1. StateRevisions { session, outputs, audio, topology, registry, workspace, telemetry, health, capabilities } bumped inside slice mutators (private fields make this enforceable).
  2. Test each mutator bumps exactly its counter. Used by P9-T13.
- **Acceptance criteria:**
  - [ ] Every slice mutator bumps its revision (test).
- **Verification:** SV (see 2.1)

#### P8-T16

**Lock the state boundary: controller does not import state; remove the shim**

- **Why:** `architecture-global-01` (high), `architecture-global-11` (medium)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** P8-T01, P8-T14 | **Parallelizable with:** P8-T15
- **Files:**
  - `src/controller/mod.rs`
  - `src/architecture_tests/rules.rs`
- **Steps:**
  1. Remove `pub(crate) use crate::state` from controller/mod.rs.
  2. Add rule `controller_does_not_reach_into_state`: violations("controller", [crate::state, crate::ui]).
  3. Update docs/architecture.md and docs/codebase-overview.md module tables.
- **Acceptance criteria:**
  - [ ] `controller_does_not_reach_into_state` passes; `grep -rn "crate::state" src/controller` returns nothing outside `#[cfg(test)]`.
  - [ ] The `pub(crate) use crate::state` shim is gone from controller/mod.rs.
- **Verification:** SV (see 2.1)

### Phase 9 - UI shell, navigation port and persistence services

Split window.rs and app.rs, narrow NavigationContext, replace pollers and thread_locals.

| ID | Title | Size | Depends on |
|---|---|---|---|
| [P9-T01](#p9-t01) | Collect reports only when needed; share EventUiContext by Rc | S | P0-T15 |
| [P9-T02](#p9-t02) | Event router (part 1): ui/shell/events/ with the exhaustive match and session/output handlers | M | P2-T06 |
| [P9-T03](#p9-t03) | Event router (part 2): scenes, obs scope, audio, workspace, topology, telemetry, feedback | M | P9-T02 |
| [P9-T04](#p9-t04) | Registry persistence and reconciliation leave the event applier | S | P9-T03, P8-T09 |
| [P9-T05](#p9-t05) | Reduce-then-render: UiEffects for event handlers | M | P9-T03, P8-T04 |
| [P9-T06](#p9-t06) | Shell assembly: window, split view, header chrome, sidebar selection | M | P9-T03 |
| [P9-T07](#p9-t07) | Sidebar and header selectors split; connection presenter | M | P4-T20, P9-T06 |
| [P9-T08](#p9-t08) | Page registry and the workspace tick | M | P9-T06, P8-T10 |
| [P9-T09](#p9-t09) | app.rs split: startup, runtime, activation, close guard with an explicit state machine | M | P1-T04, P1-T15, P2-T07 |
| [P9-T10](#p9-t10) | AppState intent methods for pages | M | P8-T09 |
| [P9-T11](#p9-t11) | Migrate pages and widgets from raw state borrows to read/update (batch per area) | M (per batch) | P0-T08, P9-T10 |
| [P9-T12](#p9-t12) | Narrow NavigationContext to a real port | M | P9-T11 |
| [P9-T13](#p9-t13) | Awaitable event pump on tokio::sync::mpsc with a separate tick | M | P0-T15, P7-T15 |
| [P9-T14](#p9-t14) | UI invalidation bus; migrate the first pollers | M | P8-T15 |
| [P9-T15](#p9-t15) | Invalidation bus: remaining pollers and dialog timers | M | P9-T14 |
| [P9-T16](#p9-t16) | Durable persistence: one pure snapshot core, a ui coordinator, no thread_locals | M (x2 PRs) | P1-T04, P9-T09, P5-T12, P8-T01, P2-T07 |
| [P9-T17](#p9-t17) | background_io: wake-up delivery instead of 20 ms polling; module split | M | P9-T16 |
| [P9-T18](#p9-t18) | Global shortcuts: portal client to infra, typed errors, stable ids | M | P4-T13 |
| [P9-T19](#p9-t19) | Replace perpetual tick callbacks with breakpoints; push the registry save status | M | P0-T15 |
| [P9-T20](#p9-t20) | Regroup the flat ui/ root by feature | L (pure move) | P9-T16, P9-T18, P2-T08 |
| [P9-T21](#p9-t21) | Accessible header buttons; one wrapper for decision dialogs | M | P9-T06 |

#### P9-T01

**Collect reports only when needed; share EventUiContext by Rc**

- **Why:** `performance-02` (high), `ui-shell-15` (medium)
- **Commit type:** `perf:` | **Size:** S | **Risk:** Low
- **Depends on:** P0-T15 | **Parallelizable with:** P9-T02
- **Files:**
  - `src/ui/window.rs 412-469`
  - `src/ui/event_pump.rs`
- **Steps:**
  1. Delete the PRE-event collect_report call (428-430): redundant with the previous post-event call and the tick (time-based alerts are covered by the tick).
  2. Skip the post-event call for AppEvent::InputLevelsUpdated (it changes nothing collect_report reads).
  3. Do NOT collect once per batch: that loses intermediate scene and alert transitions (doc comment 269-270).
  4. Store Rc<EventUiContext> in the slot (one refcount per event instead of a deep clone); compute the popout snapshot per batch; name the batch limit.
  5. Measure with perf-trace counters: collect_report calls/s with meters active (hypothesis ~60/s -> <= 20/s; with P8-T12 early-return the cost per call drops to near 0).
- **Acceptance criteria:**
  - [ ] Native and report tests green; counter shows the reduction.
- **Verification:** SV + NV (see 2.1)

#### P9-T02

**Event router (part 1): ui/shell/events/ with the exhaustive match and session/output handlers**

- **Why:** `architecture-global-04` (high), `ui-shell-02` (high), `recent-change-review-11` (medium), `performance-14` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P2-T06 | **Parallelizable with:** P9-T01
- **Files:**
  - `src/ui/window.rs 664-1253`
  - `src/ui/shell/events/{mod.rs, session.rs, output.rs}`
- **Steps:**
  1. Mechanical: events/mod.rs holds the exhaustive match; each arm is a one-line call into a context module. Keep the Observation reconcile and the recursive re-dispatch (apply_event calls at 706, 718, 755) inside events/mod.rs so ordering is unchanged. The recursion is borrow-safe today (refuted panic claim); keep it that way.
  2. Move session arms (SessionRetired, SessionEvent, Connecting, Disconnected, Connected, Error, show_no_session_ui) and output arms (including the OutputReadbackFailed arm 934-976).
  3. Move the related window tests with them.
- **Acceptance criteria:**
  - [ ] window.rs no longer contains those arms; NV green.
- **Verification:** SV + NV + TC (see 2.1)

#### P9-T03

**Event router (part 2): scenes, obs scope, audio, workspace, topology, telemetry, feedback**

- **Why:** `architecture-global-04` (high), `ui-shell-02` (high), `recent-change-review-11` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P9-T02 | **Parallelizable with:** P9-T04
- **Files:**
  - `src/ui/shell/events/{scene_catalog.rs, obs_scope.rs, audio.rs, workspace.rs, topology.rs, telemetry.rs, feedback.rs}`
  - `src/ui/window.rs`
- **Steps:**
  1. Same mechanics, one context per commit. WorkspaceScopeChanging dispatches CancelWorkspace (~862) - keep.
- **Acceptance criteria:**
  - [ ] apply_event lives only in ui/shell/events/mod.rs (< 150 lines).
- **Verification:** SV + NV + TC (see 2.1)

#### P9-T04

**Registry persistence and reconciliation leave the event applier**

- **Why:** `ui-shell-12` (medium)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** P9-T03, P8-T09 | **Parallelizable with:** P9-T05
- **Files:**
  - `src/ui/shell/events/scene_catalog.rs (ex window.rs 985-1058, 1067-1094)`
  - `src/ui/persist/registry_sync.rs (new)`
- **Steps:**
  1. registry_sync::on_inventory(nav, done), on_scene_renamed(nav, uuid, old, new) -> RenameOutcome, on_collection_changed(nav, ..). The event handler only calls them; generation bumps go through ShowWorkspaceSelection (P8-T09).
  2. No `crate::storage` import in ui/shell/events.
- **Acceptance criteria:**
  - [ ] events/ modules import no storage types.
- **Verification:** SV + NV (see 2.1)

#### P9-T05

**Reduce-then-render: UiEffects for event handlers**

- **Why:** `ui-shell-03` (high), `performance-14` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Medium
- **Depends on:** P9-T03, P8-T04 | **Parallelizable with:** P9-T06
- **Files:**
  - `src/ui/shell/events/effects.rs (new)`
  - `src/state/reducer.rs (new)`
  - `src/ui/shell/events/*.rs`
- **Steps:**
  1. `struct UiEffects { scene_cards, audio_cards, pages: PageSet (array bitset over Page; no enum-map dependency), stats, outputs: Vec<OutputKind>, indicators, toasts, selectors, live_view }`.
  2. Per context, move the pure state transition into `AppState::apply(event, now) -> UiEffects` in src/state/reducer.rs; the ui handler renders the effects. One context per PR (session and outputs first).
  3. Payoff: tests like native_output_read_failure_cannot_replace_newer_evidence_or_invent_a_stop (ui=None, asserts only AppState) become plain unit tests.
- **Acceptance criteria:**
  - [ ] At least session and output handlers are reduce+render; their state tests run without GTK.
- **Verification:** SV + NV (see 2.1)

#### P9-T06

**Shell assembly: window, split view, header chrome, sidebar selection**

- **Why:** `ui-shell-14` (medium), `ui-shell-24` (low), `ui-shell-22` (low)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P9-T03 | **Parallelizable with:** P9-T07
- **Files:**
  - `src/ui/window.rs 63-280, 513-640, 1455-1496`
  - `src/ui/shell/{window.rs, split_view.rs, lifecycle.rs, header.rs, welcome.rs}`
- **Steps:**
  1. build_main_window (210 lines) -> orchestrator (~80) + split_view (breakpoint and toggle, merge the two row_selected handlers) + lifecycle (teardown; keep 'quit' out of the action list: the close guard replaces it) + header.
  2. HeaderChrome { header, live_icon, selectors } built before EventUiContext and stored by value; delete StreamingChromeRef; sync_streaming_chrome -> HeaderChrome::set_streaming(bool).
  3. One sidebar selection closure: switch_to_page + set_show_content; Live rebuild moves to the Live page's map handler.
  4. Replace expect("static breakpoint") (~176) with a const-checked parse or a documented infallible helper.
- **Acceptance criteria:**
  - [ ] window.rs deleted or < 100 lines.
- **Verification:** SV + NV (see 2.1)

#### P9-T07

**Sidebar and header selectors split; connection presenter**

- **Why:** `ui-shell-17` (medium), `ui-pages-07` (high)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P4-T20, P9-T06 | **Parallelizable with:** P9-T08
- **Files:**
  - `src/ui/sidebar.rs 19-334, 422-425`
  - `src/ui/shell/{sidebar.rs, header_selectors.rs}`
  - `src/ui/connection.rs (new)`
  - `src/ui/output_flow.rs (new)`
  - `src/ui/dialogs.rs (new)`
  - `src/ui/pages/live.rs 310-604`
- **Steps:**
  1. HeaderSelectors (34-124), NamedSelector (127-239) and their native tests -> shell/header_selectors.rs; NAV_PAGES/build/SidebarControls -> shell/sidebar.rs.
  2. Disconnect and output start/stop flows (live.rs 310-604, owned by sidebar and transport bar: sidebar 329/422/425, transport_bar 87/91/264) -> ui/output_flow.rs using domain policy (P4-T20); one dialogs::confirm(parent, ConfirmSpec, on_accept) replaces the two near-identical dialogs (414-432, 583-595).
  3. ui/connection.rs (not ui/shell, because pages also import it) holds disconnect_label.
- **Acceptance criteria:**
  - [ ] sidebar.rs no longer imports ui::pages::live.
- **Verification:** SV + NV (see 2.1)

#### P9-T08

**Page registry and the workspace tick**

- **Why:** `ui-shell-10` (medium), `ui-shell-09` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P9-T06, P8-T10 | **Parallelizable with:** P9-T09
- **Files:**
  - `src/ui/window.rs 281-411, 296-375, 1499-1572`
  - `src/ui/shell/{page_registry.rs, ticks/workspace_tick.rs}`
  - `src/state/workspace (workspace_refresh_due)`
  - `CLAUDE.md 'New page' recipe`
- **Steps:**
  1. PageRegistry: array keyed by an exhaustive Page index fn (no strum/enum-map); each page builder returns PageBuild { widget, refresh }; delete PageRefreshers' closure-per-field and the call() match.
  2. Extract `AppState::workspace_refresh_due(&self, SurfaceVisibility { live_mapped, sources_mapped, window_active }, now) -> bool` with unit tests; the tick only gathers visibility.
  3. Remove the double rebuild in one tick (generation check ~321 and revision check ~368 both rebuild scene cards).
  4. Update the CLAUDE.md New-page recipe.
- **Acceptance criteria:**
  - [ ] Adding a page touches the Page enum, NAV_PAGES and the page module only.
- **Verification:** SV + NV (see 2.1)

#### P9-T09

**app.rs split: startup, runtime, activation, close guard with an explicit state machine**

- **Why:** `ui-shell-06` (high), `ui-shell-07` (medium), `recent-change-review-20` (medium), `recent-change-review-17` (medium), `architecture-global-20` (low), `ui-shell-21` (low)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P1-T04, P1-T15, P2-T07 | **Parallelizable with:** P9-T08
- **Files:**
  - `src/app.rs (32-159, 161-428, 431-455)`
  - `src/app/{mod.rs, startup.rs, runtime.rs, activation.rs}`
  - `src/ui/shell/{close_guard.rs, exit_policy.rs}`
  - `tools/test-native.py`
  - `i18n/*/scenedeck.ftl`
- **Steps:**
  1. app/mod.rs keeps run() only (~60 lines). startup.rs: StartupData + load_startup. runtime.rs: build_runtime, shutdown_controller (nested MainLoop 97-115).
  2. CloseGuard -> ui/shell/close_guard.rs with `enum CloseState { Idle, Draining { abandon }, AwaitingDecision, Allowed }` + shared QuitIntent; methods begin_close, drain_finished(Result), decide(Decision::{Cancel, Retry, Abandon}).
  3. Pure ExitPolicy: inputs { is_last_document, quitting, abandon, flush_outcome, checkpoint_dirty } -> ExitStep { Proceed, Drain, CheckpointAgain, ShowFailure, Quit, CloseWindow } with table tests (today only #[ignore] native tests at 574-575, 741-742, 750-751, 759 cover it).
  4. Dedicated keys app-close-failure-heading/-cancel/-retry in every locale (today borrows workflow-activity-failed, inventory-dialog-cancel-label, window-connect-btn-retry).
  5. Native tests that move out of app::tests::native_ stay discoverable (native_ prefix from P0-T02).
- **Acceptance criteria:**
  - [ ] app.rs production < 100 lines; ExitPolicy unit tests cover every row.
- **Verification:** SV + NV + IV (see 2.1)

#### P9-T10

**AppState intent methods for pages**

- **Why:** `ui-pages-01` (high)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P8-T09 | **Parallelizable with:** P9-T11
- **Files:**
  - `src/state/mixer.rs`
  - `src/state/mod.rs`
  - `src/ui/pages/{mixer.rs 585-586, 631-635, 676, 705, 837; settings.rs 48, 781-783; settings/connections.rs; doctor.rs 107}`
- **Steps:**
  1. Add select_mixer_mode(MixerMode), select_mixer_scene(SceneId) (owns the pinned rule at mixer.rs:633), set_mixer_grouping, set_mixer_search(&str), set_output_confirmation(OutputKind, ..), dismiss_startup_notice(), and similar intent methods for each direct field write found in pages.
  2. Unit test each.
- **Acceptance criteria:**
  - [ ] Each page write has an intent method.
- **Verification:** SV (see 2.1)

#### P9-T11

**Migrate pages and widgets from raw state borrows to read/update (batch per area)**

- **Why:** `architecture-global-02` (high), `ui-shell-05` (high), `ui-pages-01` (high), `controller-state-21` (medium)
- **Commit type:** `refactor:` | **Size:** M (per batch) | **Risk:** Low
- **Depends on:** P0-T08, P9-T10 | **Parallelizable with:** P9-T12
- **Files:**
  - `src/ui/pages/*.rs`
  - `src/ui/widgets/*.rs`
  - `src/ui/*.rs feature modules`
- **Steps:**
  1. One PR per area (mixer+live+stats; settings+connections+doctor+inventory; widgets; feature windows). Replace `nav.state.borrow_mut()` with `nav.update(|s| s.intent(..))?` and reads with `nav.read(..)`.
  2. Rule for closures: no dispatch, persist or refresher call inside read/update.
  3. Lower the P0-T07 borrow ratchet ceilings in each PR.
- **Acceptance criteria:**
  - [ ] Ratchet ceilings reach 0 for pages and widgets.
- **Verification:** SV + NV (see 2.1)

#### P9-T12

**Narrow NavigationContext to a real port**

- **Why:** `architecture-global-02` (high), `ui-shell-05` (high), `recent-change-review-13` (medium), `ui-shell-23` (low), `controller-state-21` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P9-T11 | **Parallelizable with:** P9-T13
- **Files:**
  - `src/ui/navigation.rs 18-32, 52-213`
  - `src/ui/scope.rs (new)`
  - `src/architecture_tests/rules.rs 640-660`
  - `CLAUDE.md`
  - `docs/architecture.md`
- **Steps:**
  1. Make `state` private; keep a narrow accessor for ui/shell and ui/persist only.
  2. Registry binding -> a CurationGuard value from a ui-side curation service (or the persistence coordinator); global-shortcut stopping becomes a reaction to SessionRetired/WorkspaceScopeChanging in the event handlers, not a dispatch side effect (201-206). parent_window stays if needed by dialogs.
  3. The pages_reach_the_controller_only_through_navigation exemption names the narrowest files (ui/shell/window.rs, ui/navigation.rs) - keep navigation.rs in ui/, not ui/shell (pages depend on it).
  4. Add rule: no `nav.state` under ui/pages or ui/widgets.
  5. Correct CLAUDE.md and docs/architecture.md wording.
- **Acceptance criteria:**
  - [ ] NavigationContext public surface: switch_to_page, dispatch, read, update, is_current, parent_window.
- **Verification:** SV + NV (see 2.1)

#### P9-T13

**Awaitable event pump on tokio::sync::mpsc with a separate tick**

- **Why:** `performance-05` (high), `ctrl-17` (medium)
- **Commit type:** `perf:` | **Size:** M | **Risk:** Medium: threading boundary
- **Depends on:** P0-T15, P7-T15 | **Parallelizable with:** P9-T12
- **Files:**
  - `src/ui/event_pump.rs 11-80`
  - `src/controller/event_sender.rs 104-145`
  - `src/app.rs 41, 674`
  - `CLAUDE.md`
  - `docs/architecture.md`
  - `docs/ui-state-updates.md`
- **Steps:**
  1. Producer: send_if uses Sender::reserve().await, then valid() and scope checks synchronously, then permit.send(event): keeps the 'no await between check and admission' invariant (event_sender.rs:104-106). async-channel cannot do this (no permit API).
  2. Consumer: glib::MainContext::default().spawn_local loop: recv().await, drain up to 127 more with try_recv, after_batch(), yield. Store the JoinHandle and abort on window_removed/shutdown (mirror event_pump.rs:24-41). Keep drain_is_bounded_and_keeps_queued_event_order as a test over drain_ready(rx, max).
  3. The wall-clock tick() (health tick, report_history tick, collect_report) keeps its own timer.
  4. EventSender::try_send (meters, thumbnails) maps to tokio try_send. Check the second sync_channel(128) at app.rs:674.
  5. Update CLAUDE.md and docs (they say std::sync::mpsc and 50 ms).
  6. Measure: observed_at -> applied_at latency for InputVolumeChanged/CurrentSceneChanged (hypothesis p50 ~25 ms -> < 2 ms) and idle wakeups (strace).
- **Acceptance criteria:**
  - [ ] No 2 ms sleep loop; no 50 ms pump timer.
- **Verification:** SV + NV (see 2.1)

#### P9-T14

**UI invalidation bus; migrate the first pollers**

- **Why:** `performance-03` (high), `uiw-14` (medium)
- **Commit type:** `perf:` | **Size:** M | **Risk:** Low
- **Depends on:** P8-T15 | **Parallelizable with:** P9-T15
- **Files:**
  - `src/ui/invalidation.rs (new)`
  - `src/ui/widgets/{transport_bar.rs 195, studio_controls.rs 65, 177, output_results_tray.rs 25-43}`
  - `src/ui/health_alerts.rs 142-175`
- **Steps:**
  1. UiInvalidation bitset computed by diffing StateRevisions (P8-T15) around each pump batch; UiInvalidationBus holds Weak closures with an interest mask; never subscribe while a borrow_mut is held.
  2. Migrate one timer per commit: transport_bar -> studio_controls (no is_mapped guard today) -> output_results_tray (clones every OutputResult per tick) -> health summary.
  3. Keep wall-clock timers where time is the input (output clock 1 s, preview cadence, output_results.expire()).
- **Acceptance criteria:**
  - [ ] Four pollers removed; native tests green.
- **Verification:** SV + NV (see 2.1)

#### P9-T15

**Invalidation bus: remaining pollers and dialog timers**

- **Why:** `performance-03` (high), `uiw-14` (medium)
- **Commit type:** `perf:` | **Size:** M | **Risk:** Low
- **Depends on:** P9-T14 | **Parallelizable with:** P9-T16
- **Files:**
  - `src/ui/window.rs 306-375 timer (or shell/ticks/workspace_tick.rs)`
  - `src/ui/previews.rs 31`
  - `src/ui/pages/inventory.rs 539-570`
  - `src/ui/widgets/operating_panel.rs 107, 118`
  - `dialogs: quick_actions.rs 434, action_shortcuts.rs 367/551, preflight.rs 428, preflight/editor.rs 366, command_palette.rs 272, audio_presets.rs 699, report_history.rs 533, bookmarks.rs 359 (200 ms), health_alerts.rs 296, health_alerts/audio_editor.rs 272, global_shortcuts.rs 399`
- **Steps:**
  1. Subscribe each to the matching interest mask; keep completion pollers for blocking work (app.rs 860/961, background_io) out of scope (P9-T17 handles them).
  2. Measure idle wakeups with `strace -f -c -e trace=ppoll -p PID` for 30 s and pidstat cswch/s (hypothesis ~70/s -> < 5/s).
- **Acceptance criteria:**
  - [ ] No state-polling timeout_add_local remains in ui (ratchet).
- **Verification:** SV + NV (see 2.1)

#### P9-T16

**Durable persistence: one pure snapshot core, a ui coordinator, no thread_locals**

- **Why:** `recent-change-review-03` (high), `architecture-global-10` (medium), `uiw-07` (medium), `uiw-06` (high), `recent-change-review-02` (high)
- **Commit type:** `refactor:` | **Size:** M (x2 PRs) | **Risk:** Medium
- **Depends on:** P1-T04, P9-T09, P5-T12, P8-T01, P2-T07 | **Parallelizable with:** P9-T17
- **Files:**
  - `src/ui/persist.rs (23-271)`
  - `src/ui/report_history.rs 21-308`
  - `src/ui/bookmarks.rs 21-79`
  - `src/ui/session_reports.rs 22-23`
  - `src/state/persistence/snapshot.rs (new, GTK-free)`
  - `src/ui/persist/{mod.rs, config.rs, registry_journal.rs, registry_writer.rs, registry_import.rs, registry_scope.rs}`
  - `src/ui/registry_status.rs (new)`
  - `src/ui/persistence/coordinator.rs (new)`
- **PR breakdown (critic pass):** PR 1 = steps 1-2 (pure `RegistryJournal` and `DurableSnapshot` in `src/state/persistence/`, with unit tests, no caller changes). PR 2 = step 3 (coordinator, thread_local removal, ratchet lowered). PR 3 = step 4's file split (pure move, TC). Keep `ui/persist.rs` as the module root (P9-T04 already created `ui/persist/registry_sync.rs`); do not add `ui/persist/mod.rs` (rule 2.2.4).
- **Steps:**
  1. Extract pure RegistryJournal { revision, operations, failures, error, retrying } (persist.rs 69-120 confirm_*/registry_write_finished) with unit tests (today only subprocess + GTK tests via isolated_test_child).
  2. Generic DurableSnapshot<T> { SaveState { Idle, Saving{revision, force_after}, Failed{revision, error, evidence: E} } } GTK-free and generic over evidence (FailureEvidence is a ui type; core cannot name crate::ui).
  3. PersistenceCoordinator in ui owns registry, report history, bookmarks, settings and export stores (DurableStore trait: retry(), status()); CloseGuard calls retry_all (replaces P1-T04's helper).
  4. Split persist.rs as listed; registry_availability_message -> ui/registry_status.rs. Replace the thread_locals (42, 207; report_history 21-24; bookmarks 21-22; session_reports 22-23) with coordinator state. Lower the thread_local ratchet. Rename isolated_test_child names that embed 'ui::persist::tests::'.
- **Acceptance criteria:**
  - [ ] No thread_local in persist.rs, report_history.rs, bookmarks.rs, session_reports.rs.
- **Verification:** SV + NV (see 2.1)

#### P9-T17

**background_io: wake-up delivery instead of 20 ms polling; module split**

- **Why:** `uiw-08` (medium), `security-13` (low)
- **Commit type:** `perf:` | **Size:** M | **Risk:** Low
- **Depends on:** P9-T16 | **Parallelizable with:** P9-T18
- **Files:**
  - `src/ui/background_io.rs (894) -> ui/background_io/{mod.rs, lanes.rs, config_batch.rs, ledger.rs, flush.rs, test_isolation.rs}`
- **Steps:**
  1. run(): gio::spawn_blocking + MainContext::spawn_local (or an async oneshot; check futures-util `channel` feature) instead of thread::spawn per job + 20 ms timers (106, 220, 324, 356).
  2. Keep PendingJob accounting (288-304) for flush_pending (318); record_failure on JoinError.
  3. Serial lanes keep their own threads; the Secret lane never moves to the GIO pool (a hung keyring would occupy a shared worker).
  4. Keep isolated_test_child reachable at crate::ui::background_io::isolated_test_child.
- **Acceptance criteria:**
  - [ ] No 20 ms poll timer in background_io.
- **Verification:** SV + NV (see 2.1)

#### P9-T18

**Global shortcuts: portal client to infra, typed errors, stable ids**

- **Why:** `uiw-15` (medium), `recent-change-review-21` (low)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P4-T13 | **Parallelizable with:** P9-T17
- **Files:**
  - `src/ui/global_shortcuts.rs 3, 62-90, 164-307`
  - `src/ui/global_shortcuts/portal.rs -> src/infra/portal/global_shortcuts.rs`
  - `src/domain/workspace/operator_action.rs (stable_key)`
  - `i18n/*/scenedeck.ftl`
- **Steps:**
  1. Move portal.rs to infra (architecture_tests do not restrict gio in infra) with `enum PortalError { Timeout, Unavailable, Bus(String), Denied }`; map to new fl! keys in every locale (today raw English at 197, portal.rs 119/121, map_err(to_string)).
  2. Split the 143-line start().
  3. OperatorAction::stable_key() and WorkspaceKey::stable_key() hashed for portal shortcut ids instead of serde_json serialization; golden test that existing ids stay byte-identical (or accept a documented one-time migration). Replace the three expects (68, 77, 79).
- **Acceptance criteria:**
  - [ ] Changing OperatorAction serde does not change shortcut ids (golden).
- **Verification:** SV + NV + IV (see 2.1)

#### P9-T19

**Replace perpetual tick callbacks with breakpoints; push the registry save status**

- **Why:** `performance-01` (high), `ui-pages-16` (medium)
- **Commit type:** `perf:` | **Size:** M | **Risk:** Low
- **Depends on:** P0-T15 | **Parallelizable with:** P9-T01..P9-T18
- **Files:**
  - `src/ui/widgets/status_bar.rs 142-165`
  - `src/ui/pages/live.rs 137-152, 270-284`
  - `src/ui/pages/inventory.rs 528-572`
  - `src/ui/responsive.rs (new)`
- **Steps:**
  1. Pure helpers status_layout(width) (already pure) and live_layout(height, expanded) -> LiveLayout in ui/responsive.rs with unit tests.
  2. Attach adw::Breakpoint conditions to the WINDOW (status bar measures window width, 143-144), not the status bar; replace both live.rs ticks together (the second reads the class set by the first, 272). Precedent: inventory.rs:507 and window.rs:175.
  3. Registry save status: persist::subscribe_registry_save_status(Fn(RegistrySaveStatus)) -> SubscriptionGuard instead of a 100 ms poll with fl! formatting.
  4. Guard: native short_windows_collapse_audio_until_expanded.
  5. Measure: GTK inspector FPS overlay and `pidstat -u 1` idle, disconnected (hypothesis idle frames -> ~0, idle CPU -50%).
- **Acceptance criteria:**
  - [ ] No add_tick_callback returning Continue forever outside meters.
- **Verification:** SV + NV (see 2.1)

#### P9-T20

**Regroup the flat ui/ root by feature**

- **Why:** `recent-change-review-27` (low)
- **Commit type:** `refactor:` | **Size:** L (pure move) | **Risk:** Low
- **Depends on:** P9-T16, P9-T18, P2-T08 | **Parallelizable with:** P10
- **Files:**
  - `src/ui/mod.rs and 27 root files`
- **Steps:**
  1. ui/shell/{window, events/, event_pump, close_guard, exit_policy}, ui/infra/{background_io, persistence, theme}, ui/operator/{catalog, shortcuts, global_shortcuts/, quick_actions, command_palette, picker}, ui/reports/{session_reports, history, bookmarks}, ui/audio/{presets/, health_alerts/}, ui/preflight. navigation.rs stays at ui/ root.
  2. Pure moves; one group per PR; update architecture exemptions; optional rule ui/operator must not import ui/reports.
- **Acceptance criteria:**
  - [ ] ui root holds mod.rs, navigation.rs and group dirs.
- **Verification:** SV + NV + TC (see 2.1)

#### P9-T21

**Accessible header buttons; one wrapper for decision dialogs**

- **Why:** `ui-shell-19` (low), `ui-shell-20` (low)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P9-T06 | **Parallelizable with:** P10
- **Files:**
  - `src/ui/window.rs 181-183, 593-600 (shell/header.rs)`
  - `src/ui/sidebar.rs 65-70`
  - `src/ui/widgets/alert.rs (new)`
  - `37 MessageDialog references`
- **Steps:**
  1. header_button sets accessible Property::Label from the tooltip; route sidebar_toggle through it; HeaderSelectors::build calls update_context() once. Native test: every content-header Button has an accessible label.
  2. Wrap dialog creation in ui/widgets/alert.rs decision_dialog(parent, heading, body, responses) and find dialogs by widget_name in tests; migration to AlertDialog waits for the v1_5 feature (AlertDialog in 1.5, MessageDialog deprecated in 1.6).
- **Acceptance criteria:**
  - [ ] All 37 MessageDialog uses go through the wrapper.
- **Verification:** SV + NV (see 2.1)

### Phase 10 - UI pages and widgets

Split each page and widget god file along the view/presenter/rules seams.

| ID | Title | Size | Depends on |
|---|---|---|---|
| [P10-T01](#p10-t01) | Sources page: move operating_panel into ui/pages/sources/ | L (pure move) + M | P0-T13, P2-T08 |
| [P10-T02](#p10-t02) | Sources page: typed handles instead of widget-name strings and tree visits | M | P10-T01 |
| [P10-T03](#p10-t03) | Sources page: SourceDrafts with intent methods | M | P10-T01 |
| [P10-T04](#p10-t04) | Sources page: media refresh request and freshness from state; drop redundant work | S | P10-T01, P8-T10 |
| [P10-T05](#p10-t05) | Inventory page split | L (pure move) + S | P0-T03, P0-T13, P5-T06, P2-T08 |
| [P10-T06](#p10-t06) | Inventory typed view instead of recursive widget-name search | M | P10-T05 |
| [P10-T07](#p10-t07) | Mixer page split | L (pure move) | P8-T14, P2-T08 |
| [P10-T08](#p10-t08) | Mixer: typed structure keys and one binding per refresh | M | P10-T07, P8-T08 |
| [P10-T09](#p10-t09) | Mixer: pure MixerBody decision, short borrows, shared group/filter helpers | M | P10-T07, P4-T13 |
| [P10-T10](#p10-t10) | Live page split: scene deck and audio strip | M | P4-T23, P9-T07, P9-T14, P2-T08 |
| [P10-T11](#p10-t11) | Stats page split; one activity-marker helper | M | P4-T20, P2-T08 |
| [P10-T12](#p10-t12) | Doctor and Graph share a topology presenter | S | P10-T08 |
| [P10-T13](#p10-t13) | Shared Cairo primitives; graph canvas with Pango text; graph layout as a service | M | P10-T12 |
| [P10-T14](#p10-t14) | Settings page split; generic enum combo row | M | P9-T11, P2-T08 |
| [P10-T15](#p10-t15) | Connections editor split | S | P4-T22, P1-T14, P0-T02 |
| [P10-T16](#p10-t16) | Help topics as data | S | - |
| [P10-T17](#p10-t17) | Audio card: split, intent methods, change detection, no test hook in production | M | P9-T10, P4-T08, P2-T08 |
| [P10-T18](#p10-t18) | One meter tick per page (MeterHub) and idle meters stop ticking | M | P7-T15 |
| [P10-T19](#p10-t19) | Action shortcuts and audio presets editors split; rules into the domain | M | P4-T19, P9-T15, P0-T02 |
| [P10-T20](#p10-t20) | Theme module split; built-in themes as one table | M | P2-T07 |
| [P10-T21](#p10-t21) | Chart colors from theme tokens; mode-neutral theme family names | M | P10-T20, P10-T13 |
| [P10-T22](#p10-t22) | UI kit helpers and test finders; remove production unwraps in factories | M | P0-T14 |
| [P10-T23](#p10-t23) | Accessible names for icon-only controls and the graph canvas | S | P10-T22 |
| [P10-T24](#p10-t24) | Decode preview images off the GTK thread | S | P7-T11 |

#### P10-T01

**Sources page: move operating_panel into ui/pages/sources/**

- **Why:** `uiw-01` (high), `architecture-global-15` (medium)
- **Commit type:** `refactor:` | **Size:** L (pure move) + M | **Risk:** Low
- **Depends on:** P0-T13, P2-T08 | **Parallelizable with:** P10-T05, P10-T07
- **Files:**
  - `src/ui/widgets/operating_panel.rs (production 1-1613)`
  - `src/ui/pages/sources.rs (9-line shim)`
  - `src/ui/widgets/mod.rs:14`
  - `src/ui/widgets/lifetime_tests.rs 3, 83`
  - `src/ui/widgets/operating_panel/media_progress.rs`
- **Steps:**
  1. git mv to ui/pages/sources/mod.rs; delete the shim; fold media_progress into pages/sources/detail/media_progress.rs.
  2. Then split: content.rs (SourcesContent 317, sources_content_from_state 509), status.rs (update_status 548), browser.rs (populate 678-1096 -> build_browser -> SourceBrowserHandle), detail/{mod, filters, text_editor, media, audio}.rs (build_source_detail 1098-1492 -> DetailHandle), helpers to ui kit.
  3. Native test paths are compiled (P0-T02); P0-T13 section tests guard each step.
- **Acceptance criteria:**
  - [ ] No file under pages/sources/ > 500 production lines.
- **Verification:** SV + NV + TC (see 2.1)

#### P10-T02

**Sources page: typed handles instead of widget-name strings and tree visits**

- **Why:** `uiw-02` (high)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P10-T01 | **Parallelizable with:** P10-T03
- **Files:**
  - `src/ui/pages/sources/*.rs (ex operating_panel 271-312, 363-402, 414-503, 548-651, 1541, 1601-1612)`
- **Steps:**
  1. Return typed handles at build time like AudioCardHandle (audio_card.rs:42-53): SourceRowHandle { check, hidden_parent } keyed by SourceItemKey { owner, item_id, path }; DetailHandles { text, media, numeric, .. }.
  2. Remove format!("control-focus-...") keys built from fl! labels (430-450 vs 1541): those are locale-dependent and break in non-English locales.
- **Acceptance criteria:**
  - [ ] No widget lookup by name in pages/sources (except tests).
- **Verification:** SV + NV (see 2.1)

#### P10-T03

**Sources page: SourceDrafts with intent methods**

- **Why:** `uiw-03` (high)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P10-T01 | **Parallelizable with:** P10-T02
- **Files:**
  - `src/ui/pages/sources/drafts.rs (new; ex LocalState 16-92 and edits at 1225-1232, 1318-1322, 1457-1461, 1494-1529)`
- **Steps:**
  1. enum NumericField { Seek, SyncOffset, Balance }; AudioTracks from P4-T07; SourceDrafts { text, origins, numeric, numeric_text, tracks } with text_draft, edit_text, discard_text, text_conflict, numeric, set_numeric_text, commit_numeric.
  2. Unit tests (existing 2276+ move).
- **Acceptance criteria:**
  - [ ] No external map edits with string field keys.
- **Verification:** SV + NV (see 2.1)

#### P10-T04

**Sources page: media refresh request and freshness from state; drop redundant work**

- **Why:** `uiw-17` (medium), `uiw-20` (low)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** P10-T01, P8-T10 | **Parallelizable with:** P10-T02, P10-T03
- **Files:**
  - `src/ui/pages/sources/*.rs (ex 128-176, 549-567, 797-798, 1134-1139, 1238-1243, 1334-1339, 1485-1490, 1601-1612)`
  - `src/state/workspace/`
- **Steps:**
  1. WorkspaceState::is_fresh() (one 30 s rule, CONTROL_FRESHNESS) and AppState::media_refresh_request() -> Option<WorkspaceRequest> with tests; the widget timer calls them.
  2. Drop the second update_status call (173) after verifying update_observed_controls writes no AppState; remove the dead `_expanded` parameter (all callers pass false) and rename expand -> section.
- **Acceptance criteria:**
  - [ ] Widget builds no WorkspaceRequest by hand.
- **Verification:** SV + NV (see 2.1)

#### P10-T05

**Inventory page split**

- **Why:** `ui-pages-03` (high), `architecture-global-16` (medium)
- **Commit type:** `refactor:` | **Size:** L (pure move) + S | **Risk:** Low
- **Depends on:** P0-T03, P0-T13, P5-T06, P2-T08 | **Parallelizable with:** P10-T01, P10-T07
- **Files:**
  - `src/ui/pages/inventory.rs (production 1-1504) -> ui/pages/inventory/{mod, presentation, scene_row, scene_options, accent, reorder, stale, yaml_transfer, save_status}.rs`
  - `src/ui/widgets/breakpoint_focus.rs (preserve_breakpoint_focus 462, if shared)`
- **Steps:**
  1. Pure parts first (presentation.rs: read 76), then view pieces: build_scene_row 598, build_role_cell 697, build_scene_options 751, build_accent_controls 898, install_scene_reordering 960, build_stale_group 1019, reordered_scene_ids 1152, YAML import/export 1384-1477, build_registry_save_status 528.
  2. Remove the 15 production unwrap/expect while moving (10 on the update_rows path 222-315).
- **Acceptance criteria:**
  - [ ] No inventory file > 400 production lines; no unwrap.
- **Verification:** SV + NV + TC (see 2.1)

#### P10-T06

**Inventory typed view instead of recursive widget-name search**

- **Why:** `ui-pages-04` (high)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P10-T05 | **Parallelizable with:** P10-T07
- **Files:**
  - `src/ui/pages/inventory/view.rs (new; ex 198-326, 1106-1118, 1474-1477)`
- **Steps:**
  1. SceneRowHandle { row, role: DropDown, curation: Label, drag: Image, options_slot: GtkBox }; InventoryView { list, content, empty, reorder_hint, deck_controls, rows: RefCell<HashMap<SceneId, SceneRowHandle>> }; build_scene_row returns the handle; update_rows uses it.
- **Acceptance criteria:**
  - [ ] No inventory_widget(..) recursive lookup.
- **Verification:** SV + NV (see 2.1)

#### P10-T07

**Mixer page split**

- **Why:** `architecture-global-17` (medium)
- **Commit type:** `refactor:` | **Size:** L (pure move) | **Risk:** Low
- **Depends on:** P8-T14, P2-T08 | **Parallelizable with:** P10-T01, P10-T05
- **Files:**
  - `src/ui/pages/mixer.rs (production 38-1223) -> ui/pages/mixer/{mod, handle, controls, body, rows, requests, inspection, view_model}.rs`
- **Steps:**
  1. Pure move per area: handle (50-170), controls (560-700 rows), body/rows (append_group 858-892), requests (1116-1194 if not moved to state in P8-T14), inspection (290-313, 941-1049; keep compiled, OnceLock flag, eprintln contract).
- **Acceptance criteria:**
  - [ ] No mixer file > 400 production lines.
- **Verification:** SV + NV + TC (see 2.1)

#### P10-T08

**Mixer: typed structure keys and one binding per refresh**

- **Why:** `ui-pages-09` (medium), `performance-09` (medium)
- **Commit type:** `perf:` | **Size:** M | **Risk:** Low
- **Depends on:** P10-T07, P8-T08 | **Parallelizable with:** P10-T09
- **Files:**
  - `src/ui/pages/mixer/structure_key.rs (new; ex mixer.rs 31-45, 106-221, 370-376)`
  - `src/state/mod.rs (presentation_owner)`
- **Steps:**
  1. PresentationOwner { epoch, scope, registry_generation, registry_status, language } via AppState::presentation_owner() reused by every page (20 show_registry.generation reads in ui).
  2. MixerStructureKey/MixerControlsKey as derive(PartialEq, Eq) structs replacing the Debug-format String signatures (Debug changes would silently change rebuild behavior).
  3. Compute the binding once per refresh; internal update_mute_unchecked/update_volume_unchecked; keep public update_* for single echoes (window 1180, 1201).
  4. Keep search in the key until a grouping-aware filter exists (cards live in several PreferencesGroups; FlowBox filtering alone would leave empty headers).
  5. Measure: refresh span with 40 inputs (hypothesis fast path >= 5x cheaper).
- **Acceptance criteria:**
  - [ ] No format!("{:?}") used as a key in ui/pages.
- **Verification:** SV + NV (see 2.1)

#### P10-T09

**Mixer: pure MixerBody decision, short borrows, shared group/filter helpers**

- **Why:** `ui-pages-11` (medium), `ui-pages-12` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P10-T07, P4-T13 | **Parallelizable with:** P10-T08
- **Files:**
  - `src/ui/pages/mixer/body.rs (ex populate 355-558)`
  - `src/domain/audio/mixer.rs (filter_inputs, group_inputs)`
- **Steps:**
  1. enum MixerBody { NoScenes, MissingScene, Loading, Error, NeedsRequest, Loaded{..} } computed inside one nav.read; render after; dispatch NeedsRequest outside any borrow (replaces drop(state) at 528).
  2. filter_inputs (894-913) and group_inputs (identical blocks 764-775 and 776-791) in domain with references instead of AudioInput clones; append_group takes &[&AudioInput]. Keep the Fluent fallback label in ui.
- **Acceptance criteria:**
  - [ ] populate < 60 lines; domain tests for both groupings and empty search.
- **Verification:** SV + NV (see 2.1)

#### P10-T10

**Live page split: scene deck and audio strip**

- **Why:** `ui-pages-08` (high), `performance-04` (high)
- **Commit type:** `perf:` | **Size:** M | **Risk:** Low
- **Depends on:** P4-T23, P9-T07, P9-T14, P2-T08 | **Parallelizable with:** P10-T11
- **Files:**
  - `src/ui/pages/live.rs (659-802, 830-967) -> ui/pages/live/{mod, scene_deck, audio_strip, responsive}.rs`
- **Steps:**
  1. rebuild_scene_cards(handle, nav): compute ScenePresentation inside a scoped borrow, early-return on the fast path WITHOUT cloning registry or inventory (today the clone at 665-672 precedes the signature check at 728). Drop the borrow before calling quick_actions_refresh, registry_availability_message, hotkeys::scene_hint, deck_controls (they borrow, some may borrow_mut).
  2. SceneDeckBody { Unavailable, ObsEmpty, NoResults, NoLiveRoles, Cards }; AudioStripView methods show_refreshing/update_in_place/show_placeholder/reconcile; replace previous.as_ref().unwrap() (921).
  3. Dedupe through invalidation so one scene cut rebuilds once (today 3+: apply_event, the 200 ms revision tick, WorkspaceLoaded).
  4. Measure: rebuild count and time per scripted scene switch (hypothesis 3+ -> 1, zero clone bytes on fast path).
- **Acceptance criteria:**
  - [ ] No live file > 400 production lines; one rebuild per cut.
- **Verification:** SV + NV (see 2.1)

#### P10-T11

**Stats page split; one activity-marker helper**

- **Why:** `ui-pages-21` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P4-T20, P2-T08 | **Parallelizable with:** P10-T10
- **Files:**
  - `src/ui/pages/stats.rs -> ui/pages/stats/{mod, popout, gauges, charts, cards, activity}.rs`
- **Steps:**
  1. Popout lifecycle (open_popout 202 with destroy/notify x2/close-request) -> one PopoutLifecycle on_closed closure in popout.rs.
  2. apply_marker(charts, status, fraction, selected) replaces the duplicated code at 766-775 and 829-837.
  3. Activity: compare workspace.activity.sequence() instead of cloning the last 15 records (776-787).
- **Acceptance criteria:**
  - [ ] No stats file > 400 production lines.
- **Verification:** SV + NV + TC (see 2.1)

#### P10-T12

**Doctor and Graph share a topology presenter**

- **Why:** `ui-pages-14` (medium)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** P10-T08 | **Parallelizable with:** P10-T13
- **Files:**
  - `src/ui/pages/topology.rs (new)`
  - `src/ui/pages/doctor.rs 28-55, 119-132`
  - `src/ui/pages/graph.rs 36-62, 74-85`
- **Steps:**
  1. TopologyPresentation { owner: PresentationOwner, connected, content_key } and topology_notice(nav).
  2. Keep content comparison (edges, registry snapshot) for the key: a generation counter would miss registry edits nobody tracks today. Avoid the full clone by hashing or comparing in place under one borrow.
- **Acceptance criteria:**
  - [ ] No duplicated notice chain.
- **Verification:** SV + NV (see 2.1)

#### P10-T13

**Shared Cairo primitives; graph canvas with Pango text; graph layout as a service**

- **Why:** `architecture-global-19` (low), `ui-pages-22` (low), `architecture-global-09` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P10-T12 | **Parallelizable with:** P10-T14
- **Files:**
  - `src/ui/widgets/cairo/{mod, palette, draw}.rs (new)`
  - `src/ui/pages/graph.rs 483-492, 548-787`
  - `src/ui/widgets/chart.rs 30-42, 88-120, 315-344`
  - `src/services/graph_layout.rs (new)`
- **Steps:**
  1. Diff the two Palette/draw_grid bodies first (signatures differ: graph draw_grid(ctx, w, h, color) vs chart multi-arg) and design one reconciling API.
  2. graph.rs canvas -> graph/canvas.rs; pangocairo layouts with EllipsizeMode::End instead of toy text and char-pop fitted_text (O(n^2) per draw); cache labels in GraphNode. pangocairo is not a direct dependency: add it and pass cargo deny.
  3. scene_order (483) and the node-ordering part of compute_layers (492) -> services/graph_layout.rs with tests; GraphModel::build (pixel layout) stays in canvas.
- **Acceptance criteria:**
  - [ ] One Color/Palette type in ui; graph text renders with Pango.
- **Verification:** SV + NV + `cargo deny check` (see 2.1)

#### P10-T14

**Settings page split; generic enum combo row**

- **Why:** `ui-pages-13` (medium), `ui-pages-18` (medium), `ui-pages-24` (low)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P9-T11, P2-T08 | **Parallelizable with:** P10-T15
- **Files:**
  - `src/ui/pages/settings.rs (25, 48-56, 102-183, 386-497, 572-593, 731-761) -> ui/pages/settings/{mod, appearance, language, outputs, hotkeys, rows, status}.rs`
  - `src/ui/pages/mixer.rs 560-599, 650-683`
  - `src/ui/widgets/enum_combo.rs (new)`
  - `src/ui/theme.rs (apply_color_scheme)`
  - `src/domain/appearance (ThemeMode::ALL)`
- **Steps:**
  1. enum_combo_row<T>(title, subtitle, all: &'static [T], label, selected, on_change) and enum_at; ThemeMode::ALL replaces the 0/1/2 index tables (109-113, 126-130). Migrate the 7 enum-backed rows (mixer mode/grouping; settings scheme, motion, language, hotkey style, leader).
  2. PasswordWrites -> settings/connections/password_writes.rs; delete build_obs_group (448-450, unused _cfg); move apply_color_scheme from window.rs:1574 to ui/theme.rs (removes the upward import at settings.rs:25).
  3. Startup notice: keep until dismissed (AppState::dismiss_startup_notice) or a window-level adw::Banner instead of take() during build; obs_status_text takes &AppState inside one nav.read.
  4. Do NOT add a debounce for the leader-timeout SpinRow (ui-pages-17 refuted: run_config_snapshot already coalesces).
- **Acceptance criteria:**
  - [ ] No settings file > 400 production lines.
- **Verification:** SV + NV + TC (see 2.1)

#### P10-T15

**Connections editor split**

- **Why:** `ui-pages-19` (medium)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** P4-T22, P1-T14, P0-T02 | **Parallelizable with:** P10-T14
- **Files:**
  - `src/ui/pages/settings/connections.rs (642) -> settings/connections/{mod, editor, secrets?, password_writes}.rs`
- **Steps:**
  1. Converting to connections/mod.rs keeps `super::` = settings (report_save_failure, obs_status_text, with_entry_icon).
  2. Endpoint parsing from P4-T22; storage converts ObsEndpoint into ObsConfig (page no longer builds storage SavedConnection/ObsConfig directly at 212).
- **Acceptance criteria:**
  - [ ] connections page imports no storage config types.
- **Verification:** SV + NV (see 2.1)

#### P10-T16

**Help topics as data**

- **Why:** `ui-pages-23` (low)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** P10
- **Files:**
  - `src/ui/pages/help.rs 128-385, 399-466`
- **Steps:**
  1. TopicSpec { icon, title: fn() -> String, subtitle, body, links: &'static [TopicLinkSpec { label: fn() -> String, page }], expanded } const tables per group replacing 22 topic() calls with 7 arguments.
  2. Links wrap (FlowBox; adw::WrapBox needs v1_7).
- **Acceptance criteria:**
  - [ ] topic() takes one argument.
- **Verification:** SV + NV (see 2.1)

#### P10-T17

**Audio card: split, intent methods, change detection, no test hook in production**

- **Why:** `uiw-16` (medium), `performance-19` (low)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P9-T10, P4-T08, P2-T08 | **Parallelizable with:** P10-T18
- **Files:**
  - `src/ui/widgets/audio_card.rs (production 1-884) -> widgets/audio_card/{mod, scope, volume, parts}.rs`
  - `src/domain/audio/mixer.rs (set_locked)`
- **Steps:**
  1. MixerSelection::set_locked(input, locked) (ex direct write at 312-319); remove the #[cfg(test)] hook at 863-864 after an equivalent receiver-based assertion exists (test_support navigation receiver).
  2. One update_visible_value helper for set_available/update_volume (block/set/unblock/format_db repeated 113-159); handle construction dedupe (551-565, 599-613).
  3. observed_muted: Cell<Option<bool>> early return in update_mute (gtk set_active is already a no-op on equal values: marginal win, keep it cheap). OBS_FADER_MARKS_DB values are asserted by tests: keep.
- **Acceptance criteria:**
  - [ ] No direct AppState field write in audio_card.
- **Verification:** SV + NV (see 2.1)

#### P10-T18

**One meter tick per page (MeterHub) and idle meters stop ticking**

- **Why:** `performance-13` (medium)
- **Commit type:** `perf:` | **Size:** M | **Risk:** Low
- **Depends on:** P7-T15 | **Parallelizable with:** P10-T17
- **Files:**
  - `src/ui/widgets/volume_meter.rs 149-237`
  - `src/ui/widgets/meter_hub.rs (new)`
  - `src/services/meter_service.rs 180-182`
- **Steps:**
  1. MeterHub on an always-mapped page root advances all meters with one AppState borrow per frame (N borrows -> 1); simpler than re-arming per-meter ticks.
  2. InputMeterState::display_into(&mut Vec<..>) -> bool reuses the buffer; add is_decaying (is_idle exists only under cfg(test)); hub skips frames when nothing decays and no new sequence.
  3. Keep meter zone colors fixed (documented OBS semantics, volume_meter.rs:60-64).
  4. Measure: sysprof samples in advance_frame with 24 meters at 144 Hz, silent (hypothesis ~0).
- **Acceptance criteria:**
  - [ ] Silent meters cause no per-frame work.
- **Verification:** SV + NV (see 2.1)

#### P10-T19

**Action shortcuts and audio presets editors split; rules into the domain**

- **Why:** `uiw-13` (medium), `uiw-12` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P4-T19, P9-T15, P0-T02 | **Parallelizable with:** P10-T17, P10-T18
- **Files:**
  - `src/ui/action_shortcuts.rs (20-561) -> ui/operator/shortcuts/{chord, manager, editor}.rs`
  - `src/domain/workspace/action_shortcut.rs`
  - `src/ui/audio_presets.rs (44-497, 699) -> ui/audio/presets/{mod, editor, view}.rs`
  - `src/domain/audio/preset/`
- **Steps:**
  1. matching_binding(&SceneHotkeyConfig, &[ActionBinding], &ActionShortcut) and binding_conflict(..) in domain next to ActionShortcut::conflict (55); conflict_for_enable (379, takes nav) splits into pure core + ui adapter.
  2. PresetLibrary (BTreeMap<String, AudioPreset>) with save(name, inputs, replacing) -> Result<(), PresetError>, rename, delete, plan(..); PresetPlan::still_matches. Editor (20 fields) splits; render (284-465) shrinks.
  3. Using AudioService/format_db for preset values changes displayed text (decimals, -inf): separate commit with updated tests.
- **Acceptance criteria:**
  - [ ] No file > 400 production lines.
- **Verification:** SV + NV (see 2.1)

#### P10-T20

**Theme module split; built-in themes as one table**

- **Why:** `uiw-09` (medium), `uiw-10` (medium)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P2-T07 | **Parallelizable with:** P10-T21
- **Files:**
  - `src/ui/theme.rs (15-614) -> ui/theme/{mod, catalog, custom_css, providers, desktop, motion}.rs`
- **Steps:**
  1. custom_css::validate is the trust-boundary check for user CSS: keep it pure and fully unit tested.
  2. BuiltInTheme { id, name: fn() -> String, description: fn() -> String, light_css, dark_css } with non-capturing closures (`|| fl!(..)`) so key checks stay compile-time; delete localized_name/localized_description (60-95) and the dead English fields.
  3. load_custom_css is an associated fn of ThemeManager: child impl block or call-site update. 8 thread_locals (14-24) stay per-window state unless P9-T16's pattern applies.
- **Acceptance criteria:**
  - [ ] Adding a built-in theme is one table entry plus CSS files.
- **Verification:** SV + NV + TC (see 2.1)

#### P10-T21

**Chart colors from theme tokens; mode-neutral theme family names**

- **Why:** `uiw-11` (medium), `uiw-21` (low)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P10-T20, P10-T13 | **Parallelizable with:** P10-T22
- **Files:**
  - `src/ui/theme/tokens.rs (new)`
  - `src/ui/widgets/chart.rs 76-119`
  - `src/ui/theme/catalog.rs 537-585`
  - `resources/themes/*.css`
  - `src/domain/appearance (ThemeId aliases)`
  - `i18n/*/scenedeck.ftl`
- **Steps:**
  1. ThemeColors parsed once from the active theme's @define-color on apply (promote parse_theme_colors from tests at theme.rs:1487 with error handling); chart Severity/Palette use it (High Contrast reaches charts). Volume meter colors stay fixed (deliberate).
  2. Rename families to mode-neutral display names; config persists ids, so keep old ids as aliases in ThemeId parsing with a migration test; display names change in every locale.
- **Acceptance criteria:**
  - [ ] Unit test: `ThemeColors` parsed from the High Contrast theme CSS differs from the default theme's, and the chart palette built from it uses those values (no hard-coded chart RGB left in `ui/widgets/chart.rs`).
  - [ ] Migration test: every old theme id in config still parses to the same `ThemeId`.
- **Verification:** SV + NV + IV (see 2.1)

#### P10-T22

**UI kit helpers and test finders; remove production unwraps in factories**

- **Why:** `uiw-19` (low), `recent-change-review-22` (low), `uiw-22` (low)
- **Commit type:** `refactor:` | **Size:** M | **Risk:** Low
- **Depends on:** P0-T14 | **Parallelizable with:** P10
- **Files:**
  - `src/ui/kit/{mod.rs, labels.rs, buttons.rs} (new)`
  - `7 `fn text` copies (bookmarks 81, quick_actions 18, health_alerts 18, preflight 48, report_history 310, audio_presets 27, operating_panel 1591)`
  - `src/ui/mod.rs 40, 114, 182, 197`
  - `src/ui/operator_picker.rs 43-56`
  - `src/ui/widgets/deck_controls.rs 308-312`
  - `src/ui/widgets/feature_window.rs (new)`
- **Steps:**
  1. wrapped_label/caption/heading and wrapped_button (sets accessible label); string_list, clear_children, rebuild_page, insert_compact_flow_child, one visit_widgets (operating_panel 661, mixer 230).
  2. FeatureWindow::new(parent, title, widget_name) -> (adw::Window, content, alive) for the feature dialogs; replace the 11 `fn find(` test helpers with test_support finders.
  3. operator_picker factories: let-else instead of unwrap (45, 49); deck_controls sync: warn and return instead of expect("deck controls sync action") (311), or a typed handle instead of a stringly-named GAction.
- **Acceptance criteria:**
  - [ ] One copy of each helper; no unwrap in list factories.
- **Verification:** SV + NV (see 2.1)

#### P10-T23

**Accessible names for icon-only controls and the graph canvas**

- **Why:** `ui-pages-20` (medium)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** P10-T22 | **Parallelizable with:** P10
- **Files:**
  - `src/ui/widgets/a11y.rs (new)`
  - `src/ui/pages/{live.rs 219-266, graph.rs 129-133, 186-193, doctor.rs 92-97, inventory 738-742, 909-920, 1071-1075, stats.rs 177-182}`
  - `i18n/*/scenedeck.ftl`
- **Steps:**
  1. icon_button(icon, label) sets tooltip and Property::Label; contextual keys inventory-remove-stale-named and inventory-role-for-scene in every locale (replaces format! concatenation at 738-742).
  2. Graph canvas: set_focusable(false), or give it a label and key navigation (today focusable with no key controller).
  3. ToggleButton already exposes pressed state; only add Expanded where it applies.
- **Acceptance criteria:**
  - [ ] Native test: every icon-only button in the listed pages has an accessible label.
- **Verification:** SV + NV + IV (see 2.1)

#### P10-T24

**Decode preview images off the GTK thread**

- **Why:** `performance-18` (low)
- **Commit type:** `perf:` | **Size:** S | **Risk:** Low
- **Depends on:** P7-T11 | **Parallelizable with:** P10
- **Files:**
  - `src/ui/previews.rs 170-213, 275-285`
  - `src/controller/thumbnails.rs`
  - `src/controller/event.rs (ThumbnailLoaded payload)`
- **Steps:**
  1. Decode base64 and validate the PNG header in the Tokio thumbnail worker; send Vec<u8> or glib-free bytes in AppEvent::ThumbnailLoaded; keep Texture::from_bytes on GTK.
  2. Do not put bounded_png in services with glib::base64_decode (services_hold_no_gtk): use a direct base64 dependency (only transitive today; add and pass cargo deny) inside controller or obs.
- **Acceptance criteria:**
  - [ ] No base64 decode on the GTK thread.
- **Verification:** SV + NV + `cargo deny check` (see 2.1)

### Phase 11 - Documentation sync and deferred items

Close the loop: docs, ADRs, and items deliberately left to the end.

| ID | Title | Size | Depends on |
|---|---|---|---|
| [P11-T01](#p11-t01) | Documentation sync: architecture, codebase overview, developer guide, CLAUDE.md | S | P8-T16, P9-T20 |
| [P11-T02](#p11-t02) | Delete the dead OBS screenshot path | S | - |
| [P11-T03](#p11-t03) | Optional TLS transport (wss) behind a setting | M | P1-T16, P6-T08 |

#### P11-T01

**Documentation sync: architecture, codebase overview, developer guide, CLAUDE.md**

- **Why:** `architecture-global-18` (medium), `recent-change-review-28` (low)
- **Commit type:** `docs:` | **Size:** S | **Risk:** Low
- **Depends on:** P8-T16, P9-T20 | **Parallelizable with:** -
- **Files:**
  - `docs/architecture.md`
  - `docs/codebase-overview.md (zero mentions of requests.rs, scene_activation.rs, show_workspaces, atomic.rs, lifecycle.rs, workspace.rs, thumbnails.rs, event_pump, persist.rs, action_catalog, workflow.rs, i18n; line 106 calls registry.rs a 'local scene role registry')`
  - `docs/developer-guide.md`
  - `CLAUDE.md`
  - `site/index.html (check product claims)`
- **Steps:**
  1. Each structural task updates the module tables it touches; this task closes remaining drift and checks site claims.
  2. Record the process rule from RC28: stacked commits per context, `refactor:` separate from `fix:`.
- **Acceptance criteria:**
  - [ ] Every module in src/ is listed in codebase-overview.md with its responsibility.
- **Verification:** SV (see 2.1)

#### P11-T02

**Delete the dead OBS screenshot path**

- **Why:** `security-07` (low)
- **Commit type:** `refactor:` | **Size:** S | **Risk:** Low
- **Depends on:** none | **Parallelizable with:** any
- **Files:**
  - `src/controller/workspace.rs 32, 50, 510-525`
  - `src/obs/client.rs 605-630 (thumbnails.rs after P6-T02)`
  - `src/controller/state.rs 222-228, 379-381 (state/workspace after P8)`
  - `src/ui/event_reconciliation.rs 95-97, 149-150`
  - `src/ui/workspace.rs 79-138, 179, 201, 291`
- **Steps:**
  1. Every production path passes screenshot=false. Delete WorkspaceRequest.screenshot, ObservedWorkspace.screenshot, ObsClient::get_source_screenshot (keep source_screenshot only if get_scene_thumbnail uses it - check 577-597), WorkspaceState screenshot fields, reconciliation branches and tests.
  2. Can run any time; doing it before P8-T10 removes screenshot.rs from that split.
- **Acceptance criteria:**
  - [ ] grep `screenshot` finds only the thumbnail path.
- **Verification:** SV + NV (see 2.1)

#### P11-T03

**Optional TLS transport (wss) behind a setting**

- **Why:** `security-02` (medium)
- **Commit type:** `feat:` | **Size:** M | **Risk:** Low
- **Depends on:** P1-T16, P6-T08 | **Parallelizable with:** any
- **Files:**
  - `src/obs/client.rs connect`
  - `Cargo.toml (obws tls feature / rustls)`
  - `deny.toml`
  - `docs/obs-websocket.md`
- **Steps:**
  1. Deferred: OBS has no native wss, so this only helps behind a TLS proxy. Requires a rustls dependency tree that passes deny.toml. Decide with the maintainer (open question).
- **Acceptance criteria:**
  - [ ] MAINTAINER DECISION recorded in the PR before starting (8.2).
  - [ ] Manual check: connect through a local TLS terminator (for example stunnel in front of OBS) with the setting on; plain `ws://` behavior unchanged with it off.
  - [ ] `cargo deny check advisories bans licenses sources` and `cargo audit` pass with the new dependency tree.
- **Verification:** SV + `cargo deny check` (see 2.1)

---

## 6. Cross-cutting fixes not tied to splits

These are already tasks in section 5. This section groups them by concern, so a reviewer can check coverage by theme.

### 6.1 Security

| Concern | Severity | Task(s) |
|---|---|---|
| Advisory exception expires 2026-11-20 | medium (time-critical) | P0-T01 |
| OBS-controlled text rendered as Pango markup (StatusPage, Toast, help rows) | high | P1-T01 |
| Plaintext password in Debug+Clone AppState for the whole process lifetime | medium | P1-T13 |
| Two keyring ports; Connect can read a stale password while a Secret-lane write is queued | medium (plausible, not reproduced) | P1-T14 |
| Startup blocks on Secret Service; keyring touched every launch | medium | P1-T15 |
| ws:// only, no warning for remote hosts; TLS | medium | P1-T16, P11-T03 |
| Endpoint as raw host string formatted in 5 places | medium (ddd) | P4-T22 |
| Show-workspace store unbounded, keyed by OBS collection names | low | P1-T20 |
| Newer config silently down-converted / moved aside | low/high | P1-T08 |
| Config read with write side effect and swallowed error | low | P1-T09 |
| YAML import default budget only | low | P1-T19 |
| Relative custom-CSS paths resolved against CWD | low | P1-T18 |
| XDG fallback to CWD and relative XDG_* values | low | P1-T17 |
| Stringly AppError hides credential/auth failure kinds | low | P5-T01 |
| Report exports always include endpoint and host paths | low (privacy) | P1-T25 |
| background_io spawns a thread per call and polls | low | P9-T17 |
| Dead screenshot request path | low | P11-T02 |

### 6.2 Performance: what to change and how to measure

Measure before and after each change. P0-T15 adds the `perf-trace` feature and counters. Every number in the hypotheses below is a reviewer estimate, not a measurement.

| Hot spot | Task | How to measure | Hypothesis |
|---|---|---|---|
| Three perpetual frame-tick callbacks keep the frame clock running | P9-T19 | GTK inspector FPS overlay; `pidstat -u 1` idle, OBS disconnected | idle frames go from the refresh rate to ~0; idle CPU at least -50% |
| collect_report ~60 calls/s (pre-event, post-event, tick) | P9-T01, P8-T12 | perf-trace counter: calls/s and time per call; heaptrack allocations/s with meters active | <= 20 calls/s, most of them early-return no-ops |
| About 35 polling timers (shell, widgets, dialogs) | P8-T15, P9-T14, P9-T15 | `strace -f -c -e trace=ppoll -p PID` for 30 s idle; `pidstat -w` cswch/s | idle wakeups ~70/s to < 5/s |
| 50 ms event pump plus a 2 ms producer busy-retry | P9-T13 | observed_at to applied_at latency histogram for InputVolumeChanged/CurrentSceneChanged | p50 ~25 ms to < 2 ms; 20 fewer wakeups/s |
| Meter frames occupy GTK queue slots that lifecycle events need | P7-T15 | injected 3 s GTK stall with meters active; count backlog retirements | 0 retirements |
| Scene cut rebuilds Live cards 3+ times, each cloning the registry | P10-T10 | span count and time per scripted scene switch | 1 rebuild; no clone on the fast path |
| Mixer refresh: Debug-string keys, ScopeBinding allocated per input | P10-T08, P8-T08 | span on MixerPageHandle::refresh with 40 inputs; heaptrack | fast path >= 5x cheaper |
| workflow_scope allocates on every call (30 sites, fader path) | P8-T08 | heaptrack during a 10 s fader drag | ~0 allocations from scope |
| Control snapshot: 7 sequential round trips | P6-T09 | ScriptedObs with 5 ms latency per request; wall time | 35 ms to ~10 ms |
| Stats poll: 3 sequential requests/s, GetVideoSettings per thumbnail | P6-T09 | ScriptedObs request counts per minute | ~300/min to ~120/min with previews on |
| Scene-audio membership N+1 GetSceneItemEnabled | P6-T10 | request count per visibility toggle in a 3-level nested scene | items+2 to ~1 |
| Per-meter tick every frame, even when silent | P10-T18 | sysprof samples in advance_frame, 24 meters at 144 Hz, silent | ~0 samples while silent |
| Audio health observe at 20 Hz with O(R^2) uniqueness | P4-T17 | perf-trace span on the health tick | linear, mostly skipped |
| Report history deep clone and compare every 10 s | P5-T11 | span on save() with 50 archived reports | Arc comparisons only |
| Base64/PNG decode on the GTK thread | P10-T24 | span in previews apply | no decode on GTK |
| Scene search rewrites show-workspaces.json per keystroke | P1-T24 | file writes while typing (mtime or counting store) | 0 writes |
| background_io thread per job plus 20 ms polls | P9-T17 | thread count and wakeups during a settings burst | no poll timers |

Benches: criterion cannot reach `pub(crate)` items, because `lib.rs` exposes only `run()`. Use `#[ignore]`d timing tests, or ScriptedObs request-count assertions, as regression gates.

### 6.3 Error handling

- **One mutex poison policy.** `lock_or_recover` is used where values are replaced whole (P0-T09). Lifecycle mutexes are decided per mutex and documented (P0-T10). `RequestGate` drops its Mutex (P6-T06).
- **No fake success:**
  - `load_config_blocking` stops returning Default on JoinError (P1-T06);
  - manual refresh publishes readback failures (P1-T02);
  - refused spawns publish a failure (P1-T05);
  - a full journal publishes OutputCommandFailed (P1-T12);
  - `NavigationContext::update` never drops a write silently (P0-T08).
- **Typed errors at every boundary:**
  - `StorageError` and structured `AppError` (P5-T01);
  - `ObsError` with RetirementReason and Admission (P6-T05);
  - `CommandError` and `SendRejected` (P7-T05);
  - `PortalError` (P9-T18);
  - `CredentialError` (P5-T01).
- **Production unwrap/expect.** There are 63 today. They are removed by:
  - P0-T10: lifecycle, 21 sites (decided per mutex);
  - P10-T05: inventory, 15;
  - P8-T12: session_reports, 6;
  - P10-T22: operator_picker and deck_controls;
  - P9-T18: global_shortcuts, 3;
  - P5-T12: persist.rs, 1;
  - P4-T24: action_catalog `unreachable!()`;
  - P4-T22: connections `port.unwrap()`;
  - P6-T04: operator_inputs `expect`.

  After Phase 10, add a ratchet on `.unwrap()`/`.expect(` in production code with ceilings at the remaining count.
- **Localization of errors.** Errors are localized only at the UI boundary. Remove English strings from the controller (P7-T05) and obs (P6-T05), and the `fl!` from storage (P5-T10).

### 6.4 UI, accessibility and design

- **Markup safety:** P1-T01.
- **Accessible names:**
  - icon-only header buttons (P9-T21);
  - page icon buttons and the focusable graph canvas (P10-T23);
  - kit buttons that set the accessible label (P10-T22).
- **Theme tokens:**
  - charts follow theme colors and High Contrast (P10-T21);
  - volume meter colors stay fixed on purpose (OBS meter semantics, `volume_meter.rs:60-64`).
- **Mode-neutral theme family names:** P10-T21.
- **Deprecated dialogs:** wrap the 37 `adw::MessageDialog` uses in one helper now (P9-T21); migrate to `AlertDialog` when the feature level is raised to v1_5.
- **Responsive layout:** breakpoints instead of tick callbacks (P9-T19); help links wrap (P10-T16).
- **Consistency:**
  - one status-bar formatter (P1-T07);
  - dedicated close-dialog i18n keys (P9-T09);
  - enum combo rows from one helper (P10-T14);
  - one feature-window scaffold (P10-T22).

### 6.5 Tests

- **Nets first:**
  - drift-proof native tests (P0-T02);
  - golden formats (P0-T05);
  - ScriptedObs (P0-T06);
  - characterization tests (P0-T11, P0-T12, P0-T13);
  - shared test support (P0-T14).
- **Structure:**
  - tests out of god files (P2-T01..P2-T08);
  - final_regression split (P2-T09, P2-T10);
  - table-driven twins and domain test_support (P2-T11);
  - one `<module>/tests.rs` convention (P2-T07).
- **Gaps to close while restructuring:**
  - OBS requests that were never scripted (P0-T06 and section 4.11);
  - pure ExitPolicy table (P9-T09);
  - pure RegistryJournal (P9-T16);
  - ObsEvent translation table (P7-T09);
  - SingleFlight (P1-T05);
  - workspace_refresh_due (P9-T08);
  - poison tests (P0-T09, P0-T10).
- **Fitness functions:** section 3.4. Every phase lowers at least one ceiling.

---

## 7. Full findings register

299 findings from 13 reviewers: 121 confirmed, 177 adjusted, 1 refuted. Severity of the 298 live findings (after verifier adjustment): critical 1, high 66, medium 150, low 81.

Every confirmed or adjusted finding maps to at least one task. Full evidence, the recommendation and the verifier note are in `findings/<reviewer>.json`. The `file:lines` column shows the first location cited, cut to fit the table.

Reviewer files: `architecture-global`, `controller-orchestration` (ids `ctrl-NN`), `controller-state`, `domain-services-ddd`, `obs-acl`, `performance`, `recent-change-review`, `security`, `storage-infra`, `tests-structure`, `ui-pages`, `ui-shell`, `ui-widgets-theme` (ids `uiw-NN`).

| ID | Sev | Verdict | Category | File:lines | Title | Task(s) |
|---|---|---|---|---|---|---|
| architecture-global-01 | high | adjusted | god-struct | src/controller/state.rs 193-229, 438-499, 501-1246 | AppState is a cross-context god aggregate (47 fields, ~745-line impl) living in the controller layer | [P8-T01](#p8-t01), [P8-T16](#p8-t16) |
| architecture-global-02 | high | adjusted | boundary | src/ui/navigation.rs 15-32, 59-115 | NavigationContext exposes raw Rc<RefCell<AppState>>, so the documented narrow page boundary is false | [P0-T08](#p0-t08), [P9-T11](#p9-t11), [P9-T12](#p9-t12) |
| architecture-global-03 | high | adjusted | ddd | src/storage/registry.rs 22-150, 167-537, 586-690 | No anti-corruption layer at storage: serde DTO SceneRegistry is the show-curation aggregate, used by UI and state dir... | [P5-T05](#p5-t05), [P5-T06](#p5-t06), [P5-T07](#p5-t07) |
| architecture-global-04 | high | confirmed | god-file | src/ui/window.rs 63-280, 281-411, 643-1253, 1254-1573 | window.rs: ~590-line apply_event switch over 45 AppEvent variants plus shell building, polling, clocks and page refre... | [P9-T02](#p9-t02), [P9-T03](#p9-t03) |
| architecture-global-05 | medium | confirmed | boundary | src/obs/client.rs 14; src/obs/mapper.rs:12,21-22; src/controller/ev... | The OBS ACL depends upward on controller::event::ConnectionInfo | [P3-T01](#p3-t01) |
| architecture-global-06 | medium | adjusted | boundary | src/controller/workspace.rs 569-575; src/storage/bookmarks.rs:81-89 | Clock utility unix_ms lives in controller::workspace and is used across layers | [P3-T02](#p3-t02) |
| architecture-global-07 | medium | confirmed | ddd | src/controller/state.rs 193-229; src/controller/workspace.rs:22-110... | Ubiquitous language collision: 'workspace' has four meanings, 'registry' three | [P4-T01](#p4-t01) |
| architecture-global-08 | medium | adjusted | boundary | src/controller/command.rs 6-51; src/controller/event.rs:273,327,331... | AppCommand and AppEvent carry state-module and controller-internal types | [P3-T03](#p3-t03), [P7-T14](#p7-t14) |
| architecture-global-09 | medium | adjusted | ddd | src/ui/pages/live.rs 358-392, 524-540; src/ui/pages/stats.rs:34-54,... | Domain policies live in UI pages (output confirmation, disconnect rules, stats severity, action resolution) | [P4-T20](#p4-t20), [P4-T25](#p4-t25), [P10-T13](#p10-t13) |
| architecture-global-10 | medium | adjusted | boundary | src/ui/persist.rs 25-122, 160, 207-270, 463-590; src/ui/background_... | Persistence application service lives in ui/ with hidden global state in thread_locals | [P9-T16](#p9-t16) |
| architecture-global-11 | medium | adjusted | test | src/architecture_tests.rs 351-365, 548-681 | Architecture tests miss most dependency edges and have no ratchets | [P0-T07](#p0-t07), [P8-T16](#p8-t16), [P3-T08](#p3-t08) |
| architecture-global-12 | medium | adjusted | ddd | src/domain/mod.rs whole module (26 flat files + health/, audio_pres... | Flat domain/ with no bounded-context grouping; presentation strings and serde schema inside the domain | [P0-T07](#p0-t07), [P4-T12](#p4-t12), [P4-T13](#p4-t13) |
| architecture-global-13 | medium | adjusted | god-struct | src/controller/lifecycle.rs 13-28, 31-128, 129-433 | SessionScope bundles 12 Arc fields plus generation (identity, task registry, client slot, journal, terminal queue) | [P0-T10](#p0-t10), [P7-T02](#p7-t02) |
| architecture-global-14 | medium | confirmed | test | src/controller/thumbnails.rs 136-144; src/controller/workspace.rs:5... | Loopback OBS mock duplicated; protocol tests scattered across layers | [P0-T06](#p0-t06) |
| architecture-global-15 | medium | adjusted | god-file | src/ui/widgets/operating_panel.rs 17-93, 94-270, 678-1097, 1098-1493 | operating_panel 'widget' is the full Sources page with two ~400-line functions | [P10-T01](#p10-t01) |
| architecture-global-16 | medium | confirmed | god-file | src/ui/pages/inventory.rs 51-130, 330-461, 598-1018, 1214-1505 | inventory.rs mixes presentation model, rows, drag reordering, accent editing, YAML dialogs and curation mutation | [P10-T05](#p10-t05) |
| architecture-global-17 | medium | adjusted | god-file | src/ui/pages/mixer.rs 38-314, 941-1115, 1116-1223; src/controller/s... | Mixer page and mixer state both mix rendering/fencing with an inspection subsystem | [P8-T14](#p8-t14), [P10-T07](#p10-t07) |
| architecture-global-18 | medium | confirmed | docs | docs/codebase-overview.md 16-169 | codebase-overview.md and CLAUDE.md are out of date | [P4-T01](#p4-t01), [P11-T01](#p11-t01) |
| architecture-global-19 | low | confirmed | code-smell | src/ui/pages/graph.rs 573, 699-787; src/ui/widgets/chart.rs:30-42, ... | Duplicated Cairo drawing primitives (Color, Palette, draw_grid) in graph.rs and chart.rs | [P10-T13](#p10-t13) |
| architecture-global-20 | low | confirmed | god-file | src/app.rs 23-161, 162-430, 431-456 | app.rs composition root mixes runtime setup, startup I/O loading and close-guard UI | [P9-T09](#p9-t09) |
| architecture-global-21 | low | adjusted | god-file | src/obs/client.rs 1-316 prod, 317-1392 tests | Many 'god files' are inflated by inline test modules | [P2-T03](#p2-t03) |
| architecture-global-22 | low | confirmed | code-smell | src/controller/refresh_controller.rs 409-587 | run_event_loop is a ~180-line async function combining worker, poll loop and transport publication | [P7-T09](#p7-t09) |
| ctrl-01 | medium | adjusted | error-handling | src/controller/lifecycle.rs 96-107,150-152,214,226-230,239-245,249-... | Mutex poison cascade: 19 lock().expect() plus 2 overflow expect() in SessionScope reach the GTK thread | [P0-T10](#p0-t10) |
| ctrl-02 | low | adjusted | error-handling | src/controller/output_controller.rs 173-196,418-439; session_contro... | Poisoned locks silently become 'command ignored' or 'disconnected' (opposite policy to ctrl-01) | [P0-T09](#p0-t09) |
| ctrl-03 | high | confirmed | correctness | src/controller/app_controller.rs 256-262 | Manual RefreshData drops output readback failures that the session scan publishes | [P1-T02](#p1-t02), [P7-T07](#p7-t07) |
| ctrl-04 | medium | adjusted | correctness | src/controller/app_controller.rs 293-358 (mixer); 219-284 (refresh,... | Hand-rolled single-flight queues: a refused spawn_reserved leaves running=true and strands the request (PLAUSIBLE) | [P1-T05](#p1-t05), [P7-T16](#p7-t16) |
| ctrl-05 | high | adjusted | god-struct | src/controller/lifecycle.rs 13-28,129-432 | SessionScope is a god struct: 13 fields (12 Arc), 6 responsibilities | [P7-T02](#p7-t02), [P7-T03](#p7-t03) |
| ctrl-06 | high | adjusted | god-struct | src/controller/event.rs 229-388; event_sender.rs:9-23,85-98; observ... | AppEvent god enum (about 50 variants) with recursive envelope variants; the variant policy table lives elsewhere | [P7-T12](#p7-t12), [P7-T13](#p7-t13) |
| ctrl-07 | high | confirmed | code-smell | src/controller/workspace.rs 226-408 | WorkspaceController::request: 183-line Long Method with the fence predicate copied 8 times | [P7-T10](#p7-t10) |
| ctrl-08 | medium | adjusted | code-smell | src/controller/refresh_controller.rs 409-586 | run_event_loop: 178-line method mixing ingress, translation, side effects and resync | [P0-T11](#p0-t11), [P7-T09](#p7-t09) |
| ctrl-09 | medium | confirmed | boundary | src/controller/output_controller.rs 31-39,140-146,173-203,219-387; ... | A test seam leaks into production: OutputCommandClient with a #[cfg(test)] Fake variant is matched in 9 methods plus ... | [P7-T06](#p7-t06) |
| ctrl-10 | medium | adjusted | ddd | src/controller/command.rs 5-51; app_controller.rs:118-215 | Flat AppCommand spans 6 contexts; handle() carries inline dB math and repeated cancellation sequences | [P7-T14](#p7-t14) |
| ctrl-11 | medium | confirmed | code-smell | src/controller/app_controller.rs 57,78-101,395-397; output_controll... | Primitive Obsession: raw Arc<Mutex<Option<ObsClient>>> held by 5 owners, with the read code copied 7 times | [P7-T01](#p7-t01) |
| ctrl-12 | medium | confirmed | correctness | src/controller/output_controller.rs 42-61; workspace.rs:87-117; app... | Sub-controllers build a private SessionScope::initial and depend on a later set_scope (temporal coupling) | [P7-T01](#p7-t01) |
| ctrl-13 | medium | adjusted | error-handling | src/controller/workspace.rs 152-176,205-212,291-375,411-442,444-563... | Stringly-typed errors (Result<_, String>, Result<(), ()>) erase the AppError kind and force probe-based classification | [P7-T05](#p7-t05) |
| ctrl-14 | medium | adjusted | i18n | src/controller/lifecycle.rs 310-316,322; event.rs:41; output_contro... | User-visible English strings bypass Fluent (CLAUDE.md i18n rule) | [P7-T05](#p7-t05) |
| ctrl-15 | medium | adjusted | ddd | src/controller/lifecycle.rs 49-60,95-127,297-319; event.rs:267-275 | Boolean flag soup allows illegal states in OperationEvidence and WorkspaceLoaded | [P7-T04](#p7-t04) |
| ctrl-16 | medium | adjusted | boundary | src/controller/thumbnails.rs 21-114 | ThumbnailController bypasses SessionScope and EventSender (raw try_send, runtime.spawn, Relaxed AtomicBool) | [P7-T11](#p7-t11) |
| ctrl-17 | medium | adjusted | performance | src/controller/lifecycle.rs 383-393; event_sender.rs:107-138 | quiesce() spins on yield_now, and send_if polls every 2 ms | [P7-T03](#p7-t03), [P9-T13](#p9-t13) |
| ctrl-18 | medium | adjusted | code-smell | src/controller/session_controller.rs 138-224 | Feature Envy: SessionController::disconnect runs the output stop protocol inline | [P7-T07](#p7-t07) |
| ctrl-19 | medium | adjusted | error-handling | src/controller/app_controller.rs 449-458; dependencies.rs:56-62; st... | load_config_blocking returns Default config on JoinError, and FileAppConfigProvider discards the startup_notice diagn... | [P1-T06](#p1-t06), [P7-T16](#p7-t16) |
| ctrl-20 | medium | confirmed | code-smell | src/controller/refresh_controller.rs 195-221,232-258,270-290,492-53... | Observation-envelope boilerplate repeated 14 times; invalidate_graph copied inline; refresh_output_statuses duplicate... | [P7-T07](#p7-t07), [P7-T08](#p7-t08) |
| ctrl-21 | low | confirmed | code-smell | src/controller/workspace/audio_presets.rs 13-77,95-245 | Preset runner: unwrap after a compound is_none guard, a 7-parameter apply_input with an output argument, a magic 30 s... | [P1-T05](#p1-t05) |
| ctrl-22 | medium | adjusted | boundary | src/controller/workspace.rs 569-575 | Clock utility unix_ms lives in controller/workspace.rs and is called from about 30 sites, including storage/ | [P3-T02](#p3-t02) |
| ctrl-23 | low | adjusted | code-smell | src/controller/lifecycle.rs 271-272,332,402; workspace.rs:138; thum... | Magic capacities and budgets, one duplicated in state.rs; operation() conflates 'journal full' with 'inactive' | [P1-T12](#p1-t12), [P3-T04](#p3-t04) |
| ctrl-24 | low | adjusted | code-smell | src/controller/event.rs 20-67,69-88,90-216 | event.rs mixes output-recovery domain logic with event definitions; a test module sits mid-file | [P4-T20](#p4-t20) |
| ctrl-25 | low | adjusted | god-file | src/controller/refresh_controller.rs 26-99 + 187-291 (snapshots); 1... | refresh_controller.rs holds 4 unrelated responsibilities | [P7-T08](#p7-t08) |
| controller-state-01 | critical | adjusted | god-struct | src/controller/state.rs 437-558 | AppState is a 41-field god struct spanning 9 bounded contexts | [P8-T01](#p8-t01), [P8-T07](#p8-t07), [P8-T08](#p8-t08) |
| controller-state-02 | high | adjusted | god-struct | src/controller/state.rs 192-435 | WorkspaceState is a second god struct (30 public fields, 6 concerns) | [P8-T10](#p8-t10) |
| controller-state-03 | high | adjusted | code-smell | src/controller/state.rs 465-474, 605-750, 1086-1141 | Stream/record data clump and duplicated set_stream_*/set_record_* method pairs | [P8-T03](#p8-t03) |
| controller-state-04 | medium | adjusted | code-smell | src/controller/state.rs 487-498, 1143-1188 | Telemetry fields are a data clump updated only together | [P8-T02](#p8-t02) |
| controller-state-05 | high | adjusted | boundary | src/ui/event_reconciliation.rs 20-31, 33-51, 55-160, 270-353 | The workspace reducer lives in ui/ and writes WorkspaceState fields directly | [P8-T11](#p8-t11) |
| controller-state-06 | high | adjusted | correctness | src/controller/state.rs 995-1025, 605-615 | Three independent fences for the same observation (revision, Instant, health) | [P3-T06](#p3-t06), [P8-T07](#p8-t07) |
| controller-state-07 | high | confirmed | ddd | src/controller/state.rs 1190-1244 | One OBS input fact is denormalized into 4 copies and patched by duplicated code | [P8-T05](#p8-t05) |
| controller-state-08 | medium | adjusted | boundary | src/controller/state.rs 32-123, 136-159, 1074-1084 | Presentation (GTK icon names, CSS classes, Fluent strings) in the controller state | [P4-T09](#p4-t09), [P4-T11](#p4-t11) |
| controller-state-09 | high | confirmed | code-smell | src/controller/state.rs 641-750 | apply_output_event is a 110-line long method that orchestrates 5 contexts | [P0-T12](#p0-t12), [P8-T04](#p8-t04) |
| controller-state-10 | high | adjusted | code-smell | src/controller/state/session_reports.rs 271-411, 136-210 | collect_report (140 lines) and begin_report reach into 7 slices; repeated unwrap on current() | [P8-T12](#p8-t12) |
| controller-state-11 | medium | confirmed | correctness | src/controller/state.rs 368-370, 1038-1041 | Four inconsistent counter-overflow policies across slices | [P3-T06](#p3-t06) |
| controller-state-12 | medium | adjusted | code-smell | src/controller/state.rs 247-266, 319-348, 382, 398-405, 755-758, 83... | Hidden clock dependencies inside reducers (Instant::elapsed, Instant::now, unix_ms) | [P3-T05](#p3-t05) |
| controller-state-13 | medium | confirmed | code-smell | src/controller/state.rs 256, 327, 331, 937 | Magic numbers for control freshness and retirement history | [P3-T05](#p3-t05) |
| controller-state-14 | medium | confirmed | ddd | src/controller/state.rs 170-188, 247-355 | Pure domain rules live as controller methods instead of in domain/ | [P8-T06](#p8-t06) |
| controller-state-15 | medium | confirmed | boundary | src/controller/state.rs 15-16, 453-456 | Storage serde types are held directly as runtime state (no anti-corruption layer) | [P5-T07](#p5-t07), [P8-T09](#p8-t09) |
| controller-state-16 | medium | adjusted | security | src/controller/state.rs 437-438, 456 | OBS password stored as plain String in a Debug + Clone god struct | [P1-T13](#p1-t13) |
| controller-state-17 | medium | adjusted | code-smell | src/controller/state/mixer.rs 34-103, 201-220, 273-313, 411-451 | Mixer view-model types and Middle Man forwarders in controller state | [P8-T14](#p8-t14) |
| controller-state-18 | medium | confirmed | code-smell | src/controller/state.rs 20-30 | Glob re-export `pub use mixer::*` and `use super::*` in submodules | [P3-T07](#p3-t07) |
| controller-state-19 | medium | confirmed | code-smell | src/controller/state/bookmarks.rs 14, 32-34, 82-87, 103-105 | Anonymous tuples as domain concepts (session, observations, health key) | [P8-T13](#p8-t13) |
| controller-state-20 | medium | confirmed | test | src/controller/state.rs 1247-3088 | 1840-line inline test module with shared fixtures leaked to sibling tests | [P2-T01](#p2-t01) |
| controller-state-21 | medium | adjusted | correctness | src/ui/navigation.rs 136-138, 180-211 | RefCell<AppState> borrow discipline is convention-only (559 borrow_mut, 0 try_borrow) | [P0-T08](#p0-t08), [P9-T11](#p9-t11), [P9-T12](#p9-t12) |
| controller-state-22 | low | confirmed | code-smell | src/controller/state.rs 896-940 | apply_terminal_evidence mixes presets, outputs, audit formatting and retention | [P8-T13](#p8-t13) |
| controller-state-23 | low | confirmed | code-smell | src/controller/state/audio_presets.rs 9, bookmarks.rs:81, bookmarks... | `matches!(obs_status, ObsStatus::Connected { .. })` duplicated in 5 places | [P3-T07](#p3-t07) |
| controller-state-24 | low | confirmed | ddd | src/controller/state/audio_presets.rs 4-144 | Preset run aggregate logic implemented as AppState methods over two loose fields | [P8-T13](#p8-t13) |
| controller-state-25 | low | adjusted | code-smell | src/controller/state/report_history.rs 7-26, 121-140 | ReportHistoryState exposes 15 public flags; owned/eligible retention duplicated | [P8-T13](#p8-t13) |
| domain-services-ddd-01 | high | adjusted | ddd | src/domain/role.rs 3-6,27-86 (also audio.rs:1-3,53-70; diagnostic.r... | Presentation (Fluent labels, CSS classes, GTK icon names) is baked into 12 domain modules | [P4-T09](#p4-t09), [P4-T10](#p4-t10), [P4-T11](#p4-t11) |
| domain-services-ddd-02 | high | confirmed | ddd | src/domain/scene.rs 3-14 (also audio.rs:39-42,73-102; operator_acti... | OBS identity is primitive and modelled four different ways | [P4-T02](#p4-t02), [P4-T04](#p4-t04), [P4-T05](#p4-t05) |
| domain-services-ddd-03 | high | adjusted | code-smell | src/domain/operator_input.rs 61-80 (true resolution dupes: health/a... | The "unambiguous identity resolution" business rule is re-implemented in several places | [P4-T02](#p4-t02), [P4-T03](#p4-t03) |
| domain-services-ddd-04 | high | confirmed | boundary | src/ui/action_catalog.rs 153-287 | Domain policy "resolve an OperatorAction against current observations" lives in the UI and uses OBS request names as ... | [P4-T06](#p4-t06), [P4-T25](#p4-t25) |
| domain-services-ddd-05 | high | adjusted | boundary | src/domain/audio_preset.rs 14-35,37-49,80-97,99-131 (also storage/r... | No anti-corruption layer at storage: domain structs are the on-disk serde schema | [P5-T08](#p5-t08) |
| domain-services-ddd-06 | high | adjusted | ddd | src/domain/audio_preset.rs 37-77 (also audio_preset/run.rs:10-38; b... | Invariants are checked by after-the-fact valid() predicates on public-field structs, and two name rules diverge | [P4-T19](#p4-t19) |
| domain-services-ddd-07 | medium | confirmed | boundary | src/domain/control.rs 198-210,287-297 (also meter.rs:89-97,104-110,... | OBS protocol facts leak into the domain (missing ACL at obs/) | [P4-T07](#p4-t07), [P4-T15](#p4-t15) |
| domain-services-ddd-08 | medium | confirmed | god-struct | src/domain/control.rs 7-22,24-116,221-285 | ControlAction is a 22-variant god enum spanning five bounded contexts; control.rs mixes them all | [P4-T14](#p4-t14) |
| domain-services-ddd-09 | medium | adjusted | ddd | src/domain/audio.rs 5-37,73-102 (also meter.rs:99-110; services/aud... | Gain/decibel knowledge is duplicated, ranges are implicit, and AudioService is a Middle Man | [P4-T08](#p4-t08) |
| domain-services-ddd-10 | medium | adjusted | code-smell | src/domain/preflight.rs 85-88,145-156,173-199 (also health.rs:143-1... | Freshness, meter-evidence and threshold rules are duplicated between Preflight and Health | [P4-T16](#p4-t16) |
| domain-services-ddd-11 | medium | confirmed | code-smell | src/domain/preflight.rs 57-65,106-214 | PreflightReview::evaluate is a ~108-line method with nesting depth 7 and positional result arrays | [P4-T16](#p4-t16) |
| domain-services-ddd-12 | medium | confirmed | code-smell | src/domain/health.rs 1-21,55-149,168-288 (also health/events.rs:6-4... | HealthMonitor: long methods, a long parameter list, and an impl split across files that reaches into private state | [P4-T17](#p4-t17) |
| domain-services-ddd-13 | low | adjusted | code-smell | src/domain/health.rs 30-41 (also health/audio.rs:149-154,172-190; c... | AlertKind::AudioSilence(usize)/AudioClipping(usize) use a rule-vector index as identity (smell; no demonstrated misla... | [P4-T17](#p4-t17) |
| domain-services-ddd-14 | medium | adjusted | ddd | src/domain/session_report.rs 39-59 | ReportEvent::Health is stringly typed, and the enum-to-string mapping lives in the controller | [P4-T21](#p4-t21) |
| domain-services-ddd-15 | medium | confirmed | ddd | src/services/graph_service.rs 11-53 (also services/doctor_service.r... | Doctor rules and edge classification are domain logic misplaced in services (Feature Envy on GraphDependencyPolicy) | [P4-T18](#p4-t18) |
| domain-services-ddd-16 | medium | confirmed | correctness | src/domain/registry.rs 112-117,146-149 (also graph.rs:164-172; serv... | Doctor finding order depends on HashMap iteration and can change between launches | [P1-T10](#p1-t10) |
| domain-services-ddd-17 | low | adjusted | code-smell | src/domain/stats.rs 162-179 | StatsHistory::push reads the wall clock inside the domain | [P3-T05](#p3-t05) |
| domain-services-ddd-18 | low | adjusted | ddd | src/domain/mod.rs 3-27 (also workflow.rs:1-57; controller/state.rs:... | Flat 24-module domain and inconsistent ubiquitous language (scope / workspace / registry / workflow) | [P4-T01](#p4-t01), [P4-T12](#p4-t12), [P4-T13](#p4-t13) |
| domain-services-ddd-19 | low | confirmed | ddd | src/domain/mixer.rs 104-134 | MixerSelection mixes persisted preference with session-only UI state | [P4-T19](#p4-t19) |
| domain-services-ddd-20 | low | confirmed | code-smell | src/domain/graph.rs 62-162,164-242 | graph.rs: tests placed mid-file before the type, and cycle_edges is a 60-line inline Kosaraju | [P4-T19](#p4-t19) |
| domain-services-ddd-21 | low | confirmed | code-smell | src/domain/hotkey.rs 48-88,143-154 (also action_shortcut.rs:54-81; ... | Boolean-flag constructor Modifiers::new(bool,bool,bool,bool), and app-reserved shortcuts duplicated between domain an... | [P4-T19](#p4-t19) |
| domain-services-ddd-22 | low | confirmed | test | src/domain/meter.rs 81-88,141-157,177-182,215-220 (also stats.rs:27... | Test-only API on production types, and cross-module test-helper coupling | [P2-T11](#p2-t11) |
| obs-acl-01 | high | confirmed | god-file | src/obs/client.rs 1-1395 (prod 6-315, 517-1097; tests 317-515, 1099... | client.rs is a god file: two interleaved impl blocks, ~800 lines of tests and one facade covering every OBS request c... | [P6-T01](#p6-t01), [P6-T02](#p6-t02) |
| obs-acl-02 | high | adjusted | code-smell | src/obs/client.rs 781-1074 | execute_control_with_identity_if is a ~290-line, 24-arm switch that mixes validation, identity resolution and wire tr... | [P4-T14](#p4-t14), [P6-T03](#p6-t03) |
| obs-acl-03 | high | adjusted | ddd | src/obs/client.rs 846-847, 927-933, 1008-1009, 1037-1038, 1050-1051... | Domain rules (valid ranges) live in the OBS adapter, and ControlAction carries raw primitives | [P4-T07](#p4-t07) |
| obs-acl-04 | high | confirmed | boundary | src/obs/mapper.rs 12, 20-30; client.rs 14, 185-195; controller/even... | The OBS adapter imports a controller type (ConnectionInfo), and no architecture test catches it | [P3-T01](#p3-t01) |
| obs-acl-05 | high | adjusted | error-handling | src/obs/client.rs 144-146 and every .map_err(AppError::request); re... | obws errors are flattened to strings, so callers detect retirement through a side channel and tests match substrings | [P6-T05](#p6-t05) |
| obs-acl-06 | medium | adjusted | correctness | src/obs/client.rs 787-793, 807-836; audio_presets.rs 38-45; operato... | Identity handling: one UUID is reused for all target kinds, ignored by some actions, parsed in four places, and nil-c... | [P6-T07](#p6-t07) |
| obs-acl-07 | medium | adjusted | boundary | src/obs/mapper.rs 36-54 vs client.rs 529-538; client.rs 188-193, 21... | The mapper is incomplete: about 9 obws-to-domain conversions are inline, duplicated, or disagree with each other | [P1-T11](#p1-t11), [P6-T04](#p6-t04) |
| obs-acl-08 | medium | confirmed | code-smell | src/obs/mapper.rs 20-30, 57-69, 151-243, 245, 259-561 | mapper.rs mixes response mapping, event mapping and placeholder data; tests take half the file | [P6-T04](#p6-t04) |
| obs-acl-09 | medium | adjusted | code-smell | src/obs/client.rs 632-761 | get_source_detail is a ~130-line method with up to 9 sequential round trips and string warnings | [P6-T02](#p6-t02), [P6-T09](#p6-t09) |
| obs-acl-10 | medium | adjusted | docs | src/obs/client.rs 847, 928, 932, 969, 982, 1009, 1038, 1051, 671-75... | User-facing English strings in the adapter skip fl!, against the i18n rule | [P6-T05](#p6-t05) |
| obs-acl-11 | medium | adjusted | performance | src/obs/client.rs 519-575 | get_control_snapshot makes 7 independent reads in sequence and drops OBS rejections silently | [P6-T09](#p6-t09) |
| obs-acl-12 | low | adjusted | error-handling | src/obs/requests.rs 14, 31-39, 101-115 | RequestGate stores retirement twice and uses Mutex::expect; its timeout cannot be injected | [P6-T06](#p6-t06) |
| obs-acl-13 | low | adjusted | code-smell | src/obs/requests.rs 133-188 | run_if_valid nests selects and uses Result<Option<Result>> sentinels to encode admission outcomes | [P6-T05](#p6-t05) |
| obs-acl-14 | medium | adjusted | god-struct | src/obs/client.rs 52-60, 99-101, 169-183 | ObsClient mixes the transport handle, two admission gates, session metadata and an app-level revision counter, with t... | [P6-T08](#p6-t08) |
| obs-acl-15 | medium | adjusted | test | src/obs/obs_fixture.rs 1-53; client.rs 331-475, 1107-1145; event.rs... | The protocol fixture is scattered: the scripted mock lives in client.rs, and handshake/response JSON is written again... | [P0-T06](#p0-t06) |
| obs-acl-16 | low | adjusted | error-handling | src/obs/operator_inputs.rs 90-165 | Infallible collection is expressed with AppError plus expect | [P6-T04](#p6-t04) |
| obs-acl-17 | low | adjusted | error-handling | src/obs/audio_presets.rs 13-62 | Preset adapter returns Result<_, String> built from localized user_message() | [P6-T05](#p6-t05) |
| obs-acl-18 | low | adjusted | correctness | src/obs/client.rs 559, 703-704, 743; operator_inputs.rs 125-126; ma... | Duration conversion is inconsistent: unchecked i128 as i64 casts in some places, try_from in others | [P6-T04](#p6-t04) |
| obs-acl-19 | low | adjusted | code-smell | src/obs/client.rs 244-249, 269-274 | get_stream_status and get_stream_health send the same GetStreamStatus request | [P6-T01](#p6-t01) |
| obs-acl-20 | low | adjusted | code-smell | src/obs/client.rs 74-76, 204-210, 570, 606, 764-770, 1077-1087; aud... | Magic numbers, and test-only convenience methods inside the production impl | [P6-T11](#p6-t11) |
| obs-acl-21 | low | confirmed | test | src/obs/regression_tests.rs 1-706 | regression_tests.rs mixes several concerns and depends on a test module inside client.rs | [P2-T04](#p2-t04) |
| performance-01 | high | confirmed | performance | src/ui/widgets/status_bar.rs 142-165 | Perpetual frame tick callbacks keep the frame clock running for the whole app lifetime (status bar + Live page) | [P9-T19](#p9-t19) |
| performance-02 | high | adjusted | performance | src/ui/window.rs 424-468 | AppState::collect_report runs twice per AppEvent and every 50 ms tick; O(activity+outputs+alerts+bookmarks) with heav... | [P8-T12](#p8-t12), [P9-T01](#p9-t01) |
| performance-03 | high | adjusted | performance | src/ui/window.rs 306-375 | UI synchronisation is built on about 15 long-lived glib polling timers because AppState has no change notification | [P8-T15](#p8-t15), [P9-T14](#p9-t14), [P9-T15](#p9-t15) |
| performance-04 | high | adjusted | performance | src/ui/pages/live.rs 659-733 | One scene switch rebuilds Live scene cards 3+ times, each deep-cloning SceneRegistry and SceneInventory | [P4-T23](#p4-t23), [P10-T10](#p10-t10) |
| performance-05 | high | adjusted | performance | src/ui/event_pump.rs 11-55 | 50 ms poll-based event pump adds about 25 ms mean latency and 20 idle wakeups/s; the producer side busy-retries every... | [P9-T13](#p9-t13) |
| performance-06 | medium | adjusted | correctness | src/controller/refresh_controller.rs 531-537 | Meter frames use slots in the bounded GTK queue that lifecycle events need; a GTK stall can turn meters into a connec... | [P7-T15](#p7-t15) |
| performance-07 | medium | adjusted | performance | src/obs/client.rs 253-274 | Stats poll sends 3 sequential requests every second, including GetVideoSettings; thumbnails send GetVideoSettings aga... | [P6-T09](#p6-t09) |
| performance-08 | medium | confirmed | performance | src/obs/client.rs 519-575 | get_control_snapshot makes 7 sequential round trips on every workspace refresh (triggered by each scene change) | [P6-T09](#p6-t09) |
| performance-09 | medium | adjusted | performance | src/ui/pages/mixer.rs 106-221 | Mixer refresh builds a Debug-format String signature and allocates ScopeBinding per input on the fast path | [P10-T08](#p10-t08) |
| performance-10 | medium | adjusted | ddd | src/controller/state.rs 788-796 | workflow_scope() rebuilds the ScopeBinding value object with allocations on every call (30 production call sites, sev... | [P8-T08](#p8-t08) |
| performance-11 | medium | adjusted | performance | src/obs/audio.rs 102-163 | Scene-audio membership is N+1: GetSceneItemEnabled per scene item, and the enable-state event payload is discarded | [P6-T10](#p6-t10) |
| performance-12 | medium | adjusted | ddd | src/controller/state.rs 447-448 | OBS capabilities are a Vec<String> scanned linearly on 100 ms timers | [P4-T06](#p4-t06) |
| performance-13 | medium | adjusted | performance | src/ui/widgets/volume_meter.rs 149-237 | One tick callback per meter runs every frame even when silent; Vec allocation and InputLevels clone per frame | [P10-T18](#p10-t18) |
| performance-14 | medium | adjusted | god-file | src/ui/window.rs 664-1248 | apply_event is a 585-line match mixing AppState mutation, persistence scheduling and widget refresh (god function in ... | [P9-T02](#p9-t02), [P9-T05](#p9-t05) |
| performance-15 | medium | confirmed | god-file | src/controller/refresh_controller.rs 409-586 | run_event_loop hand-codes OBS→AppEvent translation, invalidation policy and transport backpressure in one 178-line fu... | [P7-T09](#p7-t09) |
| performance-16 | low | confirmed | performance | src/domain/health/audio.rs 59-110 | Audio health observe runs at 20 Hz with O(R²) rule-uniqueness, a deep rules compare and String clones | [P4-T17](#p4-t17) |
| performance-17 | low | adjusted | performance | src/controller/state/report_history.rs 95-140 | Report-history staging deep-clones the archive and every eligible report and compares by value every 10 s | [P5-T11](#p5-t11) |
| performance-18 | low | adjusted | performance | src/ui/previews.rs 170-213 | Base64 and PNG decode run on the GTK thread for each preview (up to 2/s) | [P10-T24](#p10-t24) |
| performance-19 | low | adjusted | performance | src/ui/widgets/audio_card.rs 130-135 | update_mute has no change detection, so fast-path refreshes rewrite toggle state and CSS for every card | [P10-T17](#p10-t17) |
| performance-20 | medium | confirmed | test | Cargo.toml dev-dependencies | No performance instrumentation, counters or benchmarks; hot-path claims cannot be measured or regression-gated | [P0-T15](#p0-t15) |
| performance-21 | low | confirmed | test | src/obs/obs_fixture.rs whole file | The protocol fixture cannot count or reorder requests, which blocks request-budget regression tests | [P0-T06](#p0-t06) |
| performance-22 | low | confirmed | code-smell | src/obs/client.rs 317-517 | Test module sits in the middle of client.rs; client mixes connection, telemetry, outputs, thumbnails, control executi... | [P2-T04](#p2-t04), [P6-T01](#p6-t01) |
| recent-change-review-01 | high | adjusted | correctness | src/ui/navigation.rs src/ui/navigation.rs:184-187 (dispatch gate), ... | Program-scene switching is disabled whenever optional curation registry is not Ready | [P1-T03](#p1-t03) |
| recent-change-review-02 | high | adjusted | error-handling | src/app.rs 404-425 (retry branch of show_persistence_close_failure)... | Close-failure Retry re-admits only registry and history writes; bookmark, session-report export, config-snapshot and ... | [P1-T04](#p1-t04), [P9-T16](#p9-t16) |
| recent-change-review-03 | high | adjusted | code-smell | src/ui/persist.rs src/ui/persist.rs:23-205, 207-271; src/ui/report_... | Three hand-rolled thread_local durable-snapshot state machines live in ui/ (duplicated knowledge, hidden singletons) | [P9-T16](#p9-t16) |
| recent-change-review-04 | high | adjusted | god-struct | src/controller/state.rs 19-30, 438-499 (AppState), plus impl blocks... | AppState grew 9 fields; feature behaviour was relocated into 6 new `impl AppState` files, not decomposed | [P8-T01](#p8-t01), [P8-T07](#p8-t07), [P8-T12](#p8-t12) |
| recent-change-review-05 | high | adjusted | ddd | src/controller/state/report_history.rs 3-26 (also state/bookmarks.r... | Storage serde DTOs are used as live controller aggregates; storage path types leak into controller (no anti-corruptio... | [P5-T11](#p5-t11) |
| recent-change-review-06 | medium | adjusted | god-struct | src/storage/registry.rs 23-72 (struct), 167-476 (impl blocks), 530-... | SceneRegistry became a god aggregate spanning five bounded contexts in one serde struct and one file | [P5-T03](#p5-t03), [P5-T04](#p5-t04), [P5-T05](#p5-t05) |
| recent-change-review-07 | medium | adjusted | performance | src/ui/widgets/deck_controls.rs 52-62, 283-290 (with src/storage/sh... | Every scene-search change rewrites the entire show-workspaces.json store (all workspaces) | [P1-T24](#p1-t24) |
| recent-change-review-08 | low | adjusted | correctness | src/storage/show_workspaces.rs 71-93, 121-157 (limit in src/storage... | All workspaces share one file and one 8 MiB REGISTRY_LIMIT with no eviction or garbage collection | [P1-T20](#p1-t20) |
| recent-change-review-09 | medium | adjusted | ddd | src/controller/state/session_reports.rs 336-388 (also src/controlle... | Localized UI text and ad-hoc strings are persisted as report domain data (stringly-typed events) | [P4-T21](#p4-t21) |
| recent-change-review-10 | medium | adjusted | boundary | src/ui/action_catalog.rs 66-146, 154-287, 513-616, 619-665 | Operator-action policy (identity resolution, capability checks against OBS request names) lives in ui and switches on... | [P4-T24](#p4-t24), [P4-T25](#p4-t25) |
| recent-change-review-11 | medium | confirmed | god-file | src/ui/window.rs 664-1253 | apply_event is a ~590-line switch; the commit added inline domain transitions to it | [P9-T02](#p9-t02), [P9-T03](#p9-t03) |
| recent-change-review-12 | medium | adjusted | boundary | src/ui/event_reconciliation.rs 1-206 | GTK-free AppState reducer lives in ui/ (misplaced module) | [P8-T11](#p8-t11) |
| recent-change-review-13 | medium | confirmed | boundary | src/ui/navigation.rs 19-31, 53-56, 98-122, 184-207 | NavigationContext widened into a service locator, contradicting the documented page contract | [P9-T12](#p9-t12) |
| recent-change-review-14 | medium | adjusted | code-smell | src/ui/persist.rs 358-378, 511-536, 554-571 (also src/ui/window.rs:... | show_registry invariants (generation and revision bumps) are mutated directly from ui; registry snapshot lives apart ... | [P8-T09](#p8-t09) |
| recent-change-review-15 | medium | confirmed | ddd | src/ui/persist.rs 317-328 (also src/storage/secret.rs:21, src/stora... | Legacy-migration policy and magic "default" connection id live in ui; ConnectionId is a raw String | [P4-T22](#p4-t22), [P5-T12](#p5-t12) |
| recent-change-review-16 | medium | confirmed | boundary | src/obs/mapper.rs 12-21 (also src/obs/client.rs:93-101, 175-192) | obs adapter stamps placeholder connection_id/endpoint into a controller type and patches them later | [P3-T01](#p3-t01), [P6-T08](#p6-t08) |
| recent-change-review-17 | medium | adjusted | code-smell | src/controller/state/report_history.rs 7-50 (also src/app.rs:104-11... | Boolean-flag state machines and tuple or sentinel state instead of enums | [P8-T13](#p8-t13), [P9-T09](#p9-t09) |
| recent-change-review-18 | medium | adjusted | code-smell | src/controller/state/session_reports.rs 136-210, 271-411 | collect_report is a 140-line multi-concern method with repeated unwraps; report id built ad hoc | [P8-T12](#p8-t12) |
| recent-change-review-19 | medium | confirmed | test | src/controller/state/session_reports.rs 66, 120, 162-165, 213 (also... | Wall-clock reads inside state transitions; inconsistent with the injected `now` used elsewhere | [P3-T05](#p3-t05) |
| recent-change-review-20 | medium | adjusted | test | src/app.rs src/app.rs:158-425 (document_windows, CloseGuard, show_p... | Exit-checkpoint protocol is only covered by #[ignore] native GTK tests; no pure decision tests | [P9-T09](#p9-t09) |
| recent-change-review-21 | low | confirmed | correctness | src/ui/global_shortcuts.rs 62-90 | Persistent portal shortcut IDs are derived from serde_json serialization of OperatorAction | [P9-T18](#p9-t18) |
| recent-change-review-22 | low | adjusted | code-smell | src/ui/report_history.rs 310-324 (dupes: ui/quick_actions.rs:18, ui... | Duplicated widget scaffolding across the new feature modules (labels, dialog shell, test finders) | [P10-T22](#p10-t22) |
| recent-change-review-23 | low | confirmed | code-smell | src/controller/state.rs 465-474, 605-751 (also src/controller/outpu... | Stream/Record data clump duplicated across state, controller and ui | [P7-T07](#p7-t07), [P8-T03](#p8-t03) |
| recent-change-review-24 | low | adjusted | code-smell | src/storage/report_export.rs 96-120 | CSV export uses magic column indexes and a JSON-trim hack for enum names | [P5-T13](#p5-t13) |
| recent-change-review-25 | low | adjusted | code-smell | src/ui/pages/settings/connections.rs 22-38 | Second keyring port defined in a UI page duplicates controller::dependencies::ObsPasswordProvider | [P1-T14](#p1-t14) |
| recent-change-review-26 | low | confirmed | code-smell | src/storage/registry.rs 152-165, 477-503, 623-638 (also src/ui/pers... | Test-only scaffolding kept inside production modules | [P2-T05](#p2-t05) |
| recent-change-review-27 | low | confirmed | god-file | src/ui/mod.rs whole ui/ root (30 flat files) | Flat ui/ root: 13 new feature modules added beside shell infrastructure without grouping | [P9-T20](#p9-t20) |
| recent-change-review-28 | low | adjusted | docs | docs/feature-program.md commit 9488c86 as a whole | Single 28k-line commit mixes refactors, the storage migration, 10+ features and docs | [P11-T01](#p11-t01) |
| security-01 | high | confirmed | security | src/ui/pages/mixer.rs 733-746, 798-806 (append_mixer_status), 808-8... | OBS-controlled scene names and error text are rendered as Pango markup in StatusPage descriptions and Toast titles | [P1-T01](#p1-t01) |
| security-02 | medium | adjusted | security | src/obs/client.rs 69-107; Cargo.toml:20 | OBS WebSocket is plaintext ws:// only, with no TLS option and no non-loopback warning | [P1-T16](#p1-t16), [P11-T03](#p11-t03) |
| security-03 | medium | adjusted | ddd | src/storage/config.rs 99-105 (ObsConfig), 358 (legacy_registry_endp... | OBS endpoint is a raw host String, and `host:port` text is formatted in five places and used as workspace identity (p... | [P4-T22](#p4-t22) |
| security-04 | medium | confirmed | security | src/controller/state.rs 437-456 (#[derive(Debug, Clone)] AppState, ... | Plaintext OBS password cached in the shared Debug+Clone AppState for the whole process lifetime | [P1-T13](#p1-t13) |
| security-05 | medium | adjusted | performance | src/app.rs 42-73 | Startup blocks GTK initialisation on a synchronous Secret Service call, and the keyring is touched on every launch | [P1-T15](#p1-t15) |
| security-06 | medium | confirmed | correctness | src/ui/pages/settings/connections.rs 22-38 (ConnectionSecrets/Keyri... | Two uncoordinated keyring ports; Connect can read a stale password while a Secret-lane write is still queued (PLAUSIB... | [P1-T14](#p1-t14) |
| security-07 | low | adjusted | dead-code | src/controller/workspace.rs 32 (WorkspaceRequest.screenshot), 50, 5... | OBS screenshot request path is dead: never requested in production, yet kept in AppState and the obs adapter | [P11-T02](#p11-t02) |
| security-08 | low | adjusted | security | src/storage/show_workspaces.rs 69-93 (load_or_create), 121-133 (rea... | Show-workspace store grows without bound, keyed by OBS-supplied collection names; past 8 MiB every read and write fails | [P1-T20](#p1-t20) |
| security-09 | low | confirmed | correctness | src/storage/config.rs 25-63 (AppConfig), 317-323 (apply_migrations)... | A newer config is silently down-converted: unknown fields are dropped on the next save, and even on read | [P1-T08](#p1-t08) |
| security-10 | low | adjusted | error-handling | src/storage/config.rs 234-245 | `read_config_from_path` has a write side effect and swallows its error | [P1-T09](#p1-t09) |
| security-11 | low | adjusted | security | src/storage/registry.rs 655-679 (read_registry_yaml_from_path) | YAML import relies on serde-saphyr's default budget and has no semantic count or length caps | [P1-T19](#p1-t19) |
| security-12 | medium | adjusted | supply-chain | deny.toml 6-8 | RUSTSEC-2026-0374 exception expires 2026-11-20 (6 days from 2026-11-14); CI fails every build after that | [P0-T01](#p0-t01) |
| security-13 | low | confirmed | performance | src/ui/background_io.rs 99-112 (run), 347-376 (deliver_on_main_thre... | `background_io::run` spawns an OS thread per call and polls the result every 20 ms | [P9-T17](#p9-t17) |
| security-14 | low | adjusted | security | src/domain/appearance.rs 280-290 (CustomCssPreference), 335-353; sr... | Custom CSS paths may be relative and are resolved against the process working directory | [P1-T18](#p1-t18) |
| security-15 | low | adjusted | security | src/storage/xdg.rs 28-42 (report_history_path), 44-58 (config_dir) | XDG resolution falls back to the current directory when HOME is unset, and accepts relative XDG_* values | [P1-T17](#p1-t17) |
| security-16 | low | adjusted | boundary | src/obs/client.rs 14; src/obs/mapper.rs 12; src/controller/event.rs... | Anti-corruption layer is inverted: obs/ imports `controller::event::ConnectionInfo` | [P3-T01](#p3-t01) |
| security-17 | low | confirmed | error-handling | src/infra/error.rs 9-27, 30-49 | Stringly-typed AppError erases the credential and auth failure kind | [P5-T01](#p5-t01) |
| security-18 | low | confirmed | privacy | src/storage/report_export.rs 1 (module doc), 25-31 (write), 73 (obs... | Session report exports always include endpoint, connection name and OBS host paths | [P1-T25](#p1-t25) |
| security-19 | medium | adjusted | god-struct | src/storage/registry.rs 22-72 (SceneRegistry), 167-478 (impl blocks... | SceneRegistry is a storage serde struct acting as a god aggregate across several bounded contexts | [P5-T03](#p5-t03) |
| storage-infra-01 | high | adjusted | ddd | src/storage/registry.rs 22-72, 167-476; registry/identity.rs 22-131... | Storage DTO SceneRegistry is the live domain aggregate (no anti-corruption layer) | [P5-T05](#p5-t05), [P5-T06](#p5-t06), [P5-T07](#p5-t07) |
| storage-infra-02 | high | adjusted | god-struct | src/storage/registry.rs 22-72, 655-678 | SceneRegistry has 12 fields spanning 5 bounded contexts | [P5-T04](#p5-t04) |
| storage-infra-03 | high | confirmed | correctness | src/storage/config.rs 25-63, 326-334, 380-405; src/storage/registry... | Unknown fields from newer builds are silently erased on next save; registry.json has no schema version | [P1-T08](#p1-t08) |
| storage-infra-04 | medium | adjusted | ddd | src/storage/registry.rs 96-104, 115-126, 504-528, 571-581; src/stor... | Primitive obsession: accent colour and icon key stored and passed as String; LiveConfig role lists are dead config | [P5-T09](#p5-t09) |
| storage-infra-05 | high | adjusted | correctness | src/storage/config.rs 234-265 (240-242); src/controller/dependencie... | Config read has side effects (migration write-back, swallowed error; rename on parse failure) and is called from the ... | [P1-T09](#p1-t09) |
| storage-infra-06 | medium | adjusted | correctness | src/storage/show_workspaces.rs 50-66, 71-93, 121-157; src/storage/c... | No inter-process lock on config/registry/workspace stores; snap build is NON_UNIQUE | [P1-T21](#p1-t21) |
| storage-infra-07 | medium | adjusted | error-handling | src/infra/error.rs 9-83 | AppError is stringly-typed, loses source chains, and has a test-only variant | [P5-T01](#p5-t01) |
| storage-infra-08 | medium | adjusted | error-handling | src/storage/show_workspaces.rs 36-48, 81, 121-150; registry.rs 129-... | Three incompatible storage error styles; typed RegistryStorageError flattened into io::Error::other | [P5-T01](#p5-t01) |
| storage-infra-09 | medium | adjusted | god-struct | src/storage/config.rs 25-85, 99-153; config/connections.rs 14-101 | AppConfig is a 15-field public bag with a denormalised `obs` endpoint kept in sync by hand | [P4-T22](#p4-t22), [P5-T10](#p5-t10) |
| storage-infra-10 | medium | confirmed | correctness | src/storage/config.rs 234-255, 287-303; config/connections.rs 105+ | An older build moves a newer-but-valid config aside and starts on defaults | [P1-T08](#p1-t08) |
| storage-infra-11 | low | adjusted | correctness | src/storage/small_file.rs 14-22; config.rs 260-263, 383-390; regist... | O_NOFOLLOW on config files refuses symlinked dotfiles and makes every later save fail | [P1-T22](#p1-t22) |
| storage-infra-12 | low | adjusted | boundary | src/storage/config.rs 10, 16, 206-228 | Storage localises ConfigStartupNotice (fl! in storage) | [P5-T10](#p5-t10) |
| storage-infra-13 | medium | confirmed | code-smell | src/storage/registry.rs 151-165, 478-502, 622-637 | Test-only production API (#[cfg(test)] RegistryMutationError, mutate_registry, free set_scene_icon, *_from_path) | [P2-T05](#p2-t05) |
| storage-infra-14 | medium | confirmed | test | src/storage/registry.rs 693-1556; config.rs 409-771 | No golden wire-format fixtures guard the on-disk schemas before restructuring | [P0-T05](#p0-t05) |
| storage-infra-15 | low | confirmed | code-smell | src/storage/config.rs 305-374 | Migration chain is a hand-rolled while/match with a no-op step and an unreachable arm | [P5-T10](#p5-t10) |
| storage-infra-16 | medium | confirmed | performance | src/storage/show_workspaces.rs 36-66, 121-157; small_file.rs 10 | Whole multi-workspace store re-read and rewritten on every single-workspace edit, capped at one registry's limit | [P1-T20](#p1-t20) |
| storage-infra-17 | medium | adjusted | ddd | src/storage/report_history.rs 37-39, 42-84, 86-101, 104-108; bookma... | Domain invariants of SessionReport and Bookmark are enforced in storage | [P5-T11](#p5-t11) |
| storage-infra-18 | low | confirmed | code-smell | src/storage/report_export.rs 64-177 | CSV exporter builds rows by magic positional index (values[13], vec![String::new(); 14]) | [P5-T13](#p5-t13) |
| storage-infra-19 | low | adjusted | security | src/storage/xdg.rs 21, 25, 28-42, 45-58 | Relative XDG_* values accepted and a CWD fallback; base-dir logic duplicated | [P1-T17](#p1-t17) |
| storage-infra-20 | medium | adjusted | security | src/storage/secret.rs 14-26, 33-46; src/controller/state.rs 437-456 | Keyring secrets passed as plain String and cached in a Debug-derived AppState | [P1-T13](#p1-t13) |
| storage-infra-21 | low | confirmed | correctness | src/storage/registry.rs 586-594; src/app.rs 51 | Invalid registry.json at startup silently becomes empty curation (warn log only) | [P1-T23](#p1-t23) |
| storage-infra-22 | low | confirmed | code-smell | src/storage/registry.rs 277-310, 318-343, 398-437, 449-475 | Near-identical set_* field mutators repeat the get_mut / is_empty / remove dance | [P5-T03](#p5-t03) |
| storage-infra-23 | low | adjusted | boundary | src/storage/show_workspaces.rs 1, 10-15, 134-142; src/ui/background... | Persistence ordering invariant lives in ui/background_io.rs; WorkspaceKey is three bare Strings | [P5-T14](#p5-t14) |
| storage-infra-24 | low | adjusted | code-smell | src/storage/registry.rs 640-652; config.rs 380-405; bookmarks.rs 67... | The 'validate original, serialise, size-check, atomic write' sequence is duplicated in five stores | [P5-T02](#p5-t02) |
| tests-structure-01 | high | adjusted | test | src/controller/final_regression_tests.rs 22-224; src/controller/ses... | Hand-rolled loopback OBS servers duplicate the wire protocol across controller and obs tests | [P0-T06](#p0-t06) |
| tests-structure-02 | high | adjusted | god-file | src/controller/final_regression_tests.rs 1-1981 (mounted at src/con... | final_regression_tests.rs: 1981-line god test file mounted in app_controller and coupled to private fields | [P2-T09](#p2-t09), [P2-T10](#p2-t10) |
| tests-structure-03 | high | confirmed | code-smell | src/controller/final_regression_tests.rs 22-32, 51-198, 653, 753, 1... | Plan mock is a flag bag; helper fns take boolean flag triples | [P2-T09](#p2-t09) |
| tests-structure-04 | high | adjusted | test | src/storage/registry.rs 152-170, 478-520, 622-637; src/controller/s... | Test-only production APIs: tests exercise code that never ships | [P2-T05](#p2-t05) |
| tests-structure-05 | high | adjusted | boundary | src/architecture_tests.rs 548-681 | Fitness tests miss real dependency-direction violations and have no size budget | [P0-T07](#p0-t07) |
| tests-structure-06 | medium | adjusted | god-file | src/architecture_tests.rs 18-366 (scanner), 205-227 (check_path; sp... | architecture_tests.rs mixes a syn scanner engine with the rules; contains rule-specific hacks | [P0-T04](#p0-t04) |
| tests-structure-07 | medium | confirmed | god-file | src/controller/state.rs 1247-3088; src/storage/registry.rs:694-1556... | Inline test modules make up 50-90% of the largest 'god files' | [P2-T01](#p2-t01), [P2-T07](#p2-t07), [P2-T08](#p2-t08) |
| tests-structure-08 | medium | confirmed | test | src/controller/state/mixer.rs 453-456; src/obs/regression_tests.rs:... | Child test modules import the parent's test module | [P2-T01](#p2-t01) |
| tests-structure-09 | medium | adjusted | test | src/controller/app_controller.rs 483-500, 686-760; src/controller/f... | State and dependency fixtures duplicated across many test modules | [P0-T14](#p0-t14) |
| tests-structure-10 | medium | confirmed | test | src/ui/widgets/operating_panel.rs 1748-1790; src/ui/mod.rs:243-345 | GTK native-test harness re-implemented per file; operating_panel bypasses the timeout-guarded runner | [P0-T14](#p0-t14) |
| tests-structure-11 | medium | confirmed | test | src/ui/window.rs 1378-1401, 2549-2698; src/controller/app_controlle... | Tests live with the wrong owner | [P2-T02](#p2-t02) |
| tests-structure-12 | medium | confirmed | test | src/ui/pages/inventory.rs 1851-1870 | Source-text test via include_str!/split breaks on file split | [P0-T03](#p0-t03) |
| tests-structure-13 | medium | confirmed | test | tools/test-native.py 10-22 | Native test gate keys on module path prefixes and offers no subset run | [P0-T02](#p0-t02) |
| tests-structure-14 | medium | confirmed | test | src/controller/state.rs stream_/record_ test pairs within 1247-3088... | Mirror-image stream/record tests and near-identical mixer cases should be table-driven | [P2-T11](#p2-t11) |
| tests-structure-15 | medium | confirmed | boundary | src/controller/final_regression_tests.rs 51-198 (op:5 at 89), 303-3... | Controller tests speak raw OBS wire protocol | [P2-T09](#p2-t09) |
| tests-structure-16 | medium | adjusted | test | src/ui/widgets/operating_panel.rs 1-1613 (prod) vs 1614-2361; src/u... | Largest UI splits have the thinnest safety net | [P0-T13](#p0-t13) |
| tests-structure-17 | low | confirmed | test | src/controller/final_regression_tests.rs 226-237, 451; src/controll... | Ad-hoc polling waits and scattered magic timeouts | [P0-T14](#p0-t14) |
| tests-structure-18 | low | confirmed | test | src/storage/registry.rs 954-963; src/storage/bookmarks.rs:85-89 | Hand-rolled temp dir/path helpers with ad-hoc cleanup | [P0-T14](#p0-t14) |
| tests-structure-19 | low | confirmed | code-smell | src/controller/app_controller.rs 460-462; src/obs/client.rs:1393-13... | Two competing conventions for out-of-line tests | [P2-T07](#p2-t07) |
| tests-structure-20 | low | adjusted | test | src/obs/regression_tests.rs 1-706 | OBS adapter tests organized by history, not ObsClient capability | [P2-T04](#p2-t04) |
| ui-pages-01 | high | adjusted | boundary | src/ui/navigation.rs 19-20 (and call sites in pages) | NavigationContext exposes the raw Rc<RefCell<AppState>>; pages read and mutate AppState fields directly, contrary to ... | [P9-T10](#p9-t10), [P9-T11](#p9-t11) |
| ui-pages-02 | high | confirmed | security | src/ui/pages/mixer.rs 398-403, 736-745, 798-806, 815-824, 933-938 | Mixer StatusPage descriptions interpolate OBS scene names and error messages without markup escaping | [P1-T01](#p1-t01) |
| ui-pages-03 | high | confirmed | god-file | src/ui/pages/inventory.rs 1-1504 (prod), 1506-2126 (tests) | inventory.rs is a 2126-line god module: presenter, reconciler, row view, popover editor, drag and drop, YAML dialogs,... | [P10-T05](#p10-t05) |
| ui-pages-04 | high | adjusted | correctness | src/ui/pages/inventory.rs 198-211, 213-326, 1106-1118, 1474-1477 | Incremental inventory reconciliation finds widgets by string widget-name through recursive tree walks and unwraps eac... | [P10-T06](#p10-t06) |
| ui-pages-05 | high | adjusted | ddd | src/ui/pages/inventory.rs 84-114, 1025-1047, 1086-1090, 1120-1169, ... | Scene-registry business rules are implemented inside GTK closures instead of on the registry aggregate | [P5-T06](#p5-t06) |
| ui-pages-06 | high | adjusted | ddd | src/ui/pages/live.rs 606-654 | Deck slot projection lives in the Live GTK page and is consumed by hotkeys and window code | [P4-T23](#p4-t23) |
| ui-pages-07 | high | confirmed | boundary | src/ui/pages/live.rs 310-604 | Output start/stop and disconnect flows (~300 lines) live in the Live page but are owned by the sidebar and transport bar | [P4-T20](#p4-t20), [P9-T07](#p9-t07) |
| ui-pages-08 | high | confirmed | code-smell | src/ui/pages/live.rs 659-802, 830-967 | rebuild_scene_cards (144 lines) and rebuild_audio_cards (138 lines, four early exits, unwrap) mix presentation, diffi... | [P10-T10](#p10-t10) |
| ui-pages-09 | medium | adjusted | code-smell | src/ui/pages/mixer.rs 31-45, 173-221, 370-376 | The presentation-owner tuple is duplicated across pages, and Mixer detects changes with Debug-formatted strings | [P10-T08](#p10-t08) |
| ui-pages-10 | medium | adjusted | god-struct | src/ui/pages/mixer.rs 50-62, 1116-1194, 290-313, 941-1049 | MixerPageHandle combines view handles, presenter signatures and the request-dedupe tracker; debug inspection lives in... | [P8-T14](#p8-t14) |
| ui-pages-11 | medium | confirmed | code-smell | src/ui/pages/mixer.rs 355-558 | mixer::populate is 204 lines, holds an AppState borrow across widget construction and signal wiring, and needs a manu... | [P10-T09](#p10-t09) |
| ui-pages-12 | medium | confirmed | code-smell | src/ui/pages/mixer.rs 488-525, 757-792, 894-913 | Duplicate group-by blocks and pure filtering in the mixer view, with repeated full AudioInput clones | [P10-T09](#p10-t09) |
| ui-pages-13 | medium | confirmed | code-smell | src/ui/pages/settings.rs 102-141, 147-183, 386-444, 572-593; mixer.... | The enum-backed ComboRow pattern is copy-pasted 7 times; the colour-scheme row keeps a hand-written index table | [P10-T14](#p10-t14) |
| ui-pages-14 | medium | adjusted | code-smell | src/ui/pages/doctor.rs 28-55, 119-132; graph.rs 36-62, 74-85 | Doctor and Graph duplicate the topology presentation key and the notice chain, cloning the whole graph and registry o... | [P10-T12](#p10-t12) |
| ui-pages-15 | medium | adjusted | ddd | src/ui/pages/stats.rs 32-54, 846-899 | Stream-health thresholds and severity policy are defined in the Stats page and duplicate domain/health.rs | [P4-T20](#p4-t20) |
| ui-pages-16 | medium | confirmed | performance | src/ui/pages/live.rs 137-152, 270-284; inventory.rs 528-572 | Per-frame tick callbacks on Live and a 100 ms poll for registry save status | [P9-T19](#p9-t19) |
| ui-pages-17 | medium | refuted | performance | src/ui/pages/settings.rs 671-679 | The hotkey leader-timeout SpinRow writes config.json on every value-notify | refuted (8.1) |
| ui-pages-18 | medium | adjusted | boundary | src/ui/pages/settings.rs 25, 446-497 | Settings owns connections' PasswordWrites (with a poisoning lock().expect) and imports window.rs upward | [P0-T09](#p0-t09), [P10-T14](#p10-t14) |
| ui-pages-19 | medium | adjusted | ddd | src/ui/pages/settings/connections.rs 188-234 | OBS endpoint validation is primitive-typed UI logic with a post-check unwrap and a single generic error | [P4-T22](#p4-t22), [P10-T15](#p10-t15) |
| ui-pages-20 | medium | adjusted | ui-a11y | src/ui/pages/live.rs 219-266; graph.rs 129-133, 186-193; doctor.rs ... | Icon-only controls lack explicit accessible labels and states; a focusable canvas does nothing; an accessible name is... | [P10-T23](#p10-t23) |
| ui-pages-21 | medium | confirmed | god-file | src/ui/pages/stats.rs 169-286, 722-844 | Stats mixes the pop-out window lifecycle, activity timeline, gauges, charts and cards; activity marker logic is dupli... | [P10-T11](#p10-t11) |
| ui-pages-22 | low | adjusted | ui-a11y | src/ui/pages/graph.rs 548-786 | The graph canvas hard-codes its palette, uses the cairo toy text API, and ellipsizes in O(n^2) on every draw | [P10-T13](#p10-t13) |
| ui-pages-23 | low | confirmed | code-smell | src/ui/pages/help.rs 128-385, 399-466 | Help topics are hand-coded through a 7-argument topic() function about 20 times; the link row does not wrap | [P10-T16](#p10-t16) |
| ui-pages-24 | low | adjusted | correctness | src/ui/pages/settings.rs 48-56, 731-761 | The Settings builder consumes the startup notice as a side effect; the status text re-borrows state inside a match on... | [P10-T14](#p10-t14) |
| ui-pages-25 | low | adjusted | test | src/ui/pages/inventory.rs 1506-2126 (esp. 1852-1869); live.rs 977-1... | Test modules make up 30-40% of each page file, and one test scans its own file's source, which will break silently on... | [P0-T03](#p0-t03) |
| ui-shell-01 | high | confirmed | test | src/ui/mod.rs 258-299 (native_test); callers src/ui/window.rs:1596,... | Native GTK tests are addressed by string literal; moving a test during a split yields a vacuous green | [P0-T02](#p0-t02) |
| ui-shell-02 | high | confirmed | god-file | src/ui/window.rs 664-1248 | apply_event is a 585-line switch over about 45 AppEvent variants mixing reduction, persistence and widget projection | [P9-T02](#p9-t02), [P9-T03](#p9-t03) |
| ui-shell-03 | high | adjusted | ddd | src/ui/window.rs 664-1356, 1499-1572 | State reduction and UI projection are interleaved; introduce a UiEffects value object so AppState reduction is pure a... | [P9-T05](#p9-t05) |
| ui-shell-04 | high | confirmed | correctness | src/ui/window.rs 475-507 vs 1317-1338 | Two different formatters write the same status-bar slot; the 1 s clock overwrites the event-driven label | [P1-T07](#p1-t07) |
| ui-shell-05 | high | adjusted | god-struct | src/ui/navigation.rs 18-32, 52-213 | NavigationContext is advertised as a two-method surface but exposes raw AppState, shortcuts, scope bindings and busin... | [P0-T08](#p0-t08), [P9-T11](#p9-t11), [P9-T12](#p9-t12) |
| ui-shell-06 | high | adjusted | god-file | src/app.rs 32-159, 161-428, 431-455 | app.rs mixes composition root, startup I/O, runtime shutdown and a 260-line GTK close/quit persistence barrier | [P9-T09](#p9-t09) |
| ui-shell-07 | medium | adjusted | code-smell | src/app.rs 170-278 | CloseGuard is an implicit state machine encoded in five boolean Cells | [P9-T09](#p9-t09) |
| ui-shell-08 | medium | adjusted | performance | src/app.rs 42-74 | Startup reads config, registry, bookmarks, history and the Secret Service password synchronously before the first frame | [P1-T15](#p1-t15) |
| ui-shell-09 | medium | confirmed | code-smell | src/ui/window.rs 296-375 | The 200 ms poller embeds workspace-refresh policy as a 20-line boolean expression inside page assembly | [P9-T08](#p9-t08) |
| ui-shell-10 | medium | adjusted | code-smell | src/ui/window.rs 1492-1572 | PageRefreshers is a closure-per-field struct with a match in call(); adding a page is shotgun surgery | [P9-T08](#p9-t08) |
| ui-shell-11 | medium | confirmed | code-smell | src/ui/window.rs 939-969, 1317-1338, 475-507 | Stream/record data clump forces per-kind match duplication in every output path | [P8-T03](#p8-t03) |
| ui-shell-12 | medium | confirmed | boundary | src/ui/window.rs 985-1058, 1067-1094 | The event applier owns registry persistence and reconciliation policy (storage concern in the UI event path) | [P9-T04](#p9-t04) |
| ui-shell-13 | medium | confirmed | code-smell | src/ui/window.rs 977-983, 1340-1356 | Output events are re-dispatched by runtime check with debug_assert!(false) instead of a typed sub-enum | [P7-T13](#p7-t13) |
| ui-shell-14 | medium | adjusted | god-file | src/ui/window.rs 63-272 | build_main_window is a 210-line Long Method that assembles the split view, wires teardown and hard-codes app action n... | [P9-T06](#p9-t06) |
| ui-shell-15 | medium | adjusted | performance | src/ui/window.rs 412-469 | The event pump closure does three AppState borrow_mut calls and deep-clones EventUiContext per event, up to 128 event... | [P9-T01](#p9-t01) |
| ui-shell-16 | medium | adjusted | god-file | src/ui/window.rs 1587-2698 | About 1100 lines of tests in window.rs, mostly cross-context native scenarios (registry, workspace, inventory) unrela... | [P2-T06](#p2-t06) |
| ui-shell-17 | medium | confirmed | boundary | src/ui/sidebar.rs 19, 33-239, 322-334, 422-425 | sidebar.rs holds header selectors (header chrome) and depends on pages::live for disconnect policy | [P9-T07](#p9-t07) |
| ui-shell-18 | medium | adjusted | ddd | src/controller/state.rs 115-122 (used by src/ui/sidebar.rs:257,449,... | CSS class names live on the controller's ObsStatus; presentation leaks into the orchestration layer | [P4-T09](#p4-t09), [P4-T11](#p4-t11) |
| ui-shell-19 | low | confirmed | ui-a11y | src/ui/window.rs 181-183, 593-600; src/ui/sidebar.rs 65-70 | Icon-only header buttons rely on tooltip text, while the palette button sets an explicit accessible label | [P9-T21](#p9-t21) |
| ui-shell-20 | low | adjusted | ui-a11y | src/ui/window.rs 611-640; src/app.rs 373 | Deprecated adw::MessageDialog used for the welcome and close-failure dialogs | [P9-T21](#p9-t21) |
| ui-shell-21 | low | confirmed | code-smell | src/app.rs 369-388 | The close-failure dialog borrows i18n keys from unrelated contexts | [P9-T09](#p9-t09) |
| ui-shell-22 | low | adjusted | code-smell | src/ui/window.rs 116-130, 201-218 | Sidebar selection handler rebuilds Live scene cards inline and is wired in three separate closures | [P9-T06](#p9-t06) |
| ui-shell-23 | low | confirmed | boundary | src/architecture_tests.rs 640-660 | Architecture test exempts ui/window.rs by path and documents a NavigationContext contract the code no longer has | [P9-T12](#p9-t12) |
| ui-shell-24 | low | confirmed | code-smell | src/ui/window.rs 1455-1496, 111, 585-588 | StreamingChrome is published into an Rc<RefCell<Option<_>>> slot even though it is created synchronously during assembly | [P9-T06](#p9-t06) |
| uiw-01 | high | confirmed | god-file | src/ui/widgets/operating_panel.rs 1-1613 (prod), 678-1096, 1098-149... | Sources page lives in widgets/ as a 2361-line god file with 400-line routines | [P10-T01](#p10-t01) |
| uiw-02 | high | confirmed | code-smell | src/ui/widgets/operating_panel.rs 271-312, 363-402, 414-503, 548-65... | Widget-name string registry and full-tree visit() used as data binding (locale-dependent keys) | [P10-T02](#p10-t02) |
| uiw-03 | high | confirmed | ddd | src/ui/widgets/operating_panel.rs 16-92, 1225-1232, 1318-1322, 1457... | Draft-editing state is an anemic struct whose maps are edited from outside with magic field keys | [P10-T03](#p10-t03) |
| uiw-04 | high | adjusted | boundary | src/ui/action_catalog.rs 66-145, 153-287, 99-104, 207-215, 647-655 | Operator-action permission rules (resolve/observed_label) live in ui/ and repeat the target match three times | [P4-T24](#p4-t24), [P4-T25](#p4-t25) |
| uiw-05 | high | adjusted | ddd | src/ui/widgets/operating_panel.rs 1560-1562; transport_bar.rs:219-2... | Raw OBS protocol request names spread through the UI with three different capability helpers | [P4-T06](#p4-t06) |
| uiw-06 | high | confirmed | god-file | src/ui/persist.rs 23-205, 207-271, 273-287, 289-354, 380-455, 457-5... | persist.rs mixes a write journal, an import transaction gate, scope selection with migration policy, i18n messages an... | [P5-T12](#p5-t12), [P9-T16](#p9-t16) |
| uiw-07 | medium | confirmed | test | src/ui/persist.rs 69-120, 591-1032 | Pure journal-confirmation logic is only tested through subprocess, file system and GTK main-loop tests | [P9-T16](#p9-t16) |
| uiw-08 | medium | adjusted | performance | src/ui/background_io.rs 99-113, 161-243, 318-345, 347-376, 164/183/... | Each background job runs a 20 ms polling timer and an unbounded thread; config lock uses expect | [P0-T09](#p0-t09), [P9-T17](#p9-t17) |
| uiw-09 | medium | confirmed | god-file | src/ui/theme.rs 15-24, 42-96, 128-275, 277-318, 320-398, 400-475, 4... | theme.rs mixes theme catalog, CSS security policy, provider registry, desktop integration and motion generation (60% ... | [P10-T20](#p10-t20) |
| uiw-10 | medium | confirmed | code-smell | src/ui/theme.rs 42-96, 529-614 | Adding a built-in theme takes four separate edits; English name/description fields are dead fallbacks | [P10-T20](#p10-t20) |
| uiw-11 | medium | adjusted | ui-a11y | src/ui/widgets/chart.rs 77-84, 88-119; volume_meter.rs:65-70, 388-404 | Cairo charts and meters hard-code colours, bypassing theme tokens (High Contrast cannot reach them) | [P10-T21](#p10-t21) |
| uiw-12 | medium | adjusted | god-struct | src/ui/audio_presets.rs 95-116, 284-465, 44-66, 135-144, 225-252, 4... | Audio preset Editor is an 18-field god struct with a 180-line render(); preset rules live in the UI | [P10-T19](#p10-t19) |
| uiw-13 | medium | adjusted | god-file | src/ui/action_shortcuts.rs 20-63, 65-95, 97-132, 143-377, 383-561 | action_shortcuts.rs mixes key normalization, shortcut conflict rules, a manager window and an editor dialog | [P10-T19](#p10-t19) |
| uiw-14 | medium | confirmed | performance | src/ui/action_shortcuts.rs 367-376, 551-560; global_shortcuts.rs:39... | Dialogs and widgets redraw on fixed 250 ms or 1 s timers instead of on state changes | [P9-T14](#p9-t14), [P9-T15](#p9-t15) |
| uiw-15 | medium | adjusted | boundary | src/ui/global_shortcuts.rs 3, 164-307, 196-197, 302-304, 68, 79, 81... | D-Bus portal client lives in ui/; raw English errors reach the user; 143-line start() | [P9-T18](#p9-t18) |
| uiw-16 | medium | adjusted | code-smell | src/ui/widgets/audio_card.rs 306-323, 863-864, 113-127, 138-159, 86... | audio_card mutates AppState directly, has a test hook in production code, and repeats fader-reset and handle construc... | [P10-T17](#p10-t17) |
| uiw-17 | medium | confirmed | code-smell | src/ui/widgets/operating_panel.rs 128-154, 549-567, 797-798 | Widget builds a controller request DTO and repeats the 30 s freshness rule | [P10-T04](#p10-t04) |
| uiw-18 | medium | adjusted | code-smell | src/ui/action_catalog.rs 289-461, 513-616, 291-301, 619-640 | Action labels written twice, a third copy of the page list, and an unreachable!() control flow | [P4-T24](#p4-t24) |
| uiw-19 | low | confirmed | code-smell | src/ui/widgets/operating_panel.rs 1585-1600; audio_presets.rs:27-42... | Seven-plus copies of the wrapped-label and button helpers with drifting properties | [P10-T22](#p10-t22) |
| uiw-20 | low | adjusted | performance | src/ui/widgets/operating_panel.rs 162-176, 1601-1612 | Redundant status passes and a dead flag parameter | [P10-T04](#p10-t04) |
| uiw-21 | low | adjusted | ui-a11y | src/ui/theme.rs 537-550, 579-585; resources/themes/*.css | Theme family names contradict their variants; themes differ only in accent and surfaces | [P10-T21](#p10-t21) |
| uiw-22 | low | confirmed | error-handling | src/ui/operator_picker.rs 43-56; deck_controls.rs:308-312 | Production unwrap/expect in list factories and action activation | [P10-T22](#p10-t22) |

---

## 8. Appendix

### 8.1 Refuted findings and refuted sub-claims

Do not act on any item in this section.

**Fully refuted finding:**

| ID | Claim | Why refuted |
|---|---|---|
| ui-pages-17 | The hotkey leader-timeout SpinRow writes config.json on every value-notify; add a debounce | `persist_config` (ui/persist.rs:463-470) routes through `background_io::run_config_snapshot` (background_io.rs:193-207), which coalesces. Each call replaces `batch.latest` and starts a worker only if none is running, so rapid steps produce at most one write in flight plus one pending snapshot. P10-T14 therefore adds no debounce. |

**Refuted sub-claims inside confirmed or adjusted findings.** Implementers must not reintroduce these.

| Finding | Refuted sub-claim | Correct fact |
|---|---|---|
| ctrl-22 | storage/bookmarks.rs:89 is a production storage->controller layering inversion | The call sits inside `#[cfg(test)] mod tests` (bookmarks.rs:81-82). Checked at 31d512f. It is test-only; P0-T14 removes it anyway. architecture-global-06 is right. |
| architecture-global-10 | tools/test-native.py runs one process per test because of the thread_locals | CLAUDE.md explains it: each GTK test needs a fresh display and session bus. |
| architecture-global-17 | Mixer inspection is Speculative Generality; cfg-gate it | `SCENEDECK_MIXER_INSPECT=1` is a documented manual-test diagnostic (docs/manual-test-plan.md:363, 406; manual-test-runs.md:163). Keep it compiled in. |
| architecture-global-09 / chart.rs | chart.rs `Severity` duplicates `DiagnosticSeverity` | They have different semantics ({Unknown, Good, Warning, Critical} vs {Info, Warning, Error}). Do not merge them. |
| architecture-global-21 | action_catalog.rs is 1155 lines, all production | Production is 1-666; tests are 667-1155. |
| performance-14 | apply_event recursion (706, 718, 755) happens while borrows are held, so it can panic | Each borrow is a temporary dropped before the recursion. Borrow-safe today. |
| performance-02 | Run collect_report once per drained batch | That would lose intermediate scene and alert transitions. Drop only the pre-event call and skip InputLevelsUpdated (P9-T01). |
| performance-05 | Use the async-channel crate for the event pump | async-channel has no reserve/permit API, so the `valid()` re-check invariant would be lost. Use `tokio::sync::mpsc` (P9-T13). |
| performance-07 | Invalidate the video settings cache on VideoSettingsChanged | obws 0.15 and obs-websocket v5 have no such event. Use CurrentProfileChanged + reconnect + TTL (P6-T09). |
| performance-04 / performance-18 / ui-pages-06 | Put scene_deck_service over storage::SceneRegistry, or bounded_png with glib in services | Both violate architecture_tests (services may not reach storage or glib). Use domain inputs (P4-T23), or decode in the controller/obs layer (P10-T24). |
| obs-acl-07 | ControlSnapshot.scenes is effectively unused and can be dropped | The preflight review and editor read it (ui/preflight.rs:287-291, editor.rs:37-42). P1-T11 fixes its order instead. |
| obs-acl-11 | Control snapshot timeouts are indistinguishable from failures | A deadline retires the gate (requests.rs:183-186). Only OBS rejections are swallowed. |
| obs-acl-19 | GetStreamStatus is sent twice per cycle (performance) | No single code path calls both. This is duplicate code only. |
| security-07 | The screenshot is fetched on every workspace refresh | Every production path passes `screenshot=false`. It is dead code (P11-T02). |
| security-14 / security-15 | Fix with a storage deserializer on the domain type / with glib home lookup | Domain cannot name storage, and storage bans glib. Use the load-time check (P1-T18) and `std::env::home_dir` (P1-T17). |
| storage-infra-07 / storage-infra split | Move `user_message` / error text into ui | The controller and obs call it. Use `infra/error_text.rs`. |
| recent-change-review (storage split) | Move RuleConfig to domain/graph_rules.rs, or move `impl SceneRegistry` into domain before the type moves | Domain cannot name storage types. RuleConfig stays in storage, and behavior moves only after ShowCuration exists (P5-T05/T06). |
| ui-shell (navigation split) | Move navigation.rs into ui/shell | Pages depend on navigation, so this creates a pages->shell edge. It stays at the ui/ root. |
| ui-shell / architecture-global-02 | NavigationContext::update logs and silently refuses on re-entrancy | That is a lost write (fake success). It must return an error and debug-assert (P0-T08). |
| controller-state-08 / ui-shell-18 | Move ObsStatus::label to ui | The controller calls it (state.rs:755, 859, 861). It goes to `src/presentation` (P4-T09). |
| controller-state-06 | Three fences guard one observation | Two fences. HealthMonitor is downstream (called only after accept_output_observation). |
| controller-state-21 | Put an AppStore with glib::idle_add in controller/ | That fails background_layers_hold_no_gtk_or_ui. Borrow helpers live in ui (P0-T08). |
| controller-state-04 / controller-state-12 | The `Instant::now()` at state.rs:1168 duplicates `at` | It is a separate `now` argument to `observe_stats`. Keep the semantics when injecting time. |
| ctrl-06 | Derive collection scoping from the event family | The independent set cuts across families. Keep a per-variant match (P7-T12). |
| ctrl-10 | NaN handling differs between the inline and domain dB code | Both give -100. The real divergence is `+inf` (0 dB vs -100) (P7-T14). |
| ctrl-17 | Replace the task list with JoinSet | Callers keep JoinHandles. Use a counter plus Notify (P7-T03). |
| ctrl-08 | Use SmallVec for translated publications | SmallVec is not a dependency. Use Vec. |
| domain-services-ddd-13 | A stale alert index mislabels session reports | Alerts are recomputed and resolved every tick against the same vector. This is a smell only. |
| domain-services-ddd-18 | architecture_tests.rs:463-476 must change when domain::obs moves | Those strings are scanner fixtures, not real paths. |
| domain-services-ddd-09 | Fader and preset gain ranges are inconsistent | Both ranges are documented and intentional. Name them; do not unify. |
| uiw-11 | Volume meter colors should follow theme tokens | Fixed zone colors are a documented decision (volume_meter.rs:60-64). |
| ui-pages-24 | Settings consuming the startup notice is a production bug | settings::build runs once per window in production. Only tests call add_pages again. Still cleaned up in P10-T14. |
| ui-pages-15 | Stats page and domain/health disagree on thresholds | They measure different things (cumulative vs windowed). Document both. |
| ui-pages-14 | Use a topology generation counter for the key | It would miss registry edits that nothing tracks. Keep content comparison (P10-T12). |
| tests-structure-16 | Extract reduce_event as a characterization step | That extraction is a behavior refactor. Characterize with native section tests first (P0-T13). |
| ctrl-01 / ctrl-02 / obs-acl-12 / ui-pages-18 / uiw-08 | Mutex poisoning is a live risk | No reachable poison source was found; every critical section is a plain assignment. These are consistency fixes (P0-T09, P0-T10). |

### 8.2 Open questions for the maintainer

1. **Scene switching without curation (P1-T03).** Hotkey gating on curation readiness is documented as intentional (docs/feature-program.md:166-167). Should scene cards and hotkeys keep working in OBS inventory order when the registry is not Ready?
2. **Retry on an unchanged edit (P8-T09).** docs/configuration.md:215-216 documents that repeating an unchanged dirty edit retries retained intent. Keep it (default) or make retry explicit?
3. **Doctor order (P1-T10).** Is a deterministic order change acceptable? It reorders rows the user sees.
4. **Report end reason (P4-T21).** Existing reports store localized end-reason text. Approve stable codes with an `Other(String)` fallback for old reports?
5. **Name policies (P4-T19).** Bookmark names are trimmed with max 120; preset names are exact with max 80. Unify them, or keep two explicit policies (default)?
6. **Meter evidence (P4-T16).** Preflight checks `peak_db` only; health checks `peak_db` and `input_peak_db`. Which rule is correct?
7. **Curation naming (P5-T05).** Grow `domain/registry.rs` into `domain/curation/`, or the reverse? The plan defaults to `domain/curation/`, moving the snapshot types.
8. **Report export redaction default (P1-T25).** On or off by default? Support workflows may need the endpoint.
9. **Symlinked config files (P1-T22).** Keep refusing them (documented O_NOFOLLOW policy), or support them with ownership checks?
10. **TLS (P11-T03).** Worth a rustls dependency tree for proxy-only use?
11. **AppState location (P8-T01).** The plan moves it to `src/state/` and allows the `state -> controller::{event, command}` edge. Alternatively, AppCommand and AppEvent could move to a neutral `src/protocol/` module that both depend on. The plan does not do this, to limit churn.
12. **Theme ids (P10-T21).** Renaming display names is safe. Should the ids also change, with aliases kept?
13. **Audio volume source (domain-services-ddd-09 note).** OBS reports both `inputVolumeMul` and `inputVolumeDb`. Keep both fields (default), or derive one from the other in a separate test-first change?

### 8.3 Reviewer conflicts and how they were settled

| Topic | Positions | Settled as | Basis |
|---|---|---|---|
| storage->controller edge | ctrl-22: production inversion; architecture-global-06 and tests-structure-18: test-only | Test-only | Read storage/bookmarks.rs:78-92 at 31d512f: inside `#[cfg(test)] mod tests` |
| obs/client.rs split mechanics | obs-acl-01: sibling modules work through pub(super); security and performance-22: only child modules see private fields | Child modules `src/obs/client/*.rs` | client.rs:52-60: only `inner` is a pub(super) field; `requests`, `images`, `endpoint`, `connection_id`, `observation` are private; `request`/`mutation` are pub(super) methods |
| Where AppState lives | architecture-global-01: move to src/state; controller-state, recent-change-review and performance: split in place under controller/state | Move to `src/state/` after P3 prerequisites, then split | Tokio code never touches AppState (only tests do); the boundary communicates the threading model; the prerequisites remove the cycle |
| Label location | domain-services-ddd-01 originally: ui/presenters; verifier: GTK-free src/presentation | `src/presentation` for text, `ui/presenters` for css/icon | The controller calls labels; background_layers_hold_no_gtk_or_ui |
| Persistence core location | recent-change-review-03: controller/persistence; architecture-global-10: ui/app_services | Pure core in `src/state/persistence/` (generic over evidence); coordinator in ui | The core must not name ui's FailureEvidence; persistence stays a GTK-side concern per CLAUDE.md |
| Operator action resolution | uiw-04: domain/operator_action/resolution.rs; recent-change-review-10: services/operator_actions; architecture-global-09: state | Domain function over a `ResolutionContext` built by an AppState method | It reads storage and state today; domain and services cannot import those |
| Credential port injection | recent-change-review-13 vs recent-change-review-25: NavigationContext or page builder | Page-builder argument | Keeps NavigationContext narrow |
| ObsClient observation counter | obs-acl-14 original: move to session_controller | Keep it session-scoped, or move to a LiveSession passed to refresh tasks | Refresh tasks hold only a client clone (obs-acl-14 verifier) |
| AppState field count | 41, ~45, ~50 | 47 (WorkspaceState 31) | controller-state verifier counted state.rs:438-498 |
| Sources page location | ui/features/operating vs ui/pages/sources | `ui/pages/sources/` | The pages/sources.rs shim already exists; no third UI grouping concept without an ADR |

---

## 9. Critic pass

A completeness critic checked this plan against the 13 files in `findings/` and the code at `31d512f`. Method: scripted checks over every task block and every table row, then spot reads of the source files the gaps pointed at. Checks (a) to (g) come from the critic brief.

### 9.1 Results by check

| Check | Result |
|---|---|
| (a) Every confirmed or adjusted finding maps to a task | Pass. All 298 live findings (121 confirmed, 177 adjusted) are in section 7, and every task link resolves. The 1 refuted finding is in 8.1. Only 2 findings map solely to P11 (recent-change-review-28 and security-07, both low). |
| (b) Every god file over ~1000 lines has a decomposition entry | Pass. All 18 files of 1000+ lines at `31d512f` have entries 4.1-4.18, and the files of 800-999 lines are in 4.19. Every `proposed_splits` file in `findings/*.json` appears in the plan. Fixed: the summary said "seventeen" files instead of eighteen. |
| (c) Dependencies acyclic; each step stays green | Pass after fixes. The graph is acyclic (DFS over all 189 tasks), and the phase tables match the task bodies. The one cross-phase forward edge (P7-T13 -> P9-T02) is intentional and documented. Gaps were tasks that edit or create a path another task creates, or that split a file before its tests are pulled out. 19 dependencies were added (9.2). |
| (d) File paths and type names exist | Mostly pass. Every existing-file path cited in a task's Files list exists. Unresolved names are new types the plan introduces, or external types (`JoinError`, `AlertDialog`, `NonZeroU16`). Wildcards such as `MixerInspection*` match real types (`MixerInspectionInput`, `MixerInspectionSnapshot`, ...). One wrong cross-reference was fixed: P1-T08 sent notice rendering to P5-T08, but P5-T10 owns it. |
| (e) Target tree: no cycles; CLAUDE.md rules kept | Gap found and fixed. `src/infra/i18n.rs:18` imports `crate::domain::appearance::Language`, while the domain imports `infra::i18n` and `infra::error`, so domain <-> infra is a cycle today. Section 3.4 had no `infra` row. Added the `infra` row, the `infra_is_a_leaf` rule and task P3-T08. Kept: `obs/` stays the only obws importer, GTK-thread ownership (`state` has no gtk; P8-T01 adds the ban), and no task routes persistence through AppCommand/AppEvent (grep-checked). |
| (f) Tasks fit one agent session | Mostly pass. The large tasks are pure moves (P2-*, P4-T12/T13, P8-T01, P9-T20, P10-T01/T05/T07), which section 2.2 allows. P9-T16 (8 files, behavior plus move) now has an explicit 3-PR breakdown. |
| (g) Acceptance criteria are verifiable | Pass after fixes. Seven vague criteria were replaced with test-backed or grep-backed checks: P1-T08, P1-T19, P4-T11, P6-T01, P8-T16, P10-T21, P11-T03. |

### 9.2 Changes made

1. **New task P3-T08** "Make infra a leaf". `i18n::init` takes `Option<&str>` instead of `Language`, and the new rule `infra_is_a_leaf` enforces it. Also changed: the P3 table, the phase count (P3: 8, total: 189), section 1.3, the 5.2 critical path, the 3.3 tree annotation, a 3.4 `infra` row, a 3.4 rule row, and the register row for architecture-global-11.
2. **Dependencies added** (table and body both updated):
   - P1-T01 +P0-T04 (`architecture_tests/rules.rs` exists only after the split);
   - P1-T16 +P1-T13 (both create `domain/connection/`);
   - P2-T02 +P0-T02 (moves tests out of `window.rs`);
   - P4-T25 +P4-T13 (creates `domain/workspace/resolution.rs`);
   - P5-T07 +P0-T07 (lowers a ratchet that P0-T07 creates);
   - P7-T14 and P7-T16 +P2-T03 (`app_controller.rs` tests out first);
   - P9-T08 +P8-T10 (edits `src/state/workspace`);
   - P9-T09 +P2-T07 (`app.rs` tests out first);
   - P9-T16 +P8-T01 and +P2-T07 (creates `src/state/persistence/`; `persist.rs` tests out first);
   - P9-T20 +P2-T08;
   - P10-T05, P10-T10, P10-T11, P10-T14 and P10-T17 +P2-T08 (page tests out before the page splits; see the "Order" lines in section 4);
   - P10-T15 and P10-T19 +P0-T02 (they move files that hold native tests);
   - P10-T09 +P4-T13 (writes `domain/audio/mixer.rs`).
   - P1-T13's "parallelizable with" now excludes P1-T16.
3. **Ground rules.**
   - New 2.2 rule 4 "Module layout": never both `x.rs` and `x/mod.rs`. Keep `ui/persist.rs` once P9-T04 adds `ui/persist/registry_sync.rs`. Rebase when two tasks create the first child of the same module. Later rules are renumbered.
   - New 2.4 paragraph: `domain/<context>/...` paths assume P4-T12/T13. Before then the item lives in the flat file.
4. **P8-T01:** new step 5. Re-scope rules and ratchet keys that name `controller/state` to `state` in the same PR, so they do not pass vacuously.
5. **P9-T16:** explicit PR breakdown (pure cores, then coordinator, then file split), plus the module-root note.
6. **Acceptance criteria tightened:** P1-T08, P1-T19, P4-T11, P6-T01, P8-T16, P10-T21, P11-T03.
7. **Fixes:** "Seventeen" became "Eighteen" in 1.2, and P1-T08's notice-rendering reference now points to P5-T10.

### 9.3 Remaining risks (not fixed here)

- **Line numbers drift.** Every cited range is valid at `31d512f`. After Phase 2, most ranges in later phases are stale by design (2.4 says to re-locate items by name). Agents must not apply ranges blindly.
- **Long serial chain through P8-T01.** P8-T01 has to be serialized, and P9-T16 and P9-T08 now wait on Phase 8. P9 tracks that do not touch state (P9-T02/T03, P9-T13, P9-T19) can still run earlier.
- **Ordering advice that is not a hard dependency.** Some section 4 "Order" lists put a P2 test extraction before a P4/P5/P8/P9 task on the same file without a hard edge. Examples: P5-T06 and P10-T06 on inventory, P4-T20/P4-T23 on live, P4-T06/P4-T24 on action_catalog, P8-T08 on audio_card, P5-T12/P9-T04 on persist. These are small edits, not splits, so they stay unordered. Expect rebase conflicts if they run alongside the matching P2 task.
- **Ratchet scope changes.** P2 and P10 split files, so per-file ceilings keyed by old paths (P0-T07) must move with each split. A forgotten move makes the rule fail on a "new file" (the safe direction), but reviewers should check that ceilings only go down.
- **MAINTAINER DECISION items** (8.2: TLS, symlinks, theme ids, AppState location, curation home) can still reshape P5-T05, P8-T01, P10-T21 and P11-T03.
- **No task was compile-checked.** The critic's dependency and path checks are static. "Each step stays green" depends on implementers running SV/TC/NV as section 2.1 requires.
