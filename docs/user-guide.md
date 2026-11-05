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

## Saved OBS Connections

In Settings, use **Add connection** to create a separate setup, such as
“Streaming PC” or “Event laptop”. Edit its name, host and port with their Apply
controls, then save its password separately. Each connection has its own keyring
entry. New setups begin without a password.

Choose a setup from **Saved connection**, then use **Connect saved setup** when
ready. Choosing a setup alone keeps the current OBS session connected. The
status row shows the actual connected setup and endpoint, which can differ from
the setup selected for the next connection. Connection-specific scene curation
loads after OBS reports its collection. Renaming a setup keeps its curation and
credential identity.

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

Media transport shows the last observed position, duration and remaining time,
with a progress bar when both timing values are available. Times follow OBS
readbacks, including when paused; SceneDeck does not advance a guessed playback
clock. Unknown or indefinite duration has no progress percentage or countdown.
Stale workspace data shows unavailable timing. Enter a position in seconds and
press **Seek** to move playback; polling preserves a position you are editing.

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

The first ten switchable scenes in the saved Inventory order receive digits
`1` through `9`, then `0`. Each visible card shows its assigned digit. Pressing
it activates that scene using the current direct/Studio mode. The scene shortcut
hint names the configured binding.

Search, saved decks and showing favorites first do not renumber these slots.
A shortcut can still activate a scene hidden by the current filter. Reordering
scenes in the unfiltered Inventory changes slot assignments; scenes after the
tenth slot have no digit shortcut. Text-entry focus suppresses scene shortcuts.

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

Live starts with a compact audio strip; expand it for larger controls. In a
short window (under 700 px) the strip shows only its heading until you expand
it, so the scene grid keeps its room. Mixer keeps the detailed faders and scope
controls described below.

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
choice. Icons are stored in the current show workspace alongside scene roles and accents,
so they survive restarts and travel with an exported registry.

### Collection-specific show workspaces

SceneDeck automatically keeps curation separate for each OBS connection endpoint
and scene collection. Switching the collection in OBS or SceneDeck loads its
own roles, order, colors, and icons. Inventory shows loading or an error with
Retry if the local data is unavailable. A pending save or import retains its
original destination when you switch. YAML export contains the selected
workspace’s registry.

On upgrade, the previous registry is adopted once by the original connection’s
first confirmed collection, and the original file is retained. Other collections
start without assigned roles; organize them in Inventory. Manage named connections
in Settings, and use the Decks menu on Inventory or Live for saved views.

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

Open a scene's options menu in Inventory to mark it as a favorite or edit its
tags. Separate tags with commas and choose **Apply tags**. Blank labels are
removed, surrounding spaces are trimmed, and duplicate labels are combined
without changing the spelling of the first one. The row shows the saved tags
and favorite state. These are local settings for the selected workspace.

Search on Inventory or Live matches scene names and tags without regard to
letter case. Each word must match somewhere in the name or tags. **Favorites
only** narrows the view further; Live shows matching favorites before other
cards. **Show all** clears both filters. Inventory keeps the full saved order
and enables reordering when the filters are clear.

Use **Save view as deck** to give the current search and favorites filter a
unique name, such as “Interview” or “Gaming”. The **Decks** menu applies or removes
saved views. Removing a deck leaves the current view and scene metadata intact.
Decks and the current view are saved separately for each connection/collection;
they are also included in YAML exports. A deck is a dynamic filtered view, so
new scenes with matching names or tags appear automatically.

Inventory also shows stale registry entries when a locally remembered scene no
longer exists in OBS. You can remove stale entries from this page.

SceneDeck follows OBS scene UUIDs within each connection and collection. Roles,
tags, protection flags, icons, accents and card order survive renames, including
renames made while disconnected. A newly created scene reusing an old name does
not inherit the old scene's metadata. When identity or destination metadata
conflicts, Inventory shows retained entries that are not applied to a scene.
You can export them for review or remove them. A unique retained identity can
return automatically when its original scene reappears without conflicting
curation. Scenes without UUID information continue to use names.

