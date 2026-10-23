# Developer Guide

SceneDeck is a Rust 2021 GTK4/libadwaita application. GTK owns the UI thread;
Tokio owns asynchronous OBS work; controller events bridge the two.

## Local Setup

Install:

- The Rust toolchain pinned in `rust-toolchain.toml`.
- GTK4 4.10+ development libraries.
- libadwaita 1.4+ and D-Bus development libraries.
- GLib tools, including `glib-compile-resources`.
- OBS Studio for manual integration testing.

Build and run:

```sh
cargo run
```

Run with more detailed logs:

```sh
RUST_LOG=scenedeck=trace,obws=debug cargo run
```

The default logger is `scenedeck=debug,warn` when `RUST_LOG` is not set.

## Validation

Run these before committing:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-features --locked
cargo test --workspace --all-features --locked
cargo test --workspace --all-features --locked --lib infra::i18n::tests::every_shipped_locale_parses_and_loads -- --exact --ignored --test-threads=1
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo build --locked
python3 tools/check-advisory-exceptions.py &&
  cargo audit &&
  cargo deny --locked --all-features check advisories bans licenses sources
```

Run the locale test by exact name because Fluent loader state is global. Native
GTK checks have separate display, D-Bus, and XDG requirements; run each in an
isolated process with:

```sh
python3 tools/test-native.py
```

The dependency gate includes a single user-approved, expiring maintenance
exception; see the [advisory policy](advisory-exceptions.md). Run its checker
before the scanners so the UTC expiry is enforced locally as well as in CI.

The native runner requires `xvfb`, `xauth`, `dbus-run-session`, and GNU `timeout`
(or private equivalents on `PATH`). Run the publication/CI contract fixtures with
`/usr/bin/python3 -m unittest discover -s tools -p 'test_*.py' -v` (PyYAML is
required for workflow/Snap parsing). For the static site, run `npm ci --ignore-scripts`
in `site/tools/`, then `npm test`, `npm run check`, and `npm audit`.

Native visual verification is a separate gate: inspect actual screenshots and
record gaps, rather than treating a passing build as layout evidence. See the
[manual test plan](manual-test-plan.md).

## Development Workflow

1. Identify the boundary you are changing: UI, controller, OBS adapter, domain,
   service, or storage.
2. Keep OBS protocol calls in `src/obs/`.
3. Keep command routing and async session lifecycle in `src/controller/`.
4. Keep pure rules and derived behavior in `src/domain/` or `src/services/`.
5. Keep GTK widget construction in `src/ui/`.
6. Add tests for pure domain, service, config, and storage behavior when
   possible.
7. Validate with the commands above.

Avoid holding `std::sync::Mutex` guards across `.await`. The shared OBS client
slot is intentionally locked briefly, cloned, and released before async work is
spawned.

## Common Changes

Add a Live page control:

1. Add a command to `src/controller/command.rs`.
2. Handle it in `src/controller/app_controller.rs`.
3. Add OBS calls in `src/obs/client.rs` if needed.
4. Add state or events in `src/controller/event.rs` and
   `src/controller/state.rs` if the UI needs updates.
5. Wire GTK widgets in `src/ui/pages/live.rs` or a widget under
   `src/ui/widgets/`.

Add a new OBS capability:

1. Wrap the `obws` call in `src/obs/client.rs`.
2. Convert external OBS types into domain types before returning.
3. Route the action through `AppCommand` or the state through `AppEvent`.
4. Update docs in `docs/obs-websocket.md`.

Add a new page:

1. Add a page variant in `src/controller/state.rs`.
2. Add it to `NAV_PAGES` in `src/ui/sidebar.rs`, or provide deliberate topbar
   access as Sources does.
3. Create a page module in `src/ui/pages/`.
4. Add the module to `src/ui/pages/mod.rs`.
5. Wire refresh behavior if the page depends on app state.

Add a symbolic icon:

1. Place the SVG under `resources/icons/`.
2. Add it to `resources/scenedeck.gresource.xml`.
3. Add a `cargo:rerun-if-changed` line in `build.rs` if needed.
4. Use the icon name without `.svg` in GTK.

Add a built-in theme:

1. Add both light and dark CSS files under `resources/themes/`.
2. Register the theme family in `src/ui/theme.rs` with one `BuiltInTheme`
   entry that points to both files.
3. Keep CSS as a narrow overlay on stable SceneDeck classes.
4. Update `docs/custom-themes.md` and `docs/theme-css-reference.md` if new
   stable classes or semantics are introduced.

## Manual OBS Test Checklist

Use an isolated OBS instance with disposable configuration and local output
paths. Preserve real OBS profiles/collections. Record test endpoint/version,
request traces and screenshots in the current verification index.

- Connect, disconnect, reconnect and change collection with reads pending.
- Exercise direct/Studio card and shortcut behavior, external Studio changes,
  adjustable/fixed transitions and Take.
- Check preview off/on, pending cancellation, hidden/disconnected capture stops,
  session/collection races and the six-/twenty-scene performance budgets.
- Exercise compact audio and Active/Selected/Pinned Mixer behavior.
- Run output confirmations and supported record/replay/camera actions; distinguish
  configuration rejection from success and acknowledgement from file evidence.
- Verify Sources visibility/media/filter/audio/text actions and conflict handling.
- Check Inventory role readability, header/numeric resizing, persistence/reset,
  ordering and registry YAML.
- Inspect all theme variants, custom CSS/motion, compact/enlarged/localized
  layouts, keyboard focus and accessibility. Record Orca speech separately.
- Hash preserved workflow/config files; confirm retired features perform no
  routine workflow I/O. Historical research/reports remain context only.

## Packaging

Flatpak packaging lives in `packaging/flatpak/`. See
[../packaging/flatpak/README.md](../packaging/flatpak/README.md).

## Release Notes

Pushing a semantic version tag such as `v0.2.0` runs the Linux release workflow.
After the packages build, `git-cliff` generates the GitHub release body from
commits since the previous version tag using `cliff.toml`.

Use Conventional Commit subjects so changes land in the intended section:

- `feat: add a user-visible capability`
- `fix: correct broken behavior`
- `perf: reduce runtime overhead`
- `refactor: reorganize internals`
- `docs: update documentation`
- `test: expand automated coverage`
- `build:`, `ci:`, or `chore:` for maintenance

Older imperative subjects are also categorized. Version-only subjects such as
`Bump version to 0.2.0` and `Release v0.2.0` are excluded automatically. The
release job fails before publication if note generation produces an empty file.
