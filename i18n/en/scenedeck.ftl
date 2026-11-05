## SceneDeck UI strings (English, source locale).
##
## Grouped by the module each message is used from. Message ids are prefixed
## with the module name to keep them unambiguous in this single shared file.

## Internal — used only by the i18n loader's own regression test, not shown
## in the UI. Every locale must define this so the smoke test can confirm the
## locale's bundle loaded (not just the `en` fallback).
i18n-loader-smoke-test = Localization loaded.

## infra/error.rs — user-facing renderings of AppError. `detail` is raw
## upstream text (often from OBS or the OS) and is never translated.
error-connection = OBS connection failed: { $detail }
error-request = OBS request failed: { $detail }
error-config = Configuration error: { $detail }
error-storage = Storage error: { $detail }
error-notification-title = SceneDeck error: { $message }
events-resync-warning = OBS notifications were missed. SceneDeck is resynchronizing; output history may be incomplete.

## domain/audio.rs — AudioSourceScope::label()
audio-scope-global = Global
audio-scope-active = Scene
audio-scope-nested = Nested
audio-scope-group = Group

## domain/graph.rs — EdgeStatus summary label/tooltip
edge-status-ok-label = OK
edge-status-warning-label = Warning
edge-status-forbidden-label = Forbidden
edge-status-ok-tooltip = Edges that match the graph policy
edge-status-warning-tooltip = Edges outside an allow-list
edge-status-forbidden-tooltip = Edges forbidden by graph policy

## domain/output.rs — OutputRunState::label(), OutputStatus::summary()
output-state-inactive = Inactive
output-state-starting = Starting
output-state-active = Active
output-state-stopping = Stopping
output-state-reconnecting = Reconnecting
output-state-paused = Paused
output-state-unknown = Unknown
output-summary = { $name }: { $state }

## domain/role.rs — SceneRole::label()/description()
role-primary = Primary
role-secondary = Secondary
role-module = Module
role-raw = Raw
role-debug = Debug
role-archive = Archive
role-unassigned = Unassigned
role-primary-desc = Live-switchable scene
role-secondary-desc = Valid scene, hidden from Live by default
role-module-desc = Reusable nested scene, not directly switchable
role-raw-desc = Hardware or source wrapper scene
role-debug-desc = Temporary test scene
role-archive-desc = Preserved but excluded from all workflows

## domain/mixer.rs — MixerMode / MixerGrouping labels
mixer-mode-active = Active
mixer-mode-selected = Selected
mixer-mode-pinned = Pinned
mixer-mode-active-desc = Follow the OBS program scene.
mixer-mode-selected-desc = Inspect the selected scene without following OBS.
mixer-mode-pinned-desc = Keep the selected scene stable while operating.
mixer-grouping-scope = Scope
mixer-grouping-scene-path = Scene Path
mixer-grouping-none = None

## domain/diagnostic.rs — DiagnosticSeverity label/count
diag-label-info = Info
diag-label-warning = Warnings
diag-label-error = Errors
diag-count-info = { $count ->
    [one] { $count } info item
   *[other] { $count } info items
}
diag-count-warning = { $count ->
    [one] { $count } warning
   *[other] { $count } warnings
}
diag-count-error = { $count ->
    [one] { $count } error
   *[other] { $count } errors
}

## ui/pages/inventory.rs
inventory-no-role-assigned = No role assigned

## services/doctor_service.rs — diagnostic message/suggestion templates
doctor-no-role = Scene has no role assigned in the local registry.
doctor-no-role-suggestion = Open Inventory and assign a role.
doctor-stale-entry = Registry entry references a scene not found in OBS.
doctor-stale-entry-suggestion = Remove the entry from Inventory.
doctor-protected-switchable = Protected scene is in the switchable '{ $role }' role.
doctor-protected-switchable-suggestion = Protected scenes are usually building blocks; consider Module or Raw.
doctor-cycle = Circular scene reference involving '{ $parent }' and '{ $child }'.
doctor-cycle-suggestion = Remove the nested-scene loop; OBS cannot render cycles.
doctor-edge-primary-debug = Primary scene depends on a Debug scene. (→ '{ $child }')
doctor-edge-primary-debug-suggestion = Remove the Debug scene from the live path before going live.
doctor-edge-primary-raw = Primary scene directly wraps a Raw source. (→ '{ $child }')
doctor-edge-primary-raw-suggestion = Wrap the Raw source in a Module scene for reuse and clarity.
doctor-edge-module-primary = Module depends on a Primary scene, inverting the hierarchy. (→ '{ $child }')
doctor-edge-module-primary-suggestion = Modules should be building blocks, not consumers of Primary scenes.
doctor-edge-raw-nests = Raw scene nests another scene. (→ '{ $child }')
doctor-edge-raw-nests-suggestion = Raw scenes should be leaf source wrappers with no nested scenes.
doctor-edge-forbidden = Scene dependency is forbidden by the graph policy. (→ '{ $child }')
doctor-edge-outside-policy = Scene dependency is outside the configured graph policy. (→ '{ $child }')
doctor-edge-adjust-suggestion = Adjust the nested scene relationship or update the registry graph rules.

## controller/app_controller.rs
controller-not-connected = Not connected to OBS

## controller/state.rs — Page titles and ObsStatus labels
page-live = Live
page-stats = Stats
page-mixer = Mixer
page-graph = Graph
page-inventory = Inventory
page-doctor = Doctor
page-settings = Settings
page-help = Help
obs-status-disconnected = Disconnected
obs-status-connecting = Connecting…
obs-status-connected = Connected
obs-status-error = Error

## storage/config.rs — ConfigStartupNotice
config-first-launch = No saved settings yet. Defaults are loaded.
config-read-failed = Settings could not be read: { $detail }
config-parse-failed = Settings could not be parsed: { $detail }
config-parse-failed-backed-up = Settings could not be parsed: { $detail } — the file was kept at { $path } and defaults were loaded.

## graph.rs

graph-empty-title = No Dependencies
graph-empty-description = No scenes nest other scenes, or OBS is not connected. Connect and add nested scene sources to see the dependency graph.
graph-page-title = Scene Dependencies
graph-reset-tooltip = Reset graph layout
graph-edge-summary-count = { $label } { $count }

## mixer.rs

## Page chrome / empty state
mixer-empty-title = No Mixer Data
mixer-empty-description = Connect to OBS to load scenes and audio sources.
mixer-page-title = Mixer
mixer-controls-title = Mixer Controls
mixer-summary-title = Current Mixer Source

## Control rows (ComboRow / EntryRow titles+subtitles)
mixer-mode-row-title = Mode
mixer-mode-row-subtitle = Active follows OBS; Selected and Pinned keep the chosen scene stable.
mixer-scene-row-title = Scene
mixer-scene-row-subtitle = Used by Selected and Pinned modes.
mixer-grouping-row-title = Group By
mixer-grouping-row-subtitle = Controls how audio sources are arranged below.
mixer-search-row-title = Search

## Scene-loading / no-scene placeholders
mixer-no-scene-title = No Scene Selected
mixer-no-scene-description = Choose a scene to load its mixer audio.
mixer-loading-title = Loading Mixer Audio
mixer-loading-description = Loading audio sources for { $scene }.

## Audio-source empty states
mixer-current-scene-fallback = The current scene
mixer-no-audio-sources-title = No Audio Sources
mixer-no-audio-sources-description = { $scene } has no matching configured OBS audio sources.
mixer-no-matching-title = No Matching Audio Sources
mixer-no-matching-description = Adjust the search filter to show available audio sources.

## Group titles
mixer-group-all-sources = All Sources
mixer-group-global-fallback = Global

## Error placeholder + retry
mixer-error-title = Mixer Audio Unavailable
mixer-error-description = Could not load audio sources for { $scene }: { $message }
mixer-retry-button-label = Retry
mixer-retry-button-tooltip = Retry loading mixer audio

## Current-source summary row (source_summary / scene_target_summary)
mixer-summary-following-active = Following active OBS scene: { $scene }
mixer-summary-no-scene-selected = No scene selected
mixer-summary-selected-scene = Selected scene: { $scene }
mixer-summary-pinned-scene = Pinned scene: { $scene }
mixer-summary-selected-fallback = Selected scene not set; using active OBS scene: { $scene }
mixer-summary-pinned-selected-fallback = Pinned scene not set; using selected scene: { $scene }
mixer-summary-pinned-active-fallback = Pinned and selected scenes not set; using active OBS scene: { $scene }

## doctor.rs — page chrome, empty state, and all-clear text for the Doctor
## page. Diagnostic finding messages themselves (`doctor-no-role`,
## `doctor-cycle*`, `doctor-edge-*`, etc.) already exist in
## src/services/doctor_service.rs and are NOT redefined here.
doctor-page-title = Doctor
doctor-empty-state-title = Nothing to Check
doctor-empty-state-description = Connect to OBS to run architecture diagnostics.
doctor-summary-row-title = Diagnostics
doctor-rerun-tooltip = Run diagnostics again
doctor-all-clear-title = No problems found
doctor-all-clear-detail = The scene architecture satisfies all checks.

