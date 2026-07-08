# OBS WebSocket Integration

SceneDeck uses the `obws` crate to communicate with OBS Studio. The wrapper in
`src/obs/client.rs` is the only place that should call `obws` directly.

## Connection

SceneDeck connects with:

- Host from config, default `127.0.0.1`.
- Port from config, default `4455`.
- Optional password from the system keyring.

After connecting, SceneDeck reads OBS version information and starts an OBS
event stream.

## Reads

SceneDeck currently reads:

- OBS version and WebSocket version.
- Scene inventory and current program scene.
- Scene item lists for dependency graph construction.
- Scene item lists for active scene audio discovery.
- Group item lists when active scene audio discovery enters groups.
- Scene item enabled state.
- OBS profiles and current profile.
- OBS scene collections and current scene collection.
- Stream status.
- Record status.
- Special/global audio input names.
- OBS input list when an explicit audio scan is needed.
- Input mute state.
- Input volume state.

## Writes

SceneDeck currently writes:

- Current program scene.
- Current OBS profile.
- Create OBS profile.
- Remove OBS profile.
- Current scene collection.
- Create scene collection.
- Input mute state.
- Input volume multiplier.
- Start or stop streaming.
- Start or stop recording.

Scene roles are local metadata and do not write to OBS. Inventory writes the
local registry directly.

## Events

SceneDeck currently handles these OBS event categories:

- Stream state changes.
- Record state changes.
- Record file changes.
- Current program scene changes.
- Input mute changes.
- Input volume changes.
- Input created, removed, or renamed.
- Scene item created, removed, reindexed, or enabled/disabled.
- Current profile changed.
- Profile list changed.
- Current scene collection changed.
- Scene collection list changed.
- Scene list changed.

Events either update UI state directly or trigger a refresh of derived data such
as active scene audio, profile lists, collection lists, scene inventory, or the
dependency graph.

## Active Scene Audio Discovery

For the active scene, SceneDeck returns audio cards in this order:

1. Global OBS audio sources from the special input list.
2. Enabled input items from the active scene.
3. Enabled input items inside nested scenes and groups.

The scan avoids duplicates. Sources that do not expose OBS mute and volume state
are skipped.

If `live.audio_inputs` in config is non-empty, it limits the discovered scene
inputs to that configured name list. Global sources are still listed first.

## Not Yet Exposed in the UI

OBS WebSocket supports more capabilities than SceneDeck currently exposes. Good
future additions include:

- Input creation, removal, rename, and settings editing.
- Audio balance, sync offset, tracks, and monitor type.
- Scene item transform, visibility, ordering, lock state, and bounds.
- Filter management.
- Screenshot and source capture.
- Replay buffer and virtual camera controls.
- Studio mode and preview/program transitions.

Add new protocol calls through `src/obs/client.rs`, convert OBS types into
domain types, then route them through controller commands and events.
