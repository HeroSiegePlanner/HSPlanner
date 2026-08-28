import { describe, expect, it } from 'vitest'
import { affixes, items } from '@data'
import type { EquippedItem, ItemBase } from '../../types'
import {
  parseItemText,
  serializeEquippedItem,
  type AffixMathProvider,
} from './itemTextFormat'

const stubMath: AffixMathProvider = {
  async batch({ affixes: affixReqs = [], scaled = [] }) {
    const rolled = (
      a: {
        sign: '+' | '-'
        format: 'flat' | 'percent'
        valueMin: number | null
        valueMax: number | null
      },
      roll: number,
    ) => {
      if (a.valueMin === null || a.valueMax === null) return 0
      const raw =
        a.valueMin === a.valueMax
          ? a.valueMax
          : a.valueMin + (a.valueMax - a.valueMin) * roll
      const rounded = a.format === 'flat' ? Math.round(raw) : raw
      return a.sign === '-' ? -rounded : rounded
    }
    return {
      affixes: affixReqs.map((r) => {
        const a = r.affix as Parameters<typeof rolled>[0]
        return {
          value: rolled(a, r.roll ?? 0),
          rangeMin: rolled(a, 0),
          rangeMax: rolled(a, 1),
        }
      }),
      scaled: scaled.map((r) => r.value),
    }
  },
}

function findItemWithAffixSlot(): ItemBase | undefined {
  return items.find(
    (it) => it.slot === 'amulet' || it.slot === 'ring',
  )
}

function findToAllSkillsT1Affix() {
  return affixes.find(
    (a) =>
      a.statKey === 'all_skills' &&
      a.tier === 1 &&
      a.groupId === '1_to_all_skills',
  )
}

describe('itemTextFormat — customValue support', () => {
  it('parser detects custom value when user changes numeric prefix', async () => {
    const baseItem = findItemWithAffixSlot()
    expect(baseItem).toBeDefined()
    if (!baseItem) return

    const allSkillsAffix = findToAllSkillsT1Affix()
    expect(allSkillsAffix).toBeDefined()
    if (!allSkillsAffix) return

    const text = `Rarity: ${baseItem.rarity.toUpperCase()}
${baseItem.name}
${baseItem.baseType}
--------
Stars: 0
--------
Affixes:
+10 to All Skills [T1, roll 1.00]`

    const result = await parseItemText(text, baseItem.id, stubMath)
    expect(result.equipped).not.toBeNull()
    expect(result.equipped!.affixes).toHaveLength(1)
    expect(result.equipped!.affixes[0].customValue).toBe(10)
  })

  it('parser detects [T<n>, custom] explicit syntax', async () => {
    const baseItem = findItemWithAffixSlot()
    if (!baseItem) return

    const text = `Rarity: ${baseItem.rarity.toUpperCase()}
${baseItem.name}
${baseItem.baseType}
--------
Stars: 0
--------
Affixes:
+25 to All Skills [T1, custom]`

    const result = await parseItemText(text, baseItem.id, stubMath)
    expect(result.equipped).not.toBeNull()
    expect(result.equipped!.affixes[0].customValue).toBe(25)
  })

  it('parser preserves roll (no customValue) when prefix matches computed', async () => {
    const baseItem = findItemWithAffixSlot()
    if (!baseItem) return

    const text = `Rarity: ${baseItem.rarity.toUpperCase()}
${baseItem.name}
${baseItem.baseType}
--------
Stars: 0
--------
Affixes:
+1 to All Skills [T1, roll 1.00]`

    const result = await parseItemText(text, baseItem.id, stubMath)
    expect(result.equipped).not.toBeNull()
    expect(result.equipped!.affixes[0].customValue).toBeUndefined()
    expect(result.equipped!.affixes[0].roll).toBe(1)
  })

  it('serialize → parse round-trip preserves customValue', async () => {
    const baseItem = findItemWithAffixSlot()
    if (!baseItem) return
    const allSkillsAffix = findToAllSkillsT1Affix()
    if (!allSkillsAffix) return

    const equipped: EquippedItem = {
      baseId: baseItem.id,
      affixes: [
        { affixId: allSkillsAffix.id, tier: 1, roll: 1, customValue: 42 },
      ],
      socketCount: 0,
      socketed: [],
      socketTypes: [],
      stars: 0,
    }
    const text = await serializeEquippedItem(equipped, baseItem, stubMath)
    expect(text).toContain('+42')
    expect(text).toContain('[T1, custom]')

    const parsed = await parseItemText(text, baseItem.id, stubMath)
    expect(parsed.equipped).not.toBeNull()
    expect(parsed.equipped!.affixes[0].customValue).toBe(42)
  })

  it('parser does not collapse value-less affixes to empty string in fallback', async () => {
    const baseItem = findItemWithAffixSlot()
    if (!baseItem) return
    const allSkillsAffix = findToAllSkillsT1Affix()
    if (!allSkillsAffix) return

    const text = `Rarity: ${baseItem.rarity.toUpperCase()}
${baseItem.name}
${baseItem.baseType}
--------
Stars: 0
--------
Affixes:
+99 to All Skills [T1, roll 1.00]`

    const result = await parseItemText(text, baseItem.id, stubMath)
    expect(result.equipped).not.toBeNull()
    expect(result.equipped!.affixes).toHaveLength(1)
    expect(result.equipped!.affixes[0]!.affixId).toBe(allSkillsAffix.id)
    expect(result.equipped!.affixes[0]!.customValue).toBe(99)
  })

  it('parser rejects roll outside [0, 1]', async () => {
    const baseItem = findItemWithAffixSlot()
    if (!baseItem) return

    const text = `Rarity: ${baseItem.rarity.toUpperCase()}
${baseItem.name}
${baseItem.baseType}
--------
Stars: 0
--------
Affixes:
+1 to All Skills [T1, roll 1.5]`

    const result = await parseItemText(text, baseItem.id, stubMath)
    expect(result.equipped).toBeNull()
    expect(result.errors.some((e) => e.severity === 'error')).toBe(true)
  })
})

