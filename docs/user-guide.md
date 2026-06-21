# User Guide

SceneDeck controls OBS from a native Linux desktop window. It is designed for
day-to-day live operation: connect to OBS, switch prepared scenes, control audio
inputs for the active scene, and start or stop stream/record outputs.

## OBS Setup

1. Open OBS Studio.
2. Enable the OBS WebSocket server.
3. Confirm the host, port, and password.
4. Keep OBS running while using SceneDeck.

SceneDeck defaults to `127.0.0.1` on port `4455`, which matches the usual local
OBS WebSocket setup.

## First Launch

Run SceneDeck:

```sh
cargo run
```

Open Settings and configure:

- Host: OBS WebSocket host, usually `127.0.0.1`.
- Port: OBS WebSocket port, usually `4455`.
- Password: optional OBS WebSocket password. It is stored in the system keyring,
  not in the JSON config file.
- Color Scheme: System, Light, or Dark.

Use the Connect control at the bottom of the sidebar. The Live page shows a
disconnected view until OBS is connected.

## Live Page

The Live page is the main operating view.

When disconnected, it shows a placeholder message. After connection, it shows:

- Stream and record controls.
- Current program scene.
- Resizeable scene and audio panes.
- Scrollable scene cards.
- Scrollable compact audio cards.

## Mixer Page

The Mixer page is a dedicated audio control surface. It shows the same scoped
audio sources SceneDeck discovers for the current active scene, with controls
for mode, scene selection, grouping, and search.

Modes:

- Active: follows the OBS program scene.
- Selected: keeps the selected scene in the page controls.
- Pinned: keeps the selected scene as the intended stable mixer target.

The current implementation uses the active-scene audio snapshot for all modes
while the selected/pinned scene-specific OBS refresh path is being built. Source
badges identify global, active scene, nested scene, and group-derived audio.

### Scene Cards

SceneDeck shows scene cards for OBS scenes that are marked as `Primary` in the
Inventory page. Selecting a card switches the OBS program scene.

The active scene is marked as Live. Other switchable scenes are marked as Ready.

If no scene cards appear after connecting, open Inventory and assign the
`Primary` role to the scenes you want to switch from Live.

### Audio Cards

The audio section shows global OBS audio sources first, followed by
audio-capable inputs from the active scene. SceneDeck also follows enabled
nested scenes and groups when discovering active scene audio.

Each audio card contains:

- Mute/unmute button.
- Local lock button for the slider.
- Inverted vertical volume slider.
- Current dB readout.

The lock button only disables the local slider control. It does not lock the
source in OBS.

### Stream and Record

The top Live controls display streaming and recording state. Use Start Stream,
Stop Stream, Start Record, and Stop Record to control OBS outputs. When OBS
reports output state changes directly, SceneDeck updates the labels.

If recording stops and OBS returns a file path, the record status tooltip shows
that path.

## Header Selectors

After connecting to OBS, the header shows:

- Collection: switch the current OBS scene collection.
- Profile: switch the current OBS profile.

These controls are hidden while disconnected because SceneDeck does not have the
OBS lists yet.

## Inventory Page

Inventory lists OBS scenes and lets you assign local roles. Roles are stored in
SceneDeck's local registry and do not rename or modify scenes in OBS.

Roles:

- Primary: live-switchable scene shown on the Live page.
- Secondary: valid scene hidden from Live by default.
- Module: reusable nested scene, not directly switchable.
- Raw: hardware or source wrapper scene.
- Debug: temporary test scene.
- Archive: preserved but excluded from workflows.

Inventory also shows stale registry entries when a locally remembered scene no
longer exists in OBS. You can remove stale entries from this page.

The Scene Registry YAML row exports or imports the local registry as a YAML
file. This includes scene roles, tags, protection flags, and graph rule fields.
Use export to back up a scene setup or move it to another machine; use import to
replace the local registry from a YAML file.

## Graph Page

Graph shows scene dependencies from nested scene sources. It lists parent scenes
that contain other scenes and classifies the relationships against the local
role rules.

Use this page to find surprising dependencies before going live.

## Doctor Page

Doctor runs structural diagnostics over:

- OBS scene inventory.
- SceneDeck role registry.
- Scene dependency graph.

It reports errors, warnings, and informational items. Examples include
unassigned scene roles, stale registry entries, circular references, and role
relationships that invert the intended hierarchy.

## Settings Page

Settings controls appearance and OBS connection settings.

Color Scheme can follow the system preference or force light/dark mode. Themes
are light/dark-aware families, so the selected theme applies its light or dark
variant based on the effective color scheme.

Custom CSS supports separate light and dark file paths. In System mode,
SceneDeck loads the custom file matching the current libadwaita/system side.
Use Reload Custom CSS after editing a file. See
[custom-themes.md](custom-themes.md) for examples and reset instructions.

OBS host and port are stored in:

```text
$XDG_CONFIG_HOME/scenedeck/config.json
```

or, if `XDG_CONFIG_HOME` is not set:

```text
$HOME/.config/scenedeck/config.json
```

The OBS password is stored separately in the system Secret Service keyring.

## Keyboard Shortcuts

- `Ctrl+R`: reconnect to OBS.
- `Ctrl+,`: open Settings.
- `Ctrl+Q`: quit SceneDeck.

## Troubleshooting

If SceneDeck cannot connect:

- Make sure OBS is running.
- Make sure the OBS WebSocket server is enabled.
- Check that host and port match OBS.
- Re-enter the password in Settings if OBS requires one.
- Confirm that a firewall is not blocking the WebSocket port.

If the Live page has no scene cards:

- Connect to OBS.
- Open Inventory.
- Assign the `Primary` role to scenes you want on the Live page.

If the audio section is empty:

- Confirm the active OBS scene contains enabled audio-capable inputs.
- Confirm global OBS audio devices are configured if you expect them.
- Switch scenes or press refresh to force SceneDeck to re-read OBS state.

If profile or collection selectors are missing:

- Connect to OBS first.
- Check for connection errors in the sidebar status and toast messages.
