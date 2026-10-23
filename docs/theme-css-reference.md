# Theme CSS Reference

SceneDeck theme CSS is a GTK CSS overlay. Keep selectors narrow and prefer the
stable classes listed here.

## Stable Classes

- `.scenedeck-root`: top-level application window. Native popover `contents`
  and `arrow` descendants use `@scenedeck_card` and `@scenedeck_fg`, including
  the outer OBS profile/collection setup popup. This is separate from styling
  the dropdowns inside it. Explicit Light/Dark mode also sets libadwaita's
  process-local color scheme; System releases that override back to the desktop.
- `.scenedeck-root .dim-label`, `.scenedeck-root .subtitle`, and
  `.scenedeck-root label.dimmed`: enabled informational captions, preference-row
  subtitles and preference-group descriptions use opaque
  `@scenedeck_muted_fg`. Use its color to establish hierarchy; lowering opacity
  can make otherwise accessible palette colors unreadable.
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
- `.scene-card`: stable Live scene button in both card modes.
- `.scene-card-preview-enabled`: larger image-backed card. The image and scrim
  are separate layers; avoid setting opacity on the button or its content.
- `.scene-card-short`: adaptive geometry for a short scene viewport in both
  card modes, with less padding and one visible title line. Preview cards also
  use a smaller height. Full names remain available to accessibility and the
  tooltip. Do not override its minimum height in family palettes. Roomier cards
  retain two-line titles and previews use 16:9 sizing.
- `.scene-card-preview-image`: aspect-preserving preview picture.
- `.scene-card-image-placeholder`: discreet fallback icon while imagery is unavailable.
- `.scene-card-scrim`: independent image shade, darker behind the title.
- `.scene-card-content`, `.scene-card-title`: foreground content and scene name.
- `.scene-card-accent`: small registry accent chip; scope per-scene colors here.
- `.scene-card-active`: current Program; solid semantic border.
- `.scene-card-preview`: current Preview; dashed semantic border. Both classes
  can coexist. Focus has its own outer ring and never replaces a tally.
- `.scene-card-status-active`, `.scene-card-status-preview`: named tally badges.
- `.scene-card-hotkey`: scene-card shortcut digit badge.
- `.live-hotkey-hint`: shortcut caption beside the Live Scenes heading.
- `.audio-card`: full Mixer audio card; not the compact Live strip.
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
- `.live-audio-strip`: compact expandable Live audio surface.
- `.live-transition-toolbar`: conditional Studio transition/Take row.
- `.output-toolbar` (also `.live-output-toolbar`): compact Live outputs surface;
  `.output-active` and `.output-pending` retain text plus boundary state.
- `.inventory-list`: responsive native Inventory list.
  `.inventory-scene-row` styles each scene's identity and editing controls;
  `.inventory-role-selector` scopes the role dropdown and
  `.inventory-row-options` scopes its secondary actions popover.
  `.inventory-yaml-card` styles the import/export card. Rows reflow with the
  available width and text scale; there are no column headers or width controls.
- `.sources-list`, `.sources-detail`: focused Sources list and detail surfaces.
- `.diag-error`, `.diag-warning`, `.diag-info`, `.diag-ok`: Doctor status icons.

## Named colors and compatibility

Geometry, type and spacing belong to the base stylesheet; family overlays
normally define only colors. All 12 saved family IDs and both variants remain
supported. Existing `.scene-card`, `.scene-card-active`, `.scene-card-hotkey`
and Mixer selectors remain stable. The old `.scene-card > box` selector no
longer matches the overlay layout. Ready/Last badges have been removed.

| Named colors (`scenedeck_` prefix) | Use |
| --- | --- |
| `window`, `sidebar`, `header`, `surface`, `card` | Shared shell surfaces |
| `fg`, `muted_fg` | Ordinary foreground text |
| `accent`, `accent_fg` | Interaction fill and readable text on that fill |
| `success`, `warning`, `error` | Status text and icons |
| `border`, `focus` | Important boundaries and independent keyboard ring |
| `program`, `program_fg`, `preview`, `preview_fg` | Independent tally fills/text |
| `image_fg`, `image_scrim`, `image_fallback` | White image titles, shade and matte |
| `shadow` | Restrained depth, never the sole state indicator |

New colors have base defaults, so older custom overlays still load without
requiring extra definitions. SceneDeck never rewrites a user's CSS file. Broad
legacy rules can still override the redesign: remove rules that apply opaque
backgrounds or whole-widget opacity to overlay children, set tally colors from
an accent, or use the focus outline to indicate Program.

Use colors instead of replacing component states:

```css
@define-color scenedeck_accent #2f6fed;
@define-color scenedeck_accent_fg #ffffff;
```

Keep image titles white even in light mode. Test text at 4.5:1 and meaningful
boundaries/focus at 3:1. The image scrim is deliberately independent of the
ordinary theme foreground. High Contrast adds opaque backing behind the title.
Per-scene registry accents color only `.scene-card-accent`; they must not
replace the matte, shade or tally.

## Motion

SceneDeck animates a small number of live-state indicators. Scene cards use
short color/border transitions only, with no pulse or image zoom. The animations are
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

The desktop animation preference is authoritative. Reduced motion stops loops
and retains short state transitions. Off also disables native GTK/libadwaita
transitions through an application-local settings override; leaving Off releases
it to the current desktop preference. It does not change desktop settings. Theme
changes do not recreate scene buttons, clear thumbnail data, or send OBS commands. Test a custom overlay under both modes
and all Motion levels, including a desktop with animations disabled.
