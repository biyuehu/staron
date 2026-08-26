# StarOn  [![CI](https://github.com/biyuehu/staron/actions/workflows/build.yml/badge.svg)](https://github.com/biyuehu/staron/actions/workflows/build.yml) [![npm version](https://img.shields.io/npm/v/staron.svg)](https://www.npmjs.com/package/staron) [![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](https://opensource.org/licenses/GPL-3.0) [![Bun](https://img.shields.io/badge/Bun-1.x-000000?logo=bun&logoColor=white)](https://bun.sh)

A tiny, functional GitHub star gate for CLI tools. Ask users to authenticate via GitHub Device Flow, check whether they starred your repo, and cache the result — with zero required scopes and a configurable fallback when they haven't starred.

## Why

CLI tools sometimes want to encourage users to star their GitHub repo before granting full access. StarOn handles the annoying parts: OAuth Device Flow, star lookup, local caching with expiry, and token re-validation on cache miss — so you only need to plug in a `clientId` and decide what happens on failure.

## Install

```bash
npm install staron
# Or：
bun add staron
# Or:
pnpm install staron
# Or:
yarn add staron
```

## Usage

```ts
import { gate, Block } from 'staron'

await gate({
  clientId: 'your-github-oauth-app-client-id',
  owner: 'biyuehu',
  repo: 'staron',
  onFail: Block('Please star biyuehu/staron on GitHub to continue.'),
  onSuccess: ({ cached }) => {
    if (!cached) console.log('Thanks for starring!')
  }
})
```

## How it works

1. On first run, StarOn requests a device code and prompts the user to visit `github.com/login/device` and enter it.
2. Once authorized, it fetches the user's login and checks `GET /user/starred/{owner}/{repo}` — no scopes required.
3. The result is cached locally (`~/.config/staron/<repo>.json`, `0600` permissions).
4. On subsequent runs, the cache is trusted until it expires (default 7 days) or the token turns out to be invalid, in which case StarOn re-validates or re-authenticates automatically.

## API

### `gate(options: StaronOptions): Promise<void>`

| Option | Type | Description |
| --- | --- | --- |
| `clientId` | `string` | GitHub OAuth App client ID (public, no secret needed for Device Flow). |
| `owner` | `string` | Repository owner. |
| `repo` | `string` | Repository name. |
| `onFail` | `FailAction` | What to do when the user hasn't starred. Defaults to `Block()`. |
| `onSuccess` | `(ctx: SuccessContext) => void \| Promise<void>` | Optional callback run when the check passes. |
| `cacheExpireDays` | `number` | Days before a cached star status is re-checked. Defaults to `7`. |
| `cacheFilePath` | `string` | Override the default cache location. |

### `FailAction` constructors

```ts
import { Block, Warn, Fn } from 'staron'

Block('Custom message')       // print message and exit(1)
Warn('Custom message')        // print message and continue
Fn((ctx) => { /* ... */ })    // fully custom handling
```

## Security notes

- The GitHub OAuth App `client_id` is not a secret — it's meant to be public, which is exactly what Device Flow is designed for.
- StarOn requests no scopes; the cached token can only read public information about the authenticated user.
- StarOn does not attempt to prevent users from bypassing the check by editing `node_modules`. That's an accepted tradeoff for a lightweight, single-purpose library.

## License

GPL-3.0-only