describe('itemTextFormat — implicitOverrides support', () => {
  function findItemWithImplicit(): ItemBase | undefined {
    return items.find((it) => it.implicit && Object.keys(it.implicit).length > 0)
  }

  it('serialize includes [custom] suffix for implicit overrides', async () => {
    const baseItem = findItemWithImplicit()
    expect(baseItem).toBeDefined()
    if (!baseItem) return
    const firstKey = Object.keys(baseItem.implicit!)[0]!

    const equipped: EquippedItem = {
      baseId: baseItem.id,
      affixes: [],
      socketCount: 0,
      socketed: [],
      socketTypes: [],
      stars: 0,
      implicitOverrides: { [firstKey]: 777 },
    }

    const text = await serializeEquippedItem(equipped, baseItem, stubMath)
    expect(text).toContain('[custom]')
    expect(text).toMatch(/777/)
  })

  it('parser leaves untouched [min-max] implicit lines as base (no override)', async () => {
    const baseItem = findItemWithImplicit()
    if (!baseItem) return

    const equipped: EquippedItem = {
      baseId: baseItem.id,
      affixes: [],
      socketCount: 0,
      socketed: [],
      socketTypes: [],
      stars: 0,
    }

    const text = await serializeEquippedItem(equipped, baseItem, stubMath)
    const parsed = await parseItemText(text, baseItem.id, stubMath)
    expect(parsed.equipped).not.toBeNull()
    expect(parsed.equipped!.implicitOverrides).toBeUndefined()
  })

  it('parser accepts a brand new implicit not present on base.implicit', async () => {
    const baseItem = findItemWithImplicit()
    if (!baseItem) return
    const newStatKey = 'increased_strength'
    if (baseItem.implicit && newStatKey in baseItem.implicit) return

    const text = `Rarity: ${baseItem.rarity.toUpperCase()}
${baseItem.name}
${baseItem.baseType}
--------
Stars: 0
--------
Implicit:
+50% Increased Strength [custom]`

    const result = await parseItemText(text, baseItem.id, stubMath)
    expect(result.equipped).not.toBeNull()
    expect(result.equipped!.implicitOverrides).toBeDefined()
    expect(result.equipped!.implicitOverrides![newStatKey]).toBe(50)
  })

  it('serialize → parse round-trip preserves implicit override', async () => {
    const baseItem = findItemWithImplicit()
    if (!baseItem) return
    const firstKey = Object.keys(baseItem.implicit!)[0]!

    const equipped: EquippedItem = {
      baseId: baseItem.id,
      affixes: [],
      socketCount: 0,
      socketed: [],
      socketTypes: [],
      stars: 0,
      implicitOverrides: { [firstKey]: 555 },
    }

    const text = await serializeEquippedItem(equipped, baseItem, stubMath)
    const parsed = await parseItemText(text, baseItem.id, stubMath)
    expect(parsed.equipped).not.toBeNull()
    expect(parsed.equipped!.implicitOverrides).toBeDefined()
    expect(parsed.equipped!.implicitOverrides![firstKey]).toBe(555)
  })

})

