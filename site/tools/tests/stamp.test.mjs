import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdtemp, mkdir, readFile, writeFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

// Release events identify the tag, but Pages deliberately checks out current main.
test('cache identity follows the checked-out source, not the release event SHA', async () => {
  const { checkedOutBuildID, stampSite } = await import('../stamp-release.mjs')
  const siteDir = fileURLToPath(new URL('../../', import.meta.url))
  const checkout = execFileSync('git', ['-C', siteDir, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim()
  const eventSHA = (checkout[0] === 'a' ? 'b' : 'a').repeat(40)
  const out = await mkdtemp(join(tmpdir(), 'scenedeck-source-stamp-test-'))
  const previous = process.env.GITHUB_SHA
  try {
    process.env.GITHUB_SHA = eventSHA
    assert.equal(checkedOutBuildID(siteDir), checkout)
    assert.notEqual(checkedOutBuildID(siteDir), eventSHA)
    await writeFile(join(out, 'index.html'), '<script src="./scripts/main.js?v=old"></script>')
    await stampSite({ siteDir: out, build: checkedOutBuildID(siteDir),
      resolveRelease: async () => { throw new Error('offline fixture') }, warn: () => {} })
    const html = await readFile(join(out, 'index.html'), 'utf8')
    assert.ok(html.includes(`./scripts/main.js?v=${checkout}`))
    assert.ok(!html.includes(eventSHA))
  } finally {
    if (previous === undefined) delete process.env.GITHUB_SHA
    else process.env.GITHUB_SHA = previous
    await rm(out, { recursive: true, force: true })
  }
})

test('release lookup failure still invalidates CSS, nested modules and font URLs; stamping is repeatable', async () => {
  const { stampSite } = await import('../stamp-release.mjs')
  const siteDir = await mkdtemp(join(tmpdir(), 'scenedeck-stamp-test-'))
  try {
    await mkdir(join(siteDir, 'scripts')); await mkdir(join(siteDir, 'fonts'))
    await writeFile(join(siteDir, 'index.html'), '<link href="./fonts/fonts.css?v=old"><script src="./scripts/main.js?v=old"></script><span data-release-version>v0.4.0</span>')
    await writeFile(join(siteDir, 'scripts/main.js'), 'import { x } from "./x.js?v=old"; import(\'../vendor/pixi.min.js\');')
    await writeFile(join(siteDir, 'fonts/fonts.css'), 'src: url("./onest-latin.woff2?v=old");')
    const options = { siteDir, build: 'abc123', resolveRelease: async () => { throw new Error('offline') }, warn: () => {} }
    await stampSite(options)
    const html = await readFile(join(siteDir, 'index.html'), 'utf8')
    const modules = await readFile(join(siteDir, 'scripts/main.js'), 'utf8')
    assert.match(html, /fonts.css\?v=abc123/)
    assert.match(html, /v0\.4\.0/)
    assert.match(modules, /x.js\?v=abc123/)
    assert.match(modules, /pixi.min.js\?v=abc123/)
    assert.match(await readFile(join(siteDir, 'fonts/fonts.css'), 'utf8'), /woff2\?v=abc123/)
    await stampSite(options)
    assert.equal(await readFile(join(siteDir, 'scripts/main.js'), 'utf8'), modules)
    assert.equal(await readFile(join(siteDir, 'index.html'), 'utf8'), html)
    await stampSite({ ...options, resolveRelease: async () => ({ tag_name: 'not-semver', assets: [] }), build: 'def456' })
    assert.match(await readFile(join(siteDir, 'index.html'), 'utf8'), /\?v=def456/)
  } finally { await rm(siteDir, { recursive: true, force: true }) }
})

test('successful deploy stamping persists only validated assets and matching metadata', async () => {
  const { stampSite } = await import('../stamp-release.mjs')
  const siteDir = await mkdtemp(join(tmpdir(), 'scenedeck-stamp-test-'))
  const name = 'scenedeck-1.2.3-linux-amd64.AppImage'
  const raw = { tag_name: 'v1.2.3', assets: [{ name, browser_download_url: `https://github.com/worxbend/scenedeck/releases/download/v1.2.3/${name}` }] }
  try {
    await writeFile(join(siteDir, 'index.html'), await readFile(new URL('../../index.html', import.meta.url), 'utf8'))
    await stampSite({ siteDir, build: 'abc123', resolveRelease: async () => raw })
    const html = await readFile(join(siteDir, 'index.html'), 'utf8')
    assert.match(html, /"softwareVersion": "1.2.3"/)
    assert.match(html, /data-release-command="appimage-run"[^>]*><span[^>]*>\$<\/span> \.\/scenedeck-1.2.3-linux-amd64.AppImage/)
    assert.doesNotMatch(html, /href="[^"]+\.flatpak"/)
    assert.deepEqual(JSON.parse(await readFile(join(siteDir, 'release.json'), 'utf8')), raw)
  } finally { await rm(siteDir, { recursive: true, force: true }) }
})
