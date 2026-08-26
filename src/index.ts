import { Err, None, Ok, Some } from 'ts-results'
import { defaultCachePath, isCacheFresh, readCache, writeCache } from './cache'
import { checkStar, fetchLogin, pollForToken, requestDeviceCode } from './github'
import type { FailContext, GateFailure, GateResult, StaronOptions, SuccessContext } from './types'
import { Block } from './types'

const defaultMessage = (ctx: FailContext): string =>
  ctx.reason === 'not_authenticated'
    ? 'GitHub authentication failed or was not completed. Please try again.'
    : `Please star ${ctx.owner}/${ctx.repo} on GitHub to use this tool.`

const runFailAction = async (options: StaronOptions, ctx: FailContext): Promise<void> => {
  const action = options.onFail ?? Block()
  if (action.type === 'fn') return await action.run(ctx)
  console.error(action.message ?? defaultMessage(ctx))
  if (action.type === 'block') process.exit(1)
}

const authFailure = (message: string): GateFailure => ({ reason: 'not_authenticated', message })

const authenticate = async (options: StaronOptions): Promise<GateResult> => {
  const deviceRes = await requestDeviceCode(options.clientId)
  if (deviceRes.err) return Err(authFailure(deviceRes.val))
  const { device_code, user_code, verification_uri, expires_in, interval } = deviceRes.val
  console.log(`Please visit ${verification_uri} and enter code: ${user_code}`)
  const tokenRes = await pollForToken(options.clientId, device_code, interval, Date.now() + expires_in * 1000)
  if (tokenRes.err) return Err(authFailure(tokenRes.val))
  const loginRes = await fetchLogin(tokenRes.val)
  if (loginRes.err) return Err(authFailure(loginRes.val))
  const starRes = await checkStar(tokenRes.val, options.owner, options.repo)
  if (starRes.err) return Err(authFailure(starRes.val))
  const cachePath = options.cacheFilePath ?? defaultCachePath(options.repo)
  writeCache(cachePath, { login: loginRes.val, token: tokenRes.val, starred: starRes.val, checkedAt: Date.now() })
  return starRes.val
    ? Ok<SuccessContext>({ login: loginRes.val, cached: false, owner: options.owner, repo: options.repo })
    : Err({ reason: 'not_starred', message: 'user has not starred the repository' })
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
              : Err({ reason: 'not_starred', message: 'user has not starred the repository' })
          })()
        : await authenticate(options)

  if (result.ok) return await options.onSuccess?.(result.val)
  await runFailAction(options, {
    login: cacheRes.ok ? Some(cacheRes.val.login) : None,
    owner: options.owner,
    repo: options.repo,
    reason: result.val.reason
  })
}

export {
  Block,
  type FailAction,
  type FailContext,
  type FailReason,
  Fn,
  type StaronOptions,
  type SuccessContext,
  Warn
} from './types'
