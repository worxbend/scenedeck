# Manual Test Plan

Use this checklist for operational validation before a release or commit that
touches OBS connection, output controls, themes, recording paths, or audio
mixer behavior.

## Prerequisites

- OBS Studio is installed and running.
- OBS WebSocket is enabled.
- SceneDeck can connect to the configured OBS host, port, and password.
- OBS has at least two scenes, at least one audio source, and one scene-specific
  audio source.
- Recording is configured to write to a safe local test directory.
- Streaming is either pointed at a safe test target or skipped if no safe target
  is configured.

## Checklist

### OBS Connect and Disconnect

1. Start SceneDeck with OBS running.
2. Connect from the sidebar.
3. Confirm Live, Mixer, Graph, Inventory, and Doctor receive OBS scene data.
4. Disconnect from the sidebar.
5. Reconnect without restarting SceneDeck.

Expected result: connection status changes accurately, disconnected views do not
show stale controls as usable, and reconnecting restores scenes, audio, profile,
collection, and output state.

### Theme Switching

1. Open Settings.
2. Switch between System, Light, and Dark color schemes.
3. Switch between at least two built-in theme families.
4. Navigate across Live, Mixer, Inventory, Graph, Doctor, and Settings.

Expected result: the selected appearance applies immediately, text remains
readable, controls remain visible, and navigation does not reset OBS state.

### Valid Custom CSS

1. Create a small valid custom CSS file, such as changing a card border color.
2. Set it as the active custom CSS path for the current color scheme in
   Settings.
3. Navigate away from and back to Settings.
4. Restart SceneDeck.

Expected result: the CSS applies immediately, persists across restart, and does
not break built-in layout or controls.

### Invalid Custom CSS

1. Set a custom CSS path that exists but contains invalid CSS.
2. Set a custom CSS path that does not exist.
3. Reset the custom CSS path.

Expected result: SceneDeck stays usable, reports or surfaces the load failure,
keeps the built-in theme fallback, and recovers when the path is reset.

### Stream Start and Stop Confirmations

1. In Settings, enable Confirm Start Stream and Confirm Stop Stream.
2. On Live, click Start Stream and cancel the confirmation.
3. Click Start Stream again and confirm.
4. Click Stop Stream and cancel the confirmation.
5. Click Stop Stream again and confirm.
6. Disable Confirm Start Stream and Confirm Stop Stream, then repeat start and
   stop.

Expected result: canceled confirmations do not send OBS output commands,
confirmed actions update button labels and output state, pending controls are
disabled during transitions, and disabled confirmations run immediately.

### Recording Start and Stop Confirmations

1. In Settings, enable Confirm Start Recording and Confirm Stop Recording.
2. On Live, click Start Record and cancel the confirmation.
3. Click Start Record again and confirm.
4. Click Stop Record and cancel the confirmation.
5. Click Stop Record again and confirm.
6. Disable Confirm Start Recording and Confirm Stop Recording, then repeat start
   and stop.

Expected result: canceled confirmations do not send OBS output commands,
confirmed actions update button labels and recording state, pending controls are
disabled during transitions, and disabled confirmations run immediately.

### Recording Path Copy

1. Start a recording from Live.
2. Stop the recording and wait for OBS to report the final output path.
3. Use the recording path copy button.
4. Paste into a text field outside SceneDeck.

Expected result: the copied text is the latest OBS recording path, the record
status tooltip reflects the same path, and no stale path is copied after a newer
recording completes.

### Active Mixer Follows OBS Scene

1. Open Mixer and select Active mode.
2. Change the program scene from OBS.
3. Change the program scene from SceneDeck Live scene cards.
4. Repeat with scenes that have different audio sources.

Expected result: Active Mixer target follows the current OBS program scene and
updates the displayed scene-scoped audio sources after each scene change.

### Selected Mixer Remains Stable

1. Open Mixer and select Selected mode.
2. Choose a scene in the Mixer scene selector.
3. Change the OBS program scene from OBS and from Live.
4. Adjust search and grouping controls.

Expected result: the selected Mixer target does not change when the OBS program
scene changes, displayed audio stays scoped to the selected scene, and search or
grouping changes do not replace the selected scene.

### Pinned Mixer Remains Stable

1. Open Mixer and select Pinned mode.
2. Pin a scene as the Mixer target.
3. Change the OBS program scene from OBS and from Live.
4. Switch away from Mixer and back.

Expected result: the pinned Mixer target remains unchanged, displayed audio
stays scoped to the pinned scene, and navigation does not clear the pinned
selection.

### Volume and Mute Sync: SceneDeck to OBS

1. In Live or Mixer, change an audio source volume with the slider.
2. Use the +/- dB controls and reset-to-0 dB control.
3. Toggle mute from SceneDeck.
4. Observe the same source in OBS Audio Mixer.

Expected result: OBS reflects SceneDeck volume and mute changes, the SceneDeck
dB readout remains consistent with OBS, locked sliders do not send local slider
changes, and non-slider controls still behave as designed.

### Volume and Mute Sync: OBS to SceneDeck

1. In OBS Audio Mixer, change the same source volume.
2. Toggle mute in OBS.
3. Change scenes so the source appears or disappears from the active scene.
4. Return to the relevant Live or Mixer view.

Expected result: SceneDeck reflects OBS volume and mute events, source visibility
matches the active or selected Mixer scope, and no stale mute or volume state is
shown after scene changes.
