# Configuration

SceneDeck stores local settings under the XDG config directory.

## Config File

Default path:

```text
$XDG_CONFIG_HOME/scenedeck/config.json
```

Fallback path:

```text
$HOME/.config/scenedeck/config.json
```

Current schema version: `5`.

Example:

```json
{
  "version": 5,
  "connection_id": "default",
  "connections": [
    { "id": "default", "name": "OBS", "obs": { "host": "127.0.0.1", "port": 4455 } }
  ],
  "legacy_registry_endpoint": "127.0.0.1:4455",
  "obs": {
    "host": "127.0.0.1",
    "port": 4455
  },
  "live": {
    "scene_previews": false,
    "show_roles": ["primary"],
    "audio_inputs": [],
    "allow_switching_only": ["primary"]
  },
  "outputs": {
    "confirm_start_stream": false,
    "confirm_stop_stream": true,
    "confirm_start_recording": false,
    "confirm_stop_recording": true
  },
  "appearance": {
    "mode": "system",
    "theme": "adwaita-default",
    "ui_density": "comfortable",
    "motion": "full",
    "custom_css": {
      "enabled": false,
      "light_path": null,
      "dark_path": null
    }
  },
  "mixer": {
    "mode": "active",
    "selected_scene": null,
    "pinned_scene": null,
    "grouping": "scope"
  },
  "inventory": {
    "name_width": 360,
    "role_width": 240
  },
  "hotkeys": {
    "enabled": true,
    "style": "ctrl",
    "leader": "space",
    "leader_timeout_ms": 1500
  },
  "onboarding": {
    "welcome_shown": true
  }
}
```

Fields:

- `obs.host`: OBS WebSocket host.
- `obs.port`: OBS WebSocket port.
- `live.scene_previews`: scene thumbnails on Live. Defaults to `false`; change it
  in Settings. Disabling previews returns compact scene tiles.
- `live.show_roles`: roles intended for Live display. Current Live filtering is
  driven by the local registry role semantics.
- `live.audio_inputs`: optional allow-list of OBS input names for scene audio.
  Empty means discover all active scene audio inputs.
- `live.allow_switching_only`: roles intended for scene switching. Current Live
  switching uses scenes whose role is `Primary`.
- `outputs.confirm_start_stream`: whether the sidebar asks before starting
  the stream. Defaults to `false`.
- `outputs.confirm_stop_stream`: whether the sidebar asks before stopping the
  stream. Defaults to `true`.
- `outputs.confirm_start_recording`: whether the sidebar asks before starting
  a recording. Defaults to `false`.
- `outputs.confirm_stop_recording`: whether the sidebar asks before stopping a
  recording. Defaults to `true`.
- `appearance.mode`: `system`, `light`, or `dark`.
- `appearance.theme`: selected built-in theme id. `adwaita-default` is the
  fallback. Choose a built-in theme family in Settings under Appearance.
- `appearance.ui_density`: `comfortable` or `compact`. The field is reserved
  for density-specific styling; the current UI does not expose it.
- `appearance.motion`: `full`, `reduced`, or `off`. Controls how much the
  interface animates:
  - `full` — looping animations play. The recording indicator blinks like a
    tally light, the streaming indicator swells slowly, and the header band
    breathes while you are live.
  - `reduced` — nothing loops, but state *changes* still animate briefly, so
    you can see that a scene switched rather than having to spot it.
  - `off` — no animation at all.

  Animation never carries meaning on its own: every state that animates is also
  shown by colour, an icon and a text label, so nothing becomes unreadable at
  `reduced` or `off`.

  If your desktop's own "reduce animation" accessibility setting is on, `full`
  behaves as `reduced` without you having to change anything here. An explicit
  `off` is always honoured and is never promoted back up.
- `appearance.custom_css.enabled`: whether a user CSS file should be loaded.
- `appearance.custom_css.light_path`: optional path to a user CSS file used
  when the effective color scheme is light.
- `appearance.custom_css.dark_path`: optional path to a user CSS file used when
  the effective color scheme is dark.