## inventory.rs — page chrome, group titles, YAML import/export UI, and
## status messages. Role names/descriptions themselves are NOT redefined
## here — they already exist in src/domain/role.rs (`role-*`,
## `role-*-desc`, `role-unassigned`) and are reused directly via
## SceneRole::label()/description()/unassigned_label(). The already-present
## `inventory-no-role-assigned` message is also left untouched.
inventory-page-title = Inventory
inventory-empty-state-title = No Scenes
inventory-empty-state-description = Connect to OBS to load the scene list.
inventory-scenes-group-title = OBS Scenes
inventory-scenes-group-description = Drag scenes to order them, and assign roles to control which appear on Live.
inventory-stale-group-title = Stale Registry Entries
inventory-stale-group-description = Entries whose scenes are missing or whose identity conflicts need review. Retained entries are not applied to current scenes.
inventory-remove-stale-tooltip = Remove stale entry
inventory-yaml-row-title = Scene Registry YAML
inventory-yaml-row-subtitle = Export or import scene roles, colors, order, tags, protection flags, and graph rules.
inventory-yaml-filter-name = YAML files
inventory-accent-clear-tooltip = Clear scene accent color
inventory-drag-tooltip = Drag to reorder scene
inventory-accent-dialog-title = Scene accent color
inventory-accent-choose-tooltip = Choose scene accent color

# Shared between the row's Export button, the export FileChooserNative's
# accept label, and (for Cancel) both the export and import dialogs.
inventory-export-button-label = Export
inventory-export-tooltip = Export scene registry to YAML
inventory-import-button-label = Import
inventory-import-tooltip = Import scene registry from YAML
inventory-dialog-cancel-label = Cancel

inventory-export-dialog-title = Export Scene Registry
inventory-export-success = Exported scene registry to { $path }.
inventory-export-error = Export failed: { $error }
inventory-export-no-file = Export failed: no file was selected.

inventory-import-dialog-title = Import Scene Registry
inventory-import-error = Import failed: { $error }
inventory-import-no-file = Import failed: no file was selected.

## window.rs

window-stream-live-tooltip = Streaming live
window-about-tooltip = About SceneDeck
window-refresh-tooltip = Refresh current page

window-stream-status-line = Stream: { $state }{ $elapsed }
window-record-status-line = Record: { $state }{ $elapsed }

window-status-connecting = Connecting to OBS…
window-connect-btn-connecting = Connecting…
window-current-scene-none = Current scene: —
window-status-connected = Connected — OBS { $version }
window-connect-btn-disconnect = Disconnect
window-status-disconnected = Disconnected
window-connect-btn-connect = Connect to OBS
window-live-disconnected-hint = Connect to OBS to use Live controls
window-current-scene = Current scene: { $scene }
window-status-error = Error: { $error }
window-connect-btn-retry = Retry
window-obs-connection-failed = OBS connection failed
window-toast-obs-error = OBS error: { $error }

window-output-kind-stream = Stream
window-output-kind-record = Record

window-sidebar-output-starting = Starting…
window-sidebar-output-stopping = Stopping…
window-sidebar-output-reconnecting = Reconnecting…
window-sidebar-output-working = Working…

window-sidebar-start-stream = Start Stream
window-sidebar-stop-stream = Stop Stream
window-sidebar-start-recording = Start Recording
window-sidebar-stop-recording = Stop Recording

window-selector-profile-label = Profile
window-selector-profile-tooltip = Switch OBS profile
window-selector-collection-label = Collection
window-selector-collection-tooltip = Switch OBS scene collection

## live.rs

live-start-stream-label = Start Stream
live-stop-stream-label = Stop Stream
live-start-record-label = Start Record
live-stop-record-label = Stop Record
live-stream-toggle-tooltip = Start or stop streaming
live-record-toggle-tooltip = Start or stop recording
live-stream-inactive-label = Stream: Inactive
live-record-inactive-label = Record: Inactive
live-copy-last-recording-path-tooltip = Copy last recording path
live-copied-recording-path-tooltip = Copied last recording path
live-copy-recording-path-with-value-tooltip = Copy recording path: { $path }
live-stream-card-title = Stream
live-recording-card-title = Recording
live-current-scene-placeholder = Current scene: —
live-scenes-section-label = Scenes
live-scenes-connect-hint = Connect to OBS to load scenes.
live-audio-section-label = Audio
live-disconnected-title = Connect to OBS to use Live controls
live-disconnected-detail = Use the connection control at the bottom of the sidebar.
live-stream-command-error-label = Stream command failed
live-recording-command-error-label = Recording command failed
live-last-recording-detail = Last recording: { $path }
live-starting-stream = Starting stream…
live-stopping-stream = Stopping stream…
live-reconnecting-stream = Reconnecting stream…
live-starting-recording = Starting recording…
live-stopping-recording = Stopping recording…
live-reconnecting-recording = Reconnecting recording…
live-button-starting = Starting…
live-button-stopping = Stopping…
live-button-reconnecting = Reconnecting…
live-button-working = Working…
live-output-kind-stream = Stream
live-output-kind-record = Record
live-output-label = { $kind }: { $state }
live-output-label-with-elapsed = { $kind }: { $state } · { $elapsed }
live-scenes-no-primary-hint = No Primary-role scenes found. Assign roles in Inventory.
live-audio-empty-hint = No audio inputs configured.
live-cancel-button-label = Cancel
live-start-stream-confirm-heading = Start Stream?
live-start-stream-confirm-body = OBS will start sending the live stream.
live-stop-stream-confirm-heading = Stop Stream?
live-stop-stream-confirm-body = OBS will stop sending the live stream.
live-start-recording-confirm-heading = Start Recording?
live-start-recording-confirm-body = OBS will start a new recording.
live-start-recording-confirm-label = Start Recording
live-stop-recording-confirm-heading = Stop Recording?
live-stop-recording-confirm-body = OBS will stop the current recording.
live-stop-recording-confirm-label = Stop Recording

## audio_card.rs
audio-card-mute-tooltip = Mute input
audio-card-source-path-tooltip = { $scope }: { $path }
audio-card-fader-tooltip = Volume fader
audio-card-lock-tooltip = Lock volume slider
audio-card-fine-plus-tooltip = +1 dB
audio-card-fine-reset-tooltip = Reset to 0.0 dB
audio-card-fine-minus-tooltip = -1 dB
audio-card-meter-tooltip-title = Volume meter: { $channels }
audio-card-meter-tooltip-zones = Green below -20 dB · Yellow to -9 dB · Red above, close to clipping
audio-card-meter-tooltip-indicators = Bar: peak level, with fall-off · Line: loudest peak in 20 s · Dot: loudness · Base: level arriving from the device
audio-card-meter-tooltip-waiting = Volume meter: waiting for OBS levels

## icon.rs
icon-none = No icon
inventory-scene-icon-tooltip = Choose an icon for this scene
mixer-input-icon-tooltip = Choose an icon for this audio source
icon-camera = Camera
icon-desktop = Desktop
icon-game = Game
icon-film = Film
icon-images = Images
icon-television = Television
icon-browser = Browser
icon-terminal = Terminal
icon-code = Code
icon-chat = Chat
icon-guests = Guests
icon-star = Star
icon-alert = Alert
icon-break = Break
icon-countdown = Countdown
icon-start = Start
icon-pause = Pause
icon-stop = Stop
icon-settings = Settings
icon-layers = Layers
icon-microphone = Microphone
icon-headset = Headset
icon-headphones = Headphones
icon-speaker = Speaker
icon-volume = Volume
icon-music = Music
icon-instrument = Instrument
icon-radio = Radio
icon-call = Call
icon-waveform = Waveform

## meter.rs
audio-meter-zone-nominal = Green
audio-meter-zone-warning = Yellow
audio-meter-zone-error = Red
audio-meter-channel-mono = Mono
audio-meter-channel-left = Left
audio-meter-channel-right = Right
audio-meter-channel-front-left = Front left
audio-meter-channel-front-right = Front right
audio-meter-channel-front-center = Front centre
audio-meter-channel-lfe = LFE
audio-meter-channel-rear-left = Rear left
audio-meter-channel-rear-right = Rear right
audio-meter-channel-side-left = Side left
audio-meter-channel-side-right = Side right
audio-meter-channel-numbered = Channel { $index }

