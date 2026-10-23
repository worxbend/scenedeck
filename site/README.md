# SceneDeck product site

Source of <https://worxbend.github.io/scenedeck/>. Plain HTML, CSS and browser
ES modules; no framework or deploy-time bundling. Pages publishes this directory
after release/cache stamping and removal of `tools/` and this README.

## Layout

| Path | Purpose |
| --- | --- |
| `index.html` | Semantic single-page document; authored, immutable download fallbacks. |
| `styles/` | Tokens, base styles, components and sections. |
| `scripts/main.js` | Entry point; interactive demos run immediately, renderers after load/idle. |
| `scripts/mesh.js` | Shared triangulated geometry and projection. |
| `scripts/activity.js` | Shared intersection, document visibility and live reduced-motion policy. |
| `scripts/release.js` | Shared asset validation and deploy/browser release bindings. |
| `release.json` | Reviewed published asset inventory matching the authored HTML. |
| `vendor/` | Generated tree-shaken renderers, linked legal comments, licenses, metafiles and provenance. |
| `fonts/` | Generated Unbounded, Onest and JetBrains Mono latin/latin-ext variable WOFF2, CSS, OFL notices and provenance. |
| `tools/` | Locked generator toolchain, font source inventory, tests and deployment stamping. Never deployed. |

## Reproducible regeneration

Use Node ≥ 22 and npm ≥ 10. The exact direct versions are **three 0.186.1,
pixi.js 8.22.0 and esbuild 0.28.2**. The committed npm lock retains all resolved
transitives and registry integrity hashes. Install scripts are disabled both by
`tools/.npmrc` and the explicit install command; esbuild uses its platform-specific
optional binary package without running its postinstall. Do not omit optional
packages and do not install these tools globally.

From the repository root:

```sh
(cd site/tools && npm ci --ignore-scripts)
node site/tools/vendor.mjs
node site/tools/fonts.mjs
node --test site/tools/tests/*.test.mjs
node site/tools/check.mjs
(cd site/tools && npm audit)
```

`vendor.mjs` works offline after `npm ci`. Extend its `ENTRIES` export list before
importing another renderer symbol. It retains legal comments in linked
`*.LEGAL.txt` files and copies the copyright/license notices of every package
contributing output. `vendor/provenance.json` records versions, lock digest,
package integrity, exports and generated output/notice digests; `*.meta.json`
records esbuild's inputs and output contributions. The npm tarball for
`@pixi/colord` omits its license file: `tools/licenses/` retains the upstream
notice from that package's exact gitHead, with its URL and digest recorded by
the generator. These records describe a reproducible build, not a signed
supply-chain attestation or proof that dependencies are vulnerability-free.

`fonts.mjs` downloads only the reviewed, exact URLs in `tools/font-sources.json`.
It checks HTTP status, font content type, WOFF2 magic/header length and SHA-256,
and requires both subsets for all three families. OFL notices use immutable
Google Fonts commit URLs and digests. It never silently skips a missing subset.
To update fonts, review upstream CSS, record the exact URLs, CSS ranges/weights
and byte digests in that manifest, and update the pinned OFL sources if needed.
Do not change the manifest's digests merely to bypass a validation failure.
Both generators stage a complete set on the destination filesystem before
promotion; failed fetches/builds leave the old output untouched. Promotion uses
rename with rollback (not a claim of an atomic multi-file filesystem transaction).
Do not run two generators for the same output directory concurrently.

Never hand-edit `vendor/` or `fonts/`. Commit the generated sets and provenance
together with their reviewed source/lock changes. `check.mjs` verifies syntax,
local references/IDs/ARIA, fallback inventory, vendor exports/digests/notices,
font bytes/OFL digests, and exact manifest-derived font CSS (families, subsets,
weights, ranges and URLs). Node tests execute the actual demo/scheduler callbacks
with deterministic DOM/timer boundaries; they are not browser rendering tests.

PR/reusable CI and the read-only Pages preparation job install with
`npm ci --ignore-scripts`, then run `npm test`, `npm run check` and `npm audit`
from `site/tools`. Dependabot monitors that directory's locked npm dependencies.
Pages repeats the static checker after stamping, before stripping tools and
uploading the artifact. Validation never executes in the write-capable deploy job.

## Background and demos

three.js draws a jittered triangular lattice using instanced edges and nodes;
pixi.js draws travelling packets/trails on the same `mesh.js` geometry. There
is no cylinder, GOP ring or `wire.js`. Viewport/device tiers bound node/packet
counts; small screens omit the three.js network. WebGL failure leaves the CSS
fallback visible. Renderers and CSS composition retain the current design.

Decorative intervals run only while their element intersects, the document is
visible and reduced motion is off. Retained intersection state lets a hidden
tab resume without another observer event. Hero meter subscriptions are disposed
on screen swaps. The background also pauses on blur, including the sole Pixi renderer's own
`Ticker.system` scheduler (not just our animation driver). Changing reduced motion
at runtime stops/restarts animation; static composition still repaints for theme
and size changes. Previous-composition contrast measurements are not evidence
for this composition; no fresh WCAG/CWV/device-performance certification is made.

## Release and cache stamping

`tools/stamp-release.sh` is the workflow entry point and delegates to Node.
`release.js` validates a published numeric `vX.Y.Z` tag **and its actual asset
inventory**. Only canonical GitHub download URLs for known formats/architectures
are accepted. Missing formats/architectures have no download URL, their commands
are hidden, and unavailable formats show an explanation. Flatpak is optional;
the reviewed v0.4.0 release has AppImages/binaries/tarballs but no Flatpak bundle.

The authored HTML and `release.json` use real immutable `/download/v0.4.0/` URLs,
not old filenames under `/latest/download/`. To refresh the authored fallback,
retrieve the official release JSON, review the assets, save that inventory as
`release.json`, then use `normalizeRelease` + `renderReleaseHTML` from
`scripts/release.js` to regenerate `index.html`. `check.mjs` rejects disagreement.
A failed deploy lookup retains those fallbacks. Independently, **every** deploy
stamps local HTML asset URLs, nested module imports and CSS font URLs with the
**full checked-out source commit SHA**, including when GitHub is offline or
returns a non-semver tag. It does not use the triggering event's `GITHUB_SHA`:
release events redeploy current default-branch source, not the historical tag.
The CLI requires a Git checkout and fails rather than guessing an identity.

For a stamped deployment copy, run
`node site/tools/check.mjs --build "$(git rev-parse HEAD)"` before stripping tools.
This requires every local HTML/module asset cache query and every generated font
URL to use exactly that build ID. Source mode expects unstamped generated font
CSS; deployment mode permits only the exact expected `?v=BUILD_ID`, not arbitrary,
mixed, missing, repeated or extra queries/fragments. All other font CSS bytes
must still match the shared deterministic renderer.

The optional, bounded browser lookup updates badges, supported downloads,
commands, the release-page link and JSON-LD together. If it fails, authored/deploy
metadata stands; it does not promise the newest release. Published releases
trigger Pages unless created with `GITHUB_TOKEN` (GitHub suppresses downstream
workflow events); dispatch Pages manually in that case. With JavaScript disabled, install
panes remain readable, unavailable links remain hidden, and no API is required.

## Local preview

```sh
python3 -m http.server --bind 127.0.0.1 -d site 8000
```

Open <http://localhost:8000/>. ES modules require HTTP; `file://` is not a supported
preview mode. Browser smoke should include regenerated WebGL exports, CSS-only
fallback, live reduced motion, hide/show, offscreen meters, narrow viewports,
install tabs, release failure/partial inventories and JavaScript-disabled content.
