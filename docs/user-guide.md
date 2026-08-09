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

The bottom status bar shows OBS connection state, stream and record state, and
performance counters — FPS, dropped frames, CPU, and bitrate — on every page.
FPS and dropped frames are always shown while connected; the dropped-frame
counter is highlighted only once frames are actually being lost.

## Stats Page

The Stats page is the last entry in the sidebar and is always available.
Process metrics such as FPS, frame render time, and CPU are reported by OBS
whether or not you are streaming; the stream-only values — congestion, stream
frame drops, and bitrate — stay blank until the stream output is running. While
disconnected the whole page shows placeholders.

It shows, refreshed once a second:

- Gauges for FPS, average frame render time, dropped-frame percentage, and
  network congestion. Gauges turn amber and then red as a value degrades:
  dropped frames warn at 1% and turn critical at 5%, congestion at 30% and 60%,
  frame render time at half and 80% of the frame budget, and FPS when it falls
  below 95% and 80% of the highest rate seen this session.
- Trend charts for frames per second and average frame render time.
- Bar charts of output frames skipped and render frames missed per sample, which
  show *when* frames were lost rather than the running total.
- Counter cards for render, output, and stream frame totals, average frame
  render time, OBS CPU and memory usage, and stream bitrate.

Charts hold roughly the last two minutes. SceneDeck collects these samples for
the whole time it is connected, not only while the page is open, so opening
Stats mid-stream shows the preceding minutes rather than starting from empty.

Frame counters come from OBS and are cumulative since OBS (or the stream output)
started, so restarting a stream resets them.

The layout adapts to the window width: gauges, charts, and counter cards each
reflow to fewer columns as the window narrows, ending in a single column so the
page stays readable in a narrow window without horizontal scrolling.

Because obs-websocket has no push notification for statistics, these values are
polled from OBS rather than pushed by it.

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

### Scene Hotkeys

Each of the first ten scene cards carries a small badge with its digit: the
first card is `1`, the ninth is `9`, and the tenth is `0`. Pressing that digit
switches to the card, so the keyboard order is exactly the order you see. The
caption to the right of the Scenes heading names the binding in full, for
example `Press Ctrl+1 … 0`.

Slot numbers follow the scene order saved in Inventory, so reordering cards
there also reorders the shortcuts. Cards past the tenth have no shortcut.

Settings → Scene Hotkeys chooses how the digit is pressed:

- A modifier plus the digit: `Ctrl`, `Alt`, `Shift`, `Super`, `Ctrl+Alt`, or
  `Ctrl+Shift`. `Ctrl` is the default because it cannot fire by accident while
  typing.
- The bare digit, with nothing else held.
- A leader key, vim style: press the leader, release it, then press the digit.
  The leader can be Space, Comma, Semicolon, Backslash, or Backtick, and it
  stays armed for a configurable timeout — 1.5 seconds by default. While it is
  armed the caption reads `Leader armed`. Escape, or any other key, cancels it.

Shortcuts act only on the Live page, and the ones that need no modifier stand
down while a text field has focus. Pressing a digit with no scene behind it
says so in the caption for a couple of seconds instead of switching anything.

### Audio Cards

The audio section shows global OBS audio sources first, followed by
audio-capable inputs from the active scene. SceneDeck also follows enabled
nested scenes and groups when discovering active scene audio.

Each audio card is laid out like a mixer strip in OBS:

- A coloured scope bar naming where the source comes from, carrying the source's
  icon if one has been chosen.
- The source name and its current dB readout.
- An inverted vertical fader.
- A live volume meter, one bar per channel, with a decibel ruler beside it.
- Mute and local lock buttons.
- An overflow button that opens the icon chooser.

The lock button only disables the local slider control. It does not lock the
source in OBS.

### Reading the Volume Meter

The meter beside each fader shows what OBS is actually hearing, on the same
-60 dB to 0 dB scale as the fader and the printed ruler between them.

**Zones.** The colour of a lit segment says how close that level is to
distorting, using the same thresholds as OBS Studio:

- **Red**, above -9 dB: close to clipping. Speech may touch the bottom of this
  zone; nothing should sit in it.
- **Yellow**, -20 dB to -9 dB: speech belongs in the upper part of this zone,
  with game or content audio lower.
- **Green**, below -20 dB: background music, alerts, and anything else that has
  to stay under the voice.

Clipping is the distortion you hear when a signal is louder than the equipment
carrying it can reproduce. Watch the meter rather than the fader: two sources
whose faders match can be very different in loudness.

**Channels.** Each channel gets its own column, in OBS's order.

- One column: a mono source. Viewers hear it in both ears.
- Two columns: stereo, left then right. If only the left column moves, viewers
  only hear that source on the left. OBS's Advanced Audio Properties has a
  *Downmix to Mono* option for sources that should be centred.
- Three or more: surround, ordered front left, front right, front centre, LFE,
  rear left, rear right, side left, side right. OBS downmixes surround sources
  to stereo unless told otherwise, and only meters as many channels as the
  channel count set in OBS's own Audio settings.

**Indicators.** Every bar carries four readings:

- **The bright fill** is the peak program level. It jumps up instantly and falls
  off gradually, so a short transient stays visible long enough to see. The dim
  band behind it is the part of the range the level has not reached.
- **The line above it** is the loudest peak of the last twenty seconds. It keeps
  its colour, which is the quickest way to catch clipping you missed.
- **The notch across the fill** is loudness — sound pressure, measured over
  roughly 300 ms, and closer to how loud a viewer will judge the source than the
  peak is.
- **The square at the base** is the level arriving from the device, before the
  fader. It shows whether the source itself is too hot, which no amount of fader
  movement will fix.

A muted input still shows its base dot, because the device keeps sending audio
whether or not OBS is passing it on.

**Setting levels.** Work from the source outwards: the device's own gain knob
or driver settings first, then your desktop mixer, and only then the SceneDeck
fader. Record a short test and listen back before going live. If a source needs
more than unity gain, OBS's Advanced Audio Properties accepts percentages above
100%.

### Scene and Source Icons

Scenes and audio sources can each carry an icon, which makes a wall of similar
cards scannable at a glance.

Scene icons are chosen in Inventory, from the picker at the left of each scene
row. The icon then appears beside the scene name on its Live card.

Audio source icons are chosen from the overflow button at the bottom of any
audio card, on Live or on the Mixer page. The icon appears in the card's scope
bar, next to the scope name.

Both pickers offer the same thirty icons and a "no icon" entry that clears the
choice. Icons are stored in `registry.json` alongside scene roles and accents,
so they survive restarts and travel with an exported registry.

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
variant based on the effective color scheme. The `OBS` family mirrors OBS
Studio's own default look, for a control surface that matches the app it drives.

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

Scene Hotkeys controls the keyboard shortcuts that switch Live scene cards: an
on/off switch, the key combination, and — for the leader style — the leader key
and how long it stays armed. The Current Bindings row previews the result.
Changes take effect on the next key press; scene-card badges update the next
time the Live page is shown. See [Scene Hotkeys](#scene-hotkeys) above.

## Keyboard Shortcuts

- `Ctrl+R`: reconnect to OBS.
- `Ctrl+,`: open Settings.
- `Ctrl+Q`: quit SceneDeck.
- `Ctrl+1` … `Ctrl+0` on the Live page: switch to the first ten scene cards.
  The combination is configurable — see [Scene Hotkeys](#scene-hotkeys).

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
