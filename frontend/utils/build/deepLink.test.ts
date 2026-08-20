import { describe, expect, it, vi, beforeEach } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import { parseDeepLinkUrl, getInitialDeepLinkUrls } from './deepLink'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))
const mockedInvoke = vi.mocked(invoke)

describe('parseDeepLinkUrl', () => {
  it('extracts the id from a well-formed hsp://b/<ID> url', () => {
    expect(parseDeepLinkUrl('hsp://b/XK3FQ2')).toBe('XK3FQ2')
  })

  it('rejects the wrong scheme', () => {
    expect(parseDeepLinkUrl('https://b/XK3FQ2')).toBeNull()
  })

  it('rejects the wrong path', () => {
    expect(parseDeepLinkUrl('hsp://x/XK3FQ2')).toBeNull()
  })

  it('rejects an id with excluded letters or wrong length', () => {
    expect(parseDeepLinkUrl('hsp://b/XKILO2')).toBeNull()
    expect(parseDeepLinkUrl('hsp://b/SHORT')).toBeNull()
  })

  it('rejects a malformed url without throwing', () => {
    expect(parseDeepLinkUrl('not a url')).toBeNull()
  })
})

describe('fetchSharedBuildCode', () => {
  const fetchMock = vi.fn()
  beforeEach(() => {
    fetchMock.mockReset()
    vi.stubGlobal('fetch', fetchMock)
    vi.stubEnv('VITE_WEB_SHARE_GET_URL', 'https://get-build.example.appwrite.run')
  })

  it('returns the code on 200', async () => {
    fetchMock.mockResolvedValue({ status: 200, json: async () => ({ code: 'abc123', appVersion: '0.11.0-season-10' }) })
    const { fetchSharedBuildCode } = await import('./deepLink')
    const code = await fetchSharedBuildCode('XK3FQ2')
    expect(code).toBe('abc123')
    expect(fetchMock).toHaveBeenCalledWith('https://get-build.example.appwrite.run?id=XK3FQ2')
  })

  it('throws on 404', async () => {
    fetchMock.mockResolvedValue({ status: 404, json: async () => ({ error: 'not_found' }) })
    const { fetchSharedBuildCode, WebShareError } = await import('./deepLink')
    await expect(fetchSharedBuildCode('XK3FQ2')).rejects.toThrow(WebShareError)
  })
})

describe('getInitialDeepLinkUrls', () => {
  beforeEach(() => {
    mockedInvoke.mockReset()
  })

  it('returns the plugin-reported urls when present', async () => {
    mockedInvoke.mockResolvedValue(['hsp://b/XK3FQ2'])
    await expect(getInitialDeepLinkUrls()).resolves.toEqual(['hsp://b/XK3FQ2'])
    expect(mockedInvoke).toHaveBeenCalledWith('plugin:deep-link|get_current')
  })

  it('returns an empty array when the plugin reports no url', async () => {
    mockedInvoke.mockResolvedValue(null)
    await expect(getInitialDeepLinkUrls()).resolves.toEqual([])
  })

  it('returns an empty array instead of throwing when invoke rejects', async () => {
    mockedInvoke.mockRejectedValue(new Error('no tauri runtime'))
    await expect(getInitialDeepLinkUrls()).resolves.toEqual([])
  })
})

describe('createDeepLinkDispatcher', () => {
  const fetchMock = vi.fn()
  beforeEach(() => {
    fetchMock.mockReset()
    vi.stubGlobal('fetch', fetchMock)
    vi.stubEnv('VITE_WEB_SHARE_GET_URL', 'https://get-build.example.appwrite.run')
    fetchMock.mockResolvedValue({ status: 404, json: async () => ({ error: 'not_found' }) })
  })

  it('ignores a live event that repeats the initial batch exactly once, then an identical retry dispatches again', async () => {
    const onReady = vi.fn()
    const onError = vi.fn()
    const { createDeepLinkDispatcher } = await import('./deepLink')
    const { dispatchInitial, dispatchLive } = createDeepLinkDispatcher(onReady, onError)

    await dispatchInitial(['hsp://b/XK3FQ2'])
    expect(onError).toHaveBeenCalledTimes(1)

    await dispatchLive(['hsp://b/XK3FQ2'])
    expect(onError).toHaveBeenCalledTimes(1)

    await dispatchLive(['hsp://b/XK3FQ2'])
    expect(onError).toHaveBeenCalledTimes(2)
  })

  it('always dispatches a distinct event after the cold-start batch', async () => {
    const onReady = vi.fn()
    const onError = vi.fn()
    const { createDeepLinkDispatcher } = await import('./deepLink')
    const { dispatchInitial, dispatchLive } = createDeepLinkDispatcher(onReady, onError)

    await dispatchInitial(['hsp://b/XK3FQ2'])
    expect(onError).toHaveBeenCalledTimes(1)

    await dispatchLive(['hsp://b/7MZH4V'])
    expect(onError).toHaveBeenCalledTimes(2)
  })
})
