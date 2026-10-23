import test from 'node:test'
import assert from 'node:assert/strict'
import { registerHooks } from 'node:module'

class Events {
  listeners = new Map()
  addEventListener(name, fn) { if (!this.listeners.has(name)) this.listeners.set(name, []); this.listeners.get(name).push(fn) }
  dispatchEvent(event) { this.listeners.get(event.type)?.forEach(fn => fn(event)) }
}

let scenario = 0
async function withTrafficLayer(reduced, run) {
  const motion = Object.assign(new Events(), { matches: reduced })
  const particles = []; const renders = []; const flows = []; let randoms = 0; let resize; let frames = 0
  const originalRandom = Math.random
  Math.random = () => { randoms++; return 0.5 }
  globalThis.__trafficPixi = {
    WebGLRenderer: class { async init() {} resize() {} render() { renders.push(particles.map(p => ({ ...p }))) } },
    Container: class { addChild() {} },
    ParticleContainer: class { constructor() { flows.push(this) } addParticle() {} },
    Particle: class { constructor(options) { Object.assign(this, options); particles.push(this) } },
    Texture: { from: () => 'dot' }, Ticker: { system: { start() { assert.equal(reduced, false, 'reduced motion must stay static') }, stop() {} } },
  }
  const identity = ++scenario
  const hooks = registerHooks({
    resolve(specifier, context, next) {
      const result = next(specifier, context)
      if (result.url.includes('/scripts/') || result.url.endsWith('/vendor/pixi.min.js')) {
        return { ...result, url: `${result.url.split('?')[0]}?traffic-test=${identity}` }
      }
      return result
    },
    load(url, context, next) {
      if (url.split('?')[0].endsWith('/vendor/pixi.min.js')) return { format: 'module', shortCircuit: true,
        source: 'export const { WebGLRenderer, Container, ParticleContainer, Particle, Texture, Ticker } = globalThis.__trafficPixi' }
      return next(url, context)
    },
  })
  const palette = new Map([['--mesh-edge','#112233'],['--mesh-node','#445566'],['--inst-ok','#778899'],['--packet-alpha','0.28']])
  globalThis.getComputedStyle = () => ({ getPropertyValue: key => palette.get(key) || '' })
  const canvas = { setAttribute() {} }
  const toggle = Object.assign(new Events(), { setAttribute() {} })
  globalThis.document = Object.assign(new Events(), { hidden: false, hasFocus: () => true,
    documentElement: { dataset: { theme: 'light' }, style: { setProperty() {} } },
    getElementById: id => id === 'theme-toggle' ? toggle : canvas,
    createElement: () => ({ getContext: type => type === '2d' ? { createRadialGradient: () => ({ addColorStop() {} }), fillRect() {} } : {} }),
  })
  const system = Object.assign(new Events(), { matches: false })
  globalThis.window = Object.assign(new Events(), { innerWidth: 600, innerHeight: 800,
    matchMedia: query => query.includes('reduced-motion') ? motion : system, devicePixelRatio: 1, ResizeObserver: true })
  globalThis.ResizeObserver = class { constructor(fn) { resize = fn } observe() {} }
  globalThis.CustomEvent = class { constructor(type, options = {}) { this.type = type; this.detail = options.detail } }
  globalThis.requestAnimationFrame = () => { assert.equal(reduced, false, 'no animation loop under reduced motion'); return ++frames }
  globalThis.localStorage = { setItem() {}, getItem: () => null }
  globalThis.setTimeout = fn => { fn(); return 1 }
  globalThis.clearTimeout = () => {}
  try {
    const { mesh, nodeAt } = await import('../../scripts/mesh.js')
    const { initBackground } = await import('../../scripts/background.js')
    await run({ mesh, nodeAt, initBackground, particles, renders, flows, palette, toggle,
      resize: () => resize(), randomCount: () => randoms, frameCount: () => frames })
  } finally { hooks.deregister(); Math.random = originalRandom; delete globalThis.__trafficPixi }
}