describe('itemTextFormat — removing / replacing base implicits', () => {
  const blaster = () =>
    items.find((it) => it.id === 'gun_angelic_commander_s_sentry_blaster')
  const bare = (baseId: string): EquippedItem => ({
    baseId,
    affixes: [],
    socketCount: 0,
    socketed: [],
    socketTypes: [],
    stars: 0,
  })

  it('replacing a base implicit line swaps the stat instead of appending it', async () => {
    const base = blaster()
    expect(base).toBeDefined()
    if (!base) return
    const text = await serializeEquippedItem(bare(base.id), base, stubMath)
    expect(text).toContain('+25% Increased Sentry Duration')
    const edited = text.replace(
      '+25% Increased Sentry Duration',
      '+50% Increased Strength [custom]',
    )

    const parsed = await parseItemText(edited, base.id, stubMath)
    expect(parsed.equipped).not.toBeNull()
    expect(parsed.equipped!.implicitOverrides).toEqual({
      sentry_duration: 0,
      increased_strength: 50,
    })

    const again = await serializeEquippedItem(parsed.equipped!, base, stubMath)
    expect(again).not.toContain('Sentry Duration')
    expect(again).toContain('+50% Increased Strength [custom]')
  })

  it('rejects an implicit line with an unknown stat name instead of dropping it', async () => {
    const base = blaster()
    if (!base) return
    const text = await serializeEquippedItem(bare(base.id), base, stubMath)
    const edited = text.replace(
      '+25% Increased Sentry Duration',
      '+25% Increased Sentry Durationn',
    )

    const parsed = await parseItemText(edited, base.id, stubMath)
    expect(parsed.equipped).toBeNull()
    expect(parsed.errors).toContainEqual(
      expect.objectContaining({
        severity: 'error',
        message: expect.stringContaining('Unknown implicit stat'),
      }),
    )
  })

  it('deleting a base implicit line zeroes only that stat', async () => {
    const base = blaster()
    if (!base) return
    const text = await serializeEquippedItem(bare(base.id), base, stubMath)
    const edited = text.replace('+3 to All Skills\n', '')
    expect(edited).not.toBe(text)

    const parsed = await parseItemText(edited, base.id, stubMath)
    expect(parsed.equipped!.implicitOverrides).toEqual({ all_skills: 0 })

    const again = await serializeEquippedItem(parsed.equipped!, base, stubMath)
    expect(again).not.toContain('to All Skills')
    expect(again).toContain('+25% Increased Sentry Duration')
  })
})

