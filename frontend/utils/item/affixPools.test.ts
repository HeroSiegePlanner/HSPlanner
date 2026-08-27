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
      .filter(([, types]) => !types.some((t) => slots.includes(t)))
      .map(([g]) => g)
    expect(unreachable).toEqual([])
  })
})
