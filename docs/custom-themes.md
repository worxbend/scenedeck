# Custom Themes

SceneDeck themes are light/dark-aware theme families. The color mode setting
controls which side of a theme is active:

- `System`: follow libadwaita and the desktop color preference.
- `Light`: force the selected theme family's light variant.
- `Dark`: force the selected theme family's dark variant.

Custom CSS follows the same rule. Configure a light CSS file and a dark CSS file
in Settings so SceneDeck can load the matching file when the effective color
scheme changes.

## Built-In Themes

Built-in themes are compiled into the app from `resources/themes/`. Each theme
has a light and dark CSS file. The current built-in theme families are:

- `adwaita-default`
- `scenedeck-dark`
- `scenedeck-light`
- `obsidian`
- `nord`
- `dracula-inspired`
- `solarized-dark`
- `high-contrast`
- `stream-red`
- `studio-purple`
- `ubuntu-violet`

The built-in CSS is intentionally an overlay on top of libadwaita and the base
SceneDeck CSS. It should not replace GTK fundamentals such as accessible focus
rings or readable text contrast.

## User CSS

Custom CSS can be stored anywhere readable by your user account. Suggested
locations:

```text
$XDG_CONFIG_HOME/scenedeck/themes/my-theme-light.css
$XDG_CONFIG_HOME/scenedeck/themes/my-theme-dark.css
```

In Settings:

1. Choose a built-in theme family.
2. Keep Color Scheme on `System` unless you want to force one side.
3. Enable Custom CSS.
4. Set Custom Light CSS Path.
5. Set Custom Dark CSS Path.
6. Use Reload Custom CSS after editing a file.

If a custom file cannot be read or contains CSS parse errors, SceneDeck keeps the
base and built-in theme providers loaded and reports the problem in Settings and
logs. Disable Custom CSS to fall back to the selected built-in theme.

## Resetting a Broken Theme

If Settings is hard to use because of custom CSS, edit:

```text
$XDG_CONFIG_HOME/scenedeck/config.json
```

Set:

```json
"custom_css": {
  "enabled": false,
  "light_path": null,
  "dark_path": null
}
```

Then restart SceneDeck.

## Examples

Starter files live in `examples/themes/`:

- `my-custom-theme.css`
- `high-contrast-custom.css`
- `streaming-red.css`

For a complete theme, copy an example into separate light and dark files and
adjust colors for each side.
