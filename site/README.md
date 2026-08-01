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
| `scripts/` | ES modules. `main.js` is the only entry point in the document. |
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
`ENTRIES` list in `vendor.mjs` and re-run, or the import will fail at runtime.

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
