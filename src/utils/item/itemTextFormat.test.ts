import { describe, expect, it } from 'vitest'
import { affixes, items } from '../../data'
import type { EquippedItem, ItemBase } from '../../types'
import { parseItemText, serializeEquippedItem } from './itemTextFormat'

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
  it('parser detects custom value when user changes numeric prefix', () => {
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

    const result = parseItemText(text, baseItem.id)
    expect(result.equipped).not.toBeNull()
    expect(result.equipped!.affixes).toHaveLength(1)
    expect(result.equipped!.affixes[0].customValue).toBe(10)
  })

  it('parser detects [T<n>, custom] explicit syntax', () => {
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

    const result = parseItemText(text, baseItem.id)
    expect(result.equipped).not.toBeNull()
    expect(result.equipped!.affixes[0].customValue).toBe(25)
  })

  it('parser preserves roll (no customValue) when prefix matches computed', () => {
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

    const result = parseItemText(text, baseItem.id)
    expect(result.equipped).not.toBeNull()
    expect(result.equipped!.affixes[0].customValue).toBeUndefined()
    expect(result.equipped!.affixes[0].roll).toBe(1)
  })

  it('serialize → parse round-trip preserves customValue', () => {
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
    const text = serializeEquippedItem(equipped, baseItem)
    expect(text).toContain('+42')
    expect(text).toContain('[T1, custom]')

    const parsed = parseItemText(text, baseItem.id)
    expect(parsed.equipped).not.toBeNull()
    expect(parsed.equipped!.affixes[0].customValue).toBe(42)
  })

  it('parser does not collapse value-less affixes to empty string in fallback', () => {
    const baseItem = findItemWithAffixSlot()
    if (!baseItem) return
    const allSkillsAffix = findToAllSkillsT1Affix()
    if (!allSkillsAffix) return

    // Numeric-prefix replacement should match the *correct* affix by stat text,
    // not collide with any unrelated affix whose description has no leading value.
    const text = `Rarity: ${baseItem.rarity.toUpperCase()}
${baseItem.name}
${baseItem.baseType}
--------
Stars: 0
--------
Affixes:
+99 to All Skills [T1, roll 1.00]`

    const result = parseItemText(text, baseItem.id)
    expect(result.equipped).not.toBeNull()
    expect(result.equipped!.affixes).toHaveLength(1)
    expect(result.equipped!.affixes[0]!.affixId).toBe(allSkillsAffix.id)
    expect(result.equipped!.affixes[0]!.customValue).toBe(99)
  })

  it('parser rejects roll outside [0, 1]', () => {
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

    const result = parseItemText(text, baseItem.id)
    expect(result.equipped).toBeNull()
    expect(result.errors.some((e) => e.severity === 'error')).toBe(true)
  })
})

describe('itemTextFormat — implicitOverrides support', () => {
  function findItemWithImplicit(): ItemBase | undefined {
    return items.find((it) => it.implicit && Object.keys(it.implicit).length > 0)
  }

  it('serialize includes [custom] suffix for implicit overrides', () => {
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

    const text = serializeEquippedItem(equipped, baseItem)
    expect(text).toContain('[custom]')
    expect(text).toMatch(/777/)
  })

  it('parser leaves untouched [min-max] implicit lines as base (no override)', () => {
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

    const text = serializeEquippedItem(equipped, baseItem)
    const parsed = parseItemText(text, baseItem.id)
    expect(parsed.equipped).not.toBeNull()
    expect(parsed.equipped!.implicitOverrides).toBeUndefined()
  })

  it('parser accepts a brand new implicit not present on base.implicit', () => {
    // Lucifer's Crown style: pick any item, then inject a stat key that the
    // base item does NOT have. Should land in implicitOverrides as a new entry.
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

    const result = parseItemText(text, baseItem.id)
    expect(result.equipped).not.toBeNull()
    expect(result.equipped!.implicitOverrides).toBeDefined()
    expect(result.equipped!.implicitOverrides![newStatKey]).toBe(50)
  })

  it('serialize → parse round-trip preserves implicit override', () => {
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

    const text = serializeEquippedItem(equipped, baseItem)
    const parsed = parseItemText(text, baseItem.id)
    expect(parsed.equipped).not.toBeNull()
    expect(parsed.equipped!.implicitOverrides).toBeDefined()
    expect(parsed.equipped!.implicitOverrides![firstKey]).toBe(555)
  })

})
