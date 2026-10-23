import test from 'node:test'
import assert from 'node:assert/strict'
class Events {
  listeners = new Map()
  addEventListener(name, fn) { if (!this.listeners.has(name)) this.listeners.set(name, new Set()); this.listeners.get(name).add(fn) }
  removeEventListener(name, fn) { this.listeners.get(name)?.delete(fn) }
  emit(name) { this.listeners.get(name)?.forEach(fn => fn()) }
}
test('background starts static, follows live motion changes, pauses hidden/blurred and resumes once', async () => {
  const motion = Object.assign(new Events(), { matches: true })
  globalThis.document = Object.assign(new Events(), { hidden: false, hasFocus: () => true, documentElement: {} })
  globalThis.window = Object.assign(new Events(), { matchMedia: () => motion })
  const raf = new Map(); let id = 0
  globalThis.requestAnimationFrame = fn => { raf.set(++id, fn); return id }
  globalThis.cancelAnimationFrame = id => raf.delete(id)
  let paints = 0
  const activity = []
  const layer = { render: () => paints++, activity: active => activity.push(active), canvas: { setAttribute() {} } }
  const { driveBackground } = await import('../../scripts/background.js')
  assert.equal(typeof driveBackground, 'function')
  driveBackground([layer], { config: {}, burst: { step: () => 0 } })
  assert.equal(activity.at(-1), false)
  assert.equal(raf.size, 0); assert.ok(paints > 0)
  motion.matches = false; motion.emit('change'); assert.equal(raf.size, 1)
  document.hidden = true; document.emit('visibilitychange'); assert.equal(raf.size, 0)
  const hiddenPaints = paints
  window.emit('appearance:repaint'); assert.equal(paints, hiddenPaints)
  document.hidden = false; document.emit('visibilitychange'); assert.equal(raf.size, 1)
  window.emit('blur'); assert.equal(raf.size, 0)
  document.emit('visibilitychange'); assert.equal(raf.size, 0)
  window.emit('focus'); window.emit('focus'); assert.equal(raf.size, 1)
  motion.matches = true; motion.emit('change'); assert.equal(raf.size, 0)
  window.emit('appearance:repaint'); assert.ok(paints > hiddenPaints)
})