- `mixer.mode`: `active`, `selected`, or `pinned`.
- `mixer.selected_scene`: optional scene name used by Selected mode.
- `mixer.pinned_scene`: optional scene name remembered by Pinned mode.
- `mixer.grouping`: `scope`, `scene_path`, or `none`.
- Legacy `inventory.name_width` / `inventory.role_width` keys are accepted but
  ignored, and omitted when settings are saved. The responsive Inventory list
  adapts automatically; there are no width controls.
- `hotkeys.enabled`: whether Live scene cards can be switched from the keyboard.
  Defaults to `true`.
- `hotkeys.style`: which key combination activates a scene slot. One of
  `plain` (bare digits), `ctrl`, `alt`, `shift`, `super`, `ctrl-alt`,
  `ctrl-shift`, or `leader` (vim-style two-stroke). Defaults to `ctrl`, which
  cannot fire while typing. Unknown values fall back to `ctrl`.
- `hotkeys.leader`: leader key used when `style` is `leader`. One of `space`,
  `comma`, `semicolon`, `backslash`, or `grave`. Defaults to `space`. Unknown
  values fall back to `space`.
- `hotkeys.leader_timeout_ms`: how long an armed leader waits for its digit.
  Defaults to `1500`, and is clamped to `250`–`5000`.

- `onboarding.welcome_shown`: whether the first-run welcome dialog has already
  been shown. It is written the moment the dialog appears, so the greeting can
  interrupt at most one launch, and upgrading an existing config sets it without
  ever showing the dialog. Set it back to `false` to see the welcome dialog
  again; the Help page itself is always available from the
  sidebar, the header button, or `F1`.

Scene hotkeys act only on the Live page. Slot `1` is the first scene card, `9`
the ninth, and `0` the tenth; cards past the tenth have no shortcut. The card
order — and therefore the slot numbers — comes from the current workspace registry’s
`scene_order`, which the Inventory page writes, filtered to scenes whose role is
live-switchable. Bindings that need no modifier (`plain`, and the leader key
itself) stand down while a text field has focus.

Themes are light/dark-aware theme families. If `appearance.mode` is `system`,
SceneDeck follows the effective libadwaita/system color preference and applies
the selected theme family's light or dark variant. If `appearance.mode` is
`light` or `dark`, SceneDeck forces that side of the selected theme family.
Custom CSS follows the same rule by loading `light_path` or `dark_path`.

Unknown or missing fields fall back through Serde defaults. If the app cannot
read or parse the config, it starts with defaults and reports a startup notice.

## Config Migration

Schema version `1` stored the color preference as a top-level `theme_mode`
field:

```json
{
  "version": 1,
  "theme_mode": "dark"
}
```

On load, SceneDeck automatically migrates version `1` configs to version `2` by
moving that value to `appearance.mode`, preserving the existing color
preference. The migrated config is written back to disk. Unknown old or new
theme values fall back to `system`.

Version `3` added the `onboarding` section. Migrating a version `2` config sets
`onboarding.welcome_shown` to `true`, because a config file that already exists
belongs to someone who has been using SceneDeck and does not need the first-run
welcome dialog. Only a machine with no config file at all is treated as a first
run. Migrations are applied in one pass, so a version `1` config lands directly
on the current version.

## Show workspaces and registry data

Current curation is stored in `$XDG_CONFIG_HOME/scenedeck/show-workspaces.json`
(or `$HOME/.config/scenedeck/show-workspaces.json`). Its versioned entries are
keyed by saved connection ID, connected endpoint, and OBS scene collection name.
Each entry contains a registry with scene and audio-source metadata. Switching
collections automatically selects the matching entry; identical scene names in
other collections or connections do not share metadata. Changing the endpoint
also selects separate curation.

The old `registry.json` is adopted once for the original default connection’s
first confirmed collection. Version 3 configuration migration records that
connection’s endpoint in `legacy_registry_endpoint`; fresh installations have
no legacy owner. The original file remains untouched. Other workspaces start
empty. Corrupt or unsupported workspace data produces an error with Retry in
Inventory; it is never overwritten with an empty fallback. Manage named connections in Settings.

All writes and YAML imports capture their destination when accepted. Switching
while an import is running does not redirect it, and controls from the previous
workspace cannot edit the newly selected workspace. Loading disables curation
edits until the selected registry is available.