## hotkey.rs
hotkey-modifier-ctrl = Ctrl+
hotkey-modifier-alt = Alt+
hotkey-modifier-shift = Shift+
hotkey-modifier-super = Super+
hotkey-modifier-ctrl-alt = Ctrl+Alt+
hotkey-modifier-ctrl-shift = Ctrl+Shift+
hotkey-style-plain = Digit only
hotkey-style-ctrl = Ctrl + digit
hotkey-style-alt = Alt + digit
hotkey-style-shift = Shift + digit
hotkey-style-super = Super + digit
hotkey-style-ctrl-alt = Ctrl+Alt + digit
hotkey-style-ctrl-shift = Ctrl+Shift + digit
hotkey-style-leader = Leader key, then digit
hotkey-leader-space = Space
hotkey-leader-comma = Comma
hotkey-leader-semicolon = Semicolon
hotkey-leader-backslash = Backslash
hotkey-leader-grave = Backtick
hotkey-shortcut-leader = { $leader } then { $digit }
hotkey-hint-plain = Press 1 … 0
hotkey-hint-modifier = Press { $modifier }1 … 0
hotkey-hint-leader = Press { $leader }, then 1 … 0
hotkey-hint-leader-armed = Leader armed — press 1 … 0
hotkey-hint-empty-slot = No scene in slot { $slot }

## scene_card.rs
scene-card-tooltip = { $status } ({ $role })
scene-card-tooltip-with-hotkey = { $status } ({ $role }) · { $hotkey }
scene-card-role-suffix = { $role } scene
scene-card-tooltip-ready = Switch to this scene

## status_bar.rs
status-bar-stream-inactive = Stream: Inactive
status-bar-record-inactive = Record: Inactive
status-bar-fps-placeholder = FPS —
status-bar-cpu-placeholder = CPU —
status-bar-bitrate-placeholder = Bitrate —
status-bar-fps = FPS { $value }
status-bar-cpu = CPU { $value }%
status-bar-bitrate = Bitrate { $value } kbps
status-bar-dropped = { $count } dropped
status-bar-dropped-placeholder = Dropped —

## settings.rs

settings-page-title = Settings
settings-appearance-title = Appearance
settings-appearance-description = GNOME apps should follow the system style by default.
settings-theme-mode-system = System
settings-theme-mode-light = Light
settings-theme-mode-dark = Dark
settings-color-scheme-title = Color Scheme
settings-color-scheme-subtitle = Follow the system preference or force light / dark
settings-motion-title = Motion
settings-motion-subtitle = How much the interface animates. Live indicators stay readable at every level.
settings-motion-full = Full
settings-motion-reduced = Reduced
settings-motion-off = Off
settings-theme-title = Theme
settings-theme-status-title = Theme Status
settings-theme-status-initial = Theme loaded.
settings-failed-to-save = Failed to save: { $err }
settings-custom-css-title = Custom CSS
settings-custom-css-subtitle = Load separate user CSS files for light and dark mode
settings-custom-light-css-title = Custom Light CSS Path
settings-custom-dark-css-title = Custom Dark CSS Path
settings-reload-css-title = Reload Custom CSS
settings-reload-css-subtitle = Reapply the selected theme and the matching light/dark custom CSS file.
settings-reload-button = Reload
settings-language-title = Language
settings-language-description = Changes take effect after restarting SceneDeck.
settings-display-language-title = Display Language
settings-display-language-subtitle = Pick a language, or follow the system locale.
language-system-default = System Default
settings-language-status-title = Language Status
settings-language-status-initial = Restart to apply a changed language.
settings-language-saved = Language saved. Restart SceneDeck to apply it.
settings-obs-connection-title = OBS Connection
settings-obs-connection-description = WebSocket settings for OBS Studio (default port: 4455).
settings-host-title = Host
settings-port-title = Port
settings-password-title = Password (optional)
settings-obs-status-title = OBS Status
settings-invalid-port = Invalid port number.
settings-saved = Settings saved.
settings-password-saved = Password saved to keyring.
settings-keyring-error = Keyring error: { $err }
settings-output-safety-title = Output Safety
settings-output-safety-description = Optional confirmations for critical stream and recording actions.
settings-confirm-start-stream-title = Confirm Start Stream
settings-confirm-start-stream-subtitle = Ask before starting the live stream.
settings-confirm-stop-stream-title = Confirm Stop Stream
settings-confirm-stop-stream-subtitle = Ask before stopping the live stream.
settings-confirm-start-recording-title = Confirm Start Recording
settings-confirm-start-recording-subtitle = Ask before starting a recording.
settings-confirm-stop-recording-title = Confirm Stop Recording
settings-confirm-stop-recording-subtitle = Ask before stopping a recording.
settings-hotkeys-title = Scene Hotkeys
settings-hotkeys-description = Switch Live scenes from the keyboard. Slot numbers follow the scene order set in Inventory.
settings-hotkeys-enabled-title = Enable Scene Hotkeys
settings-hotkeys-enabled-subtitle = Number keys switch the scene cards on the Live page, in card order.
settings-hotkeys-style-title = Key Combination
settings-hotkeys-style-subtitle = Which keys switch a scene. Bare digits stand down while a text field has focus.
settings-hotkeys-leader-title = Leader Key
settings-hotkeys-leader-subtitle = First key of the two-stroke shortcut, vim style.
settings-hotkeys-timeout-title = Leader Timeout
settings-hotkeys-timeout-subtitle = How long the leader waits for its digit, in milliseconds.
settings-hotkeys-preview-title = Current Bindings
settings-hotkeys-preview-subtitle = { $first } … { $last } switch the first { $count } scenes on Live.
settings-hotkeys-preview-disabled = Scene hotkeys are turned off.
settings-obs-not-connected = Not connected to OBS.
settings-obs-connecting = Connecting to OBS…
settings-obs-connected = Connected — OBS { $version }
settings-obs-error = Error: { $err }
settings-theme-loaded = Loaded { $theme } ({ $variant }).
settings-theme-loaded-with-warnings = Theme loaded with warnings.
theme-variant-light = Light
theme-variant-dark = Dark

## theme.rs

theme-adwaita-default-name = Adwaita Default
theme-adwaita-default-desc = Neutral styling that follows GNOME defaults.
theme-scenedeck-dark-name = SceneDeck Dark
theme-scenedeck-dark-desc = A reserved dark console theme for live operation.
theme-scenedeck-light-name = SceneDeck Light
theme-scenedeck-light-desc = A crisp light console theme with restrained contrast.
theme-obs-name = OBS
theme-obs-desc = OBS Studio's default look, read through libadwaita.
theme-obsidian-name = Obsidian
theme-obsidian-desc = High-legibility graphite surfaces with cool accents.
theme-nord-name = Nord
theme-nord-desc = Cool blue-gray surfaces with frost-toned accents.
theme-dracula-inspired-name = Dracula Inspired
theme-dracula-inspired-desc = A dark expressive palette using original CSS.
theme-solarized-dark-name = Solarized Dark
theme-solarized-dark-desc = Low-glare contrast with teal and amber accents.
theme-high-contrast-name = High Contrast
theme-high-contrast-desc = Stronger outlines and contrast for critical controls.
theme-stream-red-name = Stream Red
theme-stream-red-desc = Broadcast-oriented red accents for live states.
theme-studio-purple-name = Studio Purple
theme-studio-purple-desc = Controlled purple accents without overpowering surfaces.
theme-ubuntu-violet-name = Ubuntu Violet
theme-ubuntu-violet-desc = Ubuntu-inspired violet surfaces with a warm live accent.
theme-custom-css-read-failed = Custom CSS could not be read from { $path }: { $err }
theme-custom-css-no-matching-file = Custom CSS is enabled but no matching light/dark file is set.
theme-css-no-display = { $label } was not loaded because no GTK display is available.
theme-css-parse-error = { $label } CSS parse error: { $message }
## stats.rs — live streaming telemetry
stats-page-title = Stream Statistics
stats-page-subtitle = Live telemetry polled from OBS once a second while connected.
stats-popout-button = Open stats in a separate window
stats-gauge-fps = FPS
stats-gauge-frame-time = Frame time (ms)
stats-gauge-dropped = Dropped frames
stats-gauge-congestion = Congestion
stats-chart-fps = Frames per second
stats-chart-frame-time = Average frame render time (ms)
stats-chart-output-skipped = Output frames skipped per sample
stats-chart-render-skipped = Render frames missed per sample
stats-card-render-frames = Render frames missed
stats-card-output-frames = Output frames skipped
stats-card-stream-frames = Stream frames dropped
stats-card-frame-time = Average frame render time
stats-card-cpu = OBS CPU usage
stats-card-memory = OBS memory usage
stats-card-bitrate = Stream bitrate
stats-value-placeholder = —
stats-value-frames = { $skipped } of { $total } ({ $percent }%)
stats-value-ms = { $value } ms
stats-value-percent = { $value }%
stats-value-mb = { $value } MB
stats-value-kbps = { $value } kbps

