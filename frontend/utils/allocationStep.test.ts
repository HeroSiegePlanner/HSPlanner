import { describe, expect, it } from 'vitest'
import { allocationStep } from './allocationStep'

const mods = (shiftKey: boolean, ctrlKey = false, metaKey = false) => ({
  shiftKey,
  ctrlKey,
  metaKey,
})

describe('allocationStep', () => {
  it('steps by one without modifiers', () => {
    expect(allocationStep(mods(false), 40)).toBe(1)
  })

  it('steps by five with shift, never past the cap', () => {
    expect(allocationStep(mods(true), 40)).toBe(5)
    expect(allocationStep(mods(true), 3)).toBe(3)
  })

  it('takes the whole cap with ctrl or cmd plus shift', () => {
    expect(allocationStep(mods(true, true), 40)).toBe(40)
    expect(allocationStep(mods(true, false, true), 40)).toBe(40)
  })
})
