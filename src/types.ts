import type { Option, Result } from 'ts-results'

interface GateTarget {
  readonly owner: string
  readonly repo: string
}

export type FailReason = 'not_authenticated' | 'not_starred'

export interface FailContext extends GateTarget {
  readonly login: Option<string>
  readonly reason: FailReason
}

export interface SuccessContext extends GateTarget {
  readonly login: string
  readonly starredAt?: string
  readonly cached: boolean
}

export type FailAction =
  | { readonly type: 'block'; readonly message?: string }
  | { readonly type: 'warn'; readonly message?: string }
  | { readonly type: 'fn'; readonly run: (ctx: FailContext) => void | Promise<void> }

export const Block = (message?: string): FailAction => ({ type: 'block', message })
export const Warn = (message: string): FailAction => ({ type: 'warn', message })
export const Fn = (run: (ctx: FailContext) => void | Promise<void>): FailAction => ({ type: 'fn', run })

export interface StaronOptions extends GateTarget {
  readonly clientId: string
  readonly onFail?: FailAction
  readonly onSuccess?: (ctx: SuccessContext) => void | Promise<void>
  readonly cacheExpireDays?: number
  readonly cacheFilePath?: string
  readonly skipStarForOwner?: boolean
}

export interface CacheData {
  readonly login: string
  readonly token: string
  readonly starred: boolean
  readonly checkedAt: number
}

export interface DeviceCodeResponse {
  readonly device_code: string
  readonly user_code: string
  readonly verification_uri: string
  readonly expires_in: number
  readonly interval: number
}

export interface GateFailure {
  readonly reason: FailReason
  readonly message: string
}

export type GateResult = Result<SuccessContext, GateFailure>
