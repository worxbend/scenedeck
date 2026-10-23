import test from 'node:test'
import assert from 'node:assert/strict'
class Element {
  constructor(dataset = {}) { this.dataset = dataset; this.attrs = {}; this.hidden = false; this.lastChild = { nodeValue: ' old command' } }
  setAttribute(name, text) { this.attrs[name] = text }
  removeAttribute(name) { delete this.attrs[name] }
}
test('real browser refresh preserves offline fallbacks and reconciles partial assets, commands and JSON-LD', async () => {
  globalThis.window = { matchMedia: () => ({ matches: false }) }
  const badge = new Element(); badge.textContent = 'v0.4.0'
  const appimage = new Element({ releaseAsset: 'appimage-arm64' })
  const flatpak = new Element({ releaseAsset: 'flatpak-amd64' }); flatpak.attrs.href = 'old-flatpak'
  const required = new Element({ releaseRequires: 'flatpak-amd64' })
  const unavailable = new Element({ releaseUnavailable: 'flatpak' })
  const command = new Element({ releaseCommand: 'flatpak-install' })
  const copy = new Element({ releaseCopy: 'flatpak-install' })
  const page = new Element()
  const json = { textContent: '{"softwareVersion":"0.4.0","name":"SceneDeck"}' }
  const groups = {
    '[data-release-version]': [badge], '[data-release-asset]': [appimage,flatpak],
    '[data-release-requires]': [required], '[data-release-unavailable]': [unavailable],
    '[data-release-command]': [command], '[data-release-copy]': [copy], '[data-release-page]': [page],
  }
  let queries = 0
  globalThis.document = { querySelectorAll: selector => { queries++; return groups[selector] }, querySelector: () => json }
  const { refreshRelease } = await import('../../scripts/ui.js')
  for (const fetchImpl of [async () => { throw new Error('offline') }, async () => new Response('', { status: 429 }), async () => new Response('{bad json'), async () => new Response(JSON.stringify({ tag_name: 'v1.2.3', assets: null }))]) {
    globalThis.fetch = fetchImpl
    await refreshRelease()
    assert.equal(queries, 0); assert.equal(badge.textContent, 'v0.4.0')
  }
  const name = 'scenedeck-1.2.3-linux-arm64.AppImage'
  globalThis.fetch = async () => new Response(JSON.stringify({ tag_name: 'v1.2.3', assets: [{ name, browser_download_url: `https://github.com/worxbend/scenedeck/releases/download/v1.2.3/${name}` }] }))
  await refreshRelease()
  assert.equal(badge.textContent, 'v1.2.3')
  assert.equal(appimage.hidden, false); assert.match(appimage.attrs.href, /v1\.2\.3/)
  assert.equal(flatpak.hidden, true); assert.equal(flatpak.attrs.href, undefined)
  assert.equal(required.hidden, true); assert.equal(unavailable.hidden, false)
  assert.equal(command.lastChild.nodeValue, ' '); assert.equal(copy.attrs['data-copy'], '')
  assert.equal(JSON.parse(json.textContent).softwareVersion, '1.2.3')
  assert.equal(JSON.parse(json.textContent).downloadUrl, 'https://github.com/worxbend/scenedeck/releases/tag/v1.2.3')
  assert.match(page.href, /\/tag\/v1\.2\.3$/)
})