Edits appear immediately in the cached view, but persistence is confirmed
separately. Inventory shows a failed-save message and Retry when a registry write
fails. Repeating an unchanged dirty edit also retries retained intent; successful
retry preserves newer accepted edits. Close/Quit keeps unresolved failures
explicit rather than treating a later unrelated save as recovery.

Scene UUID bindings are stored with the workspace. Confirmed identities carry
metadata and card order across renames, including changes made while disconnected
and simultaneous name swaps. A replacement scene that reuses an old name does
not inherit its curation. Conflicting entries remain in `retained_scenes`, outside
active scene metadata, and appear for review/removal in Inventory. If the original
UUID returns without conflicting curation, its retained entry can be restored.
Unknown identities retain name-based behavior; duplicate identities do not infer
moves. All changes use the same retryable registry writer.

YAML imports are portable named intent: imported `scene_uuids` are cleared and
rebound to this collection's current inventory. Retained entries remain in the
export, but imported retained UUIDs are cleared so they cannot attach themselves
to a scene in another collection. To resolve a retained conflict manually,
export a backup, move the desired metadata into the named `scenes` entry, and
import it. Merely removing a retained row discards that retained version.

Scene entries also support `favorite` (a boolean, default false) and editable
`tags`. Tags are normalized when edited in Inventory; clearing the last tag
never removes a favorite or another metadata field.

`scene_filter` stores the current `query` and `favorites_only` values. `decks`
maps unique view names to filters of the same shape. Both belong to the selected
workspace and are included in YAML. Applying a deck changes the current filter;
removing one does not change the current filter or scene metadata. Names are
trimmed and duplicate names are rejected without regard to letter case.

Registry payload within each workspace entry:

```json
{
  "scenes": {
    "Main": {
      "role": "primary",
      "accent_color": "#336699",
      "icon": "camera",
      "tags": [],
      "protected": false
    },
    "Camera Frame": {
      "role": "module",
      "tags": ["camera"],
      "protected": true
    }
  },
  "scene_order": ["Main", "Camera Frame"],
  "inputs": {
    "Mic/Aux": { "icon": "microphone" }
  },
  "rules": {
    "primary_can_depend_on": [],
    "module_can_depend_on": [],
    "forbidden_edges": []
  }
}
```

Roles:

- `primary`: live-switchable scene.
- `secondary`: valid scene hidden from Live by default.
- `module`: reusable nested scene.
- `raw`: hardware or source wrapper scene.
- `debug`: temporary test scene.
- `archive`: preserved but excluded from workflows.

The UI edits scene roles, optional scene accent colors, scene and source icons,
scene ordering, and stale entries. Accent colors use `#RRGGBB`; Live renders a small accent chip that stays separate
from Program/Preview state and image shading. Tags, protected flags, and custom
rule fields remain available in the file for registry metadata and Doctor logic.

`icon` holds a key from SceneDeck's curated icon catalogue — `camera`,
`microphone`, `game`, and so on — not a raw Nerd Fonts glyph name, so the
underlying glyph can change without invalidating a registry. An unknown key is
ignored and the scene or source simply shows no icon. Scene icons are chosen in
Inventory; audio source icons come from the overflow button on an audio card.

`inputs` holds per-audio-source metadata keyed by OBS input name. It currently
carries only icons, and is omitted entirely when no source has one.

`scene_order` stores the order chosen by dragging scenes in Inventory. Inventory
and Live use this order; newly discovered scenes not yet listed are appended.

### Explicit action bindings

Each workspace registry stores `action_bindings`: an ordered list of an explicit
`action`, an `enabled` flag and a `shortcut` containing a logical `key` and exact
`ctrl`/`alt`/`shift`/`logo` modifiers. Configure these in Settings rather than
editing the representation directly. Legacy registries default to an empty list.
Bindings are independent of `scene_order`, saved decks and the first ten slots.

Known target UUIDs are preserved through local saves and renames and used for OBS
mutation requests. A known UUID never falls back to a reused name. YAML imports
keep the target identity but disable every binding for review in the destination
workspace. Invalid, duplicate or reserved keys cannot dispatch a custom action.
Changing the scene-slot configuration can disable a conflicting custom binding
without discarding it. Disabling slot hotkeys does not disable explicit bindings.

### Live quick actions

