import { describe, expect, it } from 'vitest'
import { affixes } from './index'

const PIERCE_ELEMENTS = ['cold', 'fire', 'poison', 'lightning', 'arcane'] as const

describe('enemy resistance pierce affixes', () => {
  it('maps every to_enemy_<element>_resistance family onto ignore_<element>_res with a positive engine sign', () => {
    for (const element of PIERCE_ELEMENTS) {
      const family = affixes.filter((a) => a.groupId === `to_enemy_${element}_resistance`)
      expect(family, element).toHaveLength(5)
      for (const affix of family) {
        expect(affix, affix.id).toMatchObject({ statKey: `ignore_${element}_res`, sign: '+' })
        expect(affix.description, affix.id).toMatch(/^-/)
      }
    }
  })
})
