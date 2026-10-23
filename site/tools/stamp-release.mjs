import { readFile, writeFile, readdir, rename } from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { execFileSync } from 'node:child_process'
import { RELEASE_API, normalizeRelease, renderReleaseHTML } from '../scripts/release.js'

export function checkedOutBuildID(siteDir) {
  // GITHUB_SHA may refer to a release tag while Pages checks out current main.
  // Fail closed outside a checkout rather than falling back to the event/time.
  const commit = execFileSync('git', ['-C', siteDir, 'rev-parse', '--verify', 'HEAD'], { encoding: 'utf8' }).trim()
  if (!/^(?:[a-f0-9]{40}|[a-f0-9]{64})$/.test(commit)) throw new Error('Invalid checked-out source SHA')
  return commit
}

async function latestRelease() {
  const response = await fetch(RELEASE_API, {
    headers: { Accept: 'application/vnd.github+json' }, signal: AbortSignal.timeout(8000),
  })
  if (!response.ok) throw new Error(`GitHub release lookup: HTTP ${response.status}`)
  return response.json()
}

async function filesIn(directory) {
  const entries = await readdir(directory, { withFileTypes: true })
  const nested = await Promise.all(entries.map(entry => {
    const path = join(directory, entry.name)
    return entry.isDirectory() ? filesIn(path) : entry.isFile() ? [path] : []
  }))
  return nested.flat()
}

export async function stampSite({ siteDir, build, resolveRelease = latestRelease, warn = console.warn }) {
  if (!/^[A-Za-z0-9]+$/.test(build)) throw new Error('Build ID must be alphanumeric')
  let release
  try { release = normalizeRelease(await resolveRelease()) }
  catch (error) { warn(`stamp-release: ${error.message}; retaining authored release links`) }
  const files = [join(siteDir, 'index.html')]
  for (const dir of ['scripts', 'fonts', 'styles']) {
    files.push(...(await filesIn(join(siteDir, dir)).catch(error => {
      if (error.code === 'ENOENT') return []
      throw error
    })))
  }
  for (const path of files.filter(path => /\.(html|js|css)$/.test(path))) {
    let source = await readFile(path, 'utf8')
    if (path.endsWith('index.html') && release) source = renderReleaseHTML(source, release)
    // Quoted local asset URLs include static/dynamic imports and CSS font URLs.
    source = source.replace(/(["'])(\.{1,2}\/[^"'?\s]+\.(?:js|css|woff2))(?:\?v=[A-Za-z0-9]+)?\1/g,
      (_, quote, url) => `${quote}${url}?v=${build}${quote}`)
    const staged = `${path}.stamp-tmp`
    await writeFile(staged, source)
    await rename(staged, path)
  }
  if (release) {
    const inventory = {
      tag_name: release.tag,
      assets: Object.values(release.assets).map(url => ({ name: url.split('/').pop(), browser_download_url: url })),
    }
    await writeFile(join(siteDir, 'release.json'), JSON.stringify(inventory, null, 2) + '\n')
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const siteDir = fileURLToPath(new URL('../', import.meta.url))
  await stampSite({ siteDir, build: checkedOutBuildID(siteDir) })
}
