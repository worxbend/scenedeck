import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdtemp, readFile, writeFile, readdir, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { createHash } from 'node:crypto'
import { generateFonts } from '../fonts.mjs'

const digest = bytes => createHash('sha256').update(bytes).digest('hex')
const fixture = () => {
  const bytes = Buffer.alloc(48); bytes.write('wOF2'); bytes.writeUInt32BE(48, 8)
  const license = Buffer.from('Test fixture: SIL OPEN FONT LICENSE Version 1.1')
  const faces = ['a','b','c'].flatMap(family => ['latin','latin-ext'].map(subset => ({
    family, subset, file: `${family}-${subset}.woff2`, weight: '300 800', style: 'normal', range: 'U+0000-00FF',
    url: `https://fonts.gstatic.com/${family}-${subset}.woff2`, sha256: digest(bytes),
  })))
  const licenses = ['a','b','c'].map(family => ({ family, file: `${family}-ofl.txt`, sha256: digest(license),
    url: `https://raw.githubusercontent.com/google/fonts/${'a'.repeat(40)}/ofl/${family}/OFL.txt`,
  }))
  return { bytes, license, manifest: { faces, licenses } }
}

test('font generator rejects HTTP/type/magic/digest failures without replacing the old set', async () => {
  const { bytes, manifest } = fixture()
  const out = await mkdtemp(join(tmpdir(), 'scenedeck-font-test-'))
  try {
    await writeFile(join(out, 'fonts.css'), 'old complete set')
    for (const bad of [
      () => new Response('missing', { status: 404 }),
      () => new Response(bytes, { headers: { 'Content-Type': 'text/html' } }),
      () => new Response(Buffer.alloc(48), { headers: { 'Content-Type': 'font/woff2' } }),
      () => { const altered = Buffer.from(bytes); altered[47] = 1; return new Response(altered, { headers: { 'Content-Type': 'font/woff2' } }) },
    ]) {
      let requests = 0
      const fetchImpl = async () => ++requests === 3 ? bad() : new Response(bytes, { headers: { 'Content-Type': 'font/woff2' } })
      await assert.rejects(generateFonts({ out, manifest, fetchImpl }))
      assert.equal(await readFile(join(out, 'fonts.css'), 'utf8'), 'old complete set')
    }
    await assert.rejects(generateFonts({ out, manifest: { ...manifest, faces: manifest.faces.slice(1) }, fetchImpl: async () => { throw new Error('must validate before fetch') } }), /six|complete/i)
    await assert.rejects(generateFonts({ out, manifest: { ...manifest, licenses: [] } }), /license/i)
    await assert.rejects(generateFonts({ out, manifest: { ...manifest, licenses: manifest.licenses.map(license => ({ ...license, file: 'same-ofl.txt' })) } }), /duplicate/i)
  } finally { await rm(out, { recursive: true, force: true }) }
})

test('complete validated font set replaces output together with CSS, licenses and provenance', async () => {
  const { bytes, license, manifest } = fixture()
  const out = await mkdtemp(join(tmpdir(), 'scenedeck-font-test-'))
  try {
    await writeFile(join(out, 'stale.woff2'), 'old font')
    const fetchImpl = async url => new Response(url.hostname === 'fonts.gstatic.com' ? bytes : license, { headers: { 'Content-Type': 'font/woff2' } })
    await generateFonts({ out, manifest, fetchImpl })
    assert.equal((await readdir(out)).filter(file => file.endsWith('.woff2')).length, 6)
    assert.equal((await readFile(join(out, 'fonts.css'), 'utf8')).match(/@font-face/g).length, 6)
    assert.equal((await readdir(join(out, 'licenses'))).length, 3)
    assert.deepEqual(JSON.parse(await readFile(join(out, 'provenance.json'), 'utf8')), manifest)
  } finally { await rm(out, { recursive: true, force: true }) }
})
