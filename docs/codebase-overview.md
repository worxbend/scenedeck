# Codebase Overview

## Repository Layout

```text
assets/                 GTK CSS
data/                   desktop, metainfo, and GSettings metadata
docs/                   project documentation
packaging/flatpak/      Flatpak manifest and packaging notes
resources/              embedded GTK resources and icons
src/                    application source
```

## Source Map

`src/main.rs`

Entry point. Initializes logging and starts the app.

`src/app.rs`

Creates the Tokio runtime, GTK application, event channel, controller, initial
state, and main window.

`src/app_info.rs`

Shared application ID, name, and version constants.

`src/controller/`

Application orchestration.

- `command.rs`: user and UI commands.
- `event.rs`: events sent back to GTK.
- `state.rs`: UI-facing app state.
- `app_controller.rs`: OBS session lifecycle, command handling, event loop, and
  refresh helpers.

`src/domain/`

Application data types that do not depend on GTK or OBS crate types.

- `audio.rs`: OBS audio input identity and volume state.
- `diagnostic.rs`: Doctor diagnostic model.
- `graph.rs`: scene dependency graph.
- `obs.rs`: named OBS lists such as profiles and scene collections.
- `output.rs`: stream and record status.
- `role.rs`: local scene roles.
- `scene.rs`: scene inventory.

`src/infra/`

Infrastructure helpers.

- `error.rs`: application error type.
- `logging.rs`: tracing subscriber setup.
- `channel.rs`: channel-related infrastructure.

`src/obs/`

OBS WebSocket integration.

- `client.rs`: thin async wrapper around `obws::Client`.
- `mapper.rs`: conversion from OBS response types into domain types.

No other module should import `obws` directly.

`src/services/`

Higher-level logic over domain types.

- `doctor_service.rs`: architecture diagnostics.
- `graph_service.rs`: graph edge classification.
- `scene_service.rs`: scene-related service functions.
- `audio_service.rs`: audio-related service functions.

`src/storage/`

Persistence and local machine integration.

- `config.rs`: JSON config load, save, defaults, and migrations.
- `registry.rs`: local scene role registry.
- `secret.rs`: OBS password storage through Secret Service.
- `xdg.rs`: XDG config and data path resolution.

`src/ui/`

GTK and libadwaita UI.

- `window.rs`: main shell, sidebar, header selectors, event application.
- `navigation.rs`: page switching and command dispatch helper.
- `actions.rs`: app-level actions and keyboard shortcuts.
- `pages/`: Live, Graph, Inventory, Doctor, and Settings pages.
- `widgets/`: reusable scene and audio cards.

## Assets and Resources

`assets/scenedeck.css` is loaded by the UI at startup. App artwork also lives
under `assets/`, including the installable app icon and wider logo/wordmark.

`resources/scenedeck.gresource.xml` declares embedded GTK resources.
`build.rs` runs `glib-compile-resources` and rebuilds resources when declared
files change.

## Local Data Files

The main config file is:

```text
$XDG_CONFIG_HOME/scenedeck/config.json
```

or:

```text
$HOME/.config/scenedeck/config.json
```

The scene registry file is:

```text
$XDG_CONFIG_HOME/scenedeck/registry.json
```

The OBS password is not stored in either JSON file. It is stored in the system
Secret Service keyring.
