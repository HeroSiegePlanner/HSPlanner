import { describe, expect, it } from 'vitest'
import { items } from '../../data'
import type { EquippedItem, Inventory } from '../../types'
import {
  mercAuraKey,
  mercGrantedAuras,
  mercGrantedSkillRanks,
} from './mercStats'

const auraItem = items.find((i) =>
  (i.uniqueEffects ?? []).some(
    (e) => /^Grant Aura:/i.test(e) && i.skillBonuses,
  ),
)

function equip(baseId: string): EquippedItem {
  return {
    baseId,
    affixes: [],
    socketCount: 0,
    socketed: [],
    socketTypes: [],
    stars: 0,
    forgedMods: [],
  }
}

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

  it('ignores items without granted auras', () => {
    const plain = items.find(
      (i) => !(i.uniqueEffects ?? []).some((e) => /^Grant Aura:/i.test(e)),
    )
    const inv: Inventory = { helmet: equip(plain!.id) }
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
