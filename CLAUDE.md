# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

SceneDeck is a Linux desktop controller for OBS Studio: Rust 2021, GTK4 +
libadwaita, Tokio, and the OBS WebSocket protocol (`obws`). Single binary crate
(`src/lib.rs` exposes only `run()`; every module is `pub(crate)`).

Toolchain is pinned in `rust-toolchain.toml` (1.97.0). Building requires GTK4
and libadwaita dev libraries plus `glib-compile-resources` (`build.rs` compiles
`resources/scenedeck.gresource.xml`).

## Commands

```sh
cargo run                                    # run the app
RUST_LOG=scenedeck=trace,obws=debug cargo run  # default is scenedeck=debug,warn
```

Validation (same set CI runs; run all four before committing):

```sh
cargo fmt --all -- --check
cargo check --workspace --all-features
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Single test / module:

```sh
cargo test --all-features name_of_test
cargo test --all-features services::doctor_service
```

Locale validation is deliberately excluded from the default suite because it
mutates the global Fluent loader — run it alone:

```sh
cargo test -- --ignored --test-threads=1
```

Tests are inline `#[cfg(test)] mod tests` blocks; there is no `tests/`
directory. CI also runs `cargo audit` and `cargo deny check advisories bans
licenses sources` (`deny.toml`).

## Architecture

Three threads of control, and the boundaries between them are the main thing to
preserve:

- **GTK main thread** owns all widget mutation and `AppState`
  (`Rc<RefCell<AppState>>`, `src/controller/state.rs`).
- **Tokio runtime** (created in `src/app.rs`, outlives the GTK loop) runs all
  OBS WebSocket work.
- **`std::sync::mpsc`** carries `AppEvent` back; `src/ui/window.rs` polls the
  receiver on a 50 ms `glib::timeout_add_local` and applies events to widgets.

Command flow:

```
GTK signal → NavigationContext::dispatch(AppCommand) → AppController::handle
          → Tokio task → ObsClient → AppEvent over mpsc → GTK poll applies it
```

Pages never call OBS or the controller directly — they hold a
`NavigationContext` (`src/ui/navigation.rs`) exposing only `switch_to_page()`
and `dispatch()`.

### Module boundaries (enforce these)

| Path | Owns |
| --- | --- |
| `src/obs/` | The **only** module allowed to import `obws`. `client.rs` wraps the client; `mapper.rs` converts OBS types into domain types before they cross out. |
| `src/controller/` | Orchestration. `app_controller.rs` routes commands and delegates to `session_controller.rs` (connect/reconnect/disconnect task ownership), `refresh_controller.rs` (refresh helpers, stats polling, OBS event stream), `output_controller.rs` (stream/record guards). Runtime deps (config, keyring) are injected via `dependencies.rs`. |
| `src/domain/` | Pure app concepts — scenes, roles, audio, graph, diagnostics, appearance, mixer selection, output/stats. No GTK, no `obws`. |
| `src/services/` | Pure/mostly-pure logic over domain snapshots — Doctor checks, graph edge classification, audio dB conversion and debouncing. |
| `src/storage/` | Local persistence: `config.json`, `registry.json`, XDG paths, Secret Service keyring. Storage structs are the serde representation and convert to domain snapshots before services use them. |
| `src/ui/` | GTK widget construction, pages, theme/CSS, actions, `background_io.rs`. |
| `src/infra/` | `error.rs` (`AppError`), `logging.rs`, `i18n.rs`. |

### Async and threading rules

- Never hold a `std::sync::Mutex` guard across `.await`. The shared client slot
  is `Arc<Mutex<Option<ObsClient>>>`; lock briefly, clone the cheap handle,
  release, then await.
- Config reads and Secret Service access from the controller run on Tokio's
  blocking pool, never on the GTK thread.
- Local persistence is **not** routed through the `AppCommand`/`AppEvent`
  stream. Config/registry/password snapshots are cached in `AppState`; GTK page
  callbacks mutate them and use `ui::background_io::run` for the blocking write,
  with the completion callback returning to GTK.
- Prefer domain types over raw strings in config where the value has a closed
  set of states (e.g. `theme_mode` serializes as a lowercase string but is
  `ThemeMode` in Rust).
- Command-scoped failures must not reuse connection-level error events — a
  failed stream/record command should not look like a disconnect.

### i18n

All user-facing strings go through `fl!(LANGUAGE_LOADER, "message-id")` with
Fluent `.ftl` files under `i18n/<locale>/scenedeck.ftl`, embedded at compile
time via `rust-embed`. Adding a string means adding it to `i18n/en/` at minimum;
missing keys in other locales fall back to English. `i18n.toml` sets the
fallback language.

### Common changes

- **New OBS capability**: wrap the `obws` call in `src/obs/client.rs`, map to
  domain types, route via `AppCommand`/`AppEvent`, update `docs/obs-websocket.md`.
- **New page**: add a `Page` variant in `controller/state.rs`, add to
  `NAV_PAGES` in `ui/window.rs`, create the module under `ui/pages/`.
- **New icon**: SVG under `resources/icons/`, register in
  `resources/scenedeck.gresource.xml`, add a `cargo:rerun-if-changed` to
  `build.rs` if needed; reference the name without `.svg`.
- **New built-in theme**: light *and* dark CSS under `resources/themes/`,
  registered as one `BuiltInTheme` in `ui/theme.rs`; keep theme CSS a narrow
  overlay on the stable classes documented in `docs/theme-css-reference.md`.

### Testing seams

Controller components accept injectable boundaries so lifecycle behavior is
testable without a running OBS: `SessionRunner` (`session_controller.rs`),
`FakeOutputCommandClient` (`output_controller.rs`), and
`ControllerDependencies` for config/password. Prefer extracting a pure decision
helper over testing through GTK callbacks.

## Repository docs

`docs/architecture.md`, `docs/codebase-overview.md`, `docs/developer-guide.md`,
`docs/obs-websocket.md`, `docs/configuration.md`, `docs/user-guide.md`,
`docs/custom-themes.md`, `docs/theme-css-reference.md`,
`docs/manual-test-plan.md`.

## Product site

`site/` is the source of <https://worxbend.github.io/scenedeck/>, deployed
verbatim by `.github/workflows/pages.yml` on every push to `main` that touches
it. Plain HTML, CSS and ES modules — no bundler, no framework, no build step at
deploy time. See `site/README.md` before changing it; the two things that are
easy to get wrong are that `site/vendor/` and `site/fonts/` are *generated*
(regenerate with `node site/tools/vendor.mjs` and `node site/tools/fonts.mjs`,
and extend the export list in `vendor.mjs` before importing anything new from
three.js or pixi.js), and that release version literals are rewritten at deploy
time by `site/tools/stamp-release.sh`.

Site copy makes product claims, so treat it like documentation: when a feature
changes, check `site/index.html` still tells the truth.

## Commits and releases

Use Conventional Commit subjects (`feat:`, `fix:`, `perf:`, `refactor:`,
`docs:`, `test:`, `build:`, `ci:`, `chore:`) — `git-cliff` (`cliff.toml`)
generates GitHub release bodies from them. Pushing a `vX.Y.Z` tag triggers the
release workflow; version-only subjects like `Release v0.2.0` are excluded from
notes automatically.