Renames and inventory changes received during an import wait for its publication;
retired sessions cannot alter the current collection. YAML imports apply named
curation to the destination collection and rebind UUIDs there. Retained conflicts
remain visible but cannot attach themselves using an imported UUID.

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

## Command Palette

Press **Ctrl+K** or use the header search button to find scenes and actions from
any page. Search matches every word, ignoring case; scene tags are searchable too.
The palette includes every Primary scene, including scenes hidden by the current
deck or beyond the first ten digit slots. Scene actions stage Preview in Studio
mode and switch Program in direct mode.

Use the arrow keys to select a result, Enter to run it, and Escape to close.
Page navigation works offline. OBS actions require current observations and
supported server requests. Unavailable actions remain visible with guidance.
Changing connection, collection or curation identity retires an open palette's
OBS actions; close and reopen it to use the new workspace.

Source actions use the complete observed input inventory and are available across
pages. They include opening a source, muting audio, restarting media, and changing
an observed filter. Saved media and filter bindings keep their required properties
refreshed independently of the Sources editor. Shared inputs affect every
scene that uses them. Save replay requires an active replay buffer. Dispatching a
command does not confirm completion; use the existing output results and Sources
readback to check the outcome.

### Custom Action Shortcuts

Open **Settings → Action shortcuts → Manage action shortcuts**. Add a binding,
choose an action from the searchable picker, then click **Record shortcut** and
press a key combination. Use Ctrl, Alt or Super with a character, or a function
key such as F8. Escape cancels key recording. You can edit, disable or remove
bindings individually. To discover media and filter actions, select an input under
**Inspect media and filters for a source**. Its available actions appear below;
the scene and input selected in Sources stay in place.
Use **Refresh OBS observations** to retry an unavailable input list or property.

Bindings belong to the current connection and scene collection. They work on
any application page, remain attached to the action when decks are filtered or
reordered, and stand down while a text field has focus. Holding a key fires it
once until release. Desktop-wide shortcuts are an explicit opt-in below.

Application keys and enabled scene-slot keys are reserved. Duplicate enabled
bindings conflict. If later settings changes introduce a conflict, the custom
binding remains inactive and the existing shortcut keeps its behavior. The
manager shows the conflict so you can choose another key.

Scenes and inputs use their UUID when OBS supplies one, preserving bindings
across renames and refusing a replacement that reuses an old name. Imported YAML
bindings start disabled and retain their identities; review them and reselect
targets when moving to another OBS setup. Editing preserves the enabled state.
Mute/unmute and filter enable/disable bindings apply an explicit state. Toggle
actions derive the opposite value from a current observation. Media and filter actions
require fresh, successful observations of their target properties. An unavailable
shortcut requests a refresh and asks you to press again; it never queues a mutation
for a later connection. Existing output results remain the completion evidence.


### Desktop shortcuts (optional)

In **Manage action shortcuts**, choose **Enable desktop shortcuts** after connecting
and reviewing your enabled bindings. SceneDeck supports up to 64 bindings through
the [XDG GlobalShortcuts portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.GlobalShortcuts.html).
The desktop must supply this portal; support is detected at runtime. Host/source
builds also need SceneDeck’s `io.scenedeck.app.desktop` entry installed so the
desktop can identify the application. A desktop
permission/configuration dialog chooses the keys. SceneDeck shows only the
returned assignments, including when the desktop binds none or only some actions.
Those desktop keys replace the corresponding application keys while registered.
The desktop controls conflicts with other applications and may choose different keys.

Registration lasts until **Stop desktop shortcuts**, a connection/workspace or
binding change, or closing the main window. Re-enable explicitly after reconnecting;
there is no automatic registration or replay. The desktop may remember its own
assignments; SceneDeck does not store a separate global key map. Hold/repeat events
fire once until release. Visible SceneDeck modal dialogs suppress activation. Text editors suppress it
while SceneDeck is the active window; switching to another application restores
background activation. Actions still require current OBS observations and follow
the same scope, identity and capability checks as the command palette. A revoked
assignment or portal session cannot continue dispatching. If registration fails,
application shortcuts remain available; check desktop portal support and retry.

