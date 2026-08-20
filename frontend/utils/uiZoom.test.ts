import { describe, expect, it } from 'vitest'
import { autoUiZoom, isUiZoom } from './uiZoom'

describe('autoUiZoom', () => {
  it('leaves 1080p and OS-scaled displays at 100%', () => {
    expect(autoUiZoom(1920)).toBe(1)
    expect(autoUiZoom(2304)).toBe(1)
  })

  it('bumps unscaled 1440p and 2160p displays', () => {
    expect(autoUiZoom(2560)).toBe(1.25)
    expect(autoUiZoom(3840)).toBe(1.5)
  })
})

describe('isUiZoom', () => {
  it('rejects values outside the step list', () => {
    expect(isUiZoom(1.25)).toBe(true)
    expect(isUiZoom(1.3)).toBe(false)
    expect(isUiZoom('1.25')).toBe(false)
  })
})
