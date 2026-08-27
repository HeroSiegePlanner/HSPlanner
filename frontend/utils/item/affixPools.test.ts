import { describe, expect, test } from 'vitest'
import { affixes, affixPools } from '@data'
import { affixFitsPool, affixPoolLabel, affixPoolTypeFor } from './affixPools'
import type { Affix, ItemBase } from '../../types'

const base = (slot: string, baseType = 'Armor'): ItemBase =>
  ({ id: 'x', name: 'x', baseType, slot, rarity: 'common' }) as ItemBase

const affix = (groupId: string): Affix =>
  ({ id: 'a', groupId, tier: 1 }) as Affix

describe('affixPoolTypeFor', () => {
  test('maps armour slots to their game item type', () => {
    expect(affixPoolTypeFor(base('helmet'))).toBe('Helmet')
    expect(affixPoolTypeFor(base('armor'))).toBe('Chest')
    expect(affixPoolTypeFor(base('offhand', 'Shield'))).toBe('Shield')
  })

  test('strips the index off numbered slots', () => {
    expect(affixPoolTypeFor(base('ring_2'))).toBe('Ring')
    expect(affixPoolTypeFor(base('charm_7'))).toBe('Charm')
    expect(affixPoolTypeFor(base('potion_3', 'Potion'))).toBe('Flask')
  })

  test('splits weapons by base type into melee, ranged and caster', () => {
    expect(affixPoolTypeFor(base('weapon', 'Sword'))).toBe('Weapon:Melee')
    expect(affixPoolTypeFor(base('weapon', 'Rifle Gun'))).toBe('Weapon:Ranged')
    expect(affixPoolTypeFor(base('weapon', 'Spellblade'))).toBe('Weapon:Caster')
  })

  test('returns null when no pool data exists for the base', () => {
    expect(affixPoolTypeFor(base('relic_1', 'Relic'))).toBeNull()
    expect(affixPoolTypeFor(base('weapon', 'Trumpet'))).toBeNull()
    expect(affixPoolTypeFor(undefined)).toBeNull()
  })
})

describe('affixFitsPool', () => {
  test('honours the decompiled pool for a mapped group', () => {
    const exp = affix('1_increased_experience_gain')
    expect(affixFitsPool(exp, 'Helmet')).toBe(true)
    expect(affixFitsPool(exp, 'Weapon:Melee')).toBe(false)
  })

  test('leaves unmapped groups unrestricted', () => {
    expect(affixFitsPool(affix('ailment_damage_increased_by'), 'Helmet')).toBe(true)
  })

  test('filters nothing when the base has no pool', () => {
    expect(affixFitsPool(affix('1_increased_experience_gain'), null)).toBe(true)
  })
})

test('affixPoolLabel reads as a noun phrase', () => {
  expect(affixPoolLabel('Helmet')).toBe('Helmet')
  expect(affixPoolLabel('Weapon:Caster')).toBe('Caster Weapon')
})

describe('affix-pools.json', () => {
  test('every pooled group exists in affixes.json', () => {
    const groups = new Set(affixes.map((a) => a.groupId))
    const orphans = Object.keys(affixPools).filter((g) => !groups.has(g))
    expect(orphans).toEqual([])
  })

  test('every pooled group is reachable from at least one equippable slot', () => {
    const slots = [
      base('helmet'), base('armor'), base('boots'), base('gloves'), base('belt'),
      base('amulet'), base('ring_1'), base('offhand', 'Shield'), base('charm_1'),
      base('weapon', 'Sword'), base('weapon', 'Bow'), base('weapon', 'Wand'),
      base('potion_1', 'Potion'),
    ].map((b) => affixPoolTypeFor(b)!)
    const unreachable = Object.entries(affixPools)
      .filter(([, types]) => types.length > 0 && !types.some((t) => slots.includes(t)))
      .map(([g]) => g)
    expect(unreachable).toEqual([])
  })

  // Codex affixes (stat_codex_* in the Ghidra dump) carry no item types at all —
  // gear rolls its own families for the same effects (Pickpocket, Luck, …).
  test('codex-only groups roll on nothing, so no gear slot offers them', () => {
    const codexGroups = [
      '15_75_increased_codex_drop_rates',
      '1_3_chance_for_ancients_to_spawn_as_legions',
      '5_15_increased_amount_of_heroic_and_angelic_drops',
      '5_20_increased_dungeon_key_drop_rates',
      '5_20_increased_rune_drop_rates',
      'amount_of_gold_dropped_from_monsters_increased_by_5_20',
      'ancient_monster_pack_size_increased_by_25_50',
      'experience_gain_increased_by_5_20',
      'loot_amount_increased_by_1_3',
      'total_magic_find_increased_by_5_20',
      'n_a',
      'movement_phasing',
    ]
    for (const groupId of codexGroups) {
      expect(affixPools[groupId], groupId).toEqual([])
      expect(affixFitsPool(affix(groupId), 'Helmet'), groupId).toBe(false)
      expect(affixFitsPool(affix(groupId), 'Charm'), groupId).toBe(false)
    }
  })

  test('the gear families for those same effects still roll', () => {
    expect(affixFitsPool(affix('1_5_extra_gold_dropped_from_kills'), 'Belt')).toBe(true)
    expect(affixFitsPool(affix('1_increased_experience_gain'), 'Helmet')).toBe(true)
  })

  // `to_x` and `half_freeze_duration` each held two families with different pools;
  // splitting them let each half take the item types the dump records.
  test('the split families each keep their own pool', () => {
    expect(affixFitsPool(affix('to_x'), 'Chest')).toBe(true)
    expect(affixFitsPool(affix('to_x'), 'Helmet')).toBe(false)

    expect(affixFitsPool(affix('singular_skill'), 'Helmet')).toBe(true)
    expect(affixFitsPool(affix('singular_skill'), 'Weapon:Melee')).toBe(true)
    expect(affixFitsPool(affix('singular_skill'), 'Chest')).toBe(false)

    expect(affixFitsPool(affix('cannot_be_frozen'), 'Chest')).toBe(true)
    expect(affixFitsPool(affix('cannot_be_frozen'), 'Boots')).toBe(false)

    // Tundra carries no item types anywhere in the dump.
    expect(affixFitsPool(affix('half_freeze_duration'), 'Chest')).toBe(false)
  })
})