### Live Quick Actions

At the top of Live's scene area, use **Pin action** (+) to choose a frequently used
control. Pin a break scene, microphone mute/unmute or mute toggle, replay save,
media restart, filter enable/disable or toggle, or a shortcut to a source. The
picker can inspect media and filters independently of your Sources selection.

Each button shows the action and a separate **Observed** state. A mute action can
therefore show “Unmuted” until OBS reports the change. Pending, stale, missing or
failed observations disable the relevant controls. If the displayed state changes
before activation, refresh and try again; SceneDeck does not queue the action for
later. Replay save completion remains in the output results, not in the pin's label.

Open a pin's **Customize quick actions** menu (⋮) to enable or disable it, change its
action, move it up or down, or remove it. Pins persist in this connection and scene
collection, independently of scene order, search and deck views. Imported YAML
pins start disabled and retain target identities; review or change their targets
before enabling them. Shared inputs and filters affect every scene that uses them.
The strip wraps on narrow windows and scrolls with the scene area.

### Preflight Check

Open **Preflight check** in Live's scene area before a production. Use **Configure
preflight** to save an expected scene collection, required scenes, a microphone,
minimum recording space, and a manual checklist. Plans belong to the actual
connected saved connection and endpoint. Selecting another connection in Settings
does not edit the connected production's plan. Switching OBS collections does not
change the expected collection: a mismatch stays visible.

The review shows passed, needs attention, unknown, or explicitly skipped checks.
Scene and microphone UUIDs survive renames; a reused name does not satisfy a
requirement for a different UUID. Missing saved targets stay in the editor so you
can remove or replace them. Changing the expected collection resets target choices;
to choose scenes or microphones from another collection, switch OBS to it and
reopen Configure. This does not make SceneDeck switch OBS automatically.

Speak into the selected microphone. Preflight checks that it is present, unmuted,
and has a post-fader peak above −50 dBFS. That signal is remembered for ten seconds
while valid meter samples continue arriving within two seconds. It is a presence
check, not a sound-quality assessment. Missing, invalid, or stale telemetry remains
unknown. Recording space uses OBS's free space on its recording volume in MiB,
requires a reading no older than five seconds, and compares it with your saved
minimum. You may explicitly skip microphone or disk checks.

Manual checkmarks belong to this review only. Closing or resetting it, changing the
plan, or changing the OBS session or collection clears them. **Refresh OBS
observations** requests reads without starting an output or applying audio changes.

Optionally enable **Review preflight before starting a stream or recording**.
Starting then opens the review first. You may cancel or explicitly continue with
unresolved checks; your existing output confirmation settings still apply.
Nothing starts automatically when checks pass or a refresh finishes. A changed
connection or collection retires the pending start.

### Live Health Alerts

A warning area above the main workspace stays visible across Live, Mixer, Sources
and Stats when an incident needs attention. It shows the first warning and a count
of any others. Use its warning button to read every incident and its suggested
checks, or open Stats for more detail. The detached Stats window has the same
warning area. **Live health alerts** on Live and Stats opens the report even when
there are no active incidents.

Network frame loss, rendering skips and encoder skips are separate conditions.
Each requires at least 1% missed frames in consecutive recent intervals for five
seconds. While recording, less than 1 GiB free on OBS's recording volume must also
persist for five seconds before a warning appears. These warnings enter
**Recovering** when valid readings improve, then clear after ten seconds of healthy
observations. A brief relapse keeps the same incident visible. Missing, invalid,
stale or reset counters do not count as recovery. **Latest readings unavailable**
means the earlier incident cannot currently be reassessed.

The status bar still shows the cumulative encoder skipped-frame count, but no
longer colors a historical nonzero count as an ongoing problem. The warning area
uses recent measurements.

