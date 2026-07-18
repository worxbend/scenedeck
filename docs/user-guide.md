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
audio source controls as Live, with mode, scene selection, grouping, and search.

Modes:

- Active: follows the OBS program scene.
- Selected: loads audio for the selected scene without following OBS program
  scene changes.
- Pinned: keeps the selected scene as the stable mixer target.

Source badges identify global, active scene, nested scene, and group-derived
audio.

SceneDeck saves the Mixer mode, selected scene, pinned scene, and grouping
preference in the local config file. The search field is session-only.

### Scene Cards

SceneDeck shows scene cards for OBS scenes that are marked as `Primary` in the
Inventory page. Selecting a card switches the OBS program scene.

The current program scene is marked as Active. Other switchable scenes are marked as Ready.

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

Use the Start/Stop Stream and Start/Stop Recording buttons at the bottom of the
sidebar to control OBS outputs. The status bar shows output state and elapsed
time. When OBS reports a state change, SceneDeck updates both surfaces.

The sidebar buttons ask for confirmation when their Output Safety toggles are
enabled in Settings. By default, SceneDeck confirms Stop Stream and Stop
Recording, while starting either output runs immediately.

## Header Selectors

After connecting to OBS, the header shows:

- Collection: switch the current OBS scene collection.
- Profile: switch the current OBS profile.

These controls are hidden while disconnected because SceneDeck does not have the
OBS lists yet.

## Inventory Page

Inventory lists OBS scenes and lets you assign local roles. Roles are stored in
SceneDeck's local registry and do not rename or modify scenes in OBS.
Drag a scene by its handle to set the display order. The same persisted order is
used for scene cards on the Live page.
Assigned scenes also have an optional accent-color picker and clear button.
SceneDeck uses that accent to highlight the scene's Live card with a fixed 50%
alpha; picker alpha values are not stored.
If the registry file cannot be loaded, Inventory shows a warning row and falls
back to unassigned roles until the file is fixed and the page is refreshed.

Roles:

- Primary: live-switchable scene shown on the Live page by default.
- Secondary: valid scene hidden from Live by default unless Live config includes it.
- Module: reusable nested scene, not directly switchable.
- Raw: hardware or source wrapper scene.
- Debug: temporary test scene.
- Archive: preserved but excluded from workflows.

Inventory also shows stale registry entries when a locally remembered scene no
longer exists in OBS. You can remove stale entries from this page.

The Scene Registry YAML row exports or imports the local registry as a YAML
file. This includes scene roles, accent colors, scene order, tags, protection
flags, and graph rule fields.
Use export to back up a scene setup or move it to another machine; use import to
replace the local registry from a YAML file.
If the local registry file is invalid, export reports the parse error instead
of producing a default registry backup.

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

Output Safety controls whether SceneDeck asks before starting or stopping OBS
streaming and recording. The four toggles are Confirm Start Stream, Confirm Stop
Stream, Confirm Start Recording, and Confirm Stop Recording. Changes apply to
Live page output buttons immediately and are stored in the local config file.

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