## ui/pages/help.rs — onboarding guide. Bodies are multi-line on purpose: each
## line is rendered as its own line inside an expandable topic.
help-page-title = Help & Onboarding
help-hero-title = Welcome to SceneDeck
help-hero-description = A native Linux control surface for OBS Studio. This guide walks through the first connection, shows how to keep the Live page down to the scenes you actually switch, and explains every page in the sidebar. Expand a topic to read it.
help-expand-hint = Tap a topic to expand it.

help-open-settings = Open Settings
help-open-inventory = Open Inventory
help-open-doctor = Open Doctor
help-open-live = Open Live
help-open-mixer = Open Mixer
help-open-graph = Open Graph
help-open-stats = Open Stats

help-group-start-title = Getting started
help-group-start-description = The shortest path from a fresh install to switching scenes.

help-quickstart-title = Five steps to your first scene switch
help-quickstart-subtitle = Do these in order the first time
help-quickstart-body =
    1. In OBS Studio, open Tools → WebSocket Server Settings and tick "Enable WebSocket server". Leave OBS running.
    2. In that same OBS dialog, press "Show Connect Info" and note the Server Port (4455 by default) and the Server Password.
    3. In SceneDeck, open Settings and fill in Host, Port and Password. Host stays 127.0.0.1 when OBS runs on this same computer.
    4. Press Connect at the bottom of the sidebar. The status line above the button turns green and reads "Connected".
    5. Open Inventory and give the role "Primary" to the scenes you want to switch during a show. Those — and only those — become cards on the Live page.

help-concepts-title = How SceneDeck thinks about your setup
help-concepts-subtitle = Roles, the registry, and what never touches OBS
help-concepts-body =
    Saved connections have separate credentials. Each connection, endpoint and scene collection has its own show workspace.
    Roles, tags, favorites, order, colors, icons and named deck views are local curation; changing them does not edit OBS scenes. Search and saved views do not retarget explicit action bindings.
    Known scene UUIDs preserve metadata across renames. A replacement with a different UUID does not inherit that identity.

help-group-connect-title = Connecting to OBS
help-group-connect-description = On this machine, or across the room.

help-connect-local-title = Connecting to OBS on this computer
help-connect-local-subtitle = The default case — host 127.0.0.1, port 4455
help-connect-local-body =
    127.0.0.1 is the address a computer uses to talk to itself, so it is the right Host whenever OBS and SceneDeck run side by side.
    The port must match the Server Port in OBS's WebSocket Server Settings. OBS uses 4455 unless you changed it.
    If OBS has "Enable Authentication" ticked, paste its password into Settings → Password. SceneDeck stores it in your desktop keyring (the same place your browser keeps saved logins), never in the plain-text config file.
    Press Connect in the sidebar, or press Ctrl+R at any time to reconnect.

help-connect-remote-title = Connecting to OBS on another machine
help-connect-remote-subtitle = Streaming PC in the corner, control from your laptop
help-connect-remote-body =
    This is the two-computer setup: OBS runs on the machine capturing and encoding, SceneDeck runs on the machine in front of you. Both must be on the same network.
    On the OBS machine, in Tools → WebSocket Server Settings: tick "Enable WebSocket server", tick "Enable Authentication", and set a password you are willing to type. Do not leave authentication off — anyone on the network who can reach the port could otherwise start and stop your stream.
    Find the OBS machine's address on that machine. On Linux run `ip addr` and look for an address like 192.168.1.42; on Windows run `ipconfig` and read the IPv4 Address; on macOS it is in System Settings → Network. It is the address of the machine, not of OBS.
    Allow the port through the OBS machine's firewall. On Linux with ufw that is `sudo ufw allow 4455/tcp`; on Windows, allow OBS in Windows Defender Firewall for Private networks.
    In SceneDeck's Settings, put that address in Host (for example 192.168.1.42), keep Port at 4455, and paste the password. Press Connect.
    Prefer a wired connection for the OBS machine. Scene switches over Wi-Fi still work, but a dropped packet is a delayed cut.
    A tip that saves a show: reserve a fixed address for the OBS machine in your router's DHCP settings, so the Host you saved keeps working after a reboot.

help-connect-share-title = Letting a co-host or moderator run the deck
help-connect-share-subtitle = A second SceneDeck, pointed at the same rig
help-connect-share-body =
    OBS's WebSocket server accepts more than one client at once, so a second person on a second computer can run their own copy of SceneDeck against the same rig — the same setup as the previous topic, done twice.
    There is no per-person login: whoever has the Host, Port, and password in their Settings has exactly the same control you do, including starting and stopping the stream. Only hand it to someone you would hand your OBS session to.
    Trim their view first. Roles and hidden scenes live in your local Inventory, not in OBS, so set those up on your own machine and have your collaborator export or recreate the Scene Registry YAML rather than starting from a full, unfiltered scene list.
    Do not forward the WebSocket port to the internet so a remote collaborator can reach it. Put both of you on the same VPN or Tailscale network, or tunnel the connection over SSH, and point Host at the tunnel's address instead.
    Everyone connected sees the same live state — a scene switch or fader move from either side shows up for both, immediately, the same way it would if you were sitting at the same keyboard.

help-connect-trouble-title = When Connect does not work
help-connect-trouble-subtitle = Read the error, then work down this list
help-connect-trouble-body =
    "Connection refused" almost always means the WebSocket server is not enabled in OBS, or the port does not match. Re-check both in Tools → WebSocket Server Settings.
    A connection that hangs and then times out usually means a firewall is dropping the traffic, or the Host address belongs to a different machine than you think.
    "Authentication failed" means the password is wrong. Re-enter it in Settings; the field is write-only, so it looks empty even when a password is saved.
    Nothing at all in the sidebar status? Confirm OBS is actually running and not sitting behind a modal dialog of its own.
    SceneDeck reconnects on its own after a dropped link, and Ctrl+R forces the attempt immediately.

help-group-scenes-title = Curating your scenes
help-group-scenes-description = The Live page should show what you switch to, and nothing else.

help-scenes-hide-title = Hiding scenes you never cut to
help-scenes-hide-subtitle = The single most useful thing to set up first
help-scenes-hide-body =
    A working OBS setup collects scenes that exist only to be nested inside other scenes, or that were built for one test and never deleted. Putting them on a live control surface is how a wrong cut happens.
    SceneDeck shows a Live card only for scenes whose role is Primary. Every other role is hidden from Live, so "hiding a scene" simply means giving it any role other than Primary.
    Open Inventory. Each OBS scene gets a row with a role selector on the right.
    Set Primary on the handful of scenes you actually cut to on air. Everything else gets one of: Secondary (a real scene you sometimes need but do not want on Live), Module (an overlay or lower-third that only ever gets nested inside another scene), Raw (a bare camera or capture wrapper), Debug (a test scene), or Archive (kept for later, out of the way).
    Scenes with no role assigned are also kept off Live, so leaving something Unassigned hides it too. Assigning a role on purpose is still better: the Doctor page flags Unassigned scenes so you notice new ones.
    The change takes effect immediately — go back to Live and the card is gone. Nothing in OBS changed.

help-scenes-order-title = Ordering, colours and icons
help-scenes-order-subtitle = Make the right card the obvious one
help-scenes-order-body =
    Drag a scene by the handle at the left of its Inventory row to set the order. Live cards follow that order, and so do the number shortcuts, so the card you press 1 for is the one at the top.
    The accent colour picker tints that scene's Live card. Reserve a strong colour for the scenes with consequences — the "we are live" scene, the sponsor read — so your eye lands on them.
    The icon picker at the left of each row puts a symbol on the scene's Live card. Thirty icons are available, plus a "no icon" entry that clears it.
    Order, colours and icons are stored in the local registry, not in OBS.

help-scenes-registry-title = Backing up and moving your setup
help-scenes-registry-subtitle = The Scene Registry YAML row in Inventory
help-scenes-registry-body =
    Inventory exports the current workspace registry to YAML, including deck views, actions, preflight rules and audio presets. Import replaces this workspace’s curation; it does not overwrite other workspaces or OBS credentials.
    Imported bindings and quick actions start disabled. Review identities before enabling them. Known UUIDs follow renames; a reused name with a different UUID is a different target. Stale entries can be removed in Inventory.

help-group-operate-title = Running a show
help-group-operate-description = The pages you actually use while the stream is up.

help-live-title = The Live page
help-live-subtitle = Scene wall, Studio, audio and outputs
help-live-body =
    Live is a scene wall. Scene previews in Settings adds shaded thumbnails and larger cards; turning it off restores compact tiles.
    In direct mode, cards and shortcuts switch Program. The Studio switch in the topbar follows OBS: cards then stage Preview, and Take sends it to Program. Transition and supported duration controls appear only in Studio mode.
    Expand the audio strip below the scenes for more controls, or open Mixer. Stream, recording, replay buffer and virtual camera share one output bar.
    Sources in the topbar opens source visibility and detailed input controls. Inspecting a scene there never switches Program.
    No cards after connecting? Assign the Primary role in Inventory to the scenes you want on Live.