An unexpected loss of the OBS connection remains visible until reconnect or
acknowledgement. Deliberately disconnecting or changing connections does not create
a connection-loss warning. A stream or recording that was observed running and
then stopped without a recent admitted SceneDeck stop command produces a stopped
output alert. A failed command, failed status read, command acknowledgement, or
session retirement alone does not establish that an output stopped or a file was
saved. An already observed stopped-output incident remains visible if the
connection subsequently drops. The recent-stop window is thirty seconds from
command admission, measured against OBS observation time. Delayed screen updates
do not renew it. A failed or older status read cannot override a newer observation;
an unavailable current read does not mean the output stopped.

**Acknowledge** clears only the displayed event notices. It does not restart an
output, reconnect OBS or dismiss a newer incident that was not displayed. Sampled
performance warnings clear through fresh recovery measurements. A new OBS
connection begins a new incident context.


**Audio monitoring** in the report lets you select inputs and explicitly enable
silence and/or clipping rules for this connection and collection. Clear an input's
selection and save to remove its rule; clear **Enabled** to pause it. Rules run
while connected, including before streaming or recording starts. Imported rules
are disabled for review. Preflight microphone choices do not enable monitoring.

Silence means fifteen seconds of fresh readings at or below −50 dBFS after the
fader, or a muted input with fresh meter evidence. Clipping means at least one
channel stays at full scale before the fader for one second while unmuted. Ten
seconds of healthy readings clear either warning. Readings more than two seconds
old, invalid values, missing inputs and failed identity/mute reads are unavailable;
they cannot establish silence or recovery. Selected UUIDs follow renames but never
fall back to a reused name. Unavailable saved inputs remain editable so you can
pause or remove them.

Monitoring continues across pages and while only detached Stats remains open.
The report lists whether each rule is disabled, monitoring, muted, missing or
unavailable. **Refresh OBS observations** refreshes the input choices. Save errors
remain visible; press Save again to retry. No rule changes OBS mute, gain or output
state.

## Keyboard Shortcuts

- `Ctrl+K`: open the command palette.
- `F1`: open the Help page.
- `Ctrl+R`: reconnect to OBS.
- `Ctrl+,`: open Settings.
- `Ctrl+Q`: quit SceneDeck.
- `Ctrl+1` … `Ctrl+0` on Live: activate the scene assigned to that Inventory
  slot using the current direct/Studio mode, even if filtered out.
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


## Timestamp bookmarks

Open **Bookmarks** from Live, Stats, or the command palette. Enter a name and
choose **Add bookmark** (or press Enter); an empty name gets a numbered default.
The note records the instant you add it, the connection and collection, and the
last observed scene. Rename or remove notes in the same window. Existing notes
remain available when disconnected and after restarting SceneDeck.

Session elapsed time starts when SceneDeck observes a connection and includes
recording pauses. A reconnect starts a new session. Recording output time is
shown only from an accepted OBS status sample no more than five seconds old,
with its age and paused/running state. Missing, stale, transitioning or uncertain
timing stays unavailable. SceneDeck does not extrapolate this sample or claim it
is a position in a particular recorded file; file splits and reconnects can
break that relationship. A local bookmark does not create an OBS chapter.

Bookmarks save automatically to `bookmarks.json`. The window reports saving or
failed saves and offers **Retry**. A file that cannot be read is preserved;
correct the file or restore it from a backup, then retry loading. Up to 2,000
notes are retained without automatic eviction. Remove notes when that limit is
reached. Changing connection or collection disables capture in an already open
bookmark window; reopen it to capture the new context. Older notes retain their
original context.

## Session reports and export

Open **Session reports** from Stats (including its detached window) or the command
palette. SceneDeck keeps the latest eight connection sessions in memory during
the current app run. Reconnecting starts a separate report. Use Previous/Next to
select one, and **Refresh snapshot** to include newly received evidence or bookmark
edits. The view stays stable while you review or export it.

