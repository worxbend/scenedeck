import test from 'node:test'
import assert from 'node:assert/strict'

class Events {
  listeners = new Map()
  addEventListener(name, fn) { if (!this.listeners.has(name)) this.listeners.set(name, new Set()); this.listeners.get(name).add(fn) }
  removeEventListener(name, fn) { this.listeners.get(name)?.delete(fn) }
  emit(name) { this.listeners.get(name)?.forEach(fn => fn()) }
}

function environment() {
  const document = Object.assign(new Events(), { hidden: false, hasFocus: () => true })
  const motion = Object.assign(new Events(), { matches: false })
  const window = Object.assign(new Events(), { matchMedia: () => motion })
  const observers = []
  class IntersectionObserver {
    constructor(fn) { this.fn = fn; observers.push(this) }
    observe() {}
    disconnect() { this.disconnected = true }
    intersect(isIntersecting) { this.fn([{ isIntersecting }]) }
  }
  window.IntersectionObserver = IntersectionObserver
  return { document, motion, window, IntersectionObserver, observers }
}

test('retains intersection across hide/show, follows live reduced-motion changes, disposes timers', async t => {
  const env = environment()
  const previous = Object.fromEntries(['document', 'window', 'IntersectionObserver', 'setInterval', 'clearInterval'].map(name => [name, globalThis[name]]))
  t.after(() => Object.assign(globalThis, previous))
  Object.assign(globalThis, { document: env.document, window: env.window, IntersectionObserver: env.IntersectionObserver })
  const timers = new Map(); let id = 0
  globalThis.setInterval = fn => { timers.set(++id, fn); return id }
  globalThis.clearInterval = key => timers.delete(key)
  const { whileVisible } = await import('../../scripts/activity.js')
  const dispose = whileVisible({}, 100, () => {})
  assert.equal(timers.size, 0)
  env.observers[0].intersect(true); assert.equal(timers.size, 1)
  env.document.hidden = true; env.document.emit('visibilitychange'); assert.equal(timers.size, 0)
  env.document.hidden = false; env.document.emit('visibilitychange'); assert.equal(timers.size, 1)
  env.document.emit('visibilitychange'); assert.equal(timers.size, 1)
  env.motion.matches = true; env.motion.emit('change'); assert.equal(timers.size, 0)
  env.motion.matches = false; env.motion.emit('change'); assert.equal(timers.size, 1)
  env.observers[0].intersect(false); assert.equal(timers.size, 0)
  env.document.emit('visibilitychange'); assert.equal(timers.size, 0)
  env.observers[0].intersect(true); assert.equal(timers.size, 1)
  dispose(); assert.equal(timers.size, 0)
  env.motion.emit('change'); env.document.emit('visibilitychange'); assert.equal(timers.size, 0)
  assert.equal(env.observers[0].disconnected, true)
})
