import { describe, expect, it } from 'vitest'
import { conditionalItemGrantedSkills, getItem } from '@data'

describe('Stormslayer item blessings', () => {
  it('offers Adrenaline Momentum as a toggleable blessing', () => {
    const base = getItem('mace_unholy_stormslayer')
    if (!base) throw new Error('fixture item missing from game data')
    const granted = new Set(
      Object.keys(base.skillBonuses ?? {}).map((n) => n.trim().toLowerCase()),
    )
    const blessings = conditionalItemGrantedSkills().filter((b) =>
      granted.has(b.name.trim().toLowerCase()),
    )
    expect(blessings.map((b) => b.condition)).toEqual([
      'adrenaline_momentum_buff',
    ])
  })
})
