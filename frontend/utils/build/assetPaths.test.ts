import { describe, expect, it } from 'vitest'
import { classIconPath, itemImagePath, skillIconPath, socketIconPath } from './assetPaths'

describe('assetPaths', () => {
  it('resolves a known class id to its relative icon path', () => {
    const path = classIconPath('amazon')
    expect(path).toBeDefined()
    expect(path).toMatch(/^classes\/amazon\.(png|webp|jpg|jpeg)$/)
  })

  it('returns undefined for an unknown class id', () => {
    expect(classIconPath('not-a-class')).toBeUndefined()
  })

  it('resolves a known skill to its relative icon path', () => {
    const path = skillIconPath('amazon', 'thunder_fury')
    expect(path).toBe('skills/amazon/thunder_fury.png')
  })

  it('resolves a socketable name case-insensitively to its relative icon path', () => {
    expect(socketIconPath('Ber')).toBe('socketable/Ber_spr.png')
    expect(socketIconPath('ber')).toBe('socketable/Ber_spr.png')
  })

  it('itemImagePath points into items/', () => {
    expect(itemImagePath('stormlash')).toBe('items/stormlash.png')
  })
})
