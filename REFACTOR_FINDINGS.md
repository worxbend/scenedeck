# Refactor Findings

## Repository map
- `src/main.rs:5-7` calls `scenedeck::run()`.
- `src/lib.rs:22-29` exposes crate startup and keeps module layers crate-private.
- `src/app.rs:21-64` creates the Tokio runtime, event channel, GTK application, initializes state, and builds the top-level window.
- `src/app.rs:42-54` wires `app.connect_activate` to one-time UI bootstrap.
- `src/ui/window.rs:36-80` builds shell widgets (`Stack`, `NavigationSplitView`, header, sidebar) and mounts each page via `add_titled`.
- `src/ui/window.rs:65-74` mounts live/graph/inventory/doctor/settings page builders from `src/ui/pages/*`.
- `src/ui/window.rs:133-157` polls `mpsc::Receiver<AppEvent>` every 50 ms with `glib::timeout_add_local`.
- `src/ui/window.rs:219-347` applies each `AppEvent` to UI state/widg​ets.
- `src/ui/navigation.rs:23-45` centralizes command dispatch/page switching via `NavigationContext`.
- `src/controller/app_controller.rs:45-193` owns async OBS orchestration and routes events through `AppEvent`.
- `src/controller/command.rs:8-42` and `src/controller/event.rs:20-66` are typed command/event contracts between UI and controller.
- `src/controller/state.rs:2-110` stores app state as `Rc<RefCell<AppState>>` (`src/app.rs:73`) shared into GTK side modules.
- `src/obs/client.rs` and `src/obs/mapper.rs` isolate `obws` protocol types and conversion.
- `src/services/scene_service.rs:31-71`, `src/services/graph_service.rs`, `src/services/doctor_service.rs` keep domain rules/services off GTK callbacks.
- `src/storage/config.rs`, `src/storage/registry.rs`, `src/storage/secret.rs`, and `src/storage/xdg.rs` own persistence and local filesystem paths.
- `src/ui/pages/mod.rs:6-10`, `src/ui/pages/live.rs:44-56`, and `src/ui/widgets/{audio_card.rs,scene_card.rs}` define page and reusable widget composition.
- `src/ui/actions.rs:13-89` centralizes app action and keyboard-shortcut registration.
- `resources/scenedeck.gresource.xml:1-6` plus `build.rs:11-30` define resource compilation and embed icons.
- `build.rs:15-22` runs `glib-compile-resources` as a compile-time resource step.
- `assets/scenedeck.css:1` is loaded in `src/ui/mod.rs:29-42`.
- `README.md:96-105` and `docs/developer-guide.md:34-39` define validation commands (`cargo fmt --all -- --check`, `cargo check --workspace --all-features`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`).
- Unit/integration tests are in-module with `#[cfg(test)]` across storage, service, domain, and UI files (for example `src/storage/config.rs:326-483`, `src/ui/pages/graph.rs:690+`, `src/ui/pages/settings.rs:295-361`, `src/services/scene_service.rs:73-200`, and `src/obs/mapper.rs:137+`).
- No root-level `tests/` or `examples/` directories are present; no workspace (single crate in `Cargo.toml` and `Cargo.lock` present).
- Missing phase-1 referenced docs (`PLAN.md`, `MEMORY.md`, `AGENT_LOG.md`, `CONTRIBUTING.md`) were not found in this repository.

## Finding 1: Duplicate action registration boilerplate in `src/ui/actions.rs`

* Rank: duplication
* Severity: medium
* Smell: Duplicate Code
* Evidence:
  * `src/ui/actions.rs:22-60`
* Problem:
  * `install` repeats the same sequence for each action (`SimpleAction` creation, `connect_activate`, registration, optional accelerator binding).
* Candidate refactoring:
  * Extract Function (`https://refactoring.guru/extract-function`)
* Rust/GTK shape:
  * helper `register_simple_action`
* Expected blast radius:
  * `src/ui/actions.rs`
* Public API impact:
  * none
* UI behavior impact:
  * should be identical
* Test impact:
  * existing tests likely unaffected
  * no new tests required for behavior unchanged path
* Risk:
  * low
* Decision:
  * fix now

## Finding 2: Large event-dispatch `match` body in GTK event adapter

* Rank: correctness
* Severity: medium
* Smell: Large Method
* Evidence:
  * `src/ui/window.rs:219-347`
* Problem:
  * A single match arm block both mutates state and performs detailed widget updates for every event, increasing coupling and lowering local reasoning for event-specific behavior.
* Candidate refactoring:
  * Extract Function (`https://refactoring.guru/extract-function`) and Move Method (`https://refactoring.guru/move-method`)
* Rust/GTK shape:
  * event-specific helper methods under `ui::window`
* Expected blast radius:
  * `src/ui/window.rs`
* Public API impact:
  * none
* UI behavior impact:
  * potential
* Test impact:
  * add focused event-specific tests if behavior changes
* Risk:
  * medium
* Decision:
* defer

## Finding 3: Repeated callback setup and state-read/write in widget builders

* Rank: consistency
* Severity: medium
* Smell: Message Chains / Data Clumps
* Evidence:
  * `src/ui/pages/live.rs:98-111`
  * `src/ui/pages/live.rs:127-133`
  * `src/ui/pages/inventory.rs:199-227`
* Problem:
  * Several UI callbacks mirror the same pattern: read mutable state, dispatch command, and perform direct widget updates inline.
* Candidate refactoring:
  * Extract Command Handler (`https://refactoring.guru/command`) and Encapsulate Collection (`https://refactoring.guru/extract-class`)
* Rust/GTK shape:
  * small handler functions in UI page modules
* Expected blast radius:
  * `src/ui/pages/live.rs`, `src/ui/pages/inventory.rs`
* Public API impact:
  * none
* UI behavior impact:
  * should be identical
* Test impact:
  * no direct unit coverage currently; manual checks recommended
* Risk:
  * medium
* Decision:
* fix now

## Finding 4: Inline persistence and domain logic in settings UI

* Rank: correctness
* Severity: low
* Smell: Inappropriate Intimacy
* Evidence:
  * `src/ui/pages/settings.rs:124-162`
  * `src/ui/pages/settings.rs:143-156`
* Problem:
  * Settings UI callback composes parsing, config mutation, and keyring mutation together.
* Candidate refactoring:
  * Extract Class / move logic to service (`https://refactoring.guru/extract-class`)
* Rust/GTK shape:
  * dedicated settings application handler functions
* Expected blast radius:
  * `src/ui/pages/settings.rs`, `src/storage/config.rs`, `src/storage/secret.rs`
* Public API impact:
  * none
* UI behavior impact:
  * should be identical
* Test impact:
  * existing storage/service tests can be extended with handler-level unit tests
* Risk:
  * low
* Decision:
  * fix now

## Things intentionally not changed
- No refactor was made in OBS adapter boundaries (`src/obs/*`) because `docs/architecture.md` identifies them as protocol isolation boundaries.
- No refactor was made to public runtime interfaces in `src/lib.rs` because it intentionally exports only `run` and keeps internals crate-private.
- No resource/GResource or GTK styling churn was made in this pass (`resources/scenedeck.gresource.xml`, `resources/icons/...`, `assets/scenedeck.css`).
- No speculative abstractions were added beyond a direct helper function used by existing call sites in `src/ui/actions.rs`.

## Implementation plan

### Step 1: Extract shared action registration helper

* Finding addressed:
  * Finding 1: Duplicate action registration boilerplate in `src/ui/actions.rs`
* Smell removed:
  * Duplicate Code
* Technique/pattern:
  * Extract Function (`https://refactoring.guru/extract-function`)
* Goal:
  * centralize repeated `gio::SimpleAction` wiring into one utility and keep action behavior one-line in `install`.
* Files touched:
  * `src/ui/actions.rs`
* Change type:
  * internal only
* Public API impact:
  * none
* UI behavior impact:
  * should be identical
* Test strategy:
  * compile command: `cargo check --workspace --all-features`
  * test command: `cargo test --workspace --all-features`
  * UI smoke/manual check: trigger keyboard shortcuts (`Ctrl+Q`, `Ctrl+R`, `Ctrl+,`) and action menu entries; verify they still dispatch connect/reconnect/settings/about behavior.
* Stop condition:
  * if behavior changes in action name/shortcut mapping or command routing are detected.
* Commit message:
  * `refactor: extract shared action-registration helper`

### Step 2: Extract settings persistence handlers out of callbacks

* Finding addressed:
  * Finding 4: Inline persistence and domain logic in settings UI
* Smell removed:
  * Inappropriate Intimacy
* Technique/pattern:
  * Extract Function (`https://refactoring.guru/extract-function`)
* Goal:
  * move config/keyring write orchestration into dedicated functions and keep signal handlers as dispatch-only.
* Files touched:
  * `src/ui/pages/settings.rs`
* Change type:
  * internal only
* Public API impact:
  * none
* UI behavior impact:
  * none
* Test strategy:
  * compile command: `cargo check --workspace --all-features`
  * test command: `cargo test --workspace --all-features`
  * manual check: edit Settings host/port/password fields and confirm status row messages and persistence semantics remain unchanged.
* Stop condition:
  * if status messaging or keyring/storage error behavior changes from current UX.
* Commit message:
  * `refactor: extract settings persistence handlers`

### Step 3: Extract live/inventory callback handlers

* Finding addressed:
  * Finding 3: Repeated callback setup and state-read/write in widget builders
* Smell removed:
  * Message Chains / Data Clumps
* Technique/pattern:
  * Extract Function (`https://refactoring.guru/extract-function`) and Extract Command (`https://refactoring.guru/command`)
* Goal:
  * keep signal wiring stable while moving scene role/switch handlers and stale-entry actions into focused UI page helpers.
* Files touched:
  * `src/ui/pages/live.rs`
  * `src/ui/pages/inventory.rs`
* Change type:
  * internal only
* Public API impact:
  * none
* UI behavior impact:
  * should be identical
* Test strategy:
  * compile command: `cargo check --workspace --all-features`
  * test command: `cargo test --workspace --all-features`
  * manual check: Stream/Recording toggles and Inventory role/editor/removal actions should keep previous UX and error handling.
* Stop condition:
  * if Stream/Recording dispatches or Inventory registry writes diverge from current behavior.
* Commit message:
  * `refactor: extract live/inventory callback handlers`

### Step 4: Keep dense event dispatcher unchanged for now

* Finding addressed:
  * Finding 2: Large event-dispatch `match` body in GTK event adapter
* Smell removed:
  * none (deferred)
* Technique/pattern:
  * none (defer)
* Goal:
  * avoid risky behavior churn while preserving known stable event ordering.
* Files touched:
  * none
* Change type:
  * internal only (risk management)
* Public API impact:
  * none
* UI behavior impact:
  * none
* Test strategy:
  * no structural changes in this pass
* Stop condition:
  * regression risk or explicit request for this split
* Commit message:
  * not applicable

### Completed steps

- `src/ui/actions.rs`: implemented `register_simple_action` and refactored `install` to declarative action registration.
- This directly addresses Finding 1 with no public API changes.
- `src/ui/pages/settings.rs`: extracted host/port parse+config persistence and OBS password persistence into dedicated helper functions.
- This directly addresses Finding 4 with no external behavior change and no new dependencies.
- `src/ui/pages/live.rs`: extracted output toggle click logic into dedicated handlers.
- `src/ui/pages/inventory.rs`: extracted scene-role update/removal/YAML-action callback handlers into dedicated helper functions.
- This directly addresses Finding 3 with no public API changes.

### Skipped / risky steps

- Not extracting `AppEvent` handling from `src/ui/window.rs` yet (Finding 2): high local regression risk in state and UI ordering.