`quick_actions` is an ordered list of `{ action, enabled }` pins in each workspace
registry. Old registries default to an empty list. Configure pins through the
Live strip's add button and each pin's menu. Local JSON saves preserve order,
identity and enabled state. YAML imports preserve action identity and order but
disable every pin for review. No keyboard assignment or scene slot is implied.

Pins and action bindings share the operator-action representation. Explicit mute
and filter setters retain their requested values; mute/filter toggles resolve to
the opposite of a current observation before dispatching an explicit setter.
Media/filter pins contribute property reads while enabled. Unknown or failed
observations never imply an off state, and refreshing never queues a future write.

## YAML Import and Export

Inventory can export the scene registry to YAML and import it back from YAML.
The YAML structure contains only the selected workspace’s registry, preserving scenes, roles, tags,
accent colors, scene order, protected flags, and rule fields.

Example:

```yaml
scenes:
  Main:
    role: primary
    accent_color: "#336699"
    tags:
      - live
    protected: false
scene_order:
  - Main
rules:
  primary_can_depend_on: []
  module_can_depend_on: []
  forbidden_edges:
    - [primary, debug]
```

Import replaces the local registry file with the parsed YAML content.

## Saved connections

`connections` stores each connection’s immutable `id`, editable `name`, and
`obs` host/port. `connection_id` selects the setup to use for the next connection.
The top-level `obs` object remains the selected endpoint snapshot for existing
callers; loading resolves it from the selected saved profile. Use Settings to
edit profiles and apply host/port/name changes together. New connections get a
new ID and default endpoint; they do not copy another connection’s password.

The existing setup becomes the `default` connection named OBS. Config writes
materialize this profile before reconnect reads settings, and migration retains
the original endpoint used for one-time registry adoption. Renaming a connection
preserves its credential identity and curation. Choosing a different endpoint
selects separate curation once that endpoint and collection are observed.

Selecting a saved setup does not change the current OBS session. **Connect saved
setup** waits for its configuration save to succeed before requesting a new
connection. The status row identifies the connection and endpoint OBS is
actually using. A failed save does not trigger reconnect.

## Secrets

Each connection’s OBS password is stored through the system Secret Service keyring. It is not
written to configuration, registries, or workspace exports.

The default connection keeps the existing `obs-websocket` keyring username.
Other connections use `obs-websocket:<connection-id>` under the same application
ID, with no fallback to the default credential. Selecting another profile clears
the password field before its own credential is read. Failed reads remain empty
and are reported in Settings. Delayed writes retain their original destination
and cannot update another selected connection’s password field.

## Preflight plans

The additive `preflight_plans` array defaults to empty in older configuration
files. Each entry contains `connection`, `endpoint`, and `plan`. The key uses the
actual connected profile ID and endpoint; `plan.expected_collection` is an explicit
expectation, independent of OBS's currently selected collection. Renaming a saved
connection preserves the key; changing its endpoint selects a separate plan.
Ambiguous duplicate entries are not silently selected; an explicit editor save
replaces entries for that exact key.

A plan contains `required_scenes` and optional `microphone` identity targets
(`name`, optional `uuid`), `check_microphone`, `minimum_disk_mib` (null skips the
check; zero is invalid), `manual_items`, and `review_before_output` (default false).
Checklist completions, signal latches and report results are never persisted.
Saves use the ordered configuration writer; failure remains visible in the editor
and can be retried. This data contains no OBS credentials.


### Audio health rules

Each connection/collection registry has an additive `audio_alerts` array, empty by
default. A rule contains `target: { name, uuid }`, `enabled`, `silence`, and
`clipping`. All three booleans default to false when omitted. Targets preserve
identity across renames; a known UUID never falls back to a reused name. Portable
YAML imports retain targets and warning choices but force `enabled: false` for
review. Rules are separate from `preflight_plans`.

Live or Stats → Live health alerts → Audio monitoring manages these rules. The
editor preserves missing targets, uses the actual connected workspace rather than
the profile selected in Settings, and saves on the ordered registry lane. Save
failures retain their intended writes for retry. Scope or external rule changes
retire an open editor. Runtime meter evidence is not serialized. Optional session history retains alert transitions.

