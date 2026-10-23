import { mkdtemp, mkdir, rename, rm } from 'node:fs/promises'
import { dirname, basename, join } from 'node:path'
import { createHash } from 'node:crypto'

export const sha256 = bytes => createHash('sha256').update(bytes).digest('hex')

// Stage the complete set next to its destination (same filesystem). A failed
// download/build never touches the old set; a failed promotion rolls it back.
export async function replaceDirectory(out, generate) {
  await mkdir(dirname(out), { recursive: true })
  const work = await mkdtemp(join(dirname(out), `.${basename(out)}-stage-`))
  const staged = join(work, 'new')
  const backup = join(work, 'old')
  let moved = false
  try {
    await mkdir(staged)
    await generate(staged)
    try { await rename(out, backup); moved = true }
    catch (error) { if (error.code !== 'ENOENT') throw error }
    try { await rename(staged, out) }
    catch (error) {
      if (moved) {
        try { await rename(backup, out) }
        catch (rollbackError) {
          // Never delete the last complete set if rollback itself fails.
          throw new AggregateError([error, rollbackError], `Restore the retained backup at ${backup}`)
        }
        moved = false
      }
      throw error
    }
    moved = false
  } finally { if (!moved) await rm(work, { recursive: true, force: true }) }
}