describe('itemTextFormat — skillBonusOverrides support', () => {
  function findItemWithRangedSkillBonus(): [ItemBase, string] | undefined {
    for (const it of items) {
      if (!it.skillBonuses) continue
      const key = Object.keys(it.skillBonuses).find((k) => {
        const v = it.skillBonuses![k]
        return Array.isArray(v) && v[0] !== v[1]
      })
      if (key) return [it, key]
    }
    return undefined
  }

  function bareEquipped(baseId: string): EquippedItem {
    return {
      baseId,
      affixes: [],
      socketCount: 0,
      socketed: [],
      socketTypes: [],
      stars: 0,
    }
  }

  it('serialize emits a Skill Bonuses section with the ranged value', async () => {
    const found = findItemWithRangedSkillBonus()
    expect(found).toBeDefined()
    if (!found) return
    const [baseItem, skill] = found

    const text = await serializeEquippedItem(bareEquipped(baseItem.id), baseItem, stubMath)
    expect(text).toContain('Skill Bonuses:')
    expect(text).toContain(`to ${skill}`)
    expect(text).toMatch(/\+\[\d+(\.\d+)?-\d+(\.\d+)?] to /)
  })

  it('serialize includes [custom] suffix for a pinned skill bonus', async () => {
    const found = findItemWithRangedSkillBonus()
    if (!found) return
    const [baseItem, skill] = found

    const equipped: EquippedItem = {
      ...bareEquipped(baseItem.id),
      skillBonusOverrides: { [skill]: 777 },
    }
    const text = await serializeEquippedItem(equipped, baseItem, stubMath)
    expect(text).toContain(`+777 to ${skill} [custom]`)
  })

  it('parser leaves untouched ranged skill bonus lines as base (no override)', async () => {
    const found = findItemWithRangedSkillBonus()
    if (!found) return
    const [baseItem] = found

    const text = await serializeEquippedItem(bareEquipped(baseItem.id), baseItem, stubMath)
    const parsed = await parseItemText(text, baseItem.id, stubMath)
    expect(parsed.equipped).not.toBeNull()
    expect(parsed.equipped!.skillBonusOverrides).toBeUndefined()
  })

  it('serialize → parse round-trip preserves a pinned skill bonus', async () => {
    const found = findItemWithRangedSkillBonus()
    if (!found) return
    const [baseItem, skill] = found

    const equipped: EquippedItem = {
      ...bareEquipped(baseItem.id),
      skillBonusOverrides: { [skill]: 15 },
    }
    const text = await serializeEquippedItem(equipped, baseItem, stubMath)
    const parsed = await parseItemText(text, baseItem.id, stubMath)
    expect(parsed.equipped).not.toBeNull()
    expect(parsed.equipped!.skillBonusOverrides).toEqual({ [skill]: 15 })
  })

  it('parser pins a plain numeric skill bonus line without [custom]', async () => {
    const found = findItemWithRangedSkillBonus()
    if (!found) return
    const [baseItem, skill] = found

    const text = `Rarity: ${baseItem.rarity.toUpperCase()}
${baseItem.name}
${baseItem.baseType}
--------
Stars: 0
--------
Skill Bonuses:
+14 to ${skill}`

    const result = await parseItemText(text, baseItem.id, stubMath)
    expect(result.equipped).not.toBeNull()
    expect(result.equipped!.skillBonusOverrides).toEqual({ [skill]: 14 })
  })

  it('parser errors on an unknown skill bonus name', async () => {
    const found = findItemWithRangedSkillBonus()
    if (!found) return
    const [baseItem] = found

    const text = `Rarity: ${baseItem.rarity.toUpperCase()}
${baseItem.name}
${baseItem.baseType}
--------
Stars: 0
--------
Skill Bonuses:
+5 to Nonexistent Skill Xyz`

    const result = await parseItemText(text, baseItem.id, stubMath)
    expect(result.equipped).toBeNull()
    expect(result.errors.some((e) => e.severity === 'error')).toBe(true)
  })

  it('parser zeroes a skill bonus deleted from a present section', async () => {
    const multi = items.find(
      (it) => it.skillBonuses && Object.keys(it.skillBonuses).length >= 2,
    )
    expect(multi).toBeDefined()
    if (!multi) return
    const keys = Object.keys(multi.skillBonuses!)
    const removed = keys[0]!

    const text = await serializeEquippedItem(bareEquipped(multi.id), multi, stubMath)
    const withoutLine = text
      .split('\n')
      .filter((l) => !(l.includes(`to ${removed}`) && !l.startsWith('Skill Bonuses')))
      .join('\n')
    const parsed = await parseItemText(withoutLine, multi.id, stubMath)
    expect(parsed.equipped).not.toBeNull()
    expect(parsed.equipped!.skillBonusOverrides?.[removed]).toBe(0)
  })

  it('parser leaves overrides alone when the whole section is absent (legacy text)', async () => {
    const found = findItemWithRangedSkillBonus()
    if (!found) return
    const [baseItem] = found

    const text = `Rarity: ${baseItem.rarity.toUpperCase()}
${baseItem.name}
${baseItem.baseType}
--------
Stars: 0`

    const result = await parseItemText(text, baseItem.id, stubMath)
    expect(result.equipped).not.toBeNull()
    expect(result.equipped!.skillBonusOverrides).toBeUndefined()
  })
})