Built-in audio thresholds are fifteen seconds at/below −50 dBFS post-fader (or
muted) for silence and one second at full scale pre-fader for clipping while
unmuted. Both require fresh, valid meters (at most two seconds old) and ten seconds
of clear readings to recover. Rules run whenever that workspace is connected,
independently of output activity. Disable a rule for intentional silence.


## Explicit bookmarks

`bookmarks.json` lives beside `config.json`. It holds explicitly created operator
notes and is independent of optional automatic session history. Version 1 stores
monotonically allocated bookmark/session IDs, UTC creation time, local session
elapsed time, original connection/endpoint/collection/scene context, and separate
recording-time evidence (`unavailable`, `inactive`, or an observed duration with
pause state and sample age). Credentials are not included.

The startup worker reads the file before GTK. Missing files start empty; corrupt,
unsupported or unreadable files disable edits and are never replaced with defaults.
Writes use a dedicated FIFO background lane and atomic replacement, with visible
failure/retry state and shutdown draining. Every write validates the existing file
and its replacement. The archive admits at most 2,000 named notes and a 16 MiB
serialized file. IDs are retained across deletes/restarts; old notes are not
silently dropped to make room for new ones.

## Session report exports

Current and recent session reports retain eight sessions in memory, up to
4,096 events and 2,048 telemetry samples each, with explicit omission counts.
Session reports in Stats offers JSON/CSV export to a user-selected path. Exports
use atomic background writes, retain observed evidence separately from requests
and acknowledgements, and contain no configuration or connection password.
`session_history` defaults to `{ "enabled": false, "retain_sessions": 8 }`.
Retention is bounded to 1–32 sessions. Settings → Session history applies both
choices explicitly. Disabling leaves saved reports intact; reducing retention
while enabled removes the oldest saved sessions. Receipt/session order, rather
than wall-clock sorting, handles clock changes.

Optional history is stored at `$XDG_STATE_HOME/scenedeck/session-history.json`, or
`~/.local/state/scenedeck/session-history.json` when XDG_STATE_HOME is unset.
Version 1 has a 64 MiB file limit and at most 32 reports. A sibling `.lock` file
serializes cooperating writers; writes compare the loaded archive before atomic
replacement and refuse conflicting external changes. Unsupported, malformed,
symlink and special-file inputs are preserved. Reload/retry keeps locally owned
report changes while rereading disk. Explicit clear can be retried without
resurrecting the removed reports.

History contains bookmark IDs, not copies of bookmark names/content. Startup
marks reports lacking an end as interrupted with an unavailable end time. The
original archive remains the expected disk snapshot until a write succeeds.
Settings and history failures have separate retry evidence. Accepted writes
participate in the Close/Quit and shutdown drains, including when a detached
window is the last open window. A failed final save keeps an explicit retry,
cancel or informed abandonment decision available before application exit.

### Audio presets

Each workspace registry may contain `audio_presets`, a map from preset name to
selected-input settings. Existing registries default to an empty map. Names must
be nonempty, trimmed, contain no control characters and contain at most 80
characters. At most 64 presets with 1–64 unique inputs each are accepted.

```json
{
  "audio_presets": {
    "Interview": {
      "inputs": [
        {
          "target": { "name": "Microphone", "uuid": null },
          "muted": false,
          "gain": 0.5
        }
      ]
    }
  }
}
```

`gain` is a finite linear multiplier in OBS's supported range 0–20; zero means
digital silence. This preserves observed gain independently of the fader's dB
range. See [OBS SetInputVolume](https://github.com/obsproject/obs-websocket/blob/master/docs/generated/protocol.md#setinputvolume).
The editor stores a UUID whenever one was observed. A saved UUID never resolves
to a different input merely because its name was reused. Invalid gain, duplicate
targets, unknown preset fields or exceeded limits fail parsing without replacing
the original file. Workspace writes and imports use the existing registry lane.


### Desktop shortcut registration

Global shortcuts reuse the current workspace's enabled `action_bindings`.
Registration is explicitly enabled for the current connection session in the
shortcut manager and is not written to config or workspace files. Desktop-selected
keys and permissions belong to the GlobalShortcuts portal. Re-enable after
connection/workspace/binding changes or reopening the main window. App-local
bindings continue to persist normally; registered desktop actions temporarily
suppress their corresponding local keys to avoid duplicate dispatch.
