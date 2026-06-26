import { describe, expect, test } from 'vitest'
import { groupGearSlots } from './slotGroups'

const slot = (key: string) => ({ key, name: key })

describe('groupGearSlots', () => {
  test('splits slots into gear, potions and relics preserving order', () => {
    const result = groupGearSlots([
      slot('helmet'),
      slot('weapon'),
      slot('potion_1'),
      slot('relic_1'),
      slot('potion_2'),
      slot('ring_1'),
    ])
    expect(result.gear.map((s) => s.key)).toEqual(['helmet', 'weapon', 'ring_1'])
    expect(result.potions.map((s) => s.key)).toEqual(['potion_1', 'potion_2'])
    expect(result.relics.map((s) => s.key)).toEqual(['relic_1'])
  })

  test('excludes charm slots', () => {
    const result = groupGearSlots([slot('charm_1'), slot('belt')])
    expect(result.gear.map((s) => s.key)).toEqual(['belt'])
    expect(result.potions).toEqual([])
    expect(result.relics).toEqual([])
  })

  test('returns empty groups for empty input', () => {
    expect(groupGearSlots([])).toEqual({ gear: [], potions: [], relics: [] })
  })

  test('does not mutate the input array', () => {
    const input = [slot('potion_1'), slot('helmet')]
    const copy = [...input]
    groupGearSlots(input)
    expect(input).toEqual(copy)
  })
})
