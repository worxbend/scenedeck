# Theme CSS Reference

SceneDeck theme CSS is a GTK CSS overlay. Keep selectors narrow and prefer the
stable classes listed here.

## Stable Classes

- `.scenedeck-root`: top-level application window.
- `.scenedeck-content-header`, `.scenedeck-content-stack`, `.scenedeck-sidebar`,
  `.scenedeck-sidebar-list`: main shell surfaces.
- `.scenedeck-dropdown`: OBS profile and scene collection dropdown selectors.
  At rest these sit flat on the header; hover and focus lift them out of it.
- `.scenedeck-brand`, `.scenedeck-brand-logo`, `.scenedeck-brand-name`: the app
  logo and name at the top of the sidebar.
- `.scenedeck-section-icon`: icon beside a page section heading.
- `.scenedeck-row-icon`: icon prefixing a preferences row.
- `.scenedeck-status-bar-icon`: icon leading a status-bar segment. It carries the
  same state class as its text, so connection colour applies to both.
- `.scenedeck-about-window`: About window surface.
- `.app-page`, `.app-preferences-page`, `.live-page`, `.mixer-page`,
  `.graph-page`, `.inventory-page`, `.doctor-page`, `.settings-page`,
  `.help-page`: page roots.
- `.help-hero`: the welcome block at the top of the Help page.
- `.help-topic`: one expandable Help topic row.
- `.help-topic-body`: the body shown when a Help topic is expanded.
- `.scene-card`: Live page scene card.
- `.scene-card-active`: current program scene card.
- `.scene-card-status-active`: active scene status pill.
- `.scene-card-status-ready`: ready scene status pill.
- `.scene-card-hotkey`: scene-card shortcut digit badge.
- `.live-hotkey-hint`: shortcut caption beside the Live Scenes heading.
- `.audio-card`: Live page audio card.
- `.audio-card-title`: audio card source name.
- `.audio-card-controls`: mute/lock control column.
- `.audio-card-scope-bar`: the coloured header naming an audio source's scope.
  It also carries `.audio-scope-global`, `.audio-scope-active`,
  `.audio-scope-nested`, or `.audio-scope-group`.
- `.audio-card-scope-label`, `.audio-card-scope-icon`: text and icon inside it.
- `.audio-card-db`: the decibel readout under the source name.
- `.audio-card-overflow`: the row holding the icon chooser and fine controls.
- `.audio-volume-fader`: audio card volume slider. Style its `trough`,
  `highlight`, and `slider` nodes to restyle the fader.
- `.audio-meter`: volume meter surface. The bars inside are drawn in code, and
  their green/yellow/red zone colours are deliberately fixed: they carry the
  same meaning as OBS's own meter, so a theme must not recolour them.
- `.audio-meter-labels`, `.audio-meter-label`: the decibel ruler beside it.
- `.icon-picker-button`, `.icon-picker-popover`, `.icon-picker-choice`,
  `.icon-picker-choice-selected`: the scene and audio-source icon chooser.
- `.scenedeck-icon`: a chosen icon rendered on a card.
- `.output-control`: stream/record control group.
- `.role-primary`, `.role-secondary`, `.role-module`, `.role-raw`,
  `.role-debug`, `.role-archive`: role badges.
- `.diag-error`, `.diag-warning`, `.diag-info`, `.diag-ok`: Doctor status icons.

Planned stable classes include `.stream-control-card`, `.record-control-card`,
`.status-pill`, `.status-pill-live`, `.status-pill-recording`,
`.theme-preview-card`, and `.mixer-page`.

## Recommended Pattern

Use `@define-color` for theme-local colors:

```css
@define-color scenedeck_accent #2f6fed;
@define-color scenedeck_card #ffffff;

.scene-card,
.audio-card {
  background-color: @scenedeck_card;
}

.scene-card-active {
  outline-color: @scenedeck_accent;
}
```

## Motion

SceneDeck animates a small number of live-state indicators. The animations are
defined in the app's base stylesheet, not in themes, and the user controls them
with the **Motion** setting (`appearance.motion`, see
[configuration.md](configuration.md)).

These classes carry looping animations:

| Class | Meaning |
| --- | --- |
| `.scenedeck-status-bar-live.scenedeck-status-bar-record` | Recording, blinking like a tally light |
| `.scenedeck-status-bar-live.scenedeck-status-bar-stream` | Streaming, swelling slowly |
| `.scenedeck-sidebar-live-icon-streaming` | Sidebar live icon while streaming |
| `.scenedeck-top-streaming-icon-active` | Header live icon while streaming |
| `.scenedeck-content-header-streaming` | Header band while streaming |
| `.obs-connecting` | Connecting to OBS |
| `.scenedeck-status-bar-dropped-active` | Frames are being dropped (flashes a few times, then holds) |

Restyle them freely — colour, weight, background. Two cautions:

- If you set `animation` on these classes yourself, your value wins over the
  Motion setting, because user CSS is applied last. Only do that if you intend
  to opt out of the user's own preference.
- GTK 4 CSS has no `transform` property, so animations here can only use
  `opacity`, colours, backgrounds and borders. Keyframes that try to scale,
  rotate or slide will silently do nothing.

## Safe Rules

- Do not remove visible focus styling.
- Do not make text rely on color alone for status meaning.
- Keep critical buttons at comfortable click sizes.
- Test both light and dark variants.
- Keep custom CSS as an overlay; avoid broad selectors like `*`.