test('real traffic layer reprojects resize/theme/accent at zero delta without advancing packets', async () => {
  await withTrafficLayer(true, async ({ mesh, nodeAt, initBackground, particles, renders, palette, resize, randomCount }) => {
    const expectedHead = () => {
      const edge = mesh.edges[3 % mesh.edges.length]
      const a = {}; const b = {}; nodeAt(mesh.nodes[edge.a], a); nodeAt(mesh.nodes[edge.b], b)
      return { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 }
    }
    await initBackground()
    assert.ok(particles.length > 0)
    assert.equal(particles[0].x, expectedHead().x, 'initial paint must not advance initial d=0.5')
    assert.equal(particles[0].y, expectedHead().y)
    const first = { ...particles[0] }; const randoms = randomCount()
    window.innerWidth = 700; window.innerHeight = 500; resize()
    assert.equal(randomCount(), randoms, 'resize must retain progress rather than randomize it')
    assert.equal(particles[0].x, expectedHead().x); assert.equal(particles[0].y, expectedHead().y)
    assert.notEqual(particles[0].x, first.x)
    palette.set('--mesh-edge','#abcdef'); palette.set('--packet-alpha','0.5')
    window.dispatchEvent(new CustomEvent('scenedeck:theme', { detail: { dark: true } }))
    assert.equal(particles[0].tint, 0xabcdef)
    const edge = mesh.edges[3 % mesh.edges.length]
    assert.equal(particles[0].alpha, 0.5 * mesh.layers[edge.layer].dim)
    assert.equal(particles[1].tint, 0xabcdef)
    window.dispatchEvent(new CustomEvent('scenedeck:accent', { detail: { hex: '#fedcba' } }))
    assert.equal(particles[0].tint, 0xfedcba)
    const stable = particles.map(p => ({ ...p })); const time = mesh.time
    for (let i = 0; i < 3; i++) window.dispatchEvent(new CustomEvent('appearance:repaint'))
    assert.deepEqual(particles.map(p => ({ ...p })), stable)
    assert.equal(mesh.time, time); assert.equal(randomCount(), randoms)
    assert.ok(renders.length >= 5)
  })
})

for (const reduced of [false, true]) {
  test(`early selected accent survives deferred traffic init and theme toggles (reduced motion: ${reduced})`, async () => {
    await withTrafficLayer(reduced, async ({ mesh, initBackground, particles, renders, flows, palette, toggle, randomCount, frameCount }) => {
      const { applyAccent } = await import('../../scripts/demos.js')
      const { initTheme } = await import('../../scripts/ui.js')
      initTheme()
      // Selection registers the accent theme listener before deferred background init.
      applyAccent('#fedcba')
      await initBackground()
      const positions = () => particles.map(({ x, y }) => ({ x, y }))
      const initial = positions(); const time = mesh.time; const randoms = randomCount()
      const frames = frameCount()
      assert.equal(frames, reduced ? 0 : 1)
      for (const dark of [true, false]) {
        palette.set('--mesh-edge', dark ? '#abcdef' : '#112233')
        palette.set('--mesh-node', dark ? '#aabbcc' : '#445566')
        palette.set('--inst-ok', dark ? '#ddeeff' : '#778899')
        palette.set('--packet-alpha', dark ? '0.5' : '0.28')
        const before = renders.length
        toggle.dispatchEvent({ type: 'click' })
        assert.equal(document.documentElement.dataset.theme, dark ? 'dark' : 'light')
        assert.equal(renders[before][0].tint, 0xfedcba, 'the earlier theme listener emits the selected accent synchronously')
        assert.equal(particles[0].tint, 0xfedcba, 'traffic theme refresh must retain the selected accent')
        assert.equal(particles[1].tint, 0xfedcba, 'trail must retain the selected accent too')
        assert.equal(particles[2].tint, dark ? 0xaabbcc : 0x445566)
        assert.equal(particles[4].tint, dark ? 0xddeeff : 0x778899)
        const edge = mesh.edges[3 % mesh.edges.length]
        assert.equal(particles[0].alpha, (dark ? 0.5 : 0.28) * mesh.layers[edge.layer].dim)
        assert.equal(flows[0].blendMode, dark ? 'add' : 'normal')
        assert.deepEqual(positions(), initial, 'theme repaints must not advance packets')
        const stable = particles.map(p => ({ ...p }))
        window.dispatchEvent(new CustomEvent('appearance:repaint'))
        assert.deepEqual(particles.map(p => ({ ...p })), stable)
        assert.equal(mesh.time, time); assert.equal(randomCount(), randoms)
        assert.equal(frameCount(), frames, 'theme changes must not start another animation loop')
      }
    })
  })
}
