# UI state updates and action flicker

SceneDeck uses native GTK widgets. The action flicker came from state invalidation
and widget replacement, rather than a browser reload or theme animation.

## Causes found

- `InputMuteChanged` and `InputVolumeChanged` updated their audio widgets, then
  marked the entire workspace dirty. The workspace timer started another read;
  `loading` disabled scene, Studio and source controls even when their observed
  values remained valid.
- `CurrentProgramSceneChanged` scheduled the same scan used for topology changes.
  It reread profiles, collections, outputs and the graph. Graph invalidation
  replaced Graph and Doctor content with loading pages on every scene switch.
- Mixer included the entire scene inventory, including Program/Previous, in
  its presentation signature. A changed signature replaced the preferences
  page along with its search field, options, scroll container and audio cards.
- Graph and Doctor rebuilt on every refresh and map. Sources rebuilt its
  inspector when media position or checkbox values changed. Live's scene-card
  signature included the whole registry, including unrelated input metadata.

## Update rules

Audio echoes update all cached copies of the input and the existing audio cards.
They do not invalidate unrelated workspace content. A Program change refreshes
only active-scene audio; structural events still trigger the full scan. Both
refresh paths coalesce bursts and preserve session/collection observation fences.

A background read is activity, not evidence that current observations are
invalid. Recent, clean controls stay available during it. Invalidated data and
mutations awaiting readback still require a new observation. Starting a read
must not turn invalidated data into valid data just by clearing `dirty`.

Scene activation retains its own timestamped Studio-mode observation. Scene
buttons remain available during scene-audio/source readback, and rapid clicks
use the controller's existing latest-request queue. Only the requested card
shows a busy indicator until acknowledgement. Mode/control invalidation,
failure, expiry and session/collection changes retire this observation. Every
activation still checks the observed mode and lifecycle fences at the OBS
adapter before writing. Program/Preview badges continue to show observed state.

Pages compare the data they actually display. Graph and Doctor retain unchanged
content on refresh/navigation. Live and Mixer retain their last audio layout
during same-collection membership reads, with a spinner in a reserved slot.
The existing faders address collection-scoped inputs and remain usable during
that read. Live reconciles changed membership by input identity, preserving
shared inputs. Collection/session changes retire those controls; failed reads
show an explicit unavailable state. Sources updates visibility, filters,
text, advanced audio and playback values in place, suppressing callbacks during
readback and preserving drafts and text-conflict checks. Changes to targets, capabilities or editor
structure still rebuild the affected view. Inventory, Settings, Help and Stats
retain their existing incremental/static update paths.

This follows GTK's distinction between updating widget properties and changing
container membership. In particular, rebinding a [FlowBox model](https://docs.gtk.org/gtk4/method.FlowBox.bind_model.html)
clears and repopulates the box; retaining widgets is necessary to preserve their
interaction state.

## Follow-up: remaining flashes

The initial fix retained widget identity but missed transient layout and style
changes. A mapped GTK regression reproduced Live's audio strip disappearing
between `CurrentSceneChanged` and `ActiveAudioObserved`, changing the scene
viewport's height. Loading notices in Live, Mixer and Sources also changed
layout. These now use fixed activity slots. Gain readouts reserve their longest
text width so a dB change cannot rewrap the audio strip.

Transition-end events no longer invalidate scene controls. Visibility changes
refresh audio membership and source values without rereading the graph, profiles
and collections. Studio mode changes still fence scene activation at transport
ingress.

The Virtual Cam button and Studio switch separately depended on the shared
workspace `pending`/`dirty` flags. Any scene/source command disabled both, gave
Virtual Cam a pending border, and could reset the Studio thumb during its native
animation. They now use independent observed values and target-specific pending
state through acknowledgement and readback. A toggle clicked during another
write is queued until that write completes; it never silently replaces the
admitted command. Scope retirement and failed observations cancel queued work.
Direct Studio/Virtual Cam events update their values immediately, and older
workspace readbacks cannot overwrite newer events. Unchanged output text is no
longer reapplied every 250 ms.

## Regression coverage

- Loopback OBS protocol test: Program switching publishes new audio without
  another topology invalidation or setup-list request.
- Workspace tests: background reads, scene readback, rapid activation,
  acknowledgement ordering, invalidation, errors and freshness expiry.
- Native GTK tests: unchanged page and scene-card identity, control sensitivity,
  Mixer search/options/fader continuity, Sources draft/popup/media continuity,
  and widget lifetime checks.
- Mapped-window tests with animations enabled: scene viewport geometry at
  loading/readback, gain and mute changes at 640/760/1100 px, and independent
  Studio/Virtual Cam sensitivity, styling, queued clicks and completion. Set
  `SCENEDECK_UI_CAPTURE_DIR` to save rendered frames. Before/after captures of
  the global controls during an unrelated action were pixel-identical.

Run the standard Cargo checks and `python3 tools/test-native.py` from the
[developer guide](developer-guide.md). Native tests use synthetic observations
and isolated displays; protocol tests use a loopback OBS fixture. They do not
exercise the user's running OBS session.
