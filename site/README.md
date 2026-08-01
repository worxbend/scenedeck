# SceneDeck product site

The source of <https://worxbend.github.io/scenedeck/>.

This directory *is* the deployed site. `.github/workflows/pages.yml` uploads it
verbatim to GitHub Pages on every push to `main` that touches `site/`. There is
no bundler, no framework and no build step at deploy time — just HTML, CSS and
ES modules that the browser runs as written.

## Layout

| Path | What it is |
| --- | --- |
| `index.html` | The whole page. Single document, semantic landmarks. |
| `styles/` | Design tokens, base styles, components, sections. Plain CSS, cascade layers. |
| `scripts/` | ES modules. `main.js` is the only entry point in the document. `wire.js` holds the background's shared geometry; `spring.js` is the M3 Expressive motion physics. |
| `vendor/` | Tree-shaken three.js and pixi.js bundles. Generated — see below. |
| `fonts/` | Self-hosted variable woff2 + `fonts.css`. Generated — see below. |
| `assets/` | Logo, icon, social preview image. |
| `tools/` | Generator scripts and the social-card source. Stripped before deploy. |

## Regenerating the vendored dependencies

Both are committed so the site has no runtime CDN dependency and no install
step. Re-run these only when bumping a version; they need network access.

```sh
node site/tools/vendor.mjs   # three.js + pixi.js, tree-shaken with esbuild
node site/tools/fonts.mjs    # Unbounded, Inter, JetBrains Mono as woff2
```

`vendor.mjs` bundles only the exact export surface `scripts/background.js`
imports. That is what keeps the two renderers to roughly a quarter of a megabyte
gzipped between them instead of the ~405 KB the stock builds cost. If the
background starts importing something new from either library, add it to the
`ENTRIES` list in `vendor.mjs` and re-run — otherwise it fails at runtime with
`X is not a constructor`, which is exactly how the `Vector2` regression showed up.

## The background

`scripts/background.js` draws the OBS WebSocket link from the inside: a channel
receding to a vanishing point with eight bit-lanes on its wall, video packets
travelling up it and commands going the other way, and a GOP ring sweeping down
it every 2000 ms — OBS's own default keyframe interval.

three.js owns the channel as a single full-screen shader. Because the eye sits
exactly on the channel axis, intersecting the cylinder is closed form, so there
is no ray marching: two triangles, one draw call, no textures and no render
targets. pixi.js owns the packets, which is what its sprite batcher is for.

The keyframe ring is drawn **only** in the shader. It was originally also a
field of additive glow sprites, which had no upper bound — enough of them
overlapping saturates to white however small each alpha is — and that was the
one thing on the page that could push body copy under WCAG AA. The shader's
version is clamped by `--wire-ceiling`, so it is the one that survives.

Legibility is enforced in three places and each has a number: the shader clamps
the luminance it may *add* (`--wire-ceiling`), the packet layer is bounded by
`--packet-alpha` and `--bg-canvas-opacity`, and `--scrim-floor` lays a flat veil
over the whole composite. That last one matters most: the channel is brightest
around the vanishing point, which is exactly where a classic radial vignette is
most transparent. Measured worst case over ten frames: background luminance
0.067 dark / 0.760 light, which keeps every text role at AA or better.

`wire.js` is what stops them looking like two stacked canvases: both layers use
the same projection and the same `centreline()`, so a packet on lane 3 at z = 12
lands on the shader's rail for lane 3 at z = 12. **`centreline()` exists twice —
once in JS and once in GLSL — and the two copies must stay identical.**

## Release stamping

The page is authored with the real asset URLs of whatever release was current
when it was written, so it works correctly when opened straight from disk. At
deploy time `tools/stamp-release.sh` rewrites those version literals to the
newest published release. If the lookup fails the site is left as committed —
the links still work, they just point at an older version.

## Local preview

Any static server works; ES modules need a real origin, so opening
`index.html` over `file://` will not load the scripts.

```sh
python3 -m http.server -d site 8000
```

Then <http://localhost:8000/>.
