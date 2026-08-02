// Vendor three.js and pixi.js into site/vendor/ as tree-shaken ES modules.
//
// The site itself has no build step — it ships plain HTML, CSS and ES modules
// that GitHub Pages serves as-is. But shipping the full stock builds of both
// renderers would cost ~405 KB gzipped, and the background only touches a small
// slice of each library. So the two bundles below are produced once, committed,
// and then served directly. Re-run this only when bumping a renderer version:
//
//     node site/tools/vendor.mjs
//
// It needs network access and npx. The generated files are the only thing the
// site depends on at runtime.

import { execFileSync } from 'node:child_process'
import { mkdtempSync, writeFileSync, mkdirSync, copyFileSync, statSync } from 'node:fs'
import { gzipSync } from 'node:zlib'
import { readFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const SITE = join(dirname(fileURLToPath(import.meta.url)), '..')
const OUT = join(SITE, 'vendor')

// Pinned so the committed bundles are reproducible.
const THREE = 'three@0.185.1'
const PIXI = 'pixi.js@8.19.0'

// The exact public surface the background uses. Keeping this list minimal is
// what makes the bundles small: three draws one full-screen shader, so it
// needs no lights, no materials, no geometry beyond a quad and no loaders, and
// pixi is driven through its raw WebGLRenderer rather than the much heavier
// Application/Assets front door.
//
// Anything the background imports that is missing here fails at runtime with
// "X is not a constructor", so add it and regenerate before using it.
const ENTRIES = {
  'three.min.js': {
    pkg: THREE,
    source: `export {
      WebGLRenderer, Scene, OrthographicCamera,
      PlaneGeometry, InstancedBufferGeometry, InstancedBufferAttribute,
      ShaderMaterial, Mesh, Color, SRGBColorSpace
    } from 'three'`,
  },
  'pixi.min.js': {
    pkg: PIXI,
    source: `export {
      WebGLRenderer, Container, Texture, ParticleContainer, Particle, Rectangle
    } from 'pixi.js'`,
  },
}

const work = mkdtempSync(join(tmpdir(), 'scenedeck-vendor-'))
console.log(`working in ${work}`)

execFileSync('npm', ['init', '-y'], { cwd: work, stdio: 'ignore' })
execFileSync('npm', ['install', '--silent', 'esbuild', THREE, PIXI], { cwd: work, stdio: 'inherit' })

mkdirSync(OUT, { recursive: true })

for (const [outName, { source }] of Object.entries(ENTRIES)) {
  const entry = join(work, `entry-${outName}`)
  writeFileSync(entry, source)
  const built = join(work, outName)
  execFileSync(join(work, 'node_modules/.bin/esbuild'), [
    entry,
    '--bundle',
    '--format=esm',
    '--minify',
    '--tree-shaking=true',
    '--legal-comments=none',
    `--outfile=${built}`,
  ], { stdio: 'inherit' })

  const banner = outName.startsWith('three')
    ? '/*! three.js r185 — MIT — https://threejs.org (tree-shaken subset, see site/tools/vendor.mjs) */\n'
    : '/*! PixiJS v8 — MIT — https://pixijs.com (tree-shaken subset, see site/tools/vendor.mjs) */\n'

  const code = banner + readFileSync(built, 'utf8')
  const dest = join(OUT, outName)
  writeFileSync(dest, code)

  const raw = statSync(dest).size
  const gz = gzipSync(code).length
  console.log(`${outName}: ${(raw / 1024).toFixed(0)} KB raw / ${(gz / 1024).toFixed(0)} KB gzip`)
}

console.log('done')
