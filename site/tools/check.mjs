import { readFile, readdir, stat } from 'node:fs/promises'
import { execFileSync } from 'node:child_process'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import assert from 'node:assert/strict'
import { sha256 } from './generation.mjs'
import { renderFontCSS } from './fonts.mjs'
import { normalizeRelease, renderReleaseHTML } from '../scripts/release.js'

const SITE = fileURLToPath(new URL('../', import.meta.url))
const args = process.argv.slice(2)
assert.ok(args.length === 0 || (args.length === 2 && args[0] === '--build' && /^[A-Za-z0-9]+$/.test(args[1])),
  'Usage: node site/tools/check.mjs [--build BUILD_ID]')
const build = args[1]
async function localFile(base, target) {
  const path = target.split(/[?#]/)[0]
  if (build !== undefined && /\.(js|css|woff2)$/.test(path)) {
    assert.equal(target, `${path}?v=${build}`, `cache stamp mismatch: ${target}`)
  }
  assert.ok((await stat(resolve(base, path))).isFile(), `missing local file: ${target}`)
}
async function sourceFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true })
  return (await Promise.all(entries.filter(entry => entry.name !== 'node_modules').map(entry => {
    const path = join(directory, entry.name)
    return entry.isDirectory() ? sourceFiles(path) : [path]
  }))).flat()
}
const files = await sourceFiles(SITE)
const scripts = files.filter(path => /\.(m?js)$/.test(path))
for (const path of scripts) {
  // Browser .js files are ES modules but have no package.json scope. Explicit
  // module parsing avoids ambiguous .js --check false successes on newer Node.
  try {
    execFileSync(process.execPath, ['--check', '--input-type=module'], { input: await readFile(path), stdio: 'pipe' })
  } catch (error) { throw new Error(`JS syntax check failed: ${path}`, { cause: error }) }
}

const html = await readFile(join(SITE, 'index.html'), 'utf8')
const ids = [...html.matchAll(/\bid="([^"]+)"/g)].map(match => match[1])
assert.equal(new Set(ids).size, ids.length, 'duplicate HTML IDs')
for (const match of html.matchAll(/\b(?:href|src|aria-controls|aria-labelledby|aria-describedby)="([^"]+)"/g)) {
  const target = match[1]
  if (/^(https?:|data:|mailto:)/.test(target)) continue
  if (target.startsWith('#')) assert.ok(ids.includes(target.slice(1)), `missing anchor: ${target}`)
  else if (target.startsWith('.')) await localFile(SITE, target)
  else for (const id of target.split(/\s+/)) assert.ok(ids.includes(id), `missing ARIA target: ${id}`)
}
for (const path of scripts.filter(path => !path.includes('/tools/') && !path.includes('/vendor/'))) {
  const source = await readFile(path, 'utf8')
  for (const [, specifier] of source.matchAll(/(?:from\s*|import\s*\()\s*["'](\.[^"']+)["']/g)) {
    await localFile(dirname(path), specifier)
  }
}

const release = normalizeRelease(JSON.parse(await readFile(join(SITE, 'release.json'), 'utf8')))
assert.equal(renderReleaseHTML(html, release), html, 'authored HTML differs from the reviewed release inventory')
assert.ok(!html.includes('/latest/download/'), 'mutable fallback download')
const provenance = JSON.parse(await readFile(join(SITE, 'vendor/provenance.json'), 'utf8'))
assert.equal(provenance.lockSHA256, sha256(await readFile(join(SITE, 'tools/package-lock.json'))))
for (const [filename, bundle] of Object.entries(provenance.bundles)) {
  assert.equal(sha256(await readFile(join(SITE, 'vendor', filename))), bundle.sha256)
  const exports = await import(new URL(`../vendor/${filename}`, import.meta.url))
  for (const name of bundle.exports) assert.ok(Object.hasOwn(exports, name), `${filename} missing ${name}`)
}
for (const pkg of Object.values(provenance.packages)) {
  assert.equal(sha256(await readFile(join(SITE, 'vendor', pkg.notice))), pkg.noticeSHA256)
}
const fontManifest = JSON.parse(await readFile(join(SITE, 'tools/font-sources.json'), 'utf8'))
assert.deepEqual(JSON.parse(await readFile(join(SITE, 'fonts/provenance.json'), 'utf8')), fontManifest)
assert.equal(await readFile(join(SITE, 'fonts/fonts.css'), 'utf8'), renderFontCSS(fontManifest, build), 'fonts.css differs from manifest-derived output')
for (const source of [...fontManifest.faces, ...fontManifest.licenses]) {
  const path = join(SITE, 'fonts', source.file.endsWith('.txt') ? 'licenses' : '', source.file)
  assert.equal(sha256(await readFile(path)), source.sha256)
}
console.log(`Passed: ${scripts.length} JS syntax checks, local HTML/module references, release fallback, vendor exports/hashes/licenses, six font subsets/OFL hashes, exact manifest-derived font CSS${build === undefined ? '' : `, deployment cache stamps (${build})`}`)
