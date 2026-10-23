# Live redesign: scene wall

Design brief refined with the user on 2026-10-19. This supersedes the previous
Live layout and feature scope in PLAN.md. Earlier research remains preserved.
This document records a design direction, not an implemented change.
Detailed implementation tasks and theme migration are in [PLAN.md](../../PLAN.md).

## Problem

Live became overloaded with operating controls, preparation forms, explanatory
text and nested panels. Make it a calm, visually strong surface for choosing
scenes, staging transitions and making quick audio adjustments.

## Confirmed direction

- A scene wall with larger image-backed cards, restrained controls and soft depth.
- Remove both **OBS operating controls** and **Setup & show** from Live entirely.
- Remove rehearsal baselines/comparisons, saved setup expectations and manual
  show cues from the product scope. Remove their forms, result surfaces and
  navigation, including setup expectations in Doctor. There is no Show page.
- Put the **Studio mode toggle in the topbar**, synchronized with actual OBS mode.
- When Studio is enabled, show a compact transition row above the scene grid:
  transition selection, duration where adjustable, and Take.
- With Studio disabled, hide that row. Card activation switches Program directly.
- With Studio enabled, card activation stages Preview. Take changes Program.
- Put a compact audio strip below the grid, expandable for more control.
- Keep virtual-camera start/stop on Live in the compact output bar, alongside
  stream, recording and replay. Label the action from observed OBS state and
  disable it while pending or unavailable.
- Control scene-card previews from Settings. Enabling previews automatically
  gives cards more room; disabled previews use a compact text-first treatment.
- Keep titles fully readable over the preview. Use translucent shading, a stronger
  lower gradient, restrained text shadow, clipped rounded corners and soft depth.
- Update all 12 built-in theme families in both light/dark variants, including
  high contrast, motion settings and custom-theme examples.
- Widen Inventory and provide resizable Name/Role columns. Complete role names
  take priority over secondary row actions; preserve existing registry behavior.

## Visual references

- [User's compact-card reference](scene-cards-user-reference.png): aligned tiles
  and restrained active outline guide preview-off mode. Preview-on mode grows
  those cards and retains the shaded-image/title treatment above.
- [Scene-wall concept study](live-scene-wall-concept.png): initial image-backed
  direction, subject to the later Studio/navigation/output decisions below.
- [Inventory role truncation](inventory-role-truncation.png): replace the cramped
  preferences-row layout with a wider, resizable list.

## Layout

```text
Topbar    Live                 Studio [switch]             Sources

          Fade [selector]   300 ms [duration]          Take
          (transition row appears only in Studio mode)

          [ scene preview ] [ scene preview ] [ scene preview ]
          [ scene preview ] [ scene preview ] [ scene preview ]

          Audio     microphone / desktop / music       Expand

          Compact stream / record / replay / virtual camera controls
```

Cards carry the scene name, a small shortcut hint and Program/Preview indicators
when relevant. Remove redundant Ready/Last/On labels and instructional sentences
from the default deck. Program, Preview and keyboard focus need distinct visual
treatments. Hover must not change OBS state.

## Feature destinations

| Destination | Content |
| --- | --- |
| Live | Scene cards, conditional transition controls, compact audio, stream/record/replay and virtual camera |
| Sources | Scene-item inspection/visibility and contextual text, filters, media and advanced audio |
| Settings | Preferences, including scene-preview visibility |
| Doctor / Stats | Structural diagnostics / activity and performance context |

Sources can be opened through a discreet topbar action. Its layout also needs
redesign: clear task hierarchy and contextual detail. OBS connection, profile
and scene-collection configuration remain necessary application controls.

## Assumptions to validate visually

- Thumbnail shading preserves scene recognition while bright imagery never
  overwhelms titles. Keep text opaque; shade the image layer independently.
- Larger cards remain useful with many scenes and narrow windows. Grid columns
  reflow; Studio/transition/audio controls remain reachable without clipping.
- Compact audio provides useful mute/level access without dominating the deck.
- Light and dark appearances retain hierarchy and readable card overlays.

## Implementation boundaries

Use native OBS WebSocket screenshots for refreshed thumbnails. Validate refresh
cost with realistic scenes. Cache per scene/session, refresh visible cards only,
prioritize Program/Preview, and stop background preview work when Live is hidden
or previews are disabled. Thumbnail updates must not rebuild cards or steal focus.
These are periodically refreshed images, not a new video transport.

Keep existing capability, identity, cancellation and stale-state safeguards.
Handle missing images with an attractive title card and preserve usable scene
controls. Keep OBS output confirmations and shared-source scope explanations at
the relevant action/detail rather than filling Live with persistent paragraphs.

Remove the rejected rehearsal/setup/show UI and its exclusive runtime wiring.
Retain shared infrastructure required by kept OBS controls. Preserve existing
user workflow files on disk; removing these features does not authorize deleting
saved user data. No replacement workflow system is part of this redesign.

The concept image is an initial visual study. Move its Studio switch into the
topbar, make its transition row conditional and remove its Show navigation item
to reflect subsequent user decisions. Preview shading and expandable audio need
further visual refinement during implementation.
