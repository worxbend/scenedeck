import test from 'node:test'
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'

class Events {
  listeners = new Map()
  addEventListener(name, fn) { if (!this.listeners.has(name)) this.listeners.set(name, []); this.listeners.get(name).push(fn) }
  dispatchEvent(event) { this.listeners.get(event.type)?.forEach(fn => fn(event)) }
}

test('selected accent roles follow manual and system theme changes in both directions', async () => {
  const css = await readFile(new URL('../../styles/tokens.css', import.meta.url), 'utf8')
  const accents = [...css.matchAll(/--acc-[\w-]+:\s*(#[0-9a-f]{6})/gi)].map(match => match[1])
  assert.equal(accents.length, 7)
  const values = new Map()
  const root = { dataset: { theme: 'light' }, style: { setProperty: (key, value) => values.set(key, value) } }
  const toggle = Object.assign(new Events(), { setAttribute() {} })
  const system = Object.assign(new Events(), { matches: false })
  const motion = { matches: true }
  globalThis.window = Object.assign(new Events(), { matchMedia: query => query.includes('reduced-motion') ? motion : system })
  globalThis.document = { documentElement: root, getElementById: () => toggle }
  globalThis.CustomEvent = class { constructor(type, options = {}) { this.type = type; this.detail = options.detail } }
  let stored = null
  globalThis.localStorage = { getItem: () => stored, setItem: (_, value) => { stored = value } }
  globalThis.getComputedStyle = () => ({ getPropertyValue: key => key === '--md-surface' ? (root.dataset.theme === 'dark' ? '#0e0c13' : '#faf9f8') : '' })
  const { applyAccent } = await import('../../scripts/demos.js')
  const { initTheme } = await import('../../scripts/ui.js')
  const { contrast } = await import('../../scripts/color.js')
  initTheme()
  const roles = () => [...values].filter(([key]) => key.startsWith('--md-'))
  for (const hex of accents) {
    root.dataset.theme = 'dark'; applyAccent(hex); const dark = roles()
    root.dataset.theme = 'light'; applyAccent(hex); const light = roles()
    toggle.dispatchEvent({ type: 'click' })
    assert.deepEqual(roles(), dark, `${hex}: light -> dark`)
    assert.ok(contrast(values.get('--md-primary'), '#0e0c13') >= 4.5)
    toggle.dispatchEvent({ type: 'click' })
    assert.deepEqual(roles(), light, `${hex}: dark -> light`)
    assert.ok(contrast(values.get('--md-primary'), '#faf9f8') >= 4.5)
    stored = null
    system.dispatchEvent({ type: 'change', matches: true }); assert.deepEqual(roles(), dark)
    system.dispatchEvent({ type: 'change', matches: false }); assert.deepEqual(roles(), light)
  }
})
