import { describe, expect, it } from 'vitest'
import { Block, Fn, Warn } from '../src/types'

describe('FailAction constructors', () => {
  it('Block produces a block action with optional message', () => {
    expect(Block('stop')).toEqual({ type: 'block', message: 'stop' })
    expect(Block()).toEqual({ type: 'block', message: undefined })
  })

  it('Warn produces a warn action with optional message', () => {
    expect(Warn('careful')).toEqual({ type: 'warn', message: 'careful' })
  })

  it('Fn produces a fn action wrapping the given callback', () => {
    const run = () => {}
    expect(Fn(run)).toEqual({ type: 'fn', run })
  })
})
