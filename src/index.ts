import { Err, None, Ok, Some } from 'ts-results'
import { defaultCachePath, isCacheFresh, readCache, writeCache } from './cache'
import { checkStar, fetchLogin, pollForToken, requestDeviceCode } from './github'
import type { FailContext, GateResult, StaronOptions, SuccessContext } from './types'
import { Block } from './types'

const runFailAction = async (options: StaronOptions, ctx: FailContext): Promise<void> => {
  const action = options.onFail ?? Block()
  if (action.type === 'fn') return await action.run(ctx)
  const message = action.message ?? `Please star ${ctx.owner}/${ctx.repo} on GitHub to use this tool.`
  console.error(message)
  if (action.type === 'block') process.exit(1)
}

const authenticate = async (options: StaronOptions): Promise<GateResult> => {
  const deviceRes = await requestDeviceCode(options.clientId)
  if (deviceRes.err) return Err(deviceRes.val)
  const { device_code, user_code, verification_uri, expires_in, interval } = deviceRes.val
  console.log(`Please visit ${verification_uri} and enter code: ${user_code}`)
  const tokenRes = await pollForToken(options.clientId, device_code, interval, Date.now() + expires_in * 1000)
  if (tokenRes.err) return Err(tokenRes.val)
  const loginRes = await fetchLogin(tokenRes.val)
  if (loginRes.err) return Err(loginRes.val)
  const starRes = await checkStar(tokenRes.val, options.owner, options.repo)
  if (starRes.err) return Err(starRes.val)
  const cachePath = options.cacheFilePath ?? defaultCachePath(options.repo)
  writeCache(cachePath, { login: loginRes.val, token: tokenRes.val, starred: starRes.val, checkedAt: Date.now() })
  return starRes.val
    ? Ok<SuccessContext>({ login: loginRes.val, cached: false, owner: options.owner, repo: options.repo })
    : Err('not starred')
}

export const gate = async (options: StaronOptions): Promise<void> => {
  const cachePath = options.cacheFilePath ?? defaultCachePath(options.repo)
  const expireDays = options.cacheExpireDays ?? 7
  const cacheRes = readCache(cachePath)

  const result: GateResult =
    cacheRes.ok && cacheRes.val.starred && isCacheFresh(cacheRes.val.checkedAt, expireDays)
      ? Ok<SuccessContext>({ login: cacheRes.val.login, cached: true, owner: options.owner, repo: options.repo })
      : cacheRes.ok
        ? await (async () => {
            const starRes = await checkStar(cacheRes.val.token, options.owner, options.repo)
            if (starRes.err) return authenticate(options)
            writeCache(cachePath, { ...cacheRes.val, starred: starRes.val, checkedAt: Date.now() })
            return starRes.val
              ? Ok<SuccessContext>({
                  login: cacheRes.val.login,
                  cached: false,
                  owner: options.owner,
                  repo: options.repo
                })
              : Err('not starred')
          })()
        : await authenticate(options)

  if (result.ok) return await options.onSuccess?.(result.val)
  await runFailAction(options, {
    login: cacheRes.ok ? Some(cacheRes.val.login) : None,
    owner: options.owner,
    repo: options.repo
  })
}

export { Block, type FailAction, type FailContext, Fn, type StaronOptions, type SuccessContext, Warn } from './types'
