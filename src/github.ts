import { Err, Ok, type Result } from 'ts-results'
import type { DeviceCodeResponse } from './types'

const deviceCodeUrl = 'https://github.com/login/device/code'
const tokenUrl = 'https://github.com/login/oauth/access_token'

export const requestDeviceCode = async (clientId: string): Promise<Result<DeviceCodeResponse, string>> => {
  const res = await fetch(deviceCodeUrl, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
    body: JSON.stringify({ client_id: clientId })
  })
  if (res.ok) return Ok((await res.json()) as DeviceCodeResponse)
  const body = await res.text()
  return Err(`device code request failed: ${res.status} ${body}`)
}

const requestToken = async (clientId: string, deviceCode: string): Promise<Result<string, string>> => {
  const res = await fetch(tokenUrl, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
    body: JSON.stringify({
      client_id: clientId,
      device_code: deviceCode,
      grant_type: 'urn:ietf:params:oauth:grant-type:device_code'
    })
  })
  const data = (await res.json()) as { access_token?: string; error?: string }
  return data.access_token ? Ok(data.access_token) : Err(data.error ?? 'unknown error')
}

export const pollForToken = async (
  clientId: string,
  deviceCode: string,
  interval: number,
  expiresAt: number
): Promise<Result<string, string>> =>
  Date.now() > expiresAt
    ? Err('device flow expired')
    : await requestToken(clientId, deviceCode).then((res) =>
        res.ok || res.val !== 'authorization_pending'
          ? res
          : new Promise<Result<string, string>>((resolve) =>
              setTimeout(() => resolve(pollForToken(clientId, deviceCode, interval, expiresAt)), interval * 1000)
            )
      )

export const fetchLogin = async (token: string): Promise<Result<string, string>> => {
  const res = await fetch('https://api.github.com/user', { headers: { Authorization: `Bearer ${token}` } })
  return res.ok ? Ok(((await res.json()) as { login: string }).login) : Err(`failed to fetch user: ${res.status}`)
}

export const checkStar = async (token: string, owner: string, repo: string): Promise<Result<boolean, string>> => {
  const res = await fetch(`https://api.github.com/user/starred/${owner}/${repo}`, {
    headers: { Authorization: `Bearer ${token}` }
  })
  return res.status === 204 || res.status === 404 ? Ok(res.status === 204) : Err(`unexpected status ${res.status}`)
}
