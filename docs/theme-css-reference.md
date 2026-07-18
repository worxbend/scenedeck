# Theme CSS Reference

SceneDeck theme CSS is a GTK CSS overlay. Keep selectors narrow and prefer the
stable classes listed here.

## Stable Classes

- `.scenedeck-root`: top-level application window.
- `.scenedeck-content-header`, `.scenedeck-content-stack`, `.scenedeck-sidebar`,
  `.scenedeck-sidebar-list`: main shell surfaces.
- `.scenedeck-dropdown`: OBS profile and scene collection dropdown selectors.
- `.scenedeck-about-window`: About window surface.
- `.app-page`, `.app-preferences-page`, `.live-page`, `.mixer-page`,
  `.graph-page`, `.inventory-page`, `.doctor-page`, `.settings-page`: page roots.
- `.scene-card`: Live page scene card.
- `.scene-card-active`: current program scene card.
- `.scene-card-status-active`: active scene status pill.
- `.scene-card-status-ready`: ready scene status pill.
- `.audio-card`: Live page audio card.
- `.audio-card-title`: audio card source name.
- `.audio-card-controls`: mute/lock control column.
- `.output-control`: stream/record control group.
- `.role-primary`, `.role-secondary`, `.role-module`, `.role-raw`,
  `.role-debug`, `.role-archive`: role badges.
- `.diag-error`, `.diag-warning`, `.diag-info`, `.diag-ok`: Doctor status icons.

Planned stable classes include `.stream-control-card`, `.record-control-card`,
`.status-pill`, `.status-pill-live`, `.status-pill-recording`,
`.theme-preview-card`, `.volume-meter`, `.volume-slider`, and `.mixer-page`.

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

## Safe Rules

- Do not remove visible focus styling.
- Do not make text rely on color alone for status meaning.
- Keep critical buttons at comfortable click sizes.
- Test both light and dark variants.
- Keep custom CSS as an overlay; avoid broad selectors like `*`.
