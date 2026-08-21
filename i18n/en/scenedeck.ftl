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
inventory-stale-group-description = These scenes are in your local registry but no longer exist in OBS.
inventory-remove-stale-tooltip = Remove stale entry
inventory-yaml-row-title = Scene Registry YAML
inventory-yaml-row-subtitle = Export or import scene roles, colors, order, tags, protection flags, and graph rules.
inventory-yaml-filter-name = YAML files

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
settings-theme-subtitle = { $description } Swatches: { $swatches }
settings-theme-loaded = Loaded { $theme } ({ $variant }).
settings-theme-loaded-with-warnings = Theme loaded with warnings.

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
    SceneDeck never renames, deletes, or reorders anything inside OBS. It reads your scenes over the OBS WebSocket connection and keeps its own notes about them.
    A "role" is one of those notes: your own label for what a scene is for — the scene you cut to on air, a reusable overlay, a leftover test scene.
    Those notes live in registry.json next to the config file, so they survive restarts and can be exported and carried to another machine from the Inventory page.
    Because the notes are local, two people can share one OBS setup and each keep a different Live page.

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
    Export writes roles, order, accent colours, icons, tags, and graph rules to a single YAML file — a plain-text format you can read and keep in version control.
    Import replaces the local registry with the contents of such a file. Use it to move a finished setup to a second machine, or to roll back after an experiment.
    Scene names are the link between the file and OBS, so a scene renamed in OBS comes back as a stale entry. Inventory lists stale entries and lets you remove them.

help-group-operate-title = Running a show
help-group-operate-description = The pages you actually use while the stream is up.

help-live-title = The Live page
help-live-subtitle = Scene cards, audio, and the program scene
help-live-body =
    Live is the operating view: the current program scene at the top, scene cards on one side, compact audio cards on the other. Drag the divider to give whichever half you need more room.
    Clicking a scene card cuts OBS to that scene. The current one is marked Active; the rest are marked Ready.
    No cards after connecting? No scene has the Primary role yet — see "Hiding scenes you never cut to" above.
    The status bar along the bottom stays visible on every page and shows connection state, stream and record state with elapsed time, and live FPS, dropped frames, CPU and bitrate.

help-hotkeys-title = Switching scenes from the keyboard
help-hotkeys-subtitle = Ctrl+1 … Ctrl+0 by default, and configurable
help-hotkeys-body =
    Each of the first ten Live cards carries a small badge with its digit: the first card is 1, the ninth is 9, the tenth is 0. The caption beside the Scenes heading always spells out the current binding.
    Slot numbers follow the Inventory order, so reordering cards there reorders the shortcuts to match.
    Settings → Scene Hotkeys chooses how the digit is pressed. A modifier plus the digit (Ctrl by default) cannot fire by accident while you type. The bare digit is fastest. The leader style is the vim-like option: press the leader key, release it, then press the digit; the caption reads "Leader armed" while it waits.
    Shortcuts act only on the Live page, and the modifier-free styles stand down while a text field has focus. A digit with no scene behind it says so in the caption rather than switching anything.

help-audio-title = Audio: the Mixer page and the meters
help-audio-subtitle = What the coloured bars are telling you
help-audio-body =
    Audio cards appear on Live and, with more room and more controls, on the Mixer page. Global OBS audio devices come first, then the audio-capable sources in the current scene — including sources inside nested scenes and groups.
    Mixer modes decide which scene's audio you are looking at. Active follows the OBS program scene. Selected loads one scene and stays there. Pinned keeps a chosen scene as a permanent target while OBS moves on.
    The meter beside each fader runs -60 dB at the bottom to 0 dB at the top and uses OBS's own thresholds: green below -20 dB for music and background, yellow from -20 to -9 dB where speech belongs, red above -9 dB where clipping starts. Nothing should sit in the red.
    One column means a mono source; two mean stereo, left then right. If only the left column moves, half your viewers hear nothing from it.
    The line above the fill is the loudest peak of the last twenty seconds, which is the quickest way to catch a clip you missed. The square at the very bottom is the level arriving from the device, before the fader — if that is too hot, no fader move will fix it.
    The lock button on a card only freezes SceneDeck's own slider. It does not lock anything in OBS.

help-outputs-title = Starting and stopping stream and recording
help-outputs-subtitle = And the confirmations that stop an accident
help-outputs-body =
    The Start/Stop Stream and Start/Stop Recording buttons live at the bottom of the sidebar, reachable from every page. The status bar shows the state and how long it has been running.
    Settings → Output Safety decides which of the four actions asks first. Out of the box, stopping either output asks for confirmation and starting does not — the assumption being that starting early is cheap and stopping early ends the show.
    State changes made in OBS itself show up here too: SceneDeck follows OBS's own events rather than assuming its own button press worked.

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
    Settings, including the OBS host and port, live in $XDG_CONFIG_HOME/scenedeck/config.json — usually ~/.config/scenedeck/config.json.
    Scene roles, order, accents and icons live in registry.json in the same folder.
    The OBS password is not in either file. It is stored in your desktop's Secret Service keyring, the same vault your browser uses.
    Back up both JSON files to carry a full setup to another machine, or use Inventory's YAML export for the scene half alone.

help-shortcuts-title = Keyboard shortcuts
help-shortcuts-subtitle = The whole list
help-shortcuts-body =
    F1 — open this guide.
    Ctrl+R — reconnect to OBS.
    Ctrl+, — open Settings.
    Ctrl+Q — quit SceneDeck.
    Ctrl+1 … Ctrl+0 on the Live page — switch to the first ten scene cards. The combination is configurable in Settings → Scene Hotkeys.

## ui/window.rs — first-run welcome dialog
welcome-dialog-heading = Welcome to SceneDeck
welcome-dialog-body = It looks like this is your first run. The Help page walks through connecting to OBS — including OBS on another computer — and shows how to keep the Live page down to the scenes you actually switch to. It takes a couple of minutes and saves a few mistakes.
welcome-dialog-later = Not now
welcome-dialog-open = Read the guide
window-help-tooltip = Help & onboarding guide
