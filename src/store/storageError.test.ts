import { describe, expect, it, vi } from 'vitest'
import { StorageWriteError } from '../utils/build/savedBuilds'
import { guardStorage } from './storageError'

describe('guardStorage', () => {
  it('returns the body result and reports no error when the body succeeds', () => {
    const reportError = vi.fn()

    const result = guardStorage(reportError, 'fallback', () => 'ok')

    expect(result).toBe('ok')
    expect(reportError).not.toHaveBeenCalled()
  })

  it('reports the message and returns the fallback when the body throws StorageWriteError', () => {
    const reportError = vi.fn()

    const result = guardStorage(reportError, false, () => {
      throw new StorageWriteError('disk full')
    })

    expect(result).toBe(false)
    expect(reportError).toHaveBeenCalledWith('disk full')
  })

  it('rethrows errors that are not StorageWriteError', () => {
    const reportError = vi.fn()

    expect(() =>
      guardStorage(reportError, null, () => {
        throw new TypeError('a real bug')
      }),
    ).toThrow(TypeError)
    expect(reportError).not.toHaveBeenCalled()
  })
})
