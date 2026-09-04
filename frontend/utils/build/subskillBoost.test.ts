import { describe, expect, it } from 'vitest'
import type { EquippedItem } from '../../types'
import { boostedSubskillRanks } from './subskillBoost'

const DICE = 'charm_unholy_overloaded_dice'

function charm(extra: Partial<EquippedItem> = {}): EquippedItem {
  return {
    baseId: DICE,
    affixes: [],
    socketCount: 0,
    socketed: [],
    socketTypes: [],
    ...extra,
  }
}

describe('boostedSubskillRanks', () => {
  it('raises every allocated node of the picked skill', () => {
    const ranks = { 'vengeance:higher_frequency': 3, 'vengeance:swift_blade': 1 }

    const out = boostedSubskillRanks(
      { charm_1: charm({ subskillBoostSkillId: 'vengeance' }) },
      ranks,
    )

    expect(out).toEqual({
      'vengeance:higher_frequency': 4,
      'vengeance:swift_blade': 2,
    })
  })

  it('leaves unallocated nodes and other skills alone', () => {
    const ranks = { 'vengeance:idle': 0, 'smite:core': 2 }

    const out = boostedSubskillRanks(
      { charm_1: charm({ subskillBoostSkillId: 'vengeance' }) },
      ranks,
    )

    expect(out).toEqual(ranks)
  })

  it('sums the implicit across items picking the same skill', () => {
    const out = boostedSubskillRanks(
      {
        charm_1: charm({ subskillBoostSkillId: 'vengeance' }),
        charm_2: charm({ subskillBoostSkillId: 'vengeance' }),
      },
      { 'vengeance:higher_frequency': 1 },
    )

    expect(out['vengeance:higher_frequency']).toBe(3)
  })

  it('is a no-op when no item picked a skill', () => {
    const ranks = { 'vengeance:higher_frequency': 3 }

    expect(boostedSubskillRanks({ charm_1: charm() }, ranks)).toBe(ranks)
  })
})
