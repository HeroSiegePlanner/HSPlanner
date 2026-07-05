import { describe, expect, it } from 'vitest'
import {
  getItemGrantedSkillByName,
  itemGrantedRankStarBonus,
  items,
} from '../../data'
import type { EquippedItem, Inventory } from '../../types'
import {
  mercAuraKey,
  mercGrantedAuras,
  mercGrantedSkillRanks,
} from './mercStats'

const auraItem = items.find((i) =>
  Object.keys(i.skillBonuses ?? {}).some(
    (name) => getItemGrantedSkillByName(name)?.aura,
  ),
)

function equip(baseId: string, stars = 0): EquippedItem {
  return {
    baseId,
    affixes: [],
    socketCount: 0,
    socketed: [],
    socketTypes: [],
    stars,
    forgedMods: [],
  }
}

describe('itemGrantedRankStarBonus', () => {
  it('mirrors the Rust itemSpecificStaircase', () => {
    // parity with Rust item_granted_skill_rank_flat_bonus tests
    expect(itemGrantedRankStarBonus(0)).toBe(0)
    expect(itemGrantedRankStarBonus(null)).toBe(0)
    expect(itemGrantedRankStarBonus(1)).toBe(0)
    expect(itemGrantedRankStarBonus(2)).toBe(1)
    expect(itemGrantedRankStarBonus(3)).toBe(1)
    expect(itemGrantedRankStarBonus(4)).toBe(2)
    expect(itemGrantedRankStarBonus(5)).toBe(3)
    expect(itemGrantedRankStarBonus(42)).toBe(0)
  })
})

describe('mercGrantedAuras', () => {
  it('returns empty for empty inventory', () => {
    expect(mercGrantedAuras({})).toEqual([])
  })

  it('parses aura name and level range from an aura-granting item', () => {
    expect(auraItem, 'expected an aura-granting item in data').toBeDefined()
    const inv: Inventory = { boots: equip(auraItem!.id) }
    const auras = mercGrantedAuras(inv)
    expect(auras.length).toBeGreaterThan(0)
    const aura = auras[0]!
    expect(aura.itemName).toBe(auraItem!.name)
    expect(aura.baseId).toBe(auraItem!.id)
    expect(auraItem!.skillBonuses).toHaveProperty(aura.name)
    expect(aura.levelMax).toBeGreaterThanOrEqual(aura.levelMin)
    expect(aura.levelMin).toBeGreaterThan(0)
  })

  it('adds the star staircase bonus to the aura level', () => {
    const base = mercGrantedAuras({ boots: equip(auraItem!.id) })[0]!
    const starred = mercGrantedAuras({ boots: equip(auraItem!.id, 5) })[0]!
    expect(starred.levelMin).toBe(base.levelMin + 3)
    expect(starred.levelMax).toBe(base.levelMax + 3)
  })

  it('ignores items without granted auras', () => {
    const plain = items.find((i) => !i.skillBonuses)
    const inv: Inventory = { helmet: equip(plain!.id) }
    expect(mercGrantedAuras(inv)).toEqual([])
  })

  it('ignores non-aura granted skills', () => {
    const nonAuraItem = items.find((i) => {
      const names = Object.keys(i.skillBonuses ?? {})
      return (
        names.length > 0 &&
        names.every((name) => {
          const granted = getItemGrantedSkillByName(name)
          return granted != null && !granted.aura
        })
      )
    })
    if (!nonAuraItem) return
    const inv: Inventory = { helmet: equip(nonAuraItem.id) }
    expect(mercGrantedAuras(inv)).toEqual([])
  })
})

describe('mercGrantedSkillRanks', () => {
  it('returns undefined without auras', () => {
    expect(mercGrantedSkillRanks({}, {})).toBeUndefined()
  })

  it('builds rank map and honors the disabled toggle', () => {
    const inv: Inventory = { boots: equip(auraItem!.id) }
    const [aura] = mercGrantedAuras(inv)
    const ranks = mercGrantedSkillRanks(inv, {})
    expect(ranks).toBeDefined()
    expect(ranks![aura!.name]).toEqual([aura!.levelMin, aura!.levelMax])

    const disabled = mercGrantedSkillRanks(inv, {
      [mercAuraKey(aura!.name)]: true,
    })
    expect(disabled).toBeUndefined()
  })
})