help-hotkeys-title = Switching scenes from the keyboard
help-hotkeys-subtitle = Ctrl+1 … Ctrl+0 by default, and configurable
help-hotkeys-body =
    The first ten switchable scenes in the saved Inventory order receive digits 1–9, then 0. Each visible card shows its assigned digit; the scene hint names the configured binding.
    Search, saved decks and showing favorites first do not renumber slots. A shortcut can activate a filtered-out scene. Reordering the unfiltered Inventory changes the assignments.
    Settings → Scene Hotkeys chooses how the digit is pressed. A modifier plus the digit (Ctrl by default) cannot fire by accident while you type. The bare digit is fastest. The leader style is the vim-like option: press the leader key, release it, then press the digit; the caption reads "Leader armed" while it waits.
    Scene shortcuts act only on Live and pause while a text field has focus. A digit with no assigned scene reports an empty slot without switching anything.

help-audio-title = Audio: the Mixer page and the meters
help-audio-subtitle = What the coloured bars are telling you
help-audio-body =
    Compact audio on Live expands for quick adjustments; Mixer provides full faders. Global OBS audio devices come first, followed by audio sources in the current scene, including nested scenes and groups.
    Mixer modes decide which scene's audio you are looking at. Active follows the OBS program scene. Selected loads one scene and stays there. Pinned keeps a chosen scene as a permanent target while OBS moves on.
    The meter beside each fader runs -60 dB at the bottom to 0 dB at the top and uses OBS's own thresholds: green below -20 dB for music and background, yellow from -20 to -9 dB where speech belongs, red above -9 dB where clipping starts. Nothing should sit in the red.
    One column means a mono source; two mean stereo, left then right. If only the left column moves, half your viewers hear nothing from it.
    The line above the fill is the loudest peak of the last twenty seconds, which is the quickest way to catch a clip you missed. The square at the very bottom is the level arriving from the device, before the fader — if that is too hot, no fader move will fix it.
    The lock button on a card only freezes SceneDeck's own slider. It does not lock anything in OBS.

help-outputs-title = Streaming, recording, replay and virtual camera
help-outputs-subtitle = And the confirmations that stop an accident
help-outputs-body =
    The output bar on Live contains stream, recording, replay buffer and virtual camera controls. More output controls opens recording pause/resume, split/chapter actions and recent results.
    Settings → Output Safety controls confirmations for starting and stopping streaming and recording. Stops ask for confirmation by default.
    OBS events and refreshed observations determine the displayed state. An acknowledged request alone does not prove that an output started or that a file was saved. Paths belong to the OBS host.

help-group-inspect-title = Checking your setup
help-group-inspect-description = Find the surprise before it happens on air.

help-doctor-title = Doctor
help-doctor-subtitle = Structural problems, sorted by severity
help-doctor-body =
    Doctor reads your scene list, your role assignments, and the nesting between scenes, then reports what looks wrong as Errors, Warnings, and Info.
    Typical findings: scenes with no role assigned, remembered scenes that no longer exist in OBS, circular nesting, and a scene nested inside another in a direction your roles say should not happen.
    It re-runs every time you open the page, and the Re-run button forces it.
    Worth a look after any change to your OBS setup, and once more before you go live.

help-graph-title = Graph
help-graph-subtitle = Which scenes are inside which
help-graph-body =
    Nesting one scene inside another is how overlays and shared layouts are built in OBS, and it is also how a scene ends up depending on something you forgot about.
    Graph lists every parent scene and what it contains, and marks each relationship against your role rules: fine, questionable, or forbidden.
    Use it to answer "what breaks if I change this scene?" before changing it.

help-stats-title = Stats
help-stats-subtitle = Whether the machine is keeping up
help-stats-body =
    Gauges for FPS, frame render time, dropped frames, and network congestion turn amber and then red as each value degrades — dropped frames warn at 1%, congestion at 30%.
    Trend charts hold roughly the last two minutes, and the bar charts show when frames were lost rather than a running total, which is what tells you whether a stutter was one bad moment or a trend.
    Samples are collected the whole time you are connected, so opening Stats mid-stream shows the minutes leading up to now instead of starting empty.
    Frame counters come from OBS and reset when OBS or the stream output restarts.

help-group-personalise-title = Making it yours
help-group-personalise-description = Appearance, language, and where your files live.

help-appearance-title = Themes and appearance
help-appearance-subtitle = Including a look that matches OBS itself
help-appearance-body =
    Colour Scheme follows your desktop's light/dark preference by default, or you can force one side.
    Themes are light/dark-aware families: pick one and it applies the variant matching the current colour scheme. The OBS family mirrors OBS Studio's own look, for a control surface that does not clash with the app it drives.
    Custom CSS takes separate light and dark files, so a custom look follows the colour scheme too. Reload Custom CSS picks up edits without a restart.
    Motion controls how much the interface animates — set it to Reduced or Off if movement is distracting or the machine is busy.

help-files-title = Where SceneDeck keeps things
help-files-subtitle = Config, registry, and your OBS password
help-files-body =
    config.json stores settings and saved connections under $XDG_CONFIG_HOME/scenedeck (usually ~/.config/scenedeck). show-workspaces.json stores scoped curation; the legacy registry.json is preserved and adopted once.
    bookmarks.json stores operator notes. Optional session-history.json lives under $XDG_STATE_HOME/scenedeck (usually ~/.local/state/scenedeck).
    Passwords stay in Secret Service, separately for each connection. JSON/YAML exports do not carry credentials. Back up the relevant files and transfer passwords separately.

help-shortcuts-title = Keyboard shortcuts
help-shortcuts-subtitle = The whole list
help-shortcuts-body =
    F1 — open this guide.
    Ctrl+R — reconnect to OBS.
    Ctrl+, — open Settings.
    Ctrl+Q — quit SceneDeck.
    Ctrl+1 … Ctrl+0 on Live — activate the scene assigned to that Inventory slot, even if filtered out. Configure the combination in Settings → Scene Hotkeys.
    { palette-title }

## ui/window.rs — first-run welcome dialog
welcome-dialog-heading = Welcome to SceneDeck
welcome-dialog-body = It looks like this is your first run. The Help page walks through connecting to OBS — including OBS on another computer — and shows how to keep the Live page down to the scenes you actually switch to. It takes a couple of minutes and saves a few mistakes.
welcome-dialog-later = Not now
welcome-dialog-open = Read the guide
window-help-tooltip = Help & onboarding guide

# OBS controls and accessible alternatives
window-stop-disconnect-label = Stop stream/record & disconnect
window-stop-stream-disconnect-label = Stop stream & disconnect
window-stop-record-disconnect-label = Stop recording & disconnect
disconnect-confirm-heading = Stop stream and recording before disconnecting?
disconnect-confirm-body = Stream: { $stream }. Recording: { $recording }. Active streaming and recording will stop before SceneDeck disconnects. Replay buffer and virtual camera are not stopped by this action.
live-scenes-obs-empty = OBS has no scenes in this collection. Create a scene in OBS, then refresh SceneDeck.
live-organize-scenes = Organize scenes
mixer-secondary-options = Grouping and search
inventory-move-up = Move { $scene } up
inventory-move-down = Move { $scene } down
graph-relationship-list = Scene relationships — readable list
graph-no-dependencies = No nested scene dependencies
audio-source-mute = Mute { $source }
audio-source-lock = Lock local fader for { $source }
audio-source-volume = Volume for { $source }
audio-source-meter = Audio peak meter for { $source }
audio-meter-no-level = No visible level
audio-meter-no-observation = No audio level observation

# Session activity

workflow-activity-title = Recent activity
workflow-activity-detail = Current session only, newest 30 shown. Requests, observations and failures are separate. Disconnects leave gaps; timing does not prove causation.
workflow-check-item = Source visibility
workflow-activity-requested = Requested
workflow-activity-observed = Observed
workflow-activity-failed = Failed
workflow-activity-connection = Connection


