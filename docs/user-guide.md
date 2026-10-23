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

Live puts scene switching first: one scrolling scene grid, a compact expandable
audio strip beneath it, and one output bar at the bottom. Detailed source
editing is available through **Sources** in the topbar.

**Scene previews** in Settings switches between compact tiles and larger cards
with shaded scene stills. It defaults to off conservatively. Images refresh
periodically and are not live video. A missing image leaves a usable scene tile. Images time out after two
seconds and failed scenes retry less often. A slow unanswered image can pause
other image refreshes while scene and output controls remain usable. Disabling
previews stops new capture work and discards late images; an already sent request
may finish silently. Names, Program/Preview badges and keyboard focus remain
visible over the image.

Short windows reduce card height and use a single-line scene name with the full
name available to accessibility tools and in its tooltip. Images stay contained
on the card background. Larger windows retain two-line names and 16:9 cards.

The topbar **Studio** switch is synchronized with OBS. Activating a card or its
shortcut switches Program in direct mode and stages Preview in Studio mode.
Studio mode reveals transition selection, supported duration editing and one
**Take** action above the grid. Unknown or stale mode state disables affected
actions until OBS state refreshes. Toggling mode does not itself switch scenes.

The status bar shows connection/output state on every page. In short, narrow
windows its extra performance row folds away; focus or hover the status bar to
read the same health summary. Unknown readings remain placeholders.

## Sources Page

Open **Sources** in the topbar to inspect a scene without switching Program.
Search its items, change an owned item's visibility, and open contextual input
details. Wide windows show the list and details together; compact windows offer
a Back action. Shared-source usage and partial-read warnings help identify the
scope of a change. Expand **Used in — structural paths** when you need the
shared-input paths; the section starts collapsed and long paths wrap.

Supported input details include built-in text with Save/Cancel and external-edit
conflict protection, media transport/seek, existing filters, and audio
monitoring/sync/balance/tracks. These settings affect the shared OBS input,
wherever it appears. Selecting a source is separate from switching a scene.
See the [operating guide](obs-operating-workspace.md) for scope and result rules.

## Stats Page

The Stats page is available from the sidebar.
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

The window-icon button next to the page title opens the same live view in a
separate, detached window — useful for keeping stats visible on a second
monitor while you work in the Mixer or Graph pages. Closing that window has no
effect on stats collection; reopening the button just presents it again while
it is still open.

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
Inventory page. Card activation switches Program in direct mode or stages
Preview in Studio mode. Named Program and Preview badges show observed state;
both can appear on one card.

If no scene cards appear after connecting, open Inventory and assign the
`Primary` role to the scenes you want to switch from Live.

### Scene Hotkeys

Each of the first ten scene cards carries a small badge with its digit: the
first card is `1`, the ninth is `9`, and the tenth is `0`. Pressing that digit
activates that card using the current direct/Studio mode, so keyboard order
matches the visible order. The scene shortcut hint names the configured binding.

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

Live starts with a compact audio strip; expand it for larger controls. Mixer
keeps the detailed faders and scope controls described below.

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
detailed audio card on the Mixer page. The icon appears in the card's scope
bar, next to the scope name.

Both pickers offer the same thirty icons and a "no icon" entry that clears the
choice. Icons are stored in `registry.json` alongside scene roles and accents,
so they survive restarts and travel with an exported registry.

### Stream and Record

Use Live's output bar for streaming, recording, replay buffer and virtual camera.
The contextual output menu holds pause/resume, save replay, split/chapter and
output-result details. OBS may reject a supported request when its output or
recording format is incompatible. The status bar reports stream/record state
and elapsed wall-clock time on every page.

Output Safety preferences control start/stop confirmations. Stop Stream and
Stop Recording are confirmed by default; starting either runs immediately.
Output evidence distinguishes a request or acknowledgement from a separate
observed file event. Reported paths belong to the OBS host; Copy copies text.

## Header Selectors

After connecting, the **OBS setup** header menu offers profile and scene
collection selection. It is available once OBS has supplied those lists.
The topbar also provides the OBS Studio mode switch and Sources access.

## Inventory Page

Inventory lists OBS scenes and lets you assign local roles. Roles are stored in
SceneDeck's local registry and do not rename or modify scenes in OBS.
Drag a scene by its handle to set the display order. The same persisted order is
used for scene cards on the Live page.
Scenes use a responsive list rather than a table. Names and role descriptions
wrap, and role controls move below the scene details in narrow windows. There
are no column dividers or width settings to adjust.

Choose an icon beside the scene name. Open the row options menu for accent
color, clearing the accent and keyboard move-up/move-down actions. Accents are
small secondary markers on Live cards. YAML export/import includes these local
choices. No scene is renamed or reordered in OBS by changing the registry.

Roles:

- Primary: live-switchable scene shown on the Live page by default.
- Secondary: valid scene hidden from the curated Live wall.
- Module: reusable nested scene, not directly switchable.
- Raw: hardware or source wrapper scene.
- Debug: temporary test scene.
- Archive: preserved scene excluded from active use.

Inventory also shows stale registry entries when a locally remembered scene no
longer exists in OBS. You can remove stale entries from this page.

The Scene Registry YAML row exports or imports the local registry as a YAML
file. This includes scene roles, accent colors, scene order, tags, protection
flags, and graph rule fields.
Use export to back up a scene setup or move it to another machine; use import to
replace the local registry from a YAML file.
Import reports malformed YAML without replacing the saved registry.

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

## Help Page

Help is the built-in onboarding guide. Open it from the sidebar, from the
question-mark button in the header, or with `F1`.

The page is a list of topics that expand when tapped, grouped as Getting
started, Connecting to OBS, Curating your scenes, Running a show, Checking your
setup, and Making it yours. Topics cover the five steps to a first scene switch,
connecting to OBS on another computer, hiding scenes you never cut to, scene
hotkeys, reading the volume meters, output safety, and where SceneDeck keeps its
files. Most topics end with a button that jumps straight to the page being
described.

On a first run — a machine with no SceneDeck config file yet — SceneDeck shows a
welcome dialog recommending the guide, with a button that opens it. The flag
behind it (`onboarding.welcome_shown`) is written as soon as the dialog appears,
so it interrupts at most one launch. Upgrading an existing installation never
shows it.

## Settings Page

Settings controls appearance, scene previews, shortcuts, output safety and OBS
connection settings. The Scene previews switch changes Live card sizing and
thumbnail work without restarting; it currently defaults to off.

There are 12 built-in theme families, each with light and dark variants. Motion
preferences honor the desktop animation setting as well as the app preference.

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

- `F1`: open the Help page.
- `Ctrl+R`: reconnect to OBS.
- `Ctrl+,`: open Settings.
- `Ctrl+Q`: quit SceneDeck.
- `Ctrl+1` … `Ctrl+0` on Live: activate one of the first ten cards using the
  current direct/Studio mode.
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
