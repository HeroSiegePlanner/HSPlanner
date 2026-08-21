import { describe, expect, it } from 'vitest'
import { customImplicitLine, insertImplicitLine } from './itemTextInsert'

const ITEM = `Rarity: ANGELIC
Blaster
Gun
--------
Stars: 0
--------
Implicit:
+3 to All Skills
+25% Increased Sentry Duration
--------
Affixes:
--------
Sockets: N-N`

describe('insertImplicitLine', () => {
  it('appends under the last implicit line and reports its offset', () => {
    const { text, offset } = insertImplicitLine(ITEM, '+1% Increased Strength [custom]')
    expect(text.split('\n').slice(6, 11)).toEqual([
      'Implicit:',
      '+3 to All Skills',
      '+25% Increased Sentry Duration',
      '+1% Increased Strength [custom]',
      '--------',
    ])
    expect(text.slice(offset)).toMatch(/^\+1% Increased Strength \[custom]\n/)
  })

  it('creates the Implicit section before Affixes when missing', () => {
    const noImplicit = ITEM.replace(
      'Implicit:\n+3 to All Skills\n+25% Increased Sentry Duration\n--------\n',
      '',
    )
    const { text } = insertImplicitLine(noImplicit, '+1 to All Skills [custom]')
    expect(text).toContain(
      'Stars: 0\n--------\nImplicit:\n+1 to All Skills [custom]\n--------\nAffixes:',
    )
  })

  it('appends a new section at the end when neither section exists', () => {
    const { text, offset } = insertImplicitLine('Rarity: RARE\nRing', '+1 to All Skills [custom]')
    expect(text).toBe('Rarity: RARE\nRing\n--------\nImplicit:\n+1 to All Skills [custom]')
    expect(text.slice(offset)).toBe('+1 to All Skills [custom]')
  })
})

describe('customImplicitLine', () => {
  it('uses the stat format for the placeholder value', () => {
    expect(customImplicitLine({ name: 'to All Skills', format: 'flat' })).toBe(
      '+1 to All Skills [custom]',
    )
    expect(customImplicitLine({ name: 'Increased Strength', format: 'percent' })).toBe(
      '+1% Increased Strength [custom]',
    )
  })
})
