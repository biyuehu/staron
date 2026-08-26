import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { isCacheFresh, readCache, writeCache } from '../src/cache'

describe('cache', () => {
  let dir: string

  beforeEach(() => {
    dir = mkdtempSync(join(tmpdir(), 'staron-test-'))
  })

  afterEach(() => rmSync(dir, { recursive: true, force: true }))

  it('returns Err when cache file does not exist', () => {
    const result = readCache(join(dir, 'missing.json'))
    expect(result.err).toBe(true)
  })

  it('writes and reads back the same data', () => {
    const path = join(dir, 'cache.json')
    const data = { login: 'biyuehu', token: 'tok', starred: true, checkedAt: Date.now() }
    expect(writeCache(path, data).ok).toBe(true)
    const result = readCache(path)
    expect(result.ok).toBe(true)
    expect(result.val).toEqual(data)
  })

  it('returns Err when cache file is corrupted', () => {
    const path = join(dir, 'bad.json')
    writeCache(path, { login: 'x', token: 'x', starred: false, checkedAt: 0 })
    require('node:fs').writeFileSync(path, 'not json')
    const result = readCache(path)
    expect(result.err).toBe(true)
  })

  it('treats a recent timestamp as fresh', () => {
    expect(isCacheFresh(Date.now(), 7)).toBe(true)
  })

  it('treats an old timestamp as stale', () => {
    expect(isCacheFresh(Date.now() - 10 * 86400000, 7)).toBe(false)
  })
})
