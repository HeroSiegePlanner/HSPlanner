import { describe, expect, it } from 'vitest'
import { hitsPerCast } from './hitModel'
import { skills } from '@data'

const blazingTrail = {
  object: 'Pyromancer_Blazing_Trail_Create',
  tickFrequency: 0.5,
  lifetime: 2.5,
}

describe('hitsPerCast', () => {
  it('counts one hit plus one per full tick of the object lifetime', () => {
    expect(hitsPerCast(blazingTrail)).toBe(6)
  })

  it('turns Skill Duration into extra ticks', () => {
    expect(hitsPerCast(blazingTrail, 50)).toBe(8)
  })

  it('returns null when the extraction never resolved a lifetime', () => {
    expect(
      hitsPerCast({ object: 'Pyromancer_Volcano_Create', tickFrequency: 0.5 }),
    ).toBeNull()
  })

  it('returns null without a hit model', () => {
    expect(hitsPerCast(undefined)).toBeNull()
  })
})

describe('hitModel data', () => {
  it('every hitModel names an object and a positive tick frequency', () => {
    const withModel = skills.filter((s) => s.hitModel)
    expect(withModel.length).toBeGreaterThan(0)
    for (const s of withModel) {
      const model = s.hitModel!
      expect(model.object, s.id).toMatch(/^[A-Za-z][A-Za-z0-9_]*$/)
      expect(model.tickFrequency, s.id).toBeGreaterThan(0)
      if (model.lifetime !== undefined) {
        expect(model.lifetime, s.id).toBeGreaterThan(0)
      }
    }
  })

  it('Blazing Trail carries the extracted 0.5 s / 2.5 s model', () => {
    const trail = skills.find((s) => s.id === 'blazing_trail')
    expect(trail?.hitModel).toEqual(blazingTrail)
  })
})