Reports include scene changes, requested actions, acknowledgements, observed
output results and OBS-host paths, health-alert changes, sampled CPU/FPS and
other telemetry, and local bookmarks. An acknowledgement never confirms a saved
file. A cleared warning does not by itself prove recovery. Disconnects and missing
measurements leave uncertainty; sampled frame counters are cumulative values,
not a computed total for this recording.

Each report retains up to 4,096 events and 2,048 telemetry samples. The summary
shows omissions, activity gaps, and sample counts; CPU peaks and minimum FPS cover
all valid samples seen during that connection, including evicted raw samples.
The viewer shows the newest 150 events and 50 bookmarks. JSON/CSV exports include
all records retained in the selected snapshot. Exported bookmarks include their
separate session elapsed time and recording-time evidence.

Choose **Export JSON** or **Export CSV** and select a local file. Export runs in
the background and reports success or failure. JSON preserves structured values;
CSV provides event/evidence columns, telemetry columns and JSON detail cells.
CSV text that could execute as a spreadsheet formula gets a protective leading
apostrophe. Connection passwords and configuration are not part of reports.
For automatic saving, open **Session history** from Settings or the report window.
It is off by default. Enable it, choose **1–32 sessions to keep** (default eight),
and apply the settings. This saves the current report including retained earlier
events, then future connections. Checkpoints occur about every ten seconds and at
session boundaries. Disabling stops new checkpoints and preserves existing files.
Applying a smaller retention limit removes the oldest saved sessions.

Saved reports reload on startup. A report without an end from a previous app run
shows that its end time is unavailable; SceneDeck does not invent an output stop.
**Reload and retry** preserves local report changes while rereading saved history.
Unreadable, corrupt, unsupported or externally changed files are preserved and
reported. History-settings save errors are also shown and can be retried.

Closing the main window keeps reporting active while a detached window remains.
Closing the last window or choosing **Quit** saves the final report before exit.
If saving fails, choose **Cancel** to continue, **Retry** after fixing the problem,
or **Close anyway** to explicitly leave unsaved changes behind. A cancelled or
delayed exit resumes reporting and records the interruption; it does not stop OBS
outputs or claim that pending output requests completed.

**Clear saved history** asks before deleting saved and unsaved history. It keeps
bookmarks and recent in-memory reports. The current connection is excluded from
further automatic saves; the next connection is saved if history remains enabled.
Applying enabled history settings again explicitly includes the current report.

## Audio presets

In **Mixer → Audio presets → Open**, select inputs, enter a name and choose
**Save selected inputs**. The saved mute and gain values belong to the connected
show workspace and scene collection. Capture includes the complete input inventory,
independently of the current Mixer scene or search filter. Use **Refresh** if the
observations are unavailable or old.

Choose a saved preset to compare its values with current observations. A renamed
input follows its UUID when OBS provides one; a replacement input with a different
UUID appears as missing or unavailable. Shared inputs affect every scene that uses
them. **Replace selected preset** captures the currently selected inputs again;
missing selected inputs must be explicitly deselected before replacing the preset.
You can rename or delete a selected preset. Save errors remain visible, and saving
the same selection again retries after the file problem is fixed.

To apply a saved preset, choose **Review changes**, then **Apply available inputs**.
The review is held for up to 30 seconds. Apply becomes unavailable if the observed
values change, the review expires, or OBS lacks the required requests. Refresh and
review again to use newer observations. Missing inputs are skipped explicitly.

Each input shows its own result. **Values confirmed by OBS** means readback matched
the requested values; acknowledgement marks only mean OBS accepted those requests.
Some inputs can succeed while others fail. SceneDeck verifies gain before unmuting
and stops that input if gain fails or cannot be confirmed. **Stop remaining changes**
prevents further writes and keeps completed changes. Connection or collection
changes retire pending work without replaying it. Results remain available when
you close and reopen the editor and are recorded in session activity/reports.
