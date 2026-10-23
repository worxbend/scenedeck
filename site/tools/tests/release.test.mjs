import test from 'node:test'
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'

const root = new URL('../../', import.meta.url)
const inventory = (tag, names) => ({ tag_name: tag, assets: names.map(name => ({
  name, browser_download_url: `https://github.com/worxbend/scenedeck/releases/download/${tag}/${name}`,
})) })

test('only published canonical assets become download links and commands', async () => {
  const { normalizeRelease, renderReleaseHTML } = await import('../../scripts/release.js')
  const release = normalizeRelease(inventory('v1.2.3', ['scenedeck-1.2.3-linux-arm64.AppImage']))
  assert.equal(release.assets['appimage-amd64'], undefined)
  assert.match(release.assets['appimage-arm64'], /download\/v1\.2\.3\//)
  const html = renderReleaseHTML(await readFile(new URL('index.html', root), 'utf8'), release)
  assert.doesNotMatch(html, /href="[^"]+\.flatpak"/)
  assert.match(html, /data-release-asset="appimage-arm64"[^>]*href="[^"]+1\.2\.3-linux-arm64.AppImage"/)
  assert.match(html, /data-release-requires="appimage-amd64"[^>]*hidden/)
  assert.match(html, /"softwareVersion": "1.2.3"/)
  assert.equal(renderReleaseHTML(html, release), html)
})

test('invalid tags and hostile/mismatched asset URLs cannot become commands', async () => {
  const { normalizeRelease } = await import('../../scripts/release.js')
  for (const tag of ['', 'v1.2', 'v1.2.3;sh', 'v1.2.3-beta', '1.2.3']) {
    assert.throws(() => normalizeRelease(inventory(tag, [])))
  }
  const raw = inventory('v1.2.3', ['scenedeck-1.2.3-linux-amd64.flatpak'])
  raw.assets[0].browser_download_url = 'https://evil.example/scenedeck.flatpak'
  assert.deepEqual(normalizeRelease(raw).assets, {})
  assert.throws(() => normalizeRelease({ tag_name: 'v1.2.3' }))
})
