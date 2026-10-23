# SceneDeck

![SceneDeck logo](docs/scenedeck-logo.svg)

**[worxbend.github.io/scenedeck](https://worxbend.github.io/scenedeck/)** — product
site, feature tour, and every install option.
Also available at **[obs.worxbend.com](https://obs.worxbend.com/)**.

[![Get it from the Snap Store](https://snapcraft.io/en/dark/install.svg)](https://snapcraft.io/scenedeck)

SceneDeck is a Linux desktop controller for OBS Studio. It is built with Rust,
GTK4, libadwaita, Tokio, and the OBS WebSocket protocol.

The app is focused on fast live operation: switch OBS scenes, control scene
audio, start or stop streaming and recording, inspect scene dependencies, and
keep local metadata about how scenes are used.

## Current Capabilities

- **Live scene wall:** compact scene tiles or larger shaded scene previews,
  Program/Preview labels, keyboard switching, a synchronized topbar Studio mode
  switch, and a transition/Take row while Studio mode is enabled.
- **Audio and outputs:** an expandable audio strip below the scenes, a dedicated
  Mixer with Active/Selected/Pinned modes, and one Live output bar for streaming,
  recording, replay and virtual camera. Pause, save, split, chapters and OBS-host
  output evidence are available through its contextual menu where supported.
- **Sources:** a focused scene-item list and input details, opened from the
  topbar. Inspect visibility, shared-source usage, media, built-in text, filters
  and advanced audio without switching Program.
- **Organization and telemetry:** readable, resizable Inventory columns, local
  roles/icons/accents, keyboard or drag ordering, registry YAML import/export,
  nested-scene Graph, structural Doctor checks, and bounded Stats history.
- **Desktop appearance:** 12 light/dark theme families, custom CSS, motion
  preferences, eight Fluent catalogs, XDG settings and Secret Service passwords.

Scene previews are controlled in Settings and default to off conservatively.
They are periodically refreshed still images, not live video. Current validation and screenshots are tracked in the
[redesign evidence](docs/verification/redesign-2026-10-19/README.md).

All remote control uses native OBS WebSocket. There are no chat or third-party
service integrations. See the [operating workspace guide](docs/obs-operating-workspace.md)
for capabilities, freshness rules and feature limits.

## Requirements

- Linux desktop with GTK 4.10 or newer and libadwaita 1.4 or newer.
- OBS Studio with WebSocket enabled.
- Rust stable toolchain for development builds.
- `glib-compile-resources`, usually provided by GLib development packages.
- Secret Service compatible keyring for storing OBS passwords.

OBS Studio 28 and newer includes the WebSocket server. The default SceneDeck
connection target is `127.0.0.1:4455`.

## Quick Start

```sh
cargo run
```

In OBS, enable the WebSocket server and confirm the port and password. In
SceneDeck, open Settings, set the OBS host, port, and optional password, then
use the Connect control at the bottom of the sidebar.

The Live page stays in a disconnected view until OBS is connected. Profile and
scene collection selectors are available under **OBS setup** in the header
after SceneDeck receives those lists from OBS.

## Using SceneDeck

Use the sidebar for Live, Mixer, Stats, Graph, Inventory, Doctor, Settings and
Help. Open **Sources** from the topbar for detailed input control. The Studio
switch also lives in the topbar and follows changes made in OBS.

On Live, activating a scene changes Program in direct mode and stages Preview
in Studio mode. Transition selection, supported duration editing and Take appear
only in Studio mode. The scene grid scrolls independently of the audio and output
controls. Inventory stores local scene roles, order, icons and accent colors;
Doctor checks structural scene/role/dependency issues.

SceneDeck does not require you to rename or restructure OBS scenes. The app
keeps its own local registry for role metadata, then uses that registry to
decide which scenes are shown as Live page cards and how scene dependencies are
classified.

## User Guide

See [docs/obs-operating-workspace.md](docs/obs-operating-workspace.md) for the
OBS operating workspace, and [docs/user-guide.md](docs/user-guide.md) for setup, page-by-page usage,
scene roles, audio behavior, and troubleshooting.

## Developer Guide

Start with [docs/developer-guide.md](docs/developer-guide.md). More detailed
developer docs are available in:

- [docs/architecture.md](docs/architecture.md)
- [docs/codebase-overview.md](docs/codebase-overview.md)
- [docs/obs-websocket.md](docs/obs-websocket.md)
- [docs/configuration.md](docs/configuration.md)
- [docs/custom-themes.md](docs/custom-themes.md)
- [docs/theme-css-reference.md](docs/theme-css-reference.md)
- [packaging/flatpak/README.md](packaging/flatpak/README.md)

The core development rule is to keep protocol access, orchestration, domain
rules, storage, and GTK widgets in their existing boundaries. OBS calls belong
in `src/obs/`, async command routing belongs in `src/controller/`, pure app
concepts belong in `src/domain/` and `src/services/`, persisted files belong in
`src/storage/`, and widgets belong in `src/ui/`.

## Validation

Use these checks before committing changes:

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
python3 tools/test-native.py
```

The dependency gate includes a single user-approved, expiring maintenance
exception; see the [advisory policy](docs/advisory-exceptions.md). Run its checker
before the scanners so the UTC expiry is enforced locally as well as in CI.

The native runner requires Xvfb, xauth, a session D-Bus launcher and GNU timeout;
it gives each GTK test a separate process/display and disposable XDG paths.
The locale command selects only the Fluent validation test, not every ignored
display test. See the developer guide for site and publication-contract checks.

## License

[MIT](LICENSE)
