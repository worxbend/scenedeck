import test from 'node:test'
import assert from 'node:assert/strict'

class Events {
  listeners = new Map()
  addEventListener(name, fn) { if (!this.listeners.has(name)) this.listeners.set(name, new Set()); this.listeners.get(name).add(fn) }
  removeEventListener(name, fn) { this.listeners.get(name)?.delete(fn) }
  emit(name) { this.listeners.get(name)?.forEach(fn => fn()) }
}
class Element extends Events {
  dataset = {}; style = { setProperty() {} }; classList = { toggle() {} }; children = []; attributes = {}
  setAttribute(name, value) { this.attributes[name] = value }
  toggleAttribute() {}
  append(...children) { this.children.push(...children) }
  querySelectorAll(selector) { return selector === '[data-deck-meter]' ? this.meters : [] }
  querySelector() { return new Element() }
  set innerHTML(html) {
    this.meters = [...html.matchAll(/data-deck-meter/g)].map(() => ({ children: Array.from({ length: 8 }, () => new Element()) }))
  }
}

test('real deck, mixer and Stats callbacks stop hidden/offscreen, resume, and do not leak on page swap', async () => {
  const motion = Object.assign(new Events(), { matches: false })
  globalThis.window = Object.assign(new Events(), { matchMedia: () => motion })
  const observers = []
  globalThis.IntersectionObserver = class {
    constructor(callback) { this.callback = callback; observers.push(this) }
    observe(element) { this.element = element }
    disconnect() { this.disconnected = true }
    intersect(isIntersecting) { if (!this.disconnected) this.callback([{ isIntersecting }]) }
  }
  window.IntersectionObserver = IntersectionObserver
  const elements = new Map(['deck-screen','mixer','gauges','counters','stats','chart-bitrate-now','chart-frame-now','chart-bitrate','chart-bitrate-area','chart-frame','chart-frame-area'].map(id => [id, new Element()]))
  const pages = ['live','mixer','stats'].map(page => Object.assign(new Element(), { dataset: { page } }))
  globalThis.document = Object.assign(new Events(), {
    hidden: false, documentElement: { dataset: { theme: 'dark' } },
    getElementById: id => elements.get(id),
    querySelectorAll: selector => selector === '.deck__page' ? pages : [],
    querySelector: () => new Element(), createElement: () => new Element(),
  })
  globalThis.getComputedStyle = () => ({ getPropertyValue: () => '#000000' })
  const timers = new Map(); let id = 0
  globalThis.setInterval = (fn, ms) => { timers.set(++id, { fn, ms }); return id }
  globalThis.clearInterval = timer => timers.delete(timer)
  const { initDeck, initMixer, initStats } = await import('../../scripts/demos.js')
  initDeck(); initMixer(); initStats()
  observers.forEach(observer => observer.intersect(true))
  assert.deepEqual([...timers.values()].map(timer => timer.ms).sort((a,b) => a-b), [100,110,1000,1400])
  timers.forEach(timer => timer.fn())
  document.hidden = true; document.emit('visibilitychange'); assert.equal(timers.size, 0)
  document.hidden = false; document.emit('visibilitychange'); assert.equal(timers.size, 4)
  observers.forEach(observer => observer.intersect(false)); assert.equal(timers.size, 0)
  observers.forEach(observer => observer.intersect(true)); assert.equal(timers.size, 4)
  motion.matches = true; motion.emit('change'); assert.equal(timers.size, 0)
  pages[1].emit('click'); observers.forEach(observer => observer.intersect(true)); assert.equal(timers.size, 0)
  motion.matches = false; motion.emit('change'); assert.equal(timers.size, 4)
  pages[0].emit('click'); observers.forEach(observer => observer.intersect(true)); assert.equal(timers.size, 4)
  pages[2].emit('click'); assert.equal(timers.size, 3)
  document.hidden = true; document.emit('visibilitychange'); assert.equal(timers.size, 0)
})
