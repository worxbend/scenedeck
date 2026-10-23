import test from 'node:test'
import assert from 'node:assert/strict'
import { cp, mkdtemp, readFile, writeFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { basename, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawnSync } from 'node:child_process'
import { stampSite } from '../stamp-release.mjs'

const SITE = fileURLToPath(new URL('../../', import.meta.url))
async function withSite(run) {
  const siteDir = await mkdtemp(join(tmpdir(), 'scenedeck-check-test-'))
  try {
    await cp(SITE, siteDir, { recursive: true, filter: path => basename(path) !== 'node_modules' })
    await run(siteDir)
  } finally { await rm(siteDir, { recursive: true, force: true }) }
}
function check(siteDir, ...args) {
  return spawnSync(process.execPath, [join(siteDir, 'tools/check.mjs'), ...args], { encoding: 'utf8' })
}

// Keep all six verified font files, licenses and provenance intact: CSS alone drifts.
test('static checker rejects all-broken generated font URLs', async () => {
  await withSite(async siteDir => {
    const path = join(siteDir, 'fonts/fonts.css')
    const css = await readFile(path, 'utf8')
    assert.equal((css.match(/url\("\.\//g) || []).length, 6)
    await writeFile(path, css.replaceAll('url("./', 'url("./missing-'))
    const result = check(siteDir)
    assert.notEqual(result.status, 0, result.stdout + result.stderr)
    assert.match(result.stderr, /fonts\.css differs from manifest-derived output/)
  })
})

test('predeploy checker rejects a syntactically broken module after stamping', async () => {
  await withSite(async siteDir => {
    await writeFile(join(siteDir, 'scripts/main.js'), 'export const broken = ;')
    await stampSite({ siteDir, build: 'abc123', resolveRelease: async () => { throw new Error('offline fixture') }, warn: () => {} })
    const result = check(siteDir, '--build', 'abc123')
    assert.notEqual(result.status, 0, result.stdout + result.stderr)
    assert.match(result.stderr, /SyntaxError/)
  })
})

test('deployment checker accepts exactly stamped manifest CSS and rejects URL/mapping drift', async t => {
  await withSite(async siteDir => {
    assert.equal(check(siteDir).status, 0, 'pristine source fixture must pass')
    const build = 'abc123'
    await stampSite({ siteDir, build, resolveRelease: async () => { throw new Error('offline fixture') }, warn: () => {} })
    const result = check(siteDir, '--build', build)
    assert.equal(result.status, 0, result.stdout + result.stderr)
    const path = join(siteDir, 'fonts/fonts.css')
    const css = await readFile(path, 'utf8')
    for (const [name, mutate] of [
      ['all broken URLs', css => css.replaceAll('url("./', 'url("./missing-')],
      ['wrong family', css => css.replace('font-family: "Onest"', 'font-family: "Unbounded"')],
      ['wrong weight', css => css.replace('600 900', '400 500')],
      ['wrong range', css => css.replace('U+0100-02BA', 'U+0100-02BB')],
      ['wrong existing file', css => css.replace('./onest-latin.woff2', './unbounded-latin.woff2')],
      ['one unstamped URL', css => css.replace('?v=abc123', '')],
      ['mixed build IDs', css => css.replace('?v=abc123', '?v=def456')],
      ['all stale build IDs', css => css.replaceAll('?v=abc123', '?v=def456')],
      ['extra query', css => css.replace('?v=abc123', '?v=abc123&other=1')],
      ['duplicate query', css => css.replace('?v=abc123', '?v=abc123&v=abc123')],
      ['fragment', css => css.replace('?v=abc123', '?v=abc123#font')],
    ]) {
      await t.test(name, async () => {
        await writeFile(path, mutate(css))
        const rejected = check(siteDir, '--build', build)
        assert.notEqual(rejected.status, 0, rejected.stdout + rejected.stderr)
        assert.match(rejected.stderr, /fonts\.css differs from manifest-derived output/)
      })
    }
    await writeFile(path, css)
    assert.notEqual(check(siteDir).status, 0, 'source mode must not silently accept deployment CSS')
    assert.notEqual(check(siteDir, '--build', 'def456').status, 0, 'expected build must not be inferred from stamped data')
    for (const relative of ['index.html', 'scripts/main.js']) {
      const path = join(siteDir, relative)
      const source = await readFile(path, 'utf8')
      await writeFile(path, source.replace('?v=abc123', '?v=def456'))
      const rejected = check(siteDir, '--build', build)
      assert.notEqual(rejected.status, 0, rejected.stdout + rejected.stderr)
      assert.match(rejected.stderr, /cache stamp mismatch/)
      await writeFile(path, source)
    }
  })
})