# Native OBS inspector
control-refresh = Refresh OBS observations
control-inspection-note = Inspect any scene without changing Program. Visibility changes affect the owning scene or group.
control-disconnected = Connect to OBS to inspect sources.
control-loading = Reading OBS state…
control-observed = Last observation: { $seconds } seconds ago. Refresh before relying on this state.
control-scene-selector = Scene to inspect — does not change Program
control-request-hint = Refresh to load native OBS controls and source visibility.
control-resume = Resume recording
control-pause = Pause recording
control-split = Split recording file
control-replay-stop = Stop replay buffer
control-replay-start = Start replay buffer
control-replay-save = Save replay
control-camera-stop = Stop virtual camera
control-camera-start = Start virtual camera
control-chapter-name = Recording chapter name
control-chapter = Create chapter
control-host-path = Last replay path on the OBS host: { $path }
control-output-controls = Recording, replay and virtual camera
control-source-search = Search sources or owner paths
control-inspect-source = Inspect source settings and usage
control-hidden-parent = An ancestor item is hidden. Enabling this item alone may not make it visible.
control-sources-empty = No scene items in this observation.
control-shared-source = Audio, filters, media and text affect this source everywhere it is used. Visibility is local to its owner.
control-used-in = Used in — structural paths
control-image-refresh = Capture still image
control-image-alt = An on-demand OBS scene still image
control-image-error = OBS image could not be decoded.
control-image-note = Still image from the last capture. It is not live video.
control-studio-enable = Enable Studio Mode
control-program = Program: { $scene }
control-preview = Preview: { $scene }
control-stage = Stage inspected scene
control-take = Take
control-transition = Current scene transition
control-duration = Transition duration in milliseconds
control-apply = Apply
control-studio = Studio and transition
control-scene-mode-changed = Studio mode changed; refresh before activating the scene. The action was not retried.
control-filters = Source filters
control-text-file = This text source reads a file. Edit the file on the OBS host.
control-text-save = Save text to OBS
control-text = Built-in text source
control-media-play = Play
control-media-pause = Pause
control-media-stop = Stop
control-media-restart = Restart
control-media-previous = Previous
control-media-next = Next
control-media-seconds = Media position in seconds
control-media-seek = Seek
control-media = Media transport
control-monitor-host = Audio monitoring plays on the OBS host, not through SceneDeck.
control-monitor-off = Monitoring off
control-monitor-only = Monitor only — mute output
control-monitor-output = Monitor and output
control-audio-sync = Audio sync offset (milliseconds)
control-audio-balance = Balance (0 = left, 0.5 = center, 1 = right)
control-audio-track = Audio track { $track }
control-audio-tracks = Audio track routing
control-advanced-audio = Advanced source audio


# Operating workspace
workspace-action-acknowledged = Request acknowledged; state refreshed from OBS
workspace-replay-observed = OBS reported a saved replay: { $path }
workspace-disk-space = OBS recording disk space: { $value } MiB
control-stale = Last-known state. Refresh OBS observations to enable changes.
control-no-matches = No sources match this search. Clear the search to show all items.


# OBS recording volume and session activity
stats-recording-disk = Free space on OBS recording volume
stats-recording-disk-value = { $value } MiB
stats-activity-title = Recent activity and chart context
stats-activity-detail = Newest 15 session entries. Select an entry to mark its time on the sample charts. Connection gaps, missing samples and counter resets limit interpretation. Timing does not prove causation.
stats-activity-clear-marker = Clear chart marker
stats-activity-marker-visible = Dashed lines mark the selected event between observed samples. Charts use sample spacing.
stats-activity-marker-unavailable = No chart marker: this event is outside retained samples, in a sampling gap, or affected by a clock change.
stats-activity-empty = No operations observed in this session yet.
stats-activity-requested = Requested
stats-activity-observed = Observed
stats-activity-failed = Failed
stats-activity-connection = Connection
stats-activity-entry = { $time } · { $origin } · { $description }
scene-card-stage-hint = Stage this scene in Preview. Use Take to change Program.
scene-card-preview-label = Preview
hotkey-hint-stage-take = { $binding } · Stage in Preview; use Take to change Program.


header-obs-setup = OBS setup
header-obs-unknown = Unknown
header-obs-context = Profile: { $profile } · Scene collection: { $collection }

status-bar-connection-detail = { $state } · OBS { $version }

output-results-title = Output results
output-results-detail = Requests and OBS observations are separate. A saved event may come from another controller. Paths belong to the OBS host. Newest 24 results are kept for this session.
output-results-copy-path = Copy OBS host path
output-results-record-stop = Stop recording
output-results-acknowledged = Acknowledged
output-results-unconfirmed = Unconfirmed
output-results-host-path = OBS host path: { $path }
control-image-caption = Snapshot of { $scene }, captured { $seconds } seconds ago. Refresh image to capture again.


doctor-structural-title = Scene structure and roles


record-elapsed-tooltip = Wall-clock time since OBS reported recording started, including pauses. This is not the recorded file duration.

window-open-navigation = Open navigation

# Scene wall redesign
sources-title = Sources
sources-back = Back to sources
sources-select-hint = Select a source to inspect its settings.
sources-text-conflict = This text changed in OBS. Cancel to load the current text before saving.
inventory-name-column = Scene
inventory-role-column = Role
inventory-actions-column = Options
inventory-scene-options = Options for { $scene }
inventory-columns-button = Column widths
inventory-name-width = Scene width
inventory-role-width = Role width
inventory-reset-widths = Reset column widths
settings-scene-previews = Scene previews
settings-scene-previews-description = Show shaded scene thumbnails on larger Live cards.
live-audio-expand = Expand audio
live-audio-collapse = Collapse audio
studio-mode-label = Studio
scene-card-image-pending = Thumbnail unavailable
scene-card-program-label = Program
output-options = More output controls
output-replay-label = Replay · { $state }
output-camera-label = Virtual camera · { $state }

output-camera-name = Virtual cam
output-replay-name = Replay

# Hardening: save completion and safe custom-theme dependencies.
app-persistence-drain-timeout = Pending saves have not finished. Retry waits again; Cancel keeps the application open.
app-persistence-drain-worker-failed = A background operation failed. Changes may not have reached disk. Cancel keeps the application open so you can check and save again. Retry waits for later saves but cannot recover the failed operation. Close Anyway acknowledges that its changes may be lost; other pending saves will still be completed.
app-persistence-close-anyway = Close Anyway (Changes May Be Lost)
theme-custom-css-dependencies-forbidden = Custom CSS must be self-contained: @import, image url() dependencies, and malformed or excessively nested rules are not allowed. Inline the styles and use gradients or colors instead of external images.

# Topology readiness: no structural verdict before a complete observation.
topology-missing-title = Scene topology not observed
topology-missing-description = Connect to OBS and refresh observations before relying on scene relationships or structural checks.
topology-loading-title = Reading scene topology…
topology-loading-description = Waiting for a complete observation from OBS. Structural checks are not available yet.
topology-failed-title = Scene topology could not be read
topology-failed-description = Structural checks are unavailable. Refresh to retry. OBS reported: { $error }
topology-empty-title = No scenes observed
topology-empty-description = OBS successfully reported no scenes in this collection. Create a scene in OBS, then refresh.
doctor-refresh-topology-tooltip = Refresh scene topology from OBS and run structural checks again.
media-state-none = No media
media-state-playing = Playing
media-state-opening = Opening
media-state-buffering = Buffering
media-state-paused = Paused
media-state-stopped = Stopped
media-state-ended = Ended
media-state-error = Error
media-state-unknown = Unknown
workspace-warning-identity = Input identity lookup failed: { $detail }
workspace-warning-truncated = Source inventory truncated: inspection budget exhausted
workspace-warning-container = { $path }: { $detail }
workspace-warning-visibility = { $owner } / { $item } visibility unavailable: { $detail }
workspace-warning-recursion = Nested scene scan stopped at { $path }
workspace-warning-usage-partial = Usage search is partial: limited to { $scenes } scenes and { $items } inspected items
workspace-warning-omitted = { $count ->
    [one] 1 more source scan warning omitted
   *[other] { $count } more source scan warnings omitted
}
controller-workspace-superseded = Workspace request superseded before mutation
controller-workspace-admission-full = Workspace operation admission full or retired
controller-workspace-text-changed = Text changed in OBS. Refresh before saving
controller-workspace-action-cancelled = Workspace action cancelled before mutation
controller-endpoint-changed = The OBS endpoint changed; reconnect and refresh
controller-collection-changed = The OBS collection changed; refresh before continuing
controller-collection-changed-during-read = The OBS collection changed during the read; results discarded
controller-audio-changed = Audio changed during observation; refresh required
controller-outputs-still-active = OBS outputs remain active after stop
controller-shutdown-unconfirmed = Disconnected; output shutdown unconfirmed: { $detail }; commands were not retried
controller-stop-deadline = graceful stop deadline exceeded
controller-connection-deadline = OBS connection deadline exceeded
controller-publication-backlog = OBS notification publication backlog exceeded its bound; reconnect and refresh; output evidence may be incomplete
controller-event-connection-ended = OBS event connection ended; refresh required

# Scene identity changes
registry-scene-rename-conflict = Scene “{ $old }” was renamed to “{ $new }”, but that name already has local metadata. Both entries were kept; review Inventory.
registry-scene-rename-backlog = Too many registry updates arrived during import. Review Inventory after the import finishes.

