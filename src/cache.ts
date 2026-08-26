import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { homedir } from 'node:os'
import { dirname, join } from 'node:path'
import { Err, Ok, type Result } from 'ts-results'
import type { CacheData } from './types'

export const defaultCachePath = (repo: string): string => join(homedir(), '.config', 'staron', `${repo}.json`)

export const readCache = (path: string): Result<CacheData, string> => {
  if (!existsSync(path)) return Err('cache not found')
  try {
    return Ok(JSON.parse(readFileSync(path, 'utf-8')) as CacheData)
  } catch {
    return Err('cache corrupted')
  }
}

export const writeCache = (path: string, data: CacheData): Result<void, string> => {
  try {
    mkdirSync(dirname(path), { recursive: true })
    writeFileSync(path, JSON.stringify(data), { mode: 0o600 })
    return Ok(undefined)
  } catch {
    return Err('failed to write cache')
  }
}

export const isCacheFresh = (checkedAt: number, expireDays: number): boolean =>
  Date.now() - checkedAt < expireDays * 86400000
