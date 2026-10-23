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

- [UI/UX development plan](../PLAN.md): current scene-wall redesign, optional
  card previews, Inventory resizing, theme migration, feature removals and
  visual/implementation acceptance criteria.
- [Live design brief](ideas/live-scene-wall.md): confirmed user decisions and
  card/Inventory visual references.
- [Current redesign verification](verification/redesign-2026-10-19/README.md):
  actual screenshots, checks, measurements and remaining limitations.
- [Developer guide](developer-guide.md): local setup, build commands,
  validation, logging, and contribution workflow.
- [Architecture overview](architecture.md): runtime model, controller flow,
  OBS boundary, state ownership, and threading rules.
- [Codebase overview](codebase-overview.md): directory-by-directory map of the
  repository and where common changes belong.
- [OBS WebSocket integration](obs-websocket.md): what SceneDeck currently reads,
  writes, and listens to over the OBS WebSocket protocol.
- [OBS-only feature and UI/UX research](obs-only-feature-research.md): SceneDeck
  inventory, OBS Blade comparison, proposed desktop interactions and priorities.
- [Execution checklist](../tasks/todo.md): completed 31-task redesign and visual
  acceptance evidence; earlier implementation/research checklists remain archived.
- [Research review](obs-only-research-review.md): operational risks and scope
  review; proposals are not implemented changes.
- [OBS controller feature opportunities](obs-controller-feature-opportunities.md):
  second-pass research with pinned competitor sources, seven feature directions,
  differentiated workflow recommendations and a proposed delivery sequence.

## Packaging

- [Flatpak packaging](../packaging/flatpak/README.md): GNOME runtime Flatpak
  build and install workflow.

## Product site asset follow-up

The product-site copy now describes the redesign, and its gallery explicitly
labels the older release captures. A bounded follow-up remains: replace the
three gallery captures (Live, Inventory, Settings) and update the interactive
HTML deck demo to match the final native layout. Existing assets are retained;
they are not evidence for the redesign acceptance matrix.