# Collection-specific curation
workspace-curation-title = Show workspace
workspace-curation-loading = Loading this collection’s curation…
workspace-curation-unavailable = Connect to OBS and wait for its scene collection.
workspace-curation-failed = Could not load this workspace: { $error }
workspace-curation-retry = Retry

# Saved OBS connections
settings-connections-description = Save a setup for each OBS connection. Selecting a setup does not reconnect; use Connect saved setup when ready.
settings-connections-picker = Saved connection
settings-connection-name = Connection name
settings-connection-add = Add connection
settings-connection-connect = Connect saved setup
settings-connection-new-name = New connection { $number }
settings-connection-invalid = Enter a name, a host, and a port from 1 to 65535.
settings-obs-connected-profile = Connected to { $name } ({ $endpoint }), OBS { $version }

inventory-retained-identity = { $role } · Retained after a name or identity conflict; not applied to a scene.

inventory-scene-favorite = Favorite
inventory-scene-tags = Tags (separated by commas)
inventory-apply-tags = Apply tags

deck-search-placeholder = Search scenes and tags
deck-favorites-only = Favorites only
deck-reset = Show all
deck-no-results = No scenes match this view. Clear the filters to show all scenes.
deck-reorder-hint = Clear filters to reorder scenes.

deck-menu = Decks
deck-none = No saved decks yet.
deck-remove = Remove deck “{ $name }”
deck-save-title = Save view as deck
deck-save-help = Choose a unique name. The current search and favorites filter will be saved for this workspace.
deck-cancel = Cancel
deck-save = Save
deck-name = Deck name

## Command palette
palette-title = Command palette (Ctrl+K)
palette-search = Search scenes and actions…
palette-open-page = Open { $page }
palette-stage-scene = Stage { $name }
palette-switch-scene = Switch to { $name }
palette-open-source = Open source { $name }
palette-unmute = Unmute { $name }
palette-restart = Restart { $name }
palette-disable-filter = Disable { $filter } · { $name }
palette-enable-filter = Enable { $filter } · { $name }
palette-scope-changed = OBS actions are unavailable in this palette. Reopen it after connecting or changing workspace.
palette-empty = No matching actions.
palette-hint = Use ↑ / ↓ to select, Enter to run, Esc to close. Saved media and filter actions work across pages.
palette-unavailable = Unavailable: check the connection, refresh OBS state, or review the action’s source.

## Custom action shortcuts
binding-title = Action shortcuts
binding-description = Bindings belong to this workspace and work on every page except while typing. Conflicting bindings stay inactive. Imported bindings start disabled.
binding-manage = Manage action shortcuts
binding-invalid = Use Ctrl, Alt or Super with a character, or a function key F2–F24.
binding-reserved = This shortcut is reserved by the application.
binding-slots-conflict = This shortcut conflicts with the scene slot settings.
binding-duplicate = Another enabled action already uses this shortcut.
binding-empty = No action shortcuts yet.
binding-enabled = Enabled
binding-edit = Edit shortcut
binding-remove = Remove shortcut
binding-add = Add shortcut
binding-record-help = Choose an action, then record a key combination. Escape cancels recording. Shared inputs affect all scenes that use them.
binding-action = Action
binding-record = Record shortcut
binding-recording = Press a key combination…
binding-unavailable = Action unavailable. OBS state is being refreshed; press the shortcut again after checking the current state.

action-inspect-input = Inspect media and filters for a source
action-inspect-none = Choose a source…
action-inspect-help = Available actions appear below. Your Sources selection stays in place.

quick-title = Quick actions
quick-manage = Customize quick actions
quick-add = Pin action
quick-edit = Change action
quick-remove = Remove pinned action
quick-up = Move action up
quick-down = Move action down
quick-empty = Pin frequently used controls beside your scenes.
quick-description = Pinned controls belong to this workspace. Shared sources affect every scene that uses them.
quick-disabled = Disabled — review and enable in Customize.
quick-observed = Observed: { $state }
quick-ready = Ready
quick-muted = Muted
quick-unmuted = Unmuted
quick-filter-enabled = Filter enabled
quick-filter-disabled = Filter disabled

quick-toggle-mute = Toggle mute · { $name }
quick-toggle-filter = Toggle filter · { $name } / { $filter }

# Readiness review
preflight-title = Preflight check
preflight-description = Review readiness before starting. Speak into the selected microphone: post-fader signal above −50 dBFS is remembered for 10 seconds while fresh levels are available. Unavailable or stale readings remain unknown.
preflight-close = Close
preflight-passed = Passed
preflight-failed = Needs attention
preflight-unknown = Unknown
preflight-skipped = Skipped
preflight-refresh = Refresh OBS observations
preflight-reset = Reset checklist
preflight-ready = Ready: all configured checks passed.
preflight-attention = Review the checks below.
preflight-collection-result = Collection: { $status } — expected “{ $expected }”, observed “{ $actual }”
preflight-scene-result = Scene “{ $name }”: { $status }
preflight-no-scenes = Required scenes: none configured.
preflight-mic-result = Microphone “{ $name }”: present { $presence }; unmuted { $unmuted }; recent signal { $signal }
preflight-disk-result = Recording space (minimum { $minimum } MiB): { $status }

preflight-configure = Configure preflight
preflight-expected = Expected scene collection
preflight-targets-description = Changing the expectation resets scene and microphone choices. Available choices come from the observed collection; saved missing targets remain visible. To choose targets from another collection, switch OBS to it and reopen Configure.
preflight-scenes = Required scenes
preflight-check-mic = Check microphone presence, mute and signal
preflight-no-scene-choices = No scenes available for this expectation.
preflight-no-microphone = No microphone selected
preflight-check-disk = Check recording space
preflight-minimum-disk = Minimum free space (MiB)
preflight-manual-items = Manual checklist (one unique item per line)
preflight-invalid-plan = Enter an expected collection, a positive whole-number disk threshold, and unique checklist items.
preflight-saving = Saving…
preflight-save-failed = Could not save: { $error }. Your changes remain here; try Save again.
preflight-editor-retired = The connection, collection or saved plan changed. Close this editor and reopen it.

preflight-continue = Continue to start { $output }…
preflight-continue-description = You may continue with unresolved checks. Continuing keeps your usual output confirmation settings. Nothing starts automatically.
preflight-review-before-output = Review preflight before starting a stream or recording

preflight-free-space = Observed free recording space: { $space } MiB

# Sustained live health
health-title = Live health alerts
health-network = Network frame drops
health-render = Rendering overload
health-encoder = Encoding overload
health-disk = Low recording space
health-network-help = OBS is losing stream frames. Check the network connection or reduce the stream bitrate.
health-render-help = OBS is missing rendering frames. Reduce scene complexity or video settings.
health-encoder-help = OBS is skipping encoding frames. Reduce encoder load or recording/streaming settings.
health-disk-help = The recording volume has less than 1 GiB free while recording. Free space or finish recording before it runs out.
health-active = Needs attention
health-recovering = Recovering — recent readings are clear
health-unobserved = Latest readings unavailable
health-caption = { $title } — { $phase }
health-more = { $summary } · { $count } more
health-policy = Frame alerts require at least 1% missed frames in recent intervals for 5 seconds. Low-space alerts also persist for 5 seconds. Performance warnings clear after 10 seconds of healthy readings; missing or stale observations never count as recovery.
health-open-stats = Open Stats
health-no-alerts = No current health alerts.

health-connection-lost = OBS connection lost
health-stream-stopped = Stream stopped
health-record-stopped = Recording stopped
health-connection-help = SceneDeck lost its OBS connection. Check OBS and the network, then reconnect. Output state is unconfirmed until observations resume.
health-stream-stopped-help = OBS reported the stream stopped without a recent SceneDeck stop command. Check OBS and the destination before restarting.
health-record-stopped-help = OBS reported recording stopped without a recent SceneDeck stop command. Check recording space and OBS. This alert does not confirm that a file was saved.
health-acknowledge = Acknowledge

health-event-policy = Event alerts belong to the current connection. Observed recovery or a new connection clears them. Acknowledgement only clears the notice.

# Selected-input audio health
health-audio-silence = { $input }: silence
health-audio-clipping = { $input }: clipping
health-audio-silence-help = Muted or at/below −50 dBFS after the fader for 15 seconds. Check the microphone, mute and routing. Disable this rule for planned silence.
health-audio-clipping-help = At least one channel has stayed at full scale before the fader for one second. Check the input gain.
health-audio-disabled = Monitoring disabled
health-audio-missing = Selected input missing or ambiguous
health-audio-monitoring = Monitoring fresh readings
health-audio-muted = Monitoring — input muted
health-audio-settings = Audio monitoring
health-audio-select = Keep a rule for this input
health-audio-check-silence = Warn about sustained silence
health-audio-check-clipping = Warn about sustained clipping
health-audio-policy = Select inputs to add rules; clear a selection to remove its rule. Enable monitoring explicitly. Rules run while connected, even before outputs start. Silence requires 15 seconds at or below −50 dBFS after the fader (or mute); clipping requires one second at full scale before the fader. Recovery requires ten seconds of clear readings. Missing or stale readings are unavailable, not silence. Refresh to find inputs.
health-audio-invalid = Choose at least one warning for each enabled rule and keep only one rule per input.
health-audio-saved = Rules saved.
health-audio-unsaved = Unsaved changes.
health-audio-retired = The workspace or saved rules changed. Close this editor and reopen it.

