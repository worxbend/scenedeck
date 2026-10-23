# SceneDeck Documentation

This directory contains the detailed SceneDeck documentation.

## For Users

- [OBS operating workspace](obs-operating-workspace.md): native source/output
  controls, the Live scene wall, Studio Mode, Sources and output evidence.
- [User guide](user-guide.md): OBS setup, connection workflow, Live page,
  Sources, Inventory, Graph, Doctor, Settings, and troubleshooting.
- [Configuration](configuration.md): config files, registry files, keyring
  storage, and local scene roles.
- [Custom themes](custom-themes.md): built-in theme families, custom light/dark
  CSS files, fallback behavior, and reset instructions.
- [Theme CSS reference](theme-css-reference.md): stable CSS classes and safe
  theming patterns.

## For Developers

- [Developer guide](developer-guide.md): local setup, build commands,
  validation, logging, and contribution workflow.
- [Architecture overview](architecture.md): runtime model, controller flow,
  OBS boundary, state ownership, and threading rules.
- [Codebase overview](codebase-overview.md): directory-by-directory map of the
  repository and where common changes belong.
- [OBS WebSocket integration](obs-websocket.md): what SceneDeck currently reads,
  writes, and listens to over the OBS WebSocket protocol.

## Packaging

- [Flatpak packaging](../packaging/flatpak/README.md): GNOME runtime Flatpak
  build and install workflow.

## Product site asset follow-up

The product-site copy now describes the redesign, and its gallery explicitly
labels the older release captures. A bounded follow-up remains: replace the
three gallery captures (Live, Inventory, Settings) and update the interactive
HTML deck demo to match the final native layout. Existing assets are retained;
they are not evidence for the redesign acceptance matrix.
