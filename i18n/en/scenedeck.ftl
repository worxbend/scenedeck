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
page-mixer = Mixer
page-graph = Graph
page-inventory = Inventory
page-doctor = Doctor
page-settings = Settings
obs-status-disconnected = Disconnected
obs-status-connecting = Connecting…
obs-status-connected = Connected
obs-status-error = Error

## storage/config.rs — ConfigStartupNotice
config-first-launch = No saved settings yet. Defaults are loaded.
config-read-failed = Settings could not be read: { $detail }
config-parse-failed = Settings could not be parsed: { $detail }

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

## scene_card.rs
scene-card-tooltip = { $status } ({ $role })
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

## settings.rs

settings-page-title = Settings
settings-appearance-title = Appearance
settings-appearance-description = GNOME apps should follow the system style by default.
settings-theme-mode-system = System
settings-theme-mode-light = Light
settings-theme-mode-dark = Dark
settings-color-scheme-title = Color Scheme
settings-color-scheme-subtitle = Follow the system preference or force light / dark
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
