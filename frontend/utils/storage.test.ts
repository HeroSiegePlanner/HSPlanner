import { afterEach, describe, expect, it, vi } from 'vitest'
import { readStorage, writeStorage } from './storage'

afterEach(() => {
  vi.restoreAllMocks()
})

describe('writeStorage', () => {
  it('returns true and persists the value when storage is writable', () => {
    const ok = writeStorage('hsplanner.test.writeStorage', 'hello')
    expect(ok).toBe(true)
    expect(readStorage('hsplanner.test.writeStorage')).toBe('hello')
  })

  it('returns false when localStorage rejects the write (quota exceeded)', () => {
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new DOMException('quota exceeded', 'QuotaExceededError')
    })
    expect(writeStorage('hsplanner.test.writeStorage', 'hello')).toBe(false)
  })
})