# Observed media timing
media-progress-label = Media progress
media-position-observed = Observed position: { $time }
media-duration-observed = Duration: { $time }
media-time-remaining = Remaining: { $time }
media-time-unavailable = Unavailable
media-duration-unavailable = Unavailable or indefinite
media-progress-hint = Times follow the last OBS observation. Seeking uses the position field below.

# Local timestamp bookmarks
bookmarks-title = Bookmarks
bookmark-policy = Saved locally. Session time includes pauses. OBS time is a recent output-time sample, not a file offset or chapter.
bookmark-name = Bookmark name
bookmark-add = Add bookmark
bookmark-retry = Retry
bookmark-saving = Saving…
bookmark-save-error = Bookmarks unavailable or unsaved: { $error }
bookmark-scope-changed = Connect to OBS or reopen this window after changing the connection or collection. Existing notes remain available.
bookmark-saved = { $count } bookmarks saved locally
bookmark-default = Bookmark { $number }
bookmark-invalid-name = Enter a name of 1–120 characters without control characters.
bookmark-full = The 2,000-bookmark limit is reached. Remove notes to add more.
bookmark-rename = Rename
bookmark-remove = Remove
bookmark-record-unknown = Recording time unavailable
bookmark-record-inactive = Recording was inactive
bookmark-record-time = OBS output time: { $time } · { $state } · Sample age: { $age } s
bookmark-description = { $created }
    Session { $session }: { $elapsed } since connection
    { $connection } · { $endpoint }
    Collection: { $collection } · Observed scene: { $scene }
    { $recording }

# Session reports and explicit export
reports-title = Session reports
reports-previous = Previous session
reports-next = Next session
reports-refresh = Refresh snapshot
reports-export-json = Export JSON
reports-export-csv = Export CSV
reports-empty = No sessions captured yet. Connect to OBS to start a report.
reports-current = Current session
reports-scene = Observed scene: { $scene }
reports-ended = Observation session ended: { $reason }
reports-alert-cleared = Alert cleared; recovery is not implied
reports-exporting = Exporting…
reports-exported = Export saved: { $path }
reports-export-error = Export failed: { $error }
reports-policy = The latest eight connection sessions stay in memory. Saved history is also available here. Refresh to include new evidence or bookmark edits. Export keeps the selected snapshot. Acknowledgements do not confirm a saved file; paths belong to the OBS host.
reports-view-limit = This view shows the latest 150 events and 50 bookmarks. Exports include all retained records. Missing samples and counter resets limit interpretation; frame counters are cumulative.
reports-summary = { $start } → { $end }
    { $connection } · { $endpoint }
    Events retained: { $events } · Omitted: { $omitted } · Activity gaps: { $gaps }
    Samples observed: { $samples } · Retained: { $kept }
    Peak sampled CPU: { $cpu } · Lowest sampled FPS: { $fps }
    Bookmarks: { $bookmarks }

# Optional session history
history-title = Session history
history-enable = Save session history automatically
history-retain = Sessions to keep (1–32)
history-apply = Apply history settings
history-retry = Reload and retry
history-clear = Clear saved history
history-saving = Saving or loading history…
history-error = History unavailable or unsaved: { $error }
history-status = { $mode } · Saved sessions: { $count }
history-on = Automatic history on
history-off = Automatic history off
history-settings-changed = History settings changed elsewhere. Reopen this window.
history-app-closed = SceneDeck closed
reports-interrupted = Previous app run; end time unavailable
history-settings-description = Automatic disk history is off by default. Manage retention, reload and failed saves here.
history-policy = Enabling history saves the current report (including retained earlier events) and future sessions. Checkpoints run about every ten seconds and at session boundaries. Disabling stops future checkpoints and keeps saved reports. Applying a smaller retention limit removes the oldest saved sessions. Bookmarks remain saved separately.
history-clear-confirm = Remove all saved and unsaved history? Recent in-memory reports and bookmarks remain available. The current connection will not be saved again; automatic history resumes with the next connection if enabled.

history-exit-requested = SceneDeck exit requested
history-exit-resumed = Exit cancelled or delayed; reporting resumed

# Selected-input audio presets
preset-title = Audio presets
preset-open = Open
preset-shared = Select inputs to save their current mute and gain settings. Changes to an input affect every scene that uses it. Saved presets belong to this connection and scene collection.
preset-name = Preset name
preset-new = New preset
preset-save = Save selected inputs
preset-replace = Replace selected preset
preset-rename = Rename selected preset
preset-delete = Delete selected preset
preset-preview = Preview saved values
preset-missing = Missing or unavailable
preset-invalid = Choose 1–64 available inputs and a unique name of up to 80 characters. Up to 64 presets can be saved.
preset-current = Current: { $value }
preset-saved = Saved: { $value }
preset-none = Select a saved preset to preview its values.
preset-refresh-needed = Refresh audio observations before capturing a preset or reviewing changes.
preset-retired = The workspace or saved presets changed. Close this editor and reopen it.
preset-pending = Waiting
preset-running = Checking and applying
preset-observed = Values confirmed by OBS
preset-unchanged = Already matched
preset-changed = Values changed; review again
preset-failed = Request failed
preset-unconfirmed = Result not fully confirmed
preset-cancelled = Remaining changes cancelled
preset-acknowledgements = OBS acknowledgements: gain { $gain }, mute { $mute }
preset-unavailable = Cannot apply: the connection is unavailable, another preset is running, or the review has expired.
preset-review = Review changes
preset-apply = Apply available inputs
preset-cancel = Stop remaining changes
preset-apply-policy = Review before applying. Missing inputs are skipped; other inputs can succeed independently. Acknowledgements do not confirm readback. Gain is checked before unmuting. Stop prevents remaining changes; it does not undo completed ones. Refresh and review again if values change.
preset-unsupported = This OBS connection does not advertise all requests needed to apply audio presets.
preset-reviewed = The review snapshot is held for up to 30 seconds.

# Desktop portal shortcuts
global-title = Desktop shortcuts
global-description = Enable up to 64 enabled action bindings for this connected workspace. Your desktop chooses the keys and may ask permission. Confirmed desktop keys replace the corresponding application keys until stopped. Registration ends on disconnect, workspace or binding changes, or closing the main window; enable it again when ready.
global-enable = Enable desktop shortcuts
global-stop = Stop desktop shortcuts
global-off = Desktop shortcuts are off.
global-pending = Waiting for the desktop…
global-active = Desktop shortcuts are active. Assigned keys:
global-none = The desktop did not bind any shortcuts.
global-unavailable = Desktop shortcuts could not be enabled. Check that your desktop provides the GlobalShortcuts portal, then try again.

help-controls-subtitle =
    Bindings, quick actions and audio presets

help-controls-body =
    Ctrl+K searches scenes and supported actions. Settings → Action shortcuts assigns stable keys; Live quick actions pins individual controls. Shared inputs affect every scene that uses them.
    Desktop shortcuts are explicitly enabled in the shortcut manager and require a supporting desktop portal. The desktop chooses the keys; registration ends on connection, workspace or binding changes, or closing the main window.
    Mixer → Audio presets captures selected input gain/mute settings. Review the changes and missing inputs before Apply. Results separate acknowledgements from observed values; partial success is possible. Stop remaining cancels unsent changes, and reconnect never replays them.

help-readiness-subtitle =
    Preparation and persistent live warnings

help-readiness-body =
    Preflight checks the expected collection, required scenes, selected microphone mute/signal, recording disk space and a manual checklist. It never starts an output. Unknown or stale evidence is not a pass.
    Health alerts remain visible across operating pages. They track sustained recent frame loss, render/encoder overload, low disk space, lost connection and unexpected output stops. Silence/clipping rules require explicitly chosen inputs; meter readings do not prove what the audience hears.

help-session-subtitle =
    Bookmarks, media and session reports

help-session-body =
    Sources shows media progress and remaining time when OBS provides a duration, plus supported seeking. Output controls include recording pause/resume.
    Bookmarks save local session time and the recent OBS recording-time sample. Pauses and reconnects mean these are not exact encoded-file offsets or chapters.
    Session reports retain bounded events, incidents and output evidence, with CSV/JSON export. Automatic disk history is off by default; enable it and choose retention in Session history. Requested actions and acknowledged commands remain separate from observed results. Recording paths belong to the OBS host.
