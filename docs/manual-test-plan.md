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

### Focused Mixer Refresh Contract

Prerequisites:

- OBS WebSocket is reachable from SceneDeck with the configured host, port, and
  password state recorded.
- OBS has at least two scenes.
- OBS has global audio inputs available in the OBS Audio Mixer.
- At least one scene has a scene-specific audio input, and that scene-specific
  input differs between two test scenes.
- The tester can interact with SceneDeck GTK ComboRows and inspect the visible
  Mixer cards.
- Record OBS version, obs-websocket version, SceneDeck build or commit, and any
  skipped cases in `docs/manual-test-runs.md`.

1. Open Mixer and select Active mode with the mode ComboRow.
2. Change the current OBS program scene from OBS and from SceneDeck Live.
3. Record whether Active mode follows the current scene and whether any
   unexpected selected/pinned scene refresh is observed.
4. Switch to Selected mode with the mode ComboRow and no selected scene
   configured.
5. Record whether the summary copy identifies the current-scene fallback and
   whether the visible cards match that fallback scene.
6. Choose an explicit scene with the scene ComboRow.
7. Record whether the selected-scene summary and visible cards follow the
   explicit scene after additional OBS program-scene changes.
8. Switch to Pinned mode with the mode ComboRow.
9. Test pinned fallback order by using an explicit pinned scene, then a missing
   pinned scene with a selected scene available, then no pinned or selected
   scene with a current scene available.
10. Record the summary copy and visible card target for each pinned case.
11. Force a selected or pinned scene refresh failure with a non-destructive
    setup, such as selecting a temporary test scene and then removing or
    renaming only that temporary scene in OBS.
12. Use the Mixer Retry button after the failure.
13. Record whether Retry sends a new refresh attempt and whether the error,
    loading, and visible-card states recover or remain failed.
14. In OBS Audio Mixer, toggle mute for a visible Mixer source in Active,
    Selected, and Pinned modes where the source is present.
15. Record whether each OBS mute echo updates the visible Mixer card without a
    manual page change.
16. In OBS Audio Mixer, move volume for a visible Mixer source repeatedly in
    Active, Selected, and Pinned modes where the source is present.
17. Record whether each OBS volume echo updates the visible Mixer card without a
    manual page change.
18. After mute and volume echoes, check for stale visible cards by comparing the
    SceneDeck mute state and dB readout with OBS.
19. During repeated volume echoes, record perceived rebuild churn: visible
    flicker, scroll position jumps, focus loss, control resets, or no noticeable
    churn.

Expected result: cases are marked passed only when exercised. A complete pass
shows ComboRow mode and scene changes selecting the expected Mixer target,
Retry recovering or retrying after a failed selected or pinned refresh, OBS mute
and volume echoes updating visible Mixer cards, no stale visible cards after
echoes, and no noticeable rebuild churn under repeated volume echoes. If any
prerequisite or interaction path is unavailable, record the case as blocked or
skipped and make no pass/fail claim for that behavior.

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
