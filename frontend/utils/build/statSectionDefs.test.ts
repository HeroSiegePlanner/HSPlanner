import { describe, expect, it } from 'vitest'
import {
  DEFENSE_KEYS,
  OFFENSE_KEYS,
  RESISTANCES,
  attributeRowTone,
  defenseRowTone,
  effectiveStatValue,
  offenseRowTone,
  resistanceRowTone,
} from './statSectionDefs'

describe('statSectionDefs', () => {
  it('effectiveStatValue prefers the combined map over the base map', () => {
    expect(effectiveStatValue({ life: 10 }, { life: 20 }, 'life')).toBe(20)
    expect(effectiveStatValue({ life: 10 }, {}, 'life')).toBe(10)
    expect(effectiveStatValue({}, {}, 'life')).toBe(0)
  })

  it('tags gold-highlighted offense keys and leaves the rest untoned', () => {
    expect(OFFENSE_KEYS).toContain('enhanced_damage')
    expect(offenseRowTone('enhanced_damage')).toBe('gold')
    expect(offenseRowTone('attack_damage')).toBeUndefined()
  })

  it('tags life gold and mana-related keys blue in defense', () => {
    expect(DEFENSE_KEYS).toContain('life')
    expect(defenseRowTone('life')).toBe('gold')
    expect(defenseRowTone('mana')).toBe('blue')
    expect(defenseRowTone('block_chance')).toBeUndefined()
  })

  it('maps each resistance to its established tone', () => {
    const fire = RESISTANCES.find((r) => r.key === 'fire_resistance')!
    expect(resistanceRowTone(fire.className)).toBe('red')
  })

  it('maps primary-adjacent attribute colors to tones', () => {
    expect(attributeRowTone('vitality')).toBe('red')
    expect(attributeRowTone('armor')).toBeUndefined()
  })
})
